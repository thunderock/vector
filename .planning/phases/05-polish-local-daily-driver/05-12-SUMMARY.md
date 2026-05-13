---
phase: 05-polish-local-daily-driver
plan: 12
subsystem: ui
tags: [clipboard, osc-52, ns-pasteboard, gap-closure, wave-2, high-3]

# Dependency graph
requires:
  - phase: 05-polish-local-daily-driver
    provides: ClipboardRouter type (Plan 05-08), ForwardingListener.clipboard_tx (Plan 05-05/05-06), NSPasteboard FFI write_pasteboard (Plan 05-10), ToastStack.show (Plan 05-10), ConfigReloaded UserEvent (Plan 05-10 Task 3), App.term_grid_access pattern (Plan 05-11)
provides:
  - "End-to-end OSC 52 -> ForwardingListener -> ClipboardEvent::Store -> UserEvent::ClipboardStore -> App.clipboard_router -> NSPasteboard (or toast prompt)"
  - "Mux::new_with_clipboard constructor that threads a real mpsc::Sender<ClipboardEvent> into every Term::with_channels call"
  - "UserEvent::ClipboardStore { kind_is_selection, data } cross-thread variant"
  - "make_store_event helper that keeps alacritty_terminal::ClipboardType out of app.rs"
affects: [05-14 (App-shortcut dispatch — orthogonal), 05-16 (final smoke + acceptance)]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Optional channel on Mux preserves legacy Mux::new for tests; bootstrap path uses new_with_clipboard"
    - "Helper fn make_store_event in router crate avoids re-exporting alacritty's ClipboardType through app.rs"

key-files:
  created:
    - crates/vector-app/tests/clipboard_router_wiring.rs
  modified:
    - crates/vector-app/src/app.rs
    - crates/vector-app/src/clipboard_router.rs
    - crates/vector-app/src/lib.rs
    - crates/vector-app/src/main.rs
    - crates/vector-mux/src/mux.rs
    - crates/vector-term/src/lib.rs

key-decisions:
  - "HIGH-3 pre-inspection: Term construction lives in Mux::create_tab_async + split_pane_async (mux.rs:377/418), NOT in LocalDomain::spawn_local — refactor lands on Mux"
  - "Add Mux::new_with_clipboard rather than mutating Mux::new — keeps 7 tests in vector-mux compiling without churn"
  - "Re-export ClipboardType from vector-term to avoid pulling alacritty_terminal as a direct dep of vector-app"
  - "make_store_event helper in clipboard_router.rs keeps alacritty's ClipboardType out of app.rs (mirrors the bool-discriminator design of UserEvent::ClipboardStore)"

patterns-established:
  - "Optional-channel + helper builder (build_term) pattern lets a singleton thread a runtime channel through every constructed Term while preserving cheap test-only construction"

requirements-completed: [POLISH-05]

# Metrics
duration: ~10min
completed: 2026-05-13
---

# Phase 5 Plan 12: ClipboardRouter end-to-end wiring (gap #7) Summary

**OSC 52 stores now flow ForwardingListener -> Mux-threaded clipboard channel -> EventLoopProxy -> App.clipboard_router -> NSPasteboard (or toast prompt) without dummy channels on the active path.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-05-13T04:38:43Z
- **Completed:** 2026-05-13T04:48Z
- **Tasks:** 3 (Task 0 inspection, Task 1 channel plumbing, Task 2 App wiring + tests)
- **Files modified:** 6
- **Files created:** 1 (clipboard_router_wiring.rs)

## Task 0 Pre-inspection Result (HIGH-3 mandate)

**Decision: Case A — Term::new in the active path; vector-mux scope expansion required.**

Grep results:
```
crates/vector-mux/src/mux.rs:377: let term = ... vector_term::Term::new(cols, rows, 10_000) ...
crates/vector-mux/src/mux.rs:418: let term = ... vector_term::Term::new(cols, rows, 10_000) ...
```

Critically, `LocalDomain::spawn_local` does NOT construct the `Term` at all — it only owns the PTY transport. The Term lives one layer up, inside `Mux::create_tab_async` and `Mux::split_pane_async`. So the plan's "thread the sender through LocalDomain" framing was inaccurate; the right scope was the **Mux**, not `LocalDomain`.

