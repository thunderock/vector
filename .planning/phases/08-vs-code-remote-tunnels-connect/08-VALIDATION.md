---
phase: 8
slug: vs-code-remote-tunnels-connect
status: draft
nyquist_compliant: false
wave_0_complete: false
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
| **Quick run command** | `cargo test -p vector-devtunnels -p vector-tunnel-agent --lib` |
| **Full suite command** | `cargo test --workspace --all-features` |
| **Estimated runtime** | ~60s quick / ~180s full |

---

## Sampling Rate

- **After every task commit:** Run quick command for the crate(s) touched
- **After every plan wave:** Run full suite command
- **Before `/gsd:verify-work`:** Full suite must be green; arch-lints green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

Planner MUST populate this table with one row per task. Each row maps a task ID to its automated verification.

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 8-01-01 | 01 | 1 | DT-01 | unit | `cargo test -p vector-secrets microsoft_token` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*Planner: replace the stub row above with the full per-task map after wave/task numbering is locked.*

---

## Wave 0 Requirements

Wave 0 establishes test scaffolding before feature waves run:

- [ ] `crates/vector-devtunnels/tests/` directory with placeholder integration test
- [ ] `crates/vector-tunnel-agent/tests/` directory with placeholder integration test
- [ ] Mock Dev Tunnels API responses fixture (`tests/fixtures/dev_tunnels_list.json`)
- [ ] Mock relay endpoint stub for client-side transport tests
- [ ] Arch-lint coverage: extend `vector-arch-tests::no_token_in_debug_or_log` to scan the two new crates

*If existing infrastructure is sufficient for a given task, document it explicitly in the per-task map rather than skipping.*

---

## Manual-Only Verifications

Some Phase 8 behaviors require a real Linux host + real GitHub/Microsoft accounts and cannot be automated in the standard test suite. These are tracked as UAT items, not skipped.

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| End-to-end agent install + first-run device flow (GitHub) | DT-01 | Requires real apt repo + browser + GitHub account | 1) `apt install vector-tunnel-agent` on Ubuntu VM; 2) run `vector-tunnel-agent`; 3) complete device flow; 4) confirm tunnel appears in `https://tunnels.api.visualstudio.com/api/v1/tunnels` |
| End-to-end agent install + first-run device flow (Microsoft) | DT-01 | Requires Microsoft account (personal + Entra ID multi-tenant) | Same as above but pick MS provider |
| Picker → connect → live shell over relay | DT-02, DT-03 | Requires real relay + agent + Mac client | Sign in on Mac → `Cmd-Shift-T` → pick tunnel → confirm prompt appears, type commands, observe echo |
| `[remote]` badge + Microsoft-blue tint on connected pane | DT-04 | Visual verification on real Vector window | Connect, confirm `[remote]` badge in tab title, confirm `#0078d4` tint stripe on tab |
| Resize a connected pane → remote `tput cols`/`rows` matches | DT-03 | Real PTY/sigwinch path | `tput cols && tput lines` in remote shell; resize Vector window; rerun; values should match |

*Reconnect-across-wifi-drop and protocol-version-mismatch handling are explicitly Phase 9 / future work; not validated here.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies recorded
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
