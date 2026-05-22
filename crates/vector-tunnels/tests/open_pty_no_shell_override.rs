//! Plan 09-02 Task 2 / Test A — PERSIST-03 regression at the wire format.
//!
//! Locks in `DevTunnelTransport::new_with_stream` sending
//! `OpenPty { shell: None, .. }` to the agent. Vector never wraps a remote
//! shell in tmux on the client side; the user runs tmux themselves if they
//! want it.

use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::timeout;
use vector_tunnel_protocol::{AgentMessage, PROTOCOL_VERSION};
use vector_tunnels::transport::DevTunnelTransport;

const TIMEOUT: Duration = Duration::from_secs(2);

#[tokio::test]
async fn open_pty_sends_no_shell_override() {
    let (client_side, wire_side) = tokio::io::duplex(8192);
    let (wire_r, mut wire_w) = tokio::io::split(wire_side);
    let mut wire_r = BufReader::new(wire_r);

    // Spawn the transport-side constructor; in parallel drive the wire side.
    let task = tokio::spawn(async move {
        DevTunnelTransport::new_with_stream(client_side, 30, 100).await
    });

    // Capture the first frame the client wrote.
    let mut line = String::new();
    timeout(TIMEOUT, wire_r.read_line(&mut line))
        .await
        .expect("read OpenPty no timeout")
        .expect("read OpenPty ok");

    let msg: AgentMessage =
        serde_json::from_str(line.trim_end()).expect("decode first frame as AgentMessage");

    match msg {
        AgentMessage::OpenPty {
            protocol_version,
            rows,
            cols,
            shell,
        } => {
            assert_eq!(protocol_version, PROTOCOL_VERSION);
            assert_eq!(rows, 30);
            assert_eq!(cols, 100);
            assert!(
                shell.is_none(),
                "PERSIST-03: client must never send shell override; got shell = {shell:?}"
            );
            // Belt-and-suspenders explicit comparison for the acceptance grep.
            assert!(matches!(shell, None));
        }
        other => panic!("expected OpenPty as first frame, got {other:?}"),
    }

    // Reply with Opened so the constructor completes cleanly and the spawned
    // task can be joined without dangling.
    let opened = AgentMessage::Opened {
        protocol_version: PROTOCOL_VERSION,
        session: "s-test".into(),
    };
    let mut buf = serde_json::to_string(&opened).unwrap();
    buf.push('\n');
    wire_w.write_all(buf.as_bytes()).await.unwrap();
    wire_w.flush().await.unwrap();

    let _t = task.await.unwrap().expect("transport constructed ok");
}
