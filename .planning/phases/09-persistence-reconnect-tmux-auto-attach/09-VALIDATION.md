---
phase: 9
slug: persistence-reconnect-tmux-auto-attach
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-22
---

# Phase 9 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source of truth for test types and commands: `09-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (rust workspace) |
| **Config file** | workspace `Cargo.toml` (existing) |
| **Quick run command** | `cargo test -p vector-mux -p vector-app -p vector-tunnels --lib` |
| **Full suite command** | `cargo test --workspace --all-features` |
| **Estimated runtime** | ~45–90 s (excludes `#[ignore]`d live smoke tests) |
| **Live smoke test command** | `VECTOR_E2E_TUNNEL_ID=<id> cargo test -p vector-tunnels --test live_devtunnel_smoke -- --ignored --test-threads=1` |

---

## Sampling Rate

- **After every task commit:** Run quick command (`cargo test -p <crate-being-touched> --lib`).
- **After every plan wave:** Run full suite (`cargo test --workspace --all-features`, excluding `#[ignore]`d).
- **Before `/gsd:verify-work`:** Full suite green; live smoke test executed at least once locally and report pasted into VERIFICATION.md.
- **Max feedback latency:** 90 s for quick command; full suite within 3 min.

---

## Per-Task Verification Map

> Task IDs are placeholders; planner re-numbers when PLAN.md files are produced. Each row maps a requirement to its smallest verifying test. Live smoke tests are `#[ignore]`d so the default `cargo test` run stays green without the tunnel rig.

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 9-01-01 | 01 | 1 | PERSIST-01 | unit | `cargo test -p vector-mux reconnect_one_shot_trait_signature` | ❌ W0 | ⬜ pending |
| 9-01-02 | 01 | 1 | PERSIST-01 | unit | `cargo test -p vector-mux local_domain_reconnect_returns_none` | ❌ W0 | ⬜ pending |
| 9-02-01 | 02 | 2 | PERSIST-01 | unit | `cargo test -p vector-mux devtunnel_domain_reconnect_returns_transport` | ❌ W0 | ⬜ pending |
| 9-03-01 | 03 | 2 | PERSIST-01 | integration | `cargo test -p vector-app pty_actor_enters_reconnecting_on_eof` | ❌ W0 | ⬜ pending |
| 9-03-02 | 03 | 2 | PERSIST-02 | integration | `cargo test -p vector-app pty_actor_exponential_backoff_schedule` | ❌ W0 | ⬜ pending |
| 9-03-03 | 03 | 2 | PERSIST-02 | integration | `cargo test -p vector-app pty_actor_cancels_backoff_on_pane_close` | ❌ W0 | ⬜ pending |
| 9-04-01 | 04 | 2 | PERSIST-01 | integration | `cargo test -p vector-app reconnect_drains_old_transport_before_swap` | ❌ W0 | ⬜ pending |
| 9-04-02 | 04 | 2 | PERSIST-01 | integration | `cargo test -p vector-app reconnect_zero_byte_loss_under_urandom` | ❌ W0 | ⬜ pending |
| 9-05-01 | 05 | 3 | PERSIST-01 | unit | `cargo test -p vector-app reconnect_pass_renders_status_line` | ❌ W0 | ⬜ pending |
| 9-05-02 | 05 | 3 | PERSIST-01 | integration | `cargo test -p vector-app input_locked_in_reconnecting_state` | ❌ W0 | ⬜ pending |
| 9-06-01 | 06 | 5 | PERSIST-04 | live smoke | `VECTOR_E2E_TUNNEL_ID=<id> cargo test -p vector-tunnels --test live_devtunnel_smoke osc52_round_trip -- --ignored` | ❌ W0 | ⬜ pending |
| 9-06-02 | 06 | 5 | PERSIST-04 | live smoke | `VECTOR_E2E_TUNNEL_ID=<id> cargo test -p vector-tunnels --test live_devtunnel_smoke decscusr_and_mouse_modes -- --ignored` | ❌ W0 | ⬜ pending |
| 9-06-03 | 06 | 5 | PERSIST-04 | live smoke | `VECTOR_E2E_TUNNEL_ID=<id> cargo test -p vector-tunnels --test live_devtunnel_smoke term_xterm_256color_advertised -- --ignored` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Files the planner must schedule as Wave 0 stubs (compile-only, all tests `#[ignore]` or `unimplemented!()` until the implementing wave touches them):

- [ ] `crates/vector-mux/tests/reconnect_trait.rs` — stubs for PERSIST-01 trait signature.
- [ ] `crates/vector-app/tests/pty_actor_reconnect.rs` — stubs for PERSIST-01/02 state-machine + backoff behavior.
- [ ] `crates/vector-app/tests/reconnect_byte_integrity.rs` — stubs for PERSIST-01 zero-byte-loss assertion (uses `tokio::io::duplex` fake transport pattern from `transport_protocol.rs`).
- [ ] `crates/vector-app/tests/reconnect_pass_render.rs` — stubs for PERSIST-01 status-line render + input lock.
- [ ] `crates/vector-tunnels/tests/live_devtunnel_smoke.rs` — `#[ignore]`d live smoke tests gated on `VECTOR_E2E_TUNNEL_ID` (mirror `crates/vector-app/tests/osc52_tmux.rs` gating pattern).
- [ ] No framework install — `cargo test` is workspace-wide.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Inline status bar visual placement, font, and color contrast | PERSIST-01 | Pixel-perfect wgpu output is verified visually, not asserted in unit tests | (1) `cargo run --bin vector` → open Dev Tunnels pane. (2) Force-drop the tunnel (e.g., `pkill -f vector-agent` on remote). (3) Confirm "Reconnecting to {profile}… (attempt N)" overlays the top of the pane, no scrollback loss, and input is dropped (not queued) with a one-shot toast on first dropped keystroke. |
| Lid-close → resume round-trip (real laptop hardware) | PERSIST-01, PERSIST-02 | Requires actual sleep/wake; impractical to script | Open Dev Tunnels pane → run `htop` → close lid → wait 60 s → reopen → confirm pane reconnects within backoff window and `htop` continues to update without scrollback loss. |
| User-driven tmux on remote with OSC 52 clipboard copy | PERSIST-03, PERSIST-04 | Verifies the human-facing contract (user runs `tmux` themselves) | Open Dev Tunnels pane → user runs `tmux` → inside tmux, run a clipboard-copy program (e.g., `printf '\\e]52;c;%s\\a' "$(printf hello \| base64)"`) → confirm macOS clipboard contains "hello". Live smoke test automates the byte path; this confirms the human flow. |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies (live smoke tests count as automated even when `#[ignore]`d).
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify.
- [ ] Wave 0 covers all MISSING references (5 new test files listed above).
- [ ] No watch-mode flags (`cargo test`, never `cargo watch`).
- [ ] Feedback latency < 90 s for quick command.
- [ ] `nyquist_compliant: true` set in frontmatter once planner finalizes per-task mapping.

**Approval:** pending
