//! Wave 0 stubs for Plan 09-06 (PERSIST-04 live e2e against a real
//! Dev Tunnel agent + user-started tmux 3.4+ on the remote).
//!
//! Gated on `VECTOR_E2E_TUNNEL_ID` env var. CI mirrors `tmux-smoke`
//! pattern at `.github/workflows/ci.yml:97-113` (`continue-on-error: true`).

#[tokio::test]
#[ignore = "Wave 0 — implemented in Plan 09-06; requires VECTOR_E2E_TUNNEL_ID"]
async fn osc52_round_trip() {
    unimplemented!()
}

#[tokio::test]
#[ignore = "Wave 0 — implemented in Plan 09-06; requires VECTOR_E2E_TUNNEL_ID"]
async fn decscusr_and_mouse_modes() {
    unimplemented!()
}

#[tokio::test]
#[ignore = "Wave 0 — implemented in Plan 09-06; requires VECTOR_E2E_TUNNEL_ID"]
async fn term_xterm_256color_advertised() {
    unimplemented!()
}
