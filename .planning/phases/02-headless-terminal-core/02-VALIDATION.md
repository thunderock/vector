---
phase: 2
slug: headless-terminal-core
status: draft
nyquist_compliant: false
wave_0_complete: false
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

> Populated by gsd-planner from RESEARCH.md §Validation Architecture. Each row links a plan task to its automated harness or marks it as Manual-Only with justification.

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 2-XX-XX | TBD | TBD | CORE-01..06 | unit / integration | `cargo test -p <crate> --test <file>` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Per RESEARCH.md §Validation Architecture, Wave 0 must scaffold these test files with `#[test] #[ignore]` stubs so later waves un-ignore them:

- [ ] `crates/vector-term/tests/csi_dispatch.rs` — CORE-01 CSI cursor/erase
- [ ] `crates/vector-term/tests/osc_dispatch.rs` — CORE-01 OSC title/colors
- [ ] `crates/vector-term/tests/dcs_dispatch.rs` — CORE-01 DCS pass-through
- [ ] `crates/vector-term/tests/partial_utf8.rs` — CORE-01 split-codepoint input
- [ ] `crates/vector-term/tests/alt_screen_1049.rs` — CORE-01 DECSET 1049 save/restore
- [ ] `crates/vector-term/tests/decstbm_scroll_region.rs` — CORE-01 scroll region
- [ ] `crates/vector-term/tests/ed_el_erase.rs` — CORE-01 erase in display/line
- [ ] `crates/vector-term/tests/sgr_truecolor.rs` — CORE-02 24-bit + 256-color SGR
- [ ] `crates/vector-term/tests/grapheme_width.rs` — CORE-02 emoji ZWJ + East Asian width
- [ ] `crates/vector-term/tests/scrollback_search.rs` — CORE-03 10k-line regex search
- [ ] `crates/vector-term/tests/term_env_advertise.rs` — CORE-05 `TERM=xterm-256color`
- [ ] `crates/vector-pty/tests/lifecycle.rs` — CORE-04 SIGWINCH + no-zombie + clean exit
- [ ] `crates/vector-mux/tests/trait_object_safety.rs` — D-38 `Box<dyn PtyTransport>` / `Box<dyn Domain>` compile-time check
- [ ] `crates/vector-headless/tests/no_tokio_main.rs` — Phase 1 architecture lint inherited (if new crate adopted)
- [ ] Workspace dep: add `async-trait = "0.1"` to `[workspace.dependencies]` (RESEARCH Open Q 5)

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

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references (11 test files + 2 trait/lifecycle files + 1 lint file + async-trait dep)
- [ ] No watch-mode flags (`cargo watch` / `cargo test -- --nocapture --test-threads=1` only for local dev, never CI)
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
