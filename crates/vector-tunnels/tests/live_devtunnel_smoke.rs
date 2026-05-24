//! Plan 09-06: PERSIST-04 live e2e against a real Dev Tunnel relay +
//! USER-STARTED tmux 3.4+ on the remote.
//!
//! CONTEXT D-04/D-05: Vector does NOT detect, create, attach, share, or name
//! tmux sessions. These tests refuse to bootstrap tmux themselves — the OSC 52
//! test pre-checks `$TMUX` is set and fails fast if the user hasn't prepared
//! tmux per the USER-RUN setup section of `09-SMOKE.md`.
//!
//! Gated on both `VECTOR_E2E_TUNNEL_ID` and `VECTOR_E2E_MICROSOFT_TOKEN`. CI
//! mirrors `tmux-smoke` pattern at `.github/workflows/ci.yml`
//! (`continue-on-error: true`, runs `-- --ignored` only when secrets are set).

use std::time::Duration;

use anyhow::{anyhow, Result};
use base64::Engine as _;
use tokio::sync::mpsc;
use tokio::time::timeout;
use vector_mux::PtyTransport;
use vector_tunnels::{connect_tunnel, AuthProvider, DevTunnelsApi, TunnelRecord};

const READ_TIMEOUT: Duration = Duration::from_secs(30);
const PROMPT: &[u8] = b"READY> ";

/// Resolve the test env vars. Returns `None` (with an `eprintln!`) when either
/// is missing, so `--ignored` runs without secrets are a no-op.
fn env_or_skip(test: &str) -> Option<(String, String)> {
    let tid = std::env::var("VECTOR_E2E_TUNNEL_ID").ok()?;
    let tok = std::env::var("VECTOR_E2E_MICROSOFT_TOKEN").ok()?;
    if tid.is_empty() || tok.is_empty() {
        eprintln!("{test}: VECTOR_E2E_TUNNEL_ID or VECTOR_E2E_MICROSOFT_TOKEN unset; skipping");
        return None;
    }
    Some((tid, tok))
}

/// Find the user's vector-labelled tunnel by id.
async fn find_tunnel(
    api: &DevTunnelsApi,
    auth: &AuthProvider,
    tunnel_id: &str,
) -> Result<TunnelRecord> {
    let tunnels = api.list_tunnels(auth).await?;
    tunnels
        .into_iter()
        .find(|t| t.tunnel_id == tunnel_id)
        .ok_or_else(|| anyhow!("tunnel id {tunnel_id} not found in vector-labelled tunnels"))
}

/// Drain the reader until `marker` appears in the accumulated bytes. Times
/// out per `READ_TIMEOUT` so a hung remote shell fails fast.
async fn read_until(rx: &mut mpsc::Receiver<Vec<u8>>, marker: &[u8]) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let chunk = timeout(READ_TIMEOUT, rx.recv())
            .await
            .map_err(|_| {
                anyhow!(
                    "read_until timed out waiting for {:?}",
                    String::from_utf8_lossy(marker)
                )
            })?
            .ok_or_else(|| anyhow!("read_until: transport closed before marker"))?;
        buf.extend_from_slice(&chunk);
        if buf.windows(marker.len()).any(|w| w == marker) {
            return Ok(buf);
        }
    }
}

/// Connect to the user's tunnel, install a deterministic `READY> ` prompt,
/// and return the transport + its receiver.
async fn connect_and_login(
    api: &DevTunnelsApi,
    auth: &AuthProvider,
    tunnel: &TunnelRecord,
) -> Result<(Box<dyn PtyTransport>, mpsc::Receiver<Vec<u8>>)> {
    let mut transport = connect_tunnel(api, auth, tunnel, 24, 80).await?;
    let mut rx = transport
        .take_reader()
        .ok_or_else(|| anyhow!("transport reader already taken"))?;
    // Install deterministic prompt (single-quoted so the remote shell evals it).
    transport.write(b"PS1='READY> '\n").await?;
    let _ = read_until(&mut rx, PROMPT).await?;
    Ok((transport, rx))
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "live e2e — requires VECTOR_E2E_TUNNEL_ID + VECTOR_E2E_MICROSOFT_TOKEN"]
async fn osc52_round_trip() {
    let Some((tunnel_id, token)) = env_or_skip("osc52_round_trip") else {
        return;
    };
    let api = DevTunnelsApi::new();
    let auth = AuthProvider::Microsoft(token);
    let tunnel = find_tunnel(&api, &auth, &tunnel_id)
        .await
        .expect("find tunnel");
    let (mut transport, mut rx) = connect_and_login(&api, &auth, &tunnel)
        .await
        .expect("connect+login");

    // PRE-CHECK $TMUX — user must have started tmux themselves per 09-SMOKE.md
    // USER-RUN setup. We never issue any tmux control commands (CONTEXT D-04/D-05).
    transport
        .write(b"printf '%s' \"$TMUX\"; echo END_TMUX\n")
        .await
        .expect("probe $TMUX");
    let probe = read_until(&mut rx, b"END_TMUX")
        .await
        .expect("read $TMUX probe");
    let probe_s = String::from_utf8_lossy(&probe);
    // Extract whatever came back before END_TMUX; trim shell echoes.
    let tmux_val = probe_s
        .lines()
        .find(|l| !l.contains("printf") && !l.contains("READY>") && l.contains("END_TMUX"))
        .map(|l| l.trim_end_matches("END_TMUX").trim().to_string())
        .unwrap_or_default();
    assert!(
        !tmux_val.is_empty(),
        "tmux must be running on remote before running this smoke test — see 09-SMOKE.md setup"
    );

    // Send 200-byte payload via DCS-wrapped OSC 52 (Phase 5 D-71 chunking).
    // Pitfall 7: confirms multi-chunk reassembly through the relay + tmux.
    let payload_cmd =
        b"printf '\\eP\\e]52;c;%s\\a\\e\\\\' \"$(printf '%200s' x | tr ' ' x | base64)\"\n";
    transport.write(payload_cmd).await.expect("send OSC 52");
    let _ = read_until(&mut rx, PROMPT)
        .await
        .expect("await prompt after OSC 52");

    // Read the tmux clipboard buffer back as base64.
    transport
        .write(b"tmux show-buffer | base64\n")
        .await
        .expect("send tmux show-buffer");
    let buf_response = read_until(&mut rx, PROMPT)
        .await
        .expect("read show-buffer output");
    let response_s = String::from_utf8_lossy(&buf_response);

    let expected_payload = "x".repeat(200);
    let expected_b64 =
        base64::engine::general_purpose::STANDARD.encode(expected_payload.as_bytes());
    assert!(
        response_s.contains(&expected_b64),
        "expected base64 of 200 'x' bytes in tmux show-buffer output; got: {response_s}"
    );

    drop(transport);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "live e2e — requires VECTOR_E2E_TUNNEL_ID + VECTOR_E2E_MICROSOFT_TOKEN"]
