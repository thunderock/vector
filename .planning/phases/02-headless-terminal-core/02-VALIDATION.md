---
phase: 2
slug: headless-terminal-core
status: draft
nyquist_compliant: false
wave_0_complete: true
created: 2026-05-11
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (Rust 1.88 stable, workspace tests + unit tests) |
| **Config file** | `Cargo.toml` (workspace), per-crate `Cargo.toml` `[dev-dependencies]` |
| **Quick run command** | `cargo test -p vector-term -p vector-pty -p vector-mux --tests` |
| **Full suite command** | `cargo test --workspace --tests` |
| **Estimated runtime** | ~30s for quick (post-incremental), ~60s for full |

---

## Sampling Rate

- **After every task commit:** Run quick command (`cargo test -p <crate-touched> --tests`)
- **After every plan wave:** Run `cargo test --workspace --tests`
- **Before `/gsd:verify-work`:** Full suite + `cargo clippy --workspace -- -D warnings` + `cargo fmt --all -- --check` must be green
- **Max feedback latency:** 60s

---

## Per-Task Verification Map

> One row per task across all 5 plans. Wave 0 (plan 02-01) scaffolds files; later waves un-ignore + fill them.

| Task ID | Plan | Wave | Requirement(s) | Test Type | Automated Command | File Exists | Status |
|---------|------|------|----------------|-----------|-------------------|-------------|--------|
| 2-01-01 | 02-01 | 0 | CORE-01..06 (workspace setup) | integration | `cargo build --workspace && cargo test --workspace --tests` | ✅ (new vector-headless crate) | ⬜ pending |
| 2-01-02 | 02-01 | 0 | CORE-01..03 (API spike) | unit | `cargo build -p vector-term && grep -c "Confirmed Import Paths" .planning/phases/02-headless-terminal-core/02-01-API-SPIKE.md` | ✅ (spike doc) | ⬜ pending |
| 2-01-03 | 02-01 | 0 | CORE-01..06 + D-38 (test scaffolds) | unit/integration | `cargo test --workspace --tests` (all `#[ignore]`d, must be green) | ✅ (13 stub files) | ⬜ pending |
| 2-02-01 | 02-02 | 1 | CORE-01, CORE-06 | unit | `cargo test -p vector-term --test csi_dispatch --test osc_dispatch --test dcs_dispatch --test partial_utf8 --test alt_screen_1049 --test decstbm_scroll_region --test ed_el_erase` | ✅ W0 (un-ignore) | ⬜ pending |
| 2-02-02 | 02-02 | 1 | CORE-02, CORE-03 | unit | `cargo test -p vector-term --test sgr_truecolor --test grapheme_width --test scrollback_search` | ✅ W0 (un-ignore) | ⬜ pending |
| 2-03-01 | 02-03 | 2 | CORE-04, CORE-05 (impl) | unit | `cargo build -p vector-pty && cargo clippy -p vector-pty --all-targets -- -D warnings` | ✅ (new src files) | ⬜ pending |
| 2-03-02 | 02-03 | 2 | CORE-04, CORE-05 (tests) | integration | `cargo test -p vector-pty --test lifecycle --test term_env_advertise` | ✅ W0 (un-ignore) | ⬜ pending |
| 2-04-01 | 02-04 | 3 | D-38 (traits) | unit | `cargo build -p vector-mux` | ✅ (new transport.rs, domain.rs) | ⬜ pending |
| 2-04-02 | 02-04 | 3 | D-38 (LocalDomain + stubs + object-safety) | integration | `cargo test -p vector-mux --test trait_object_safety` | ✅ W0 (un-ignore) | ⬜ pending |
| 2-05-01 | 02-05 | 4 | CORE-01, CORE-02, CORE-04, CORE-05 (skeleton + actor pattern + stubs for render/sigwinch) | build | `cargo build -p vector-headless && cargo clippy -p vector-headless --all-targets -- -D warnings && cargo test -p vector-headless --tests` (must exit 0 — no held-mutex-across-await; stubs compile clean) | ✅ (new src files incl. stub render.rs + sigwinch.rs) | ⬜ pending |
| 2-05-02 | 02-05 | 4 | CORE-01, CORE-02, CORE-04 (real render + real sigwinch) | build + smoke | `cargo build -p vector-headless && cargo clippy -p vector-headless --all-targets -- -D warnings && timeout 5 cargo run --bin vector-headless --quiet -- --cols 80 --rows 24 < /dev/null` (exit 0) | ✅ W0 (replace stubs) | ⬜ pending |
| 2-05-03 | 02-05 | 4 | CORE-01, CORE-02, CORE-04, CORE-05 (manual smoke) | manual | manual TUI matrix (vim/tmux/htop/less +F) per Manual-Only Verifications table below | n/a (manual checkpoint) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Per RESEARCH.md §Validation Architecture, Wave 0 must scaffold these test files with `#[test] #[ignore]` stubs so later waves un-ignore them:

