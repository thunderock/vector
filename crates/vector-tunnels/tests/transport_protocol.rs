//! Plan 08-04 Task 2: `DevTunnelTransport` JSON-protocol handshake + pump tests.
//!
//! Strategy: bridge the transport's stream end with a test-controlled "wire"
//! end via `tokio::io::duplex(8192)`. The test drives the wire end as the
//! agent side of the conversation.

use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;
use vector_mux::{PtyTransport, TransportKind};
use vector_tunnel_protocol::{AgentMessage, PROTOCOL_VERSION};
use vector_tunnels::transport::{DevTunnelTransport, TransportError};

const SESSION: &str = "s-1";
const TIMEOUT: Duration = Duration::from_secs(2);

/// Build (transport, wire-side reader, wire-side writer).
/// Spawns the OpenPty consumer + Opened replier on the wire side so the
/// transport constructor can complete.
async fn spawn_with_handshake(
    rows: u16,
    cols: u16,
) -> (
    DevTunnelTransport,
    BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    tokio::io::WriteHalf<tokio::io::DuplexStream>,
) {
    let (client_side, wire_side) = tokio::io::duplex(8192);
    let (wire_r, mut wire_w) = tokio::io::split(wire_side);
    let mut wire_r = BufReader::new(wire_r);

    // Spawn the transport-side constructor; in parallel drive the wire side.
    let task =
        tokio::spawn(
            async move { DevTunnelTransport::new_with_stream(client_side, rows, cols).await },
        );

    // Read OpenPty.
    let mut line = String::new();
    timeout(TIMEOUT, wire_r.read_line(&mut line))
        .await
        .expect("read OpenPty no timeout")
        .expect("read OpenPty ok");
    let msg: AgentMessage = serde_json::from_str(line.trim_end()).expect("decode OpenPty");
    match msg {
        AgentMessage::OpenPty {
            protocol_version,
            rows: r,
            cols: c,
            shell,
        } => {
            assert_eq!(protocol_version, PROTOCOL_VERSION);
            assert_eq!(r, rows);
            assert_eq!(c, cols);
            assert!(shell.is_none());
        }
        other => panic!("expected OpenPty, got {other:?}"),
    }

    // Reply with Opened.
    let opened = AgentMessage::Opened {
        protocol_version: PROTOCOL_VERSION,
        session: SESSION.into(),
    };
    let mut buf = serde_json::to_string(&opened).unwrap();
    buf.push('\n');
    wire_w.write_all(buf.as_bytes()).await.unwrap();
    wire_w.flush().await.unwrap();

    let transport = task.await.unwrap().expect("transport ok");
    (transport, wire_r, wire_w)
}

#[tokio::test]
async fn handshake_sends_open_pty_with_protocol_version_and_dims() {
    let (_t, _r, _w) = spawn_with_handshake(24, 80).await;
}

#[tokio::test]
async fn opened_reply_yields_reader() {
    let (mut t, _r, _w) = spawn_with_handshake(24, 80).await;
    assert!(t.take_reader().is_some(), "reader should be present");
    assert!(t.take_reader().is_none(), "second take returns None");
}

#[tokio::test]
async fn protocol_mismatch_error_surfaces_typed_variant() {
    let (client_side, wire_side) = tokio::io::duplex(8192);
    let (wire_r, mut wire_w) = tokio::io::split(wire_side);
    let mut wire_r = BufReader::new(wire_r);

    let task =
        tokio::spawn(async move { DevTunnelTransport::new_with_stream(client_side, 24, 80).await });

    // Drain OpenPty.
    let mut line = String::new();
    wire_r.read_line(&mut line).await.unwrap();

    // Reply with explicit protocol-mismatch error.
    let err = AgentMessage::Error {
        reason: "protocol_version_mismatch".into(),
    };
    let mut buf = serde_json::to_string(&err).unwrap();
    buf.push('\n');
    wire_w.write_all(buf.as_bytes()).await.unwrap();
    wire_w.flush().await.unwrap();

    let res = task.await.unwrap();
    assert!(
        matches!(res, Err(TransportError::ProtocolVersion)),
        "expected ProtocolVersion err, got {res:?}"
    );
}

#[tokio::test]
async fn write_emits_data_frame_with_session() {
    let (mut t, mut wire_r, _w) = spawn_with_handshake(24, 80).await;
    t.write(b"hello\n").await.unwrap();

    let mut line = String::new();
    timeout(TIMEOUT, wire_r.read_line(&mut line))
        .await
        .expect("no timeout")
        .unwrap();
    let msg: AgentMessage = serde_json::from_str(line.trim_end()).unwrap();
    match msg {
        AgentMessage::Data { session, bytes } => {
            assert_eq!(session, SESSION);
            assert_eq!(bytes, b"hello\n".to_vec());
        }
        other => panic!("expected Data, got {other:?}"),
    }
}

#[tokio::test]
async fn read_path_delivers_data_bytes_via_reader() {
    let (mut t, _r, mut wire_w) = spawn_with_handshake(24, 80).await;
    let mut rx = t.take_reader().expect("reader present");

    let data = AgentMessage::Data {
        session: SESSION.into(),
        bytes: b"out".to_vec(),
    };
    let mut buf = serde_json::to_string(&data).unwrap();
    buf.push('\n');
    wire_w.write_all(buf.as_bytes()).await.unwrap();
    wire_w.flush().await.unwrap();

    let got = timeout(TIMEOUT, rx.recv())
        .await
        .expect("no timeout")
        .expect("Some bytes");
    assert_eq!(got, b"out".to_vec());
}

#[tokio::test]
async fn resize_emits_resize_frame() {
    let (mut t, mut wire_r, _w) = spawn_with_handshake(24, 80).await;
    t.resize(42, 120, 0, 0).expect("resize ok");

    let mut line = String::new();
    timeout(TIMEOUT, wire_r.read_line(&mut line))
        .await
        .expect("no timeout")
        .unwrap();
    let msg: AgentMessage = serde_json::from_str(line.trim_end()).unwrap();
    match msg {
        AgentMessage::Resize {
            session,
            rows,
            cols,
        } => {
            assert_eq!(session, SESSION);
            assert_eq!(rows, 42);
            assert_eq!(cols, 120);
        }
        other => panic!("expected Resize, got {other:?}"),
    }
}

#[tokio::test]
async fn exit_frame_resolves_wait_with_code() {
    let (mut t, _r, mut wire_w) = spawn_with_handshake(24, 80).await;
    let exit = AgentMessage::Exit {
        session: SESSION.into(),
        code: 0,
    };
    let mut buf = serde_json::to_string(&exit).unwrap();
    buf.push('\n');
    wire_w.write_all(buf.as_bytes()).await.unwrap();
    wire_w.flush().await.unwrap();
    // Drop the wire writer so the read side sees EOF after Exit.
    drop(wire_w);

    let code = timeout(TIMEOUT, t.wait())
        .await
        .expect("no timeout")
        .unwrap();
    assert_eq!(code, Some(0));
}

#[tokio::test]
async fn kind_is_devtunnel() {
    let (t, _r, _w) = spawn_with_handshake(24, 80).await;
    assert_eq!(t.kind(), TransportKind::DevTunnel);
}
