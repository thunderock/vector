//! Per-channel session: protocol handshake + PTY spawn + biased-select pump.
//! Mirrors `vector-ssh/src/transport.rs` biased-select pattern: resize > write > read.
//! D-14: one shell per channel; D-15: protocol_version mismatch → Error + close.

use std::time::Duration;

use anyhow::Result;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::mpsc;
use vector_tunnel_protocol::{AgentMessage, PROTOCOL_VERSION};

/// Run one Vector session over a single relay channel (D-14: one shell per channel).
// Biased-select pump is intentionally one function — mirrors vector-ssh/src/transport.rs.
#[allow(clippy::too_many_lines)]
pub async fn run<S>(stream: S) -> Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    // Step 1: handshake — first frame must be OpenPty.
    reader.read_line(&mut line).await?;
    let msg: AgentMessage = serde_json::from_str(line.trim_end())?;
    let (rows, cols, shell) = if let AgentMessage::OpenPty {
        protocol_version,
        rows,
        cols,
        shell,
    } = msg
    {
        if protocol_version != PROTOCOL_VERSION {
            let err = AgentMessage::Error {
                reason: "protocol_version_mismatch".into(),
            };
            write_frame(&mut write_half, &err).await?;
            return Ok(());
        }
        (rows, cols, shell)
    } else {
        let err = AgentMessage::Error {
            reason: "expected open_pty as first frame".into(),
        };
        write_frame(&mut write_half, &err).await?;
        return Ok(());
    };

    // Step 2: spawn PTY.
    let session_id = uuid_like_id();
    let pty = native_pty_system().openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let shell_path =
        shell.unwrap_or_else(|| std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into()));
    let mut cmd = CommandBuilder::new(shell_path);
    cmd.env("TERM", "xterm-256color");
    let mut child = pty.slave.spawn_command(cmd)?;
    // Pitfall 3: close slave or zombie shell.
    drop(pty.slave);

    let mut master_writer = pty.master.take_writer()?;
    let mut master_reader = pty.master.try_clone_reader()?;
    let master = pty.master;

    // Step 3: handshake reply.
    let opened = AgentMessage::Opened {
        protocol_version: PROTOCOL_VERSION,
        session: session_id.clone(),
    };
    write_frame(&mut write_half, &opened).await?;

    // PTY reader → mpsc (blocking read on PTY master).
    let (pty_out_tx, mut pty_out_rx) = mpsc::channel::<Vec<u8>>(64);
    tokio::task::spawn_blocking(move || {
        let mut buf = vec![0u8; 8192];
        loop {
            use std::io::Read;
            match master_reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if pty_out_tx.blocking_send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
    });

    // PTY writer → mpsc (so the select loop never blocks on PTY writes).
    let (pty_in_tx, mut pty_in_rx) = mpsc::channel::<Vec<u8>>(64);
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        while let Some(bytes) = pty_in_rx.blocking_recv() {
            if master_writer.write_all(&bytes).is_err() {
                break;
            }
            let _ = master_writer.flush();
        }
    });

    // Resize requests → channel (so the loop can prioritize them).
    let (resize_tx, mut resize_rx) = mpsc::unbounded_channel::<(u16, u16)>();

    // Step 4: bidirectional pump.
    let mut child_poll = tokio::time::interval(Duration::from_millis(100));
    child_poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut exit_code: Option<i32> = None;
    let mut wire_buf = String::new();

    loop {
        tokio::select! {
            biased;
            // Priority 1: resize — rare, must not starve.
            Some((rows, cols)) = resize_rx.recv() => {
                if let Err(e) = master.resize(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 }) {
                    tracing::warn!(error=%e, rows, cols, "pty resize failed");
                } else {
                    tracing::debug!(rows, cols, "pty resized");
                }
            }
            // Priority 2: inbound wire frames.
            r = reader.read_line(&mut wire_buf) => {
                match r {
                    Ok(0) => {
                        tracing::debug!("wire EOF — closing session");
                        break;
                    }
                    Ok(_) => {
                        let parsed: Result<AgentMessage, _> = serde_json::from_str(wire_buf.trim_end());
                        wire_buf.clear();
                        match parsed {
                            Ok(AgentMessage::Data { bytes, .. }) => {
                                if pty_in_tx.send(bytes).await.is_err() {
                                    break;
                                }
                            }
                            Ok(AgentMessage::Resize { rows, cols, .. }) => {
                                let _ = resize_tx.send((rows, cols));
                            }
                            Ok(AgentMessage::OpenPty { .. }) => {
                                let err = AgentMessage::Error { reason: "open_pty already handled".into() };
                                write_frame(&mut write_half, &err).await?;
                            }
                            Ok(AgentMessage::Exit { .. } | AgentMessage::Error { .. }) => {
                                tracing::debug!("client requested close");
                                break;
                            }
                            Ok(_) => {
                                // Opened/Unknown — ignore from client side.
                            }
                            Err(e) => {
                                tracing::warn!(error=%e, "malformed inbound frame; ignoring");
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error=%e, "wire read error — closing session");
                        break;
                    }
                }
            }
            // Priority 3: outbound PTY bytes.
            Some(bytes) = pty_out_rx.recv() => {
                let frame = AgentMessage::Data { session: session_id.clone(), bytes };
                if write_frame(&mut write_half, &frame).await.is_err() {
                    break;
                }
            }
            // Priority 4: child exit poll.
            _ = child_poll.tick() => {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        exit_code = Some(status_to_code(&status));
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!(error=%e, "child.try_wait failed");
                        break;
                    }
                }
            }
            else => break,
        }
    }

    // Drain any remaining PTY output (best-effort, short window).
    let drain_until = tokio::time::Instant::now() + Duration::from_millis(50);
    while let Ok(Some(bytes)) = tokio::time::timeout_at(drain_until, pty_out_rx.recv()).await {
        let frame = AgentMessage::Data {
            session: session_id.clone(),
            bytes,
        };
        if write_frame(&mut write_half, &frame).await.is_err() {
            break;
        }
    }

    let code = exit_code.unwrap_or_else(|| {
        // If we didn't see the child exit yet, force a kill and report -1.
        let _ = child.kill();
        -1
    });
    let exit_frame = AgentMessage::Exit {
        session: session_id,
        code,
    };
    let _ = write_frame(&mut write_half, &exit_frame).await;
    Ok(())
}

async fn write_frame<W: tokio::io::AsyncWrite + Unpin>(
    w: &mut W,
    m: &AgentMessage,
) -> std::io::Result<()> {
    let mut s = serde_json::to_string(m).map_err(std::io::Error::other)?;
    s.push('\n');
    w.write_all(s.as_bytes()).await?;
    w.flush().await?;
    Ok(())
}

fn uuid_like_id() -> String {
    format!(
        "s-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    )
}

fn status_to_code(s: &portable_pty::ExitStatus) -> i32 {
    // portable_pty 0.9 exposes `exit_code()` returning u32.
    i32::try_from(s.exit_code()).unwrap_or(-1)
}