- [x] `crates/vector-term/tests/csi_dispatch.rs` — CORE-01 CSI cursor/erase
- [x] `crates/vector-term/tests/osc_dispatch.rs` — CORE-01 OSC title/colors
- [x] `crates/vector-term/tests/dcs_dispatch.rs` — CORE-01 DCS pass-through + CORE-06 mode flags
- [x] `crates/vector-term/tests/partial_utf8.rs` — CORE-01 split-codepoint input
- [x] `crates/vector-term/tests/alt_screen_1049.rs` — CORE-01 DECSET 1049 save/restore
- [x] `crates/vector-term/tests/decstbm_scroll_region.rs` — CORE-01 scroll region
- [x] `crates/vector-term/tests/ed_el_erase.rs` — CORE-01 erase in display/line
- [x] `crates/vector-term/tests/sgr_truecolor.rs` — CORE-02 24-bit + 256-color SGR
- [x] `crates/vector-term/tests/grapheme_width.rs` — CORE-02 emoji ZWJ + East Asian width
- [x] `crates/vector-term/tests/scrollback_search.rs` — CORE-03 10k-line regex search
- [x] `crates/vector-pty/tests/term_env_advertise.rs` — CORE-05 `TERM=xterm-256color` (PTY-layer concern)
- [x] `crates/vector-pty/tests/lifecycle.rs` — CORE-04 SIGWINCH + no-zombie + clean exit
- [x] `crates/vector-mux/tests/trait_object_safety.rs` — D-38 `Box<dyn PtyTransport>` / `Box<dyn Domain>` compile-time check
- [x] `crates/vector-headless/tests/no_tokio_main.rs` — Phase 1 architecture lint inherited
- [x] Workspace dep: add `async-trait = "0.1"` to `[workspace.dependencies]` (RESEARCH Open Q 5)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `vim` opens, edits, saves, exits cleanly inside `vector-headless` | CORE-01 (alt-screen 1049, DECSTBM), CORE-02 (SGR colors) | Nested-PTY mocking is high-cost / low-signal vs. eyes-on smoke | `cargo run --bin vector-headless` → `vim /tmp/x.txt` → type, `:wq` → confirm no visual corruption + file written |
| `tmux` attaches and splits inside `vector-headless` | CORE-01 (full CSI/OSC), CORE-04 (SIGWINCH) | Nested PTYs + parent-child mux interaction not unit-testable | `cargo run --bin vector-headless` → `tmux` → `Ctrl-b "` → confirm split renders correctly, resize parent terminal, confirm child resize |
| `htop` renders box-drawing + bar graphs | CORE-01 partial-UTF-8, CORE-02 width | Realtime repaint behavior + UTF-8 chunking depends on real PTY pacing | `cargo run --bin vector-headless` → `htop` → confirm no width drift, exit with `q` |
| `less +F` follow mode | CORE-01 alt-screen + cursor positioning | Interactive paging depends on stdin pass-through timing | `tail -f /var/log/system.log` piped into `less +F` inside `vector-headless` → confirm live updates |
| Ctrl-D / shell exit leaves no zombies | CORE-04 | `ps` snapshot after process tree teardown is OS-state, not test-state | `cargo run --bin vector-headless` → `exit` → `ps -ef \| grep -E "zsh\|bash"` → no orphaned children of dead vector-headless PID |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (11 vector-term test files + 2 vector-pty + 1 vector-mux + 1 vector-headless lint file + async-trait dep)
- [x] No watch-mode flags (`cargo watch` / `cargo test -- --nocapture --test-threads=1` only for local dev, never CI)
- [x] Feedback latency < 60s
- [ ] `nyquist_compliant: true` — pending green execution suite

**Approval:** pending execution