async fn decscusr_and_mouse_modes() {
    let Some((tunnel_id, token)) = env_or_skip("decscusr_and_mouse_modes") else {
        return;
    };
    let api = DevTunnelsApi::new();
    let auth = AuthProvider::Microsoft(token);
    let tunnel = find_tunnel(&api, &auth, &tunnel_id)
        .await
        .expect("find tunnel");
    let (mut transport, mut rx) = connect_and_login(&api, &auth, &tunnel)
        .await
        .expect("connect+login");

    // DECSCUSR 3 — request bar cursor. Smoke-pass: must not error / hang.
    transport
        .write(b"printf '\\e[3 q'\n")
        .await
        .expect("send DECSCUSR");
    let _ = read_until(&mut rx, PROMPT)
        .await
        .expect("prompt after DECSCUSR");

    // Mouse 1000 + SGR 1006.
    transport
        .write(b"printf '\\e[?1000h\\e[?1006h'\n")
        .await
        .expect("send mouse 1000+1006");
    let _ = read_until(&mut rx, PROMPT)
        .await
        .expect("prompt after mouse 1000");

    // Mouse 1002 (button-event tracking).
    transport
        .write(b"printf '\\e[?1002h'\n")
        .await
        .expect("send mouse 1002");
    let _ = read_until(&mut rx, PROMPT)
        .await
        .expect("prompt after mouse 1002");

    // Mouse 1003 (any-event tracking).
    transport
        .write(b"printf '\\e[?1003h'\n")
        .await
        .expect("send mouse 1003");
    let _ = read_until(&mut rx, PROMPT)
        .await
        .expect("prompt after mouse 1003");

    // Window-size propagation check.
    transport
        .write(b"stty -a 2>&1 | head -1\n")
        .await
        .expect("send stty -a");
    let stty = read_until(&mut rx, PROMPT).await.expect("stty output");
    let s = String::from_utf8_lossy(&stty);
    assert!(
        s.contains("rows 24") || s.contains("24 rows"),
        "expected 'rows 24' in stty output (window size propagation); got: {s}"
    );
    assert!(
        s.contains("columns 80") || s.contains("80 columns"),
        "expected 'columns 80' in stty output; got: {s}"
    );

    drop(transport);
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "live e2e — requires VECTOR_E2E_TUNNEL_ID + VECTOR_E2E_MICROSOFT_TOKEN"]
async fn term_xterm_256color_advertised() {
    let Some((tunnel_id, token)) = env_or_skip("term_xterm_256color_advertised") else {
        return;
    };
    let api = DevTunnelsApi::new();
    let auth = AuthProvider::Microsoft(token);
    let tunnel = find_tunnel(&api, &auth, &tunnel_id)
        .await
        .expect("find tunnel");
    let (mut transport, mut rx) = connect_and_login(&api, &auth, &tunnel)
        .await
        .expect("connect+login");

    transport
        .write(b"printf %s \"$TERM\"; echo END_TERM\n")
        .await
        .expect("send $TERM probe");
    let probe = read_until(&mut rx, b"END_TERM").await.expect("$TERM probe");
    let s = String::from_utf8_lossy(&probe);
    // Find the line containing END_TERM but not the input echo.
    let term_val = s
        .lines()
        .find(|l| l.contains("END_TERM") && !l.contains("printf"))
        .map(|l| l.trim_end_matches("END_TERM").trim().to_string())
        .unwrap_or_default();
    assert_eq!(
        term_val, "xterm-256color",
        "expected TERM=xterm-256color (set by vector-tunnel-agent per Phase 8 contract); got {term_val:?}"
    );

    drop(transport);
}
