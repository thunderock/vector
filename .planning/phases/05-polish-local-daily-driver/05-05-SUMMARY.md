---
phase: 05-polish-local-daily-driver
plan: 05
subsystem: vector-term
tags: [osc, sniffer, hyperlinks, dynamic-color, polish-04, d-78, d-79, d-70, pitfall-3, pitfall-4]

# Dependency graph
requires:
  - "alacritty_terminal 0.26 (workspace; transitive vte 0.15)"
  - "percent-encoding 2 (workspace; new direct dep on vector-term)"
  - "tokio (workspace; new direct dep on vector-term — mpsc channels for listener)"
provides:
  - "vector_term::osc_sniff::{OscSniff, OscEvents, PromptKind, PromptMark} — byte-level OSC 7 + 133 sniffer (vte::Perform)"
  - "vector_term::hyperlink::{is_allowed_scheme, HyperlinkRun, group_row} — D-78 allowlist + Pitfall-4 grouping"
  - "vector_term::listener::{ForwardingListener, ClipboardEvent} — replaces Phase-2 NoopListener"
  - "vector_term::Term::cwd_ring() -> &VecDeque<PathBuf> (cap 16, most-recent at back)"
  - "vector_term::Term::prompt_marks() -> &VecDeque<PromptMark> (cap 1000 per D-79)"
  - "vector_term::Term::with_channels(cols, rows, scroll, write_tx, clip_tx) — live channel wiring"
  - "OSC 10/11/12 reply path: alacritty Event::ColorRequest(idx, fmt) → listener invokes fmt(default_color_for(idx)) → write_tx"
  - "OSC 52 read denial (D-70) path: Event::ClipboardLoad → ClipboardEvent::LoadDenied (callback NEVER invoked)"
affects:
  - "05-06 (clipboard + OSC 52) — consumes ForwardingListener.clipboard_tx (Store, LoadDenied)"
  - "05-07 (theme integration) — replaces default_color_for() with palette lookup"
  - "05-08 (cwd inheritance) — reads Term::cwd_ring().back() at new-pane spawn (D-79)"
  - "05-08+ future prompt-jump features — reads Term::prompt_marks() for navigation"

# Tech tracking
tech-stack:
  added:
    - "percent-encoding 2 (direct dep on vector-term; OSC 7 path decode)"
    - "vte 0.15 (direct dep on vector-term; explicit Parser+Perform for OSC sniffer)"
    - "tokio (direct dep on vector-term; mpsc::Sender in ForwardingListener)"
  patterns:
    - "Two-layer OSC sniff (Pattern 1 of 05-RESEARCH): one vte::Parser drives a
      vector_term::osc_sniff::OscSniff observer for OSC 7+133 in parallel with
      alacritty's feed; the byte stream flows through alacritty unchanged."
    - "Unix-tolerant percent-decode (Pitfall 3): OSC 7 paths use
      OsString::from_vec(decoded_bytes) on Unix so non-UTF-8 path bytes survive
      round-trip; falls back to String::from_utf8 on non-Unix."
    - "Bounded ring buffer eviction: VecDeque with pop_front() on cap-reached;
      cwd cap 16, prompt-mark cap 1000 (D-79)."
    - "Event::ColorRequest dispatch (alacritty 0.26): OSC 10/11/12 queries
      surface as ColorRequest(index, Arc<Fn(Rgb) -> String>), NOT PtyWrite.
      Listener invokes the callback with a sensible default Rgb and forwards
      the resulting bytes to write_tx."
    - "Non-blocking listener: try_send + tracing::warn on full channel keeps
      the main thread off any await/block (CLAUDE.md `don't block main, never
      lose events silently`)."
    - "Term backward compat: Term::new keeps Phase-2 shape via internal dummy
      mpsc channels (recv-side dropped); Term::with_channels for live wiring."

key-files:
  created:
    - crates/vector-term/src/osc_sniff.rs
    - crates/vector-term/src/hyperlink.rs
    - crates/vector-term/tests/osc_sniff.rs
    - crates/vector-term/tests/hyperlinks.rs
    - crates/vector-term/tests/dynamic_color_response.rs
  modified:
    - crates/vector-term/Cargo.toml          # +percent-encoding, +vte=0.15, +tokio (dep + dev-dep)
    - crates/vector-term/src/lib.rs          # pub mod osc_sniff, hyperlink, listener; re-exports
    - crates/vector-term/src/listener.rs     # NoopListener → ForwardingListener; ColorRequest handled
    - crates/vector-term/src/term.rs         # osc_parser + sniff drive; rings; with_channels constructor

