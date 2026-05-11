//! CORE-05: TERM=xterm-256color advertised by LocalPty to child.

use std::path::Path;
use std::time::Duration;

use tokio::runtime::Builder;
use vector_pty::{LocalPty, SpawnCommand};

async fn read_for(rx: &mut tokio::sync::mpsc::Receiver<Vec<u8>>, dur: Duration) -> Vec<u8> {
    let mut out = Vec::new();
    let deadline = tokio::time::Instant::now() + dur;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(chunk)) => out.extend_from_slice(&chunk),
            Ok(None) | Err(_) => break,
        }
    }
    out
}

#[test]
fn term_env_var_is_xterm_256color() {
    let rt = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("rt");
    rt.block_on(async {
        let mut pty = LocalPty::spawn(
            Path::new("/bin/sh"),
            SpawnCommand {
                argv: Some(vec!["/bin/sh".into(), "-c".into(), "printenv TERM".into()]),
                cwd: None,
                rows: 24,
                cols: 80,
                env: vec![],
            },
        )
        .expect("spawn");

        let mut rx = pty.take_reader().expect("take_reader");
        let out = read_for(&mut rx, Duration::from_secs(3)).await;
        let s = String::from_utf8_lossy(&out);
        assert!(
            s.contains("xterm-256color"),
            "expected TERM=xterm-256color, got: {s:?}"
        );

        let _ = pty.wait().await;
    });
}
