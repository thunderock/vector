---
phase: 05-polish-local-daily-driver
plan: 06
subsystem: clipboard
tags: [osc52, tmux, base64, clipboard, alacritty, dcs-passthrough]

# Dependency graph
requires:
  - phase: 05-polish-local-daily-driver
    provides: ForwardingListener + ClipboardEvent + Term::with_channels (Plan 05-05); Wave-0 stub files (Plan 05-01)
provides:
  - osc52_outbound emitter with 58-byte chunking per envelope (D-71 + Pitfall 5)
  - MAX_CHUNK_BASE64 = 58 public const (D-71 locked)
  - 3 OSC 52 inbound tests proving Store/DCS-peel/Read-denied semantics
  - Real-tmux 3.4+ DCS passthrough round-trip integration test (CI-gated via --ignored)
  - Empirical resolution of Open Question #1: alacritty 0.26 auto-peels DCS envelopes containing OSC
affects: [05-07 (selection -> clipboard write), 05-08 (clipboard router wiring), 05-09 (CI tmux-smoke job consumes osc52_tmux test body)]

# Tech tracking
tech-stack:
  added: [base64 0.22 workspace dep on vector-input (dep) + vector-term (dev-dep)]
  patterns:
    - "Raw OSC 52 emission (no DCS rewrap) — D-71 mandate: 'Vector never re-wraps outbound, that's tmux's job.'"
    - "One OSC 52 envelope per 58-byte chunk; tmux passthrough resumes per envelope"
    - "Empirical OQ resolution via test-driven probing (DCS auto-peel discovered green-bar on first run)"

key-files:
  created:
    - crates/vector-input/src/clipboard.rs
    - crates/vector-input/tests/clipboard.rs
    - crates/vector-term/tests/osc52.rs
    - crates/vector-term/tests/osc52_tmux.rs
  modified:
    - crates/vector-input/Cargo.toml (base64 dep)
    - crates/vector-input/src/lib.rs (clipboard module + re-export)
    - crates/vector-term/Cargo.toml (base64 dev-dep)

key-decisions:
  - "Open Question #1 — alacritty_terminal 0.26 auto-peels DCS envelopes containing OSC. No DCS-unwrap shim needed in osc_sniff.rs. Empirically confirmed by dcs_wrapped_round_trip test passing on first attempt against unmodified Plan-05-05 listener pipeline."
  - "D-70 OSC 52 read denial is enforced silently at alacritty's default Osc52::OnlyCopy mode — neither clipboard event nor PtyWrite reply fires. Test asserts the *absence* of any event within 50ms rather than a positive LoadDenied dispatch (alacritty source mod.rs:1727-1728 returns early before invoking the listener)."
  - "ClipboardStore payload contract: alacritty 0.26 base64-DECODES the OSC 52 payload before delivering it via Event::ClipboardStore — consumers receive plaintext, not base64. Test asserts data == \"hello\" not data == \"aGVsbG8=\"."
  - "Outbound chunking shape: one OSC 52 envelope per 58-byte b64 chunk, raw (no DCS wrap), envelopes back-to-back. Each chunk is a self-contained valid OSC 52 — tmux passthrough resumes naturally per envelope; non-tmux receivers decode cumulatively."

patterns-established:
  - "Self-contained outbound emitter pattern: pure function over &[u8] returning Vec<u8> with no listener / channel surface. Plan 05-08 will pipe Cmd-C selection through this for OSC 52 output."
  - "CI-gated integration test pattern: real-tmux test marked #[ignore = \"Requires tmux 3.4+; enabled by CI tmux-smoke or manual --ignored\"]; tmux-smoke CI job in Plan 05-01 runs with --ignored, default cargo test does not."

requirements-completed: [POLISH-05]

# Metrics
duration: 38min
completed: 2026-05-12
---

# Phase 5 Plan 06: OSC 52 raw + DCS-wrapped inbound + 58-byte outbound chunking + read-denial Summary

**Outbound OSC 52 emitter with 58-byte chunking via base64 0.22; inbound test trio (raw / DCS-peeled / read-denied) proves alacritty 0.26 auto-peels DCS envelopes and silently denies reads at OnlyCopy mode; real-tmux 3.4+ round-trip integration harness wired for the Plan-05-01 tmux-smoke CI job.**

## Performance

- **Duration:** 38 min
- **Started:** 2026-05-12T17:39:38Z
- **Completed:** 2026-05-12T18:18:00Z (approximate)
- **Tasks:** 2
- **Files modified/created:** 7 (4 created, 3 modified)

## Accomplishments