decisions:
  - "POLISH-04 D-79 captured: per-Term cwd_ring (cap 16) + prompt_marks (cap 1000); accessors public."
  - "POLISH-04 D-78 enforced: hyperlink scheme allowlist = {https://, http://, mailto:, file://}; others tracing::info!'d + ignored."
  - "POLISH-04 D-70 path declared: ClipboardEvent::LoadDenied — Plan 05-06 renders the denied toast."
  - "Pitfall 3 honored: percent-decode + OsString::from_vec for non-UTF-8 path resilience on Unix."
  - "Pitfall 4 honored: OSC 8 anonymous links grouped by URI + cell contiguity; id-tagged links group by id."
  - "Rule-1 deviation (alacritty 0.26 source-verified): OSC 10/11/12 emits Event::ColorRequest not Event::PtyWrite; listener invokes the closure with default_color_for(idx) returning sensible fg/bg/cursor defaults (Plan 05-07 will replace with palette lookup)."

metrics:
  duration_min: 25
  completed: "2026-05-12"
  task_commits: 4
  tests_added: 8
  tests_passing_total: 36   # 28 Phase-2 baseline + 8 new (Plan 05-05)
---

# Phase 5 Plan 05: OSC Sniffer + ForwardingListener Summary

**One-liner:** Two-layer OSC parser captures OSC 7 (cwd) + OSC 133 (prompt marks) into bounded per-Term rings while a fresh ForwardingListener round-trips OSC 10/11/12 color queries and declares the D-70 OSC 52 read-denial path.

## What landed

| Surface | Outcome |
|---------|---------|
| OSC 7 cwd capture | `file://localhost/path/` → `PathBuf` into `Term::cwd_ring()` (cap 16, most-recent at `back()`). Percent-encoded paths (`%20`) decode correctly. Unix path bytes preserved via `OsString::from_vec`. |
| OSC 133 prompt marks | `A`/`B`/`C`/`D` + optional `;exit_code` for `D` → `PromptMark` ring (cap 1000 per D-79). Bounded eviction on overflow. |
| OSC 8 hyperlinks | `hyperlink::group_row` walks per-row cells and emits `HyperlinkRun`s: id-tagged links group by id; anonymous links group by URI + contiguity (Pitfall 4). `is_allowed_scheme` enforces D-78 allowlist `{https://, http://, mailto:, file://}`. |
| OSC 10/11/12 reply | `ForwardingListener` handles `Event::ColorRequest(idx, fmt)` by invoking `fmt(default_color_for(idx))` and forwarding the bytes to `write_tx` — vim/neovim dark-mode probes round-trip. |
| OSC 52 inbound (D-70) | `Event::ClipboardLoad` emits `ClipboardEvent::LoadDenied` (the alacritty callback is NEVER invoked, so the shell never receives clipboard contents back). Plan 05-06 consumes this for the denied-toast. |
| OSC 52 outbound | `Event::ClipboardStore` emits `ClipboardEvent::Store(kind, data)` — Plan 05-06 wires the actual NSPasteboard write. |

## Commits

1. `cad3a1c` — test(05-05): add failing tests for OSC 7 + OSC 133 sniffer (TDD RED, Task 1)
2. `50baa1f` — feat(05-05): OSC 7 cwd + OSC 133 prompt-mark sniffer (D-79) (TDD GREEN, Task 1)
3. `8745f8c` — test(05-05): add failing tests for OSC 8 grouping + OSC 10/11/12 reply (TDD RED, Task 2)
4. `2127fb0` — feat(05-05): ForwardingListener + OSC 8 grouping + OSC 10/11/12 reply (TDD GREEN, Task 2)

## Verification

- `cargo test -p vector-term --test osc_sniff --test hyperlinks --test dynamic_color_response` — **8/8 pass** (osc7_file_url_parses, osc7_percent_encoded, osc133_marks, prompt_ring_1000, id_groups_run, anonymous_by_uri, scheme_allowlist, osc10_query_response).
- `cargo test -p vector-term --test alt_screen_1049 --test csi_dispatch --test dcs_dispatch --test decstbm_scroll_region --test ed_el_erase --test grapheme_width --test no_tokio_main --test no_transport_discrimination --test osc_dispatch --test partial_utf8 --test scrollback_search --test sgr_truecolor` — **28/28 Phase-2 baseline still pass** (no regression).
- `cargo clippy -p vector-term --lib -- -D warnings` — clean.
- `cargo clippy -p vector-term --test osc_sniff --test hyperlinks --test dynamic_color_response -- -D warnings` — clean.
- `grep -rn NoopListener crates/vector-term/src/` — empty (NoopListener fully retired per acceptance criterion).
- `grep -q "PROMPT_RING_CAP: usize = 1000" crates/vector-term/src/term.rs` — D-79 bound explicit.
- `grep -q "self.osc_parser.advance" crates/vector-term/src/term.rs` — parallel parser running before alacritty feed.
- `grep -q '"https://", "http://", "mailto:", "file://"' crates/vector-term/src/hyperlink.rs` — D-78 exact allowlist.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] OSC 10/11/12 reply path: alacritty 0.26 emits ColorRequest, not PtyWrite**

