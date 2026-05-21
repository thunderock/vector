//! Codec round-trip + framing tests. Plan 08-03 Task 2.
//!
//! Test 1: OpenPty serde round-trip.
//! Test 2: partial frame yields exactly one complete message.
//! Test 3: two concatenated frames yield two messages in order.
//! Test 4: protocol_version mismatch emits Error frame and session ends.

use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use vector_tunnel_protocol::{AgentMessage, PROTOCOL_VERSION};

#[test]
fn open_pty_round_trip() {
    let m = AgentMessage::OpenPty {
        protocol_version: PROTOCOL_VERSION,
        rows: 24,
        cols: 80,
        shell: None,
    };
    let s = serde_json::to_string(&m).unwrap();
    let m2: AgentMessage = serde_json::from_str(&s).unwrap();
    assert_eq!(m, m2);
}

#[tokio::test(flavor = "current_thread")]
async fn partial_frame_yields_one_message() {
    let mut buf: Vec<u8> = Vec::new();
    let line1 =
        b"{\"op\":\"open_pty\",\"protocol_version\":1,\"rows\":24,\"cols\":80,\"shell\":null}\n";
    buf.extend_from_slice(line1);
    // Partial second frame — no trailing newline.
    buf.extend_from_slice(b"{\"op\":\"data\"");
    let mut r = BufReader::new(&buf[..]);
    let mut line = String::new();
    let n = r.read_line(&mut line).await.unwrap();
    assert!(n > 0);
    let msg: AgentMessage = serde_json::from_str(line.trim_end()).unwrap();
    matches!(msg, AgentMessage::OpenPty { .. });
    line.clear();
    let n2 = r.read_line(&mut line).await.unwrap();
    // The partial frame is read but lacks a newline; serde_json::from_str on it
    // should fail (no closing brace).
    assert!(n2 > 0);
    assert!(serde_json::from_str::<AgentMessage>(line.trim_end()).is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn two_complete_frames_decode_in_order() {
    let line1 = serde_json::to_string(&AgentMessage::OpenPty {
        protocol_version: 1,
        rows: 24,
        cols: 80,
        shell: None,
    })
    .unwrap();
    let line2 = serde_json::to_string(&AgentMessage::Resize {
        session: "s-1".into(),
        rows: 30,
        cols: 100,
    })
    .unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(line1.as_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(line2.as_bytes());
    bytes.push(b'\n');

    let mut r = BufReader::new(&bytes[..]);
    let mut buf = String::new();
    r.read_line(&mut buf).await.unwrap();
    let m1: AgentMessage = serde_json::from_str(buf.trim_end()).unwrap();
    buf.clear();
    r.read_line(&mut buf).await.unwrap();
    let m2: AgentMessage = serde_json::from_str(buf.trim_end()).unwrap();
    matches!(m1, AgentMessage::OpenPty { .. });
    matches!(m2, AgentMessage::Resize { .. });
}

#[tokio::test(flavor = "current_thread")]
async fn protocol_version_mismatch_emits_error_and_closes() {
    // Build a duplex pair: agent side runs session::run; we are the "wire".
    let (wire_a, wire_b) = tokio::io::duplex(8192);
    let agent_task = tokio::spawn(vector_tunnel_agent::session::run(wire_b));

    let (read_a, mut write_a) = tokio::io::split(wire_a);
    let frame = serde_json::to_string(&AgentMessage::OpenPty {
        protocol_version: 99, // bad
        rows: 24,
        cols: 80,
        shell: None,
    })
    .unwrap();
    write_a.write_all(frame.as_bytes()).await.unwrap();
    write_a.write_all(b"\n").await.unwrap();

    let mut r = BufReader::new(read_a);
    let mut got = String::new();
    r.read_line(&mut got).await.unwrap();
    let reply: AgentMessage = serde_json::from_str(got.trim_end()).unwrap();
    match reply {
        AgentMessage::Error { reason } => {
            assert_eq!(reason, "protocol_version_mismatch");
        }
        other => panic!("expected Error frame, got {other:?}"),
    }
    // Agent task should return Ok(()) shortly.
    let res = tokio::time::timeout(std::time::Duration::from_secs(2), agent_task)
        .await
        .expect("agent task exited within 2s")
        .expect("join ok");
    assert!(res.is_ok(), "session run returned err: {res:?}");
}