**Files actually modified in Task 1:**
- `crates/vector-mux/src/mux.rs` (new field + new constructor + build_term helper + replaced both Term::new callsites)
- `crates/vector-app/src/lib.rs` (UserEvent::ClipboardStore variant)
- `crates/vector-app/src/main.rs` (clip_tx channel, drain task, Mux::new_with_clipboard call)
- `crates/vector-term/src/lib.rs` (re-export ClipboardType)
- NOT `local_domain.rs` (LocalDomain doesn't construct Term)
- NOT `pane.rs` (Pane::new accepts a pre-built Arc<Mutex<Term>>; doesn't construct it)

The frontmatter's `# HIGH-3: included conditionally` annotation on `local_domain.rs` and `pane.rs` correctly hedged — they were ultimately NOT touched. `mux.rs` was touched as predicted.

## Accomplishments

- **POLISH-05 gap #7 closed.** The verifier's "App.clipboard_router consumes ClipboardEvent::Store — ✗ NOT WIRED" diagnosis is no longer true. End-to-end grep proof below.
- **UserEvent::ClipboardStore { kind_is_selection, data }** — added to `vector-app/src/lib.rs`. Uses a `bool` discriminator rather than re-exporting `alacritty_terminal::term::ClipboardType` so the public enum keeps a tight dep surface; the conversion happens in `clipboard_router::make_store_event`.
- **Mux::new_with_clipboard** — additive constructor; `Mux::new` is kept for the 7 vector-mux integration tests (cwd_inheritance, mux_topology, mux_tab_cycle, mux_close_cascade, profile_local_spawn, pane_resize_propagates, trait_object_safety) that do not exercise the clipboard path. The production bootstrap in `main.rs` always calls `new_with_clipboard`.
- **build_term helper on Mux** — branches on `self.clipboard_tx`: `Some` → `Term::with_channels(..., clip_tx.clone())`; `None` → `Term::new(...)`. The active production path (Mux::new_with_clipboard) always uses `Term::with_channels`. The `Term::new` line survives only on the test-only `None` branch.
- **I/O thread drain task** in `main.rs` — `tokio::spawn` next to the existing write / resize / split tasks. Converts `ClipboardEvent::Store(kind, data)` into `UserEvent::ClipboardStore { kind_is_selection, data }` via `matches!(kind, vector_term::ClipboardType::Selection)`; logs `LoadDenied` per D-70.
- **App.clipboard_router field** — initialized in `App::new` with `active_profile = "default"`, `policy = None` (prompt-first). On every `UserEvent::ConfigReloaded`, the active profile's `clipboard_write` is resolved and pushed into `router.policy`. The `ClipboardStore` arm dispatches: `WritePasteboard` → `write_pasteboard` (existing NSPasteboard FFI), `ShowPrompt` → `toasts.show` + `request_redraw_all`, `DenyRead` → `tracing::info!`.
- **make_store_event helper** in `clipboard_router.rs` — translates the `bool` discriminator back into `vector_term::ClipboardType` so app.rs never names `alacritty_terminal` directly.
- **Re-export `vector_term::ClipboardType`** — pinned at `crates/vector-term/src/lib.rs:11`. Avoids adding `alacritty_terminal` as a direct dep of `vector-app`.

## End-to-End Grep Proof (HIGH-3 acceptance)

```text
$ grep -n "Term::with_channels" crates/vector-mux/src/mux.rs
75:                vector_term::Term::with_channels(cols, rows, scrollback, write_tx, clip_tx.clone())

$ grep -n "Term::new" crates/vector-mux/src/mux.rs
77:            None => vector_term::Term::new(cols, rows, scrollback),
    # ^ test-only branch; only reachable via Mux::new (no clipboard_tx)

$ grep -n "clip_tx" crates/vector-app/src/main.rs
50:    let (clip_tx, mut clip_rx) = mpsc::channel::<vector_term::ClipboardEvent>(32);
76:                let mux = Mux::new_with_clipboard(local_domain, clip_tx);

$ grep -nE "clip_rx\.recv|ClipboardEvent::Store" crates/vector-app/src/main.rs
131:                    while let Some(ev) = clip_rx.recv().await {
133:                            vector_term::ClipboardEvent::Store(kind, data) => {

$ grep -n "ClipboardStore" crates/vector-app/src/lib.rs
69:    ClipboardStore { kind_is_selection: bool, data: String },

$ grep -n "UserEvent::ClipboardStore" crates/vector-app/src/app.rs
92:    /// OSC 52 Store events forwarded as UserEvent::ClipboardStore. Policy is
815:            UserEvent::ClipboardStore { kind_is_selection, data } => {

$ grep -n "clipboard_router:" crates/vector-app/src/app.rs
94:    clipboard_router: crate::clipboard_router::ClipboardRouter,
117:            clipboard_router: crate::clipboard_router::ClipboardRouter {

$ grep -n "ClipboardAction::WritePasteboard" crates/vector-app/src/app.rs
819:                    crate::clipboard_router::ClipboardAction::WritePasteboard(s) => {
```

Chain proven: `main.rs::clip_tx` (channel sender, line 50) → `Mux::new_with_clipboard(.., clip_tx)` (line 76) → stored on `Mux.clipboard_tx` (mux.rs:37) → `build_term(...)` (mux.rs:72) → `Term::with_channels(.., clip_tx.clone())` (mux.rs:75) → invoked from `create_tab_async` (mux.rs:381) and `split_pane_async` (mux.rs:422). Drain task (`main.rs:131`) converts `Store` events into `UserEvent::ClipboardStore` and the App arm routes them through `clipboard_router.handle(...)` (app.rs:818).

## Task Commits

1. **Task 0/1 — Pre-inspection + clipboard_tx plumbing (Case A vector-mux refactor)** — `90f91cf` (feat)
2. **Task 2 RED — failing tests for ClipboardRouter policy dispatch** — `31cbc1d` (test)
3. **Task 2 GREEN — App.clipboard_router field + UserEvent::ClipboardStore arm** — `2c30fc0` (feat)

## Files Created/Modified

- `crates/vector-app/src/lib.rs` — added `UserEvent::ClipboardStore { kind_is_selection: bool, data: String }`.
- `crates/vector-app/src/main.rs` — created `(clip_tx, mut clip_rx)` channel; called `Mux::new_with_clipboard(local_domain, clip_tx)`; spawned drain task converting `ClipboardEvent::Store` -> `UserEvent::ClipboardStore` and logging `LoadDenied`.
- `crates/vector-mux/src/mux.rs` — added `clipboard_tx: Option<mpsc::Sender<vector_term::ClipboardEvent>>` field, `Mux::new_with_clipboard` constructor, `build_term` helper; replaced both `Term::new(cols, rows, 10_000)` callsites with `self.build_term(cols, rows, 10_000)`.
- `crates/vector-term/src/lib.rs` — added `ClipboardType` to the alacritty re-export list.
- `crates/vector-app/src/app.rs` — added `clipboard_router: ClipboardRouter` field + initializer; resolved policy from `current_config.profile[active_profile].clipboard_write` in `UserEvent::ConfigReloaded`; added `UserEvent::ClipboardStore` arm dispatching `WritePasteboard` / `ShowPrompt` / `DenyRead`.
- `crates/vector-app/src/clipboard_router.rs` — added `pub fn make_store_event(kind_is_selection: bool, data: String) -> ClipboardEvent` helper.
- `crates/vector-app/tests/clipboard_router_wiring.rs` (new) — 3 integration tests for Allow / None / Block policy dispatch.

## Decisions Made

- **Scope of refactor lives on Mux, not LocalDomain.** Task 0 grep revealed `LocalDomain::spawn_local` returns a `SpawnedPane { transport, pid, master_fd }` — no Term. The `Term::new` calls happen one layer up in `Mux::create_tab_async` (line 377) and `Mux::split_pane_async` (line 418). Putting the channel on `LocalDomain` would have forced LocalDomain to grow a Term-unrelated field for no reason. The Mux owns the Term-construction sites, so the channel lives on the Mux.
- **Additive `Mux::new_with_clipboard` instead of mutating `Mux::new`.** Seven `vector-mux/tests/*.rs` call `Mux::new(Arc::new(LocalDomain::with_shell(...)))` and don't exercise OSC 52. Changing the signature would have spread the diff across every test file for no functional gain. The bootstrap binary calls `new_with_clipboard`; tests stay on `new`. The `build_term` helper branches on `Option<Sender>` so both paths compile without code duplication.
- **`bool` discriminator on UserEvent::ClipboardStore.** Plan explicitly specified this — keeps `alacritty_terminal::term::ClipboardType` from leaking into `vector-app/src/lib.rs`'s public enum. The conversion (`bool` <-> enum) happens once each direction: in the drain task (`main.rs:135`) and in `make_store_event` (`clipboard_router.rs:29`).
- **Re-export `ClipboardType` from `vector_term` instead of adding alacritty as a direct dep of vector-app.** vector-term already re-exports `LineDamageBounds`, `TermDamage`, `TermDamageIterator`. Adding `ClipboardType` to the same `pub use` line is one extra symbol with no new transitive deps. This lets `main.rs` write `matches!(kind, vector_term::ClipboardType::Selection)` without importing `alacritty_terminal`.
- **`fg_proc = "shell"` hardcoded.** Documented in the plan as I10 (info-only deferral). The toast still reads correctly per UI-SPEC §6.1; threading the active pane's foreground-process label is a follow-up.

## Deviations from Plan

### None

Plan executed exactly as written, with one important pre-inspection-driven adjustment that the plan explicitly required Task 0 to identify:

- **Task 0 finding shifted the refactor target from `LocalDomain` to `Mux`.** The plan said "If Term::new found, expand scope to vector-mux: LocalDomain, pane.rs, mux.rs". Task 0 found that LocalDomain doesn't construct Term at all — that lives in Mux. So the actual files touched are `mux.rs` only (not `local_domain.rs` or `pane.rs`). This is the planned behavior of Task 0 (HIGH-3): the pre-inspection determines the precise scope. The frontmatter's `# HIGH-3: included conditionally` hedged correctly — `local_domain.rs` and `pane.rs` are listed but in practice were not modified.

No Rule-1/2/3 auto-fixes were needed during execution. All three tasks compiled and tested cleanly on first attempt.

## Known Stubs

- `fg_proc = "shell"` (app.rs:816) — documented in the plan as deferred-item I10. The active pane's real foreground-process label is available via `Mux::pane(pane_id).last_proc_name`, but plumbing the active pane through to the App layer for toast generation is a follow-up; the v1 default keeps toasts readable per UI-SPEC §6.1.

## Issues Encountered

None.

## Self-Check

- `crates/vector-app/tests/clipboard_router_wiring.rs`: **FOUND** (3/3 passing)
- Commit `90f91cf`: **FOUND** in `git log`
- Commit `31cbc1d`: **FOUND** in `git log`
- Commit `2c30fc0`: **FOUND** in `git log`
- Acceptance greps: **ALL PASS** (see "End-to-End Grep Proof" above)
- `cargo build -p vector-app --release`: **SUCCEEDS**
- `cargo build --workspace --release`: **SUCCEEDS**
- `cargo test -p vector-app`: **ALL PASS** (clipboard_router_wiring 3/3, plus all existing tests)
- `cargo test -p vector-mux`: **ALL PASS** (legacy Mux::new tests untouched)
- HIGH-3 invariant: clip_tx -> Mux -> Term::with_channels with no dummy channel on the active path — **CONFIRMED** (Term::new in mux.rs:77 is the test-only `clipboard_tx: None` branch only)

## Self-Check: PASSED

## Manual Reproduce

1. Build: `cargo build --release -p vector-app`
2. Run Vector with a default config (`~/.config/vector/config.toml` auto-seeded on first launch). The bundled default does NOT set `clipboard_write`, so the router's policy resolves to `None` (prompt-first).
3. In the pane, run `printf '\e]52;c;%s\a' "$(echo hello | base64)"`. A toast should appear: `allow "default : shell" to write to your clipboard?` with three buttons `[allow once] [always] [block]`.
4. Edit `~/.config/vector/config.toml` to add:
   ```toml
   [profile.default]
   clipboard_write = "allow"
   ```
   Save. The config watcher fires `UserEvent::ConfigReloaded`, which resolves `policy = Some(Allow)`.
5. Re-issue `printf '\e]52;c;%s\a' "$(echo hello | base64)"`. No toast appears; `pbpaste` returns `hello`.
6. (Negative test) Change `clipboard_write = "block"`, re-issue the OSC 52 printf — an info toast appears: `clipboard write from default blocked`; `pbpaste` is unchanged.

## Next Phase Readiness

- POLISH-05 fully wired end-to-end; remaining Phase 5 gap-closure plans (05-14, 05-15, 05-16) are unaffected (they modify the same `app.rs` but the depends_on chain serializes them).
- Plan 05-14 (App-shortcut dispatch) will replace the `Some(EncodedKey::App(_)) => {}` no-op arm Plan 05-11 added.
- Plan 05-16 final smoke matrix should include the OSC 52 -> NSPasteboard reproduce above.
- Follow-up (info-only, I10): plumb active pane's foreground-process label into the toast text instead of the hardcoded `"shell"`.

---
*Phase: 05-polish-local-daily-driver*
*Completed: 2026-05-13*