- **Found during:** Task 2 implementation (test `osc10_query_response` initially hung on `write_rx.recv().await`).
- **Issue:** The plan's `<interfaces>` snippet listed `Event::PtyWrite(String)` as the variant for OSC 10/11/12 replies. Verifying against `~/.cargo/registry/.../alacritty_terminal-0.26.0/src/term/mod.rs:1675-1688` showed `dynamic_color_sequence(...)` emits `Event::ColorRequest(index, Arc<dyn Fn(Rgb) -> String>)`. The listener must invoke the closure with an `Rgb` value to produce the reply string, then push the bytes to the write channel.
- **Fix:** Added an explicit `Event::ColorRequest(idx, fmt)` arm in `ForwardingListener::send_event` that calls `fmt(default_color_for(idx))` and forwards the result to `write_tx`. `default_color_for` returns sensible defaults (fg/cursor = light gray `#ebebeb`, bg = near-black `#181818`) so vim/neovim dark-mode detection round-trips. **Plan 05-07 (theme integration) will replace the defaults with palette lookup.**
- **Files modified:** `crates/vector-term/src/listener.rs`.
- **Commit:** `2127fb0`.

**2. [Rule 1 - Lint] needless_continue in hyperlink::group_row**

- **Found during:** Task 2 clippy sweep.
- **Issue:** `(None, None) => continue,` flagged by `clippy::needless_continue` (last arm of a for-loop body iteration).
- **Fix:** Replaced with `(None, None) => {}`.
- **Files modified:** `crates/vector-term/src/hyperlink.rs`.
- **Commit:** `2127fb0` (folded in).

**3. [Rule 1 - Lint] match_same_arms in default_color_for**

- **Found during:** Task 2 clippy sweep.
- **Issue:** OSC 10 (fg = idx 256) and OSC 12 (cursor = idx 258) used identical Rgb arms.
- **Fix:** Merged into `256 | 258 => Rgb { 0xeb, 0xeb, 0xeb }`.
- **Files modified:** `crates/vector-term/src/listener.rs`.
- **Commit:** `2127fb0` (folded in).

### Pre-existing Out-of-Scope Issues (Deferred — Not Mine)

- `crates/vector-term/tests/osc52.rs` + `crates/vector-term/tests/osc52_tmux.rs` are produced by the parallel **Plan 05-06** executor running in the same target tree. They have 2 clippy errors (`match_wildcard_for_single_variants`, `trim_split_whitespace`) and one `items_after_statements`. **Not fixed here** per SCOPE BOUNDARY — these are 05-06's files. My `cargo clippy -p vector-term --lib` and per-test invocations (osc_sniff, hyperlinks, dynamic_color_response) are clean.
- `crates/vector-app/Cargo.toml` and root `Cargo.toml` carried uncommitted changes from another parallel agent at executor start (vector-app double-declared lints; root Cargo.toml gained `vector-arch-tests` + workspace deps). They were either self-healing or owned by another plan. **Untouched.**

## Authentication Gates

None — fully autonomous plan with no external auth dependencies.

## Hand-off to downstream plans

- **Plan 05-06 (clipboard + OSC 52)**: Consume `ForwardingListener.clipboard_tx` — `ClipboardEvent::Store(kind, data)` for outbound writes; `ClipboardEvent::LoadDenied` for the D-70 denied toast. `Term::with_channels(...)` is the constructor that wires both channels.
- **Plan 05-07 (theme integration)**: Replace `default_color_for(idx)` in `listener.rs` with a `Palette` lookup so OSC 10/11/12 replies reflect the active theme. Same `Event::ColorRequest` dispatch path; only the default-color helper changes.
- **Plan 05-08 (cwd inheritance)**: On new pane spawn, read `parent_term.cwd_ring().back()` and pass to the child PTY's `cwd` field; D-79 ring cap of 16 is sufficient for "most recent" inheritance.

## Self-Check: PASSED

- `crates/vector-term/src/osc_sniff.rs` — FOUND
- `crates/vector-term/src/hyperlink.rs` — FOUND
- `crates/vector-term/tests/osc_sniff.rs` — FOUND
- `crates/vector-term/tests/hyperlinks.rs` — FOUND
- `crates/vector-term/tests/dynamic_color_response.rs` — FOUND
- Commit `cad3a1c` — FOUND (test RED Task 1)
- Commit `50baa1f` — FOUND (feat GREEN Task 1)
- Commit `8745f8c` — FOUND (test RED Task 2)
- Commit `2127fb0` — FOUND (feat GREEN Task 2)
- All 8 new tests pass; all 28 Phase-2 baseline tests pass; lib + new-test clippy clean.
