//! End-to-end session tests against a real PTY. Plan 08-03 Task 2.
//!
//! Test 5 (Linux+macOS): PTY echo round-trip via `/bin/sh -c "echo HELLO"`.
//! Test 6 (Linux only): resize forwards new dimensions to the PTY master.
//! Test 7: Exit frame emitted when child shell exits.

use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use vector_tunnel_protocol::AgentMessage;

async fn read_frame<R: tokio::io::AsyncBufRead + Unpin>(r: &mut R) -> AgentMessage {
    let mut line = String::new();
    r.read_line(&mut line).await.unwrap();
    serde_json::from_str(line.trim_end()).expect("valid AgentMessage frame")
}

async fn write_frame<W: tokio::io::AsyncWrite + Unpin>(w: &mut W, m: &AgentMessage) {
    let mut s = serde_json::to_string(m).unwrap();
    s.push('\n');
    w.write_all(s.as_bytes()).await.unwrap();
    w.flush().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pty_echo_round_trip() {
    let (wire_a, wire_b) = tokio::io::duplex(64 * 1024);
    let agent = tokio::spawn(vector_tunnel_agent::session::run(wire_b));

    let (read_a, mut write_a) = tokio::io::split(wire_a);
    let mut r = BufReader::new(read_a);

    // Open PTY running /bin/sh.
    write_frame(
        &mut write_a,
        &AgentMessage::OpenPty {
            protocol_version: 1,
            rows: 24,
            cols: 80,
            shell: Some("/bin/sh".into()),
        },
    )
    .await;

    let opened = read_frame(&mut r).await;
    let session_id = match opened {
        AgentMessage::Opened { session, .. } => session,
        other => panic!("expected Opened, got {other:?}"),
    };

    // Send a command. Use `printf` to avoid trailing newline ambiguity.
    write_frame(
        &mut write_a,
        &AgentMessage::Data {
            session: session_id.clone(),
            bytes: b"printf HELLO_WORLD; exit\n".to_vec(),
        },
    )
    .await;

    // Collect output frames until we see the marker or timeout.
    let mut combined = Vec::<u8>::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        let line_fut = async {
            let mut s = String::new();
            r.read_line(&mut s).await.unwrap();
            s
        };
        let line = match tokio::time::timeout(Duration::from_millis(500), line_fut).await {
            Ok(s) => s,
            Err(_) => continue,
        };
        if line.is_empty() {
            break;
        }
        if let Ok(m) = serde_json::from_str::<AgentMessage>(line.trim_end()) {
            match m {
                AgentMessage::Data { bytes, .. } => combined.extend_from_slice(&bytes),
                AgentMessage::Exit { .. } => break,
                _ => {}
            }
        }
    }
    let _ = agent.await;
    let combined_str = String::from_utf8_lossy(&combined);
    assert!(
        combined_str.contains("HELLO_WORLD"),
        "expected HELLO_WORLD in output, got: {combined_str:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exit_frame_on_shell_exit() {
    let (wire_a, wire_b) = tokio::io::duplex(64 * 1024);
    let agent = tokio::spawn(vector_tunnel_agent::session::run(wire_b));

    let (read_a, mut write_a) = tokio::io::split(wire_a);
    let mut r = BufReader::new(read_a);

    write_frame(
        &mut write_a,
        &AgentMessage::OpenPty {
            protocol_version: 1,
            rows: 24,
            cols: 80,
            shell: Some("/bin/sh".into()),
        },
    )
    .await;
    let opened = read_frame(&mut r).await;
    let session_id = match opened {
        AgentMessage::Opened { session, .. } => session,
        other => panic!("expected Opened, got {other:?}"),
    };

    // Tell the shell to exit immediately.
    write_frame(
        &mut write_a,
        &AgentMessage::Data {
            session: session_id,
            bytes: b"exit 0\n".to_vec(),
        },
    )
    .await;

    // Drain frames until we see Exit or wire EOF.
    let mut got_exit = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        let line_fut = async {
            let mut s = String::new();
            let n = r.read_line(&mut s).await.unwrap_or(0);
            (n, s)
        };
        let (n, line) = match tokio::time::timeout(Duration::from_millis(500), line_fut).await {
            Ok(t) => t,
            Err(_) => continue,
        };
        if n == 0 {
            break; // wire EOF
        }
        if let Ok(AgentMessage::Exit { .. }) = serde_json::from_str(line.trim_end()) {
            got_exit = true;
            break;
        }
    }
    let _ = agent.await;
    assert!(got_exit, "expected Exit frame after `exit 0`");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resize_forwards_to_pty() {
    // Resize observation: send a Resize frame, then verify the shell sees new
    // dimensions via `stty size`. Linux-only ground-truth, but the assertion
    // works on macOS too in practice. Mark as cfg-gated to keep CI on Linux.
    let (wire_a, wire_b) = tokio::io::duplex(64 * 1024);
    let agent = tokio::spawn(vector_tunnel_agent::session::run(wire_b));

    let (read_a, mut write_a) = tokio::io::split(wire_a);
    let mut r = BufReader::new(read_a);

    write_frame(
        &mut write_a,
        &AgentMessage::OpenPty {
            protocol_version: 1,
            rows: 24,
            cols: 80,
            shell: Some("/bin/sh".into()),
        },
    )
    .await;
    let opened = read_frame(&mut r).await;
    let session_id = match opened {
        AgentMessage::Opened { session, .. } => session,
        other => panic!("expected Opened, got {other:?}"),
    };

    // Resize then query.
    write_frame(
        &mut write_a,
        &AgentMessage::Resize {
            session: session_id.clone(),
            rows: 42,
            cols: 120,
        },
    )
    .await;
    // Small delay so the kernel propagates SIGWINCH before we ask.
    tokio::time::sleep(Duration::from_millis(200)).await;
    write_frame(
        &mut write_a,
        &AgentMessage::Data {
            session: session_id,
            bytes: b"stty size; exit\n".to_vec(),
        },
    )
    .await;

    let mut combined = Vec::<u8>::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        let line_fut = async {
            let mut s = String::new();
            let n = r.read_line(&mut s).await.unwrap_or(0);
            (n, s)
        };
        let (n, line) = match tokio::time::timeout(Duration::from_millis(500), line_fut).await {
            Ok(t) => t,
            Err(_) => continue,
        };
        if n == 0 {
            break;
        }
        if let Ok(m) = serde_json::from_str::<AgentMessage>(line.trim_end()) {
            match m {
                AgentMessage::Data { bytes, .. } => combined.extend_from_slice(&bytes),
                AgentMessage::Exit { .. } => break,
                _ => {}
            }
        }
    }
    let _ = agent.await;
    let s = String::from_utf8_lossy(&combined);
    assert!(
        s.contains("42 120"),
        "expected `42 120` in stty size output, got: {s:?}"
    );
}
