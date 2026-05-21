---
phase: 8
slug: vs-code-remote-tunnels-connect
status: ready
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-21
---

# Phase 8 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust workspace, std test harness) |
| **Config file** | `Cargo.toml` (workspace) / `crates/*/Cargo.toml` per-crate |
| **Quick run command** | `cargo test -p vector-tunnels -p vector-tunnel-agent -p vector-tunnel-protocol --lib --tests` |
| **Full suite command** | `make test` (= `cargo test --workspace --tests`) |
| **Lint command** | `make lint` (= `cargo fmt --all -- --check && cargo clippy --all-targets --all-features -- -D warnings`) |
| **Estimated runtime** | ~60s quick / ~180s full |

---

## Sampling Rate

- **After every task commit:** Run quick command for the crate(s) touched
- **After every plan wave:** Run full suite command (`make test`)
- **Before `/gsd:verify-work`:** `make lint` green + `make test` green + arch-lints green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement(s) | Test Type | Automated Command | File Exists | Status |
|---------|------|------|----------------|-----------|-------------------|-------------|--------|
| 08-01-01 | 01 | 1 | DT-01, DT-02, DT-03, DT-04 | unit + build + doc-check | `test -f .planning/research/spikes/dev-tunnels-decision.md && cargo build --workspace --all-targets && cargo test -p vector-tunnel-protocol --tests && cargo test -p vector-secrets microsoft_account_constant_value` | ✅ created | ⬜ pending |
| 08-01-02 | 01 | 1 | DT-01..04 (Pitfall 14 cross-cutting) | unit | `cargo test -p vector-arch-tests --tests` | ✅ pre-existing (extended) | ⬜ pending |
| 08-02-01 | 02 | 2 | DT-02 | unit + integration (wiremock) | `cargo test -p vector-tunnels --test microsoft_device_flow && cargo test -p vector-arch-tests --tests && cargo clippy -p vector-tunnels --all-targets -- -D warnings` | ✅ created (W0 stub flipped green) | ⬜ pending |
| 08-02-02 | 02 | 2 | DT-02 | unit + Keychain integration (`#[ignore]` per Phase 6 pattern) | `cargo build -p vector-tunnels --tests && cargo test -p vector-tunnels --test microsoft_token_store -- --include-ignored` | ✅ created (W0 stub flipped green) | ⬜ pending |
| 08-03-01 | 03 | 2 | DT-01, DT-03 | unit (file mode + token cache round-trip) | `cargo build -p vector-tunnel-agent && cargo test -p vector-tunnel-agent --test auth_token_cache && ./target/debug/vector-tunnel-agent --version && cargo test -p vector-arch-tests --tests` | ✅ created (Phase 8 new file) | ⬜ pending |
| 08-03-02 | 03 | 2 | DT-01, DT-03 | unit + duplex-stream protocol + real-PTY (Linux-gated) | `cargo build -p vector-tunnel-agent && cargo test -p vector-tunnel-agent --test protocol_codec && cargo test -p vector-tunnel-agent --test session_lifecycle` | ✅ W0 stub (protocol_codec) flipped green; session_lifecycle new | ⬜ pending |
| 08-04-01 | 04 | 2 | DT-02, DT-03, DT-04 | unit + wiremock integration | `cargo test -p vector-tunnels --test list_tunnels && cargo test -p vector-tunnels --lib model && cargo test -p vector-arch-tests --tests` | ✅ W0 stub flipped green; fixture from W0 | ⬜ pending |
| 08-04-02 | 04 | 2 | DT-02, DT-03, DT-04 | unit + duplex-stream protocol | `cargo build -p vector-tunnels && cargo test -p vector-tunnels --test transport_protocol && cargo test -p vector-mux --tests` | ✅ created (Phase 8 new file) | ⬜ pending |
| 08-05-01 | 05 | 3 | DT-02, DT-03, DT-04 | unit (keymap + tint + actor + menu rows) | `cargo build -p vector-app -p vector-input -p vector-render && cargo test -p vector-input keymap && cargo test -p vector-render tint_stripe && cargo test -p vector-app --tests && cargo test -p vector-arch-tests --tests` | ✅ created (Phase 8 new file: microsoft_signin_menu.rs) | ⬜ pending |
| 08-05-02 | 05 | 3 | DT-02, DT-03, DT-04 | unit (modal frame + footer + row format pure-Rust helpers; AppKit gated) | `cargo build -p vector-app && cargo test -p vector-app --test devtunnels_picker && cargo test -p vector-app --test microsoft_signin_menu` | ✅ created (Phase 8 new file: devtunnels_picker.rs) | ⬜ pending |
| 08-06-01 | 06 | 3 | DT-01 | build + file existence checks | `cargo build -p vector-tunnel-agent --release && test -x crates/vector-tunnel-agent/debian/postinst && test -x crates/vector-tunnel-agent/debian/prerm && grep -q "package.metadata.deb" crates/vector-tunnel-agent/Cargo.toml && grep -q "agent-dist" xtask/src/main.rs && test -f crates/vector-tunnel-agent/README.md` | ✅ created (Phase 8 new files) | ⬜ pending |
| 08-06-02 | 06 | 3 | DT-01 | YAML lint + file content grep | `test -f .github/workflows/agent-release.yml && grep -q "tags:.*'v\\*'" .github/workflows/agent-release.yml && grep -q "x86_64-unknown-linux-gnu" .github/workflows/agent-release.yml && grep -q "aarch64-unknown-linux-gnu" .github/workflows/agent-release.yml && grep -q "cargo deb" .github/workflows/agent-release.yml && grep -q "gh release upload" .github/workflows/agent-release.yml` | ✅ created (Phase 8 new file) | ⬜ pending |
| 08-06-03 | 06 | 3 | DT-01 | MANUAL — checkpoint:human-verify (cargo-deb install/remove on Linux box) | MANUAL — see plan task `<how-to-verify>`; tracked under "Manual-Only Verifications" below | n/a (checkpoint) | ⬜ pending |
| 08-07-01 | 07 | 4 | DT-01, DT-02, DT-03, DT-04 | file structure + grep matrix template | `test -f .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md && grep -c "### Item " .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md == 9 && grep -c "PASS / \\[ \\] FAIL" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md >= 8 && test -f .planning/research/spikes/dev-tunnels-decision.md` | ✅ created (Phase 8 new file) | ⬜ pending |
| 08-07-02 | 07 | 4 | DT-01, DT-02, DT-03, DT-04 | MANUAL — checkpoint:human-verify (9-item UAT smoke matrix) | MANUAL — user walks 08-SMOKE.md end-to-end; tracked under "Manual-Only Verifications" below | n/a (checkpoint) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 (= Plan 08-01) establishes the spike doc + test scaffolding before feature waves run:

- [x] DT-01 spike decision doc at `.planning/research/spikes/dev-tunnels-decision.md` (gates ROADMAP §Phase 8 SC#1 — landed in 08-01 Task 1 Step 0)
- [x] `crates/vector-tunnels/tests/list_tunnels.rs` with 3 `#[ignore]` integration test stubs (flipped green by 08-04 Task 1)
- [x] `crates/vector-tunnel-agent/tests/protocol_codec.rs` with 2 `#[ignore]` test stubs (flipped green by 08-03 Task 2)
- [x] Mock Dev Tunnels API fixture at `crates/vector-tunnels/tests/fixtures/dev_tunnels_list.json` (5 records, 2 with `vector-agent: true` label)
- [x] `crates/vector-tunnel-protocol/tests/messages.rs` with 4 passing serde round-trip tests
- [x] `crates/vector-secrets/tests/microsoft_account.rs` constant-value test
- [x] Arch-lint coverage: `vector-arch-tests::no_token_in_debug_or_log` scans `vector-tunnels/src` + `vector-tunnel-agent/src` + `vector-tunnel-protocol/src` and recognizes Phase 8 token identifiers (`agent_token`, `tunnel_access_token`)

All Wave 0 deliverables produced by Plan 08-01 Tasks 1+2. Downstream waves flip the `#[ignore]` stubs by implementing the corresponding behavior.

---

## Manual-Only Verifications

Some Phase 8 behaviors require a real Linux host + real GitHub/Microsoft accounts and cannot be automated in the standard test suite. These are tracked as UAT items, not skipped.

| Behavior | Requirement | Why Manual | Test Instructions | Tracked By |
|----------|-------------|------------|-------------------|------------|
| End-to-end agent install + first-run device flow (GitHub) | DT-01 | Requires real apt repo + browser + GitHub account | 1) `apt install vector-tunnel-agent` on Ubuntu VM; 2) run `vector-tunnel-agent`; 3) complete device flow; 4) confirm tunnel appears in `https://global.rel.tunnels.api.visualstudio.com/api/v1/tunnels` | 08-SMOKE.md Item 3 |
| End-to-end agent install + first-run device flow (Microsoft) | DT-01 | Requires Microsoft account (personal + Entra ID multi-tenant) | Same as above but pick MS provider | 08-SMOKE.md Item 3 (alt provider) |
| Picker → connect → live shell over relay | DT-02, DT-03 | Requires real relay + agent + Mac client | Sign in on Mac → `Cmd-Shift-T` → pick tunnel → confirm prompt appears, type commands, observe echo | 08-SMOKE.md Items 4 + 5 |
| `[remote]` badge + Microsoft-blue tint on connected pane | DT-04 | Visual verification on real Vector window | Connect, confirm `[remote]` badge in tab title, confirm `#0078d4` tint stripe on tab | 08-SMOKE.md Item 6 |
| Resize a connected pane → remote `tput cols`/`rows` matches | DT-03 | Real PTY/sigwinch path | `tput cols && tput lines` in remote shell; resize Vector window; rerun; values should match | 08-SMOKE.md Item 7 |
| Token-leak audit (Pitfall 14 — runtime log scrape) | DT-01..04 (cross-cutting) | Requires running app + signing in + capturing logs | `RUST_LOG=trace cargo run ... 2>&1 | tee /tmp/vector.log` then `grep -E 'gho_\|ghp_\|eyJ\|Bearer [A-Za-z0-9._-]{20,}'` must return zero hits | 08-SMOKE.md Item 8 |
| Sign out keeps live pane alive | DT-04 (UX promise) | Live state across sign-out | After connect, sign out of Microsoft; pane still echoes | 08-SMOKE.md Item 9 |
| `cargo-deb` local install + remove on Linux | DT-01 | Requires Linux box / Docker container | `cargo deb` → `sudo apt install ./vector-tunnel-agent_*.deb` → `--version` → `sudo apt remove vector-tunnel-agent` | 08-06 Task 3 (checkpoint) |

*Reconnect-across-wifi-drop and protocol-version-mismatch handling are explicitly Phase 9 / future work; not validated here.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies recorded
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (checkpoints 08-06-03 and 08-07-02 follow automated verifications 08-06-02 and 08-07-01)
- [x] Wave 0 covers all MISSING references (Plan 08-01 lands every prerequisite stub + fixture)
- [x] No watch-mode flags
- [x] Feedback latency < 60s (per-crate `cargo test -p ...` invocations dominate)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** planner-side sign-off complete (this revision). Pending executor + user UAT per checkpoints.