- `vector_input::osc52_outbound(&[u8]) -> Vec<u8>` ships with 58-byte chunking, MAX_CHUNK_BASE64 public const, D-71 locked
- 3 OSC 52 inbound tests green: `raw_clipboard_store` (alacritty native dispatch), `dcs_wrapped_round_trip` (Open Question #1 empirically resolved), `read_denied` (silent denial at alacritty default)
- Real-tmux integration test `dcs_round_trip_through_tmux` body landed — tmux 3.4+ version gate, `allow-passthrough on`, `pbpaste` assertion; #[ignore]-gated for tmux-smoke CI job
- Open Question #1 closed: alacritty_terminal 0.26 auto-peels DCS envelopes containing OSC sequences (test passed first try with no osc_sniff.rs DCS-unwrap shim required)

## Task Commits

1. **Task 2: Outbound OSC 52 with 58-byte chunking** — `cb2a4fd` (feat) — added base64 dep, created clipboard.rs + tests/clipboard.rs, 1 test green
2. **Task 1: OSC 52 raw + DCS inbound + read-denied + tmux smoke harness** — `7f23320` (feat) — created tests/osc52.rs + tests/osc52_tmux.rs, 3 tests green + tmux test compiles

_Note: Task 2 was executed BEFORE Task 1 — Task 2 is fully self-contained (only depends on workspace base64 dep), Task 1 depends on Plan 05-05's `Term::with_channels` + `ClipboardEvent` API surface. Running Task 2 first allowed Task 1 to proceed cleanly once 05-05's parallel commit landed in the working tree._

## Files Created/Modified

- `crates/vector-input/src/clipboard.rs` (created) — `osc52_outbound` + `MAX_CHUNK_BASE64 = 58` + `TMUX_CHUNK_MAX` constants; raw OSC 52 emission, one envelope per 58-byte b64 chunk
- `crates/vector-input/tests/clipboard.rs` (created) — `outbound_58_byte_chunks` test (single-chunk short payload + multi-chunk 300-byte payload + walk-runs invariant)
- `crates/vector-input/src/lib.rs` (modified) — `pub mod clipboard;` + re-export
- `crates/vector-input/Cargo.toml` (modified) — added `base64 = { workspace = true }` dep
- `crates/vector-term/tests/osc52.rs` (created) — 3 tokio tests via `Term::with_channels`: raw_clipboard_store + dcs_wrapped_round_trip + read_denied
- `crates/vector-term/tests/osc52_tmux.rs` (created) — #[ignore]'d real-tmux integration with version gate, allow-passthrough, pbpaste verify
- `crates/vector-term/Cargo.toml` (modified) — added `base64 = { workspace = true }` dev-dep

## Decisions Made

See `key-decisions` in frontmatter. Headline:

1. **Open Question #1 — DCS auto-peel:** alacritty_terminal 0.26 auto-peels DCS envelopes containing OSC sequences. No DCS-unwrap shim required in `osc_sniff.rs`. Verified empirically by `dcs_wrapped_round_trip` passing on first attempt against the unmodified Plan-05-05 listener pipeline.
2. **D-70 silent denial:** alacritty's default `Osc52::OnlyCopy` mode denies OSC 52 reads silently at `mod.rs:1727-1728` — no `Event::ClipboardLoad` fires, no `PtyWrite` reply emits, no `ClipboardEvent::LoadDenied` reaches the listener. The test asserts the *absence* of any event within 50ms.
3. **ClipboardStore payload is plaintext:** alacritty 0.26 base64-DECODES OSC 52 payloads before delivering them. Tests assert decoded `"hello"`, not encoded `"aGVsbG8="`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Test expectation off by base64 decoding step**
- **Found during:** Task 1 (`raw_clipboard_store` test)
- **Issue:** Plan body's test snippet asserted `data == "aGVsbG8="` (base64 string) per the comment "alacritty's Handler delivers the base64-ENCODED string". Empirically false — alacritty 0.26 source (`term/mod.rs:1715-1719`) base64-decodes the payload before dispatching `Event::ClipboardStore`. Test failed with `left: "hello", right: "aGVsbG8="`.
- **Fix:** Updated both `raw_clipboard_store` and `dcs_wrapped_round_trip` to assert `data == "hello"`. Updated doc comment to record the empirical finding.
- **Files modified:** `crates/vector-term/tests/osc52.rs`
- **Verification:** All 3 tests pass after the fix.
- **Committed in:** `7f23320`

**2. [Rule 1 - Bug] `read_denied` test design relied on a non-existent dispatch**
- **Found during:** Task 1 (`read_denied` test hung indefinitely)
- **Issue:** Plan body asserted the test should receive `ClipboardEvent::LoadDenied`. But alacritty's default `Osc52::OnlyCopy` mode early-returns at `mod.rs:1727-1728` before invoking the listener — the `Event::ClipboardLoad` never fires, so the `ForwardingListener::send_event` `ClipboardLoad(_,_)` arm (which is what would map to `LoadDenied`) never runs. The recv() blocks forever.
- **Fix:** Rewrote `read_denied` to assert silent denial — both `clip_rx.recv()` AND `write_rx.recv()` MUST time out within 50ms. This honors D-70 ("OSC 52 reads MUST be denied in v1") at the observable layer: no clipboard event, no PTY write reply.
- **Files modified:** `crates/vector-term/tests/osc52.rs`
- **Verification:** `read_denied` passes in 102ms (50ms × 2 timeouts).
- **Committed in:** `7f23320`
- **Note:** An alternative would have been to set the alacritty `Config.osc52 = Osc52::CopyPaste` so reads DO reach the listener, then `LoadDenied` would fire. That requires modifying Plan 05-05's `Term::with_channels`. Path-of-least-intrusion (silent-denial assertion) chosen because it (a) matches the D-70 outcome semantically — reads ARE denied; (b) avoids reaching into 05-05's territory while 05-05 was running in parallel; (c) is the actual user-observable behavior on v1 today.

**3. [Rule 1 - Bug] Clippy `match_wildcard_for_single_variants` lints in test match arms**
- **Found during:** Task 1 final clippy sweep
- **Issue:** `match` on `ClipboardEvent` used `other => panic!(...)` wildcard for the only remaining variant `LoadDenied`. Workspace lint `pedantic` flags this.
- **Fix:** Replaced wildcard with explicit `ClipboardEvent::LoadDenied => panic!("expected Store, got LoadDenied")`.
- **Files modified:** `crates/vector-term/tests/osc52.rs`
- **Verification:** `cargo clippy -p vector-term --all-targets -- -D warnings` clean.
- **Committed in:** `7f23320`

**4. [Rule 1 - Bug] Clippy `trim_split_whitespace` + `items_after_statements` lints in osc52_tmux**
- **Found during:** Task 1 final clippy sweep
- **Issue:** Two pedantic lints in `osc52_tmux.rs`: (a) `ver.trim().split_whitespace()` — `split_whitespace` already handles surrounding whitespace; (b) `use base64::Engine as _` placed mid-function after a `let` (items-after-statements).
- **Fix:** Removed redundant `trim()`; moved `use base64::Engine as _` to module top.
- **Files modified:** `crates/vector-term/tests/osc52_tmux.rs`
- **Verification:** `cargo clippy -p vector-term --test osc52_tmux -- -D warnings` clean.
- **Committed in:** `7f23320`

---

**Total deviations:** 4 auto-fixed (4 Rule-1 bugs — 2 plan-body test-design errors against empirical alacritty 0.26 behavior, 2 mechanical clippy lints)

**Impact on plan:** All auto-fixes preserve the plan's intent (D-70 read-denial + D-71 outbound chunking + Open-Question-#1 resolution). The two D-70-related fixes are documented empirical findings about alacritty 0.26 that should be carried forward into Plan 05-08 (clipboard router) and any future tests touching OSC 52.

## Issues Encountered

- **Parallel-agent cargo lock contention:** Multiple parallel executors (05-01, 05-05, 05-06) competing for cargo's filesystem lock during the early test runs. Some background cargo invocations died with empty stdout. Resolved naturally as parallel agents completed their builds.

## User Setup Required

None.

## Next Phase Readiness

- POLISH-05 (OSC 52 inbound + outbound + tmux-smoke) feature-complete at the unit-test level; ready for integration:
  - Plan 05-07 (selection -> clipboard): can use `vector_input::osc52_outbound` for Cmd-C → OSC 52 → PTY write
  - Plan 05-08 (clipboard router): consumes `ClipboardEvent::Store` from `ForwardingListener` and writes to `NSPasteboard.generalPasteboard()`
  - Plan 05-09 (CI tmux-smoke job): has a real test body (`dcs_round_trip_through_tmux`) to invoke via `cargo test --test osc52_tmux -- --ignored`
- All 3 inbound OSC 52 tests green on macOS local; tmux integration test builds clean (deferred to CI for real-tmux execution)
- D-70 (read-denial) and D-71 (58-byte chunking) both empirically validated and documented

## Self-Check: PASSED

- `crates/vector-input/src/clipboard.rs` — FOUND
- `crates/vector-input/tests/clipboard.rs` — FOUND
- `crates/vector-term/tests/osc52.rs` — FOUND
- `crates/vector-term/tests/osc52_tmux.rs` — FOUND
- Commit `cb2a4fd` (Task 2) — FOUND in git log
- Commit `7f23320` (Task 1) — FOUND in git log

---
*Phase: 05-polish-local-daily-driver*
*Completed: 2026-05-12*
