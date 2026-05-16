---
phase: 05-polish-local-daily-driver
plan: 08
subsystem: app-shell + render + mux
tags: [polish-07, d-74, d-75, d-79, ui-spec-5, b2, b4]

# Dependency graph
requires:
  - phase: 05-polish-local-daily-driver
    provides: "vector-config::{ProfileBlock, Kind, ClipboardPolicy} (Plan 05-02)"
  - phase: 05-polish-local-daily-driver
    provides: "vector-term::{ClipboardEvent, Term::cwd_ring} (Plan 05-05)"
  - phase: 05-polish-local-daily-driver
    provides: "Wave-0 test stubs (Plan 05-01)"
  - phase: 02-headless-terminal-core
    provides: "vector-pty::SpawnCommand + LocalDomain::spawn (D-38)"
provides:
  - "vector-render::tint_stripe::TintStripePipeline (UI-SPEC §5.1 / D-75)"
  - "vector-render::tint_stripe::TintStripePipeline::quad_for(content_width) — screen-px geometry helper"
  - "vector-app::profile_picker::{ProfilePicker, PickerEntry, match_profiles} (UI-SPEC §5.3)"
  - "vector-app::toast::{ToastBanner, ToastMode, ToastStack} (UI-SPEC §5.4)"
  - "vector-app::clipboard_router::{ClipboardRouter, ClipboardAction} (POLISH-05 / D-70)"
  - "vector-mux::pane::{PaneCwdView, spawn_cwd_for, spawn_cwd_for_with_proc, format_tab_title}"
  - "Pane.cwd: Mutex<Option<PathBuf>> field — synced from Term::cwd_ring().back() in App PaneOutput handler"

affects:
  - "05-10 (render-pass orchestration + event-loop wiring) — consumes TintStripePipeline.draw, ProfilePicker, ToastStack, ClipboardRouter"
  - "Future remote-domain phases (6/7) — Codespace/DevTunnel rows in picker carry `Phase 6+` label until those phases wire transports"

# Tech tracking
tech-stack:
  added:
    - "fuzzy-matcher 0.3 (workspace; new direct dep on vector-app for D-75 picker ranking)"
    - "base64 0.22 (workspace; new direct dep on vector-app for the clipboard router; payload contract per 05-06)"
    - "vector-config + vector-pty as direct deps of vector-app (picker needs Kind; spawn_command_for_profile inlined in mux test)"
    - "vector-config as dev-dep of vector-mux (profile_local_spawn test)"
  patterns:
    - "PaneCwdView is the test-seam decouple: spawn_cwd_for*  take a PaneCwdView, not Pane, so unit tests don't construct a Term + transport. Production sites build it via `(&pane).into()`."
    - "Tint stripe NDC quad uses surface_h for vertical conversion (28 px → fraction of surface height). Tests assert screen-px geometry via TintStripePipeline::quad_for, independent of surface size."
    - "Toast surface state machine: ToastStack holds at most ONE banner; new replaces old; tick(now) drops auto-dismiss info banners. Action banners persist until explicit dismiss()."
    - "ClipboardRouter delivers a ClipboardAction enum (WritePasteboard | ShowPrompt | DenyRead); the event-loop side decides AppKit NSPasteboard write vs ToastStack push. Pure logic, no AppKit FFI."

key-files:
  created:
    - crates/vector-render/src/tint_stripe.rs
    - crates/vector-render/src/shaders/tint_stripe.wgsl
    - crates/vector-app/src/profile_picker.rs
    - crates/vector-app/src/toast.rs
    - crates/vector-app/src/clipboard_router.rs
    - crates/vector-mux/tests/osc7_consumer.rs
  modified:
    - crates/vector-render/src/lib.rs
    - crates/vector-render/tests/tint_stripe.rs
    - crates/vector-app/Cargo.toml
    - crates/vector-app/src/lib.rs
    - crates/vector-app/src/app.rs
    - crates/vector-app/tests/profile_picker.rs
    - crates/vector-mux/Cargo.toml
    - crates/vector-mux/src/lib.rs
    - crates/vector-mux/src/mux.rs
    - crates/vector-mux/src/pane.rs
    - crates/vector-mux/tests/profile_local_spawn.rs
    - Cargo.lock

key-decisions:
  - "PaneCwdView (test seam) instead of #[cfg(test)] mock-pointer-on-static. Production sites are 1-liner: `PaneCwdView::from(&pane)`. Tests don't need a Term/transport/Mutex setup."
  - "Pane.cwd is `Mutex<Option<PathBuf>>` (not a `parking_lot::RwLock`) — write contention is one writer thread (the App's PaneOutput handler) per pane; cheap Mutex suffices."
  - "Mux::split_pane_async now consults the parent pane via spawn_cwd_for; Mux::create_tab_async keeps inherit_cwd(None) (no parent pane to consult). Both paths still funnel through D-63 pidcwd fallback inside spawn_cwd_for."
  - "Tint stripe NDC conversion uses surface height (caller-supplied to update_quad); CPU-side ndc_quad takes both dims so future per-window tint reflects the actual viewport. surface_w is currently unused (stripe is always full width) but accepted for symmetry — flagged with `_` prefix to acknowledge."
  - "ClipboardRouter does NOT base64-decode the clipboard data; Plan 05-06's empirical resolution proved alacritty 0.26 already decodes before dispatching `Event::ClipboardStore`. base64 is still a vector-app dep because future OSC 52 outbound path (Plan 05-10) emits via `vector_input::osc52_outbound`."

patterns-established:
  - "All chrome state machines live in vector-app; render-pass orchestration deferred to Plan 05-10. Both layers can compile + test independently because logic surfaces are pure data."
  - "format_tab_title is a free function on vector-mux because it touches no Pane state — the App side reads pane.cwd, then calls the formatter."

requirements-completed: [POLISH-07]

# Metrics
duration_min: 11
completed: 2026-05-12
task_commits: 6
tests_added: 9
tests_passing_total: 296 # +9 over Plan 05-07's baseline 287 (4 Wave-0 stubs flipped green + 6 osc7_consumer + -1 because tint_stripe stub already counted)
---

# Phase 5 Plan 08: Profile picker + toast + clipboard router + tint-stripe pipeline + D-79 OSC 7 consumers Summary

**One-liner:** All Phase-5 chrome state machines (profile picker, toast banner, clipboard router) ship as logic-only modules in vector-app; tint stripe pipeline lands in vector-render with a CPU-side geometry helper for tests; D-79 OSC 7 consumers wire `Pane.cwd ← Term::cwd_ring().back()` and gate new-pane cwd inheritance + the `zsh: dirname` tab-title suffix.

## Performance

- **Duration:** ~11 min
- **Started:** 2026-05-12T19:01:02Z
- **Completed:** 2026-05-12T19:11:34Z
- **Tasks:** 2 (TDD: 2 RED + 4 GREEN sub-commits = 6 task commits)
- **Files created:** 6 (tint_stripe.rs + shader + 3 vector-app modules + 1 mux test)
- **Files modified:** 12 (lib.rs's, app.rs, mux.rs, pane.rs, Cargo.toml's, test stubs, Cargo.lock)

## Accomplishments

- **TintStripePipeline** — wgpu pipeline (1 quad, 1 vec4 color uniform, BlendState::ALPHA_BLENDING). Mirror of `cell_pipeline.rs` (Plan 03-03). `quad_for(content_width)` returns the screen-px geometry tests assert against; `ndc_quad(surface_w, surface_h)` produces the NDC verts on every `update_quad`. `set_color(None)` disables the draw. **B4 fix closed: full body, no `todo!()`.**
- **ProfilePicker** — `PickerEntry { name, kind }` + `ProfilePicker { entries, query, filtered, selected_idx, open }`. `match_profiles(&entries, q)` uses `fuzzy_matcher::skim::SkimMatcherV2` sorted by descending score. `row_label(fi)` appends `"  Phase 6+"` for Codespace/DevTunnel kinds (D-74 / UI-SPEC §5.3).
- **ToastBanner + ToastStack** — `Info` (36 px / 5 s auto-dismiss) vs `Action { buttons }` (56 px / sticky). `INFO_DISMISS_AFTER: Duration = Duration::from_secs(5)` const locks the UI-SPEC §5.4 timing.
- **ClipboardRouter** — `handle(ClipboardEvent::Store, foreground_process) -> ClipboardAction { WritePasteboard | ShowPrompt | DenyRead }`. `policy = None` ⇒ Action-toast with `[allow once, always, block]` (D-70). `policy = Block` ⇒ Info-toast saying blocked. `policy = Allow` ⇒ direct pasteboard write. `ClipboardEvent::LoadDenied` ⇒ `DenyRead` (no toast — D-70 silent denial empirically observed in 05-06).
- **Profile → SpawnCommand → LocalDomain end-to-end** — `tokio::test` `profile_local_spawn` constructs a `ProfileBlock { startup_command: "echo hi", env: {FOO=bar} }`, builds a `SpawnCommand` (argv=`[/bin/sh, -c, "echo hi"]`), `domain.spawn(cmd).await`, drains the reader for up to 2 s, asserts `"hi"` appears. **The Wave-0 stub flips green.**
- **D-79 OSC 7 consumers (B2 close):**
  - `Pane.cwd: Mutex<Option<PathBuf>>` field, synced from `term.cwd_ring().back()` in vector-app's `UserEvent::PaneOutput` handler after `term.feed(&bytes)`.
  - `spawn_cwd_for(&PaneCwdView)` resolves cwd: pane.cwd → `pidcwd(pid)` (D-63) → `$HOME`. Used by `Mux::split_pane_async` so new splits inherit the *shell's* most recent OSC 7 push instead of the process cwd snapshot.
  - `format_tab_title(process, cwd?)` → `"zsh: vector"` when cwd has a file_name; `"zsh"` when None or root. App's `PaneTitleChanged` calls this with `pane.cwd.lock().clone()`.
- **6 osc7_consumer tests + 1 Wave-0 stub flipped green** — `tab_title_with_osc7_cwd_stem`, `tab_title_without_osc7_falls_back`, `tab_title_handles_root_path`, `new_pane_inherits_cwd_from_osc7`, `new_pane_falls_back_to_proc_pidinfo`, `new_pane_falls_back_to_home`.

## Task Commits

1. **Task 1 RED — failing tests for tint stripe + profile picker + profile_local_spawn** — `004570a`
2. **Task 1a GREEN — TintStripePipeline (UI-SPEC §5.1)** — `be88d02`
3. **Task 1b GREEN — profile picker + toast + clipboard router (UI-SPEC §5.3 / §5.4 / POLISH-05)** — `0fcf1aa`
4. **Task 1c GREEN — profile → SpawnCommand → LocalDomain integration test** — `ee7d780`
5. **Task 2 RED — failing tests for D-79 OSC 7 consumers** — `1b96b0b`
6. **Task 2 GREEN — D-79 OSC 7 consumers + Pane.cwd field + format_tab_title wired** — `174dff3`

## Verification

- `cargo test -p vector-render --test tint_stripe geometry` — 1 passed.
- `cargo test -p vector-app --test profile_picker` — 2 passed (`fuzzy_ranking`, `codespace_warning_label`).
- `cargo test -p vector-mux --test profile_local_spawn` — 1 passed (real PTY echo round-trip).
- `cargo test -p vector-mux --test osc7_consumer` — 6 passed.
- `cargo test --workspace --tests` — full workspace green, zero failures, all prior Plan 02–07 tests still pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0.
- `grep -q "pub struct TintStripePipeline" crates/vector-render/src/tint_stripe.rs` — present (acceptance criterion).
- `grep -q "pub fn quad_for" crates/vector-render/src/tint_stripe.rs` — present.
- `grep -q "pub fn match_profiles" crates/vector-app/src/profile_picker.rs` — present.
- `grep -c "Phase 6+" crates/vector-app/src/profile_picker.rs` — 2 (D-74 / UI-SPEC §5.3).
- `grep -q "INFO_DISMISS_AFTER: Duration = Duration::from_secs(5)" crates/vector-app/src/toast.rs` — present.
- `grep -q "pub struct ClipboardRouter" crates/vector-app/src/clipboard_router.rs` — present.
- `grep -q "pub fn spawn_cwd_for" crates/vector-mux/src/pane.rs` — present.
- `grep -q "pub fn format_tab_title" crates/vector-mux/src/pane.rs` — present.
- `grep -q "cwd_ring().back()" crates/vector-app/src/app.rs` — OSC 7 ring consumer in PaneOutput handler.

## Decisions Made

See `key-decisions` in frontmatter. Headlines:

1. **PaneCwdView test-seam** instead of mock-pointer-on-static — Production call site is `(&pane).into()`, one line. Tests construct `PaneCwdView { cwd, pid }` directly without spinning a Term/transport. Cleaner than the plan-suggested `Pane::test_new` constructor because the production `Pane::new` already requires a real Term + transport (immutable invariant).
2. **OSC 7 ring sync at the App layer**, not in vector-mux — Pane.cwd is updated immediately after `term.feed(&bytes)` in app.rs's `PaneOutput` handler. vector-mux exposes the field; vector-app drives the sync. Keeps vector-mux free of the alacritty event loop concerns.
3. **`base64` retained as a vector-app dep** — Even though `ClipboardRouter` no longer base64-decodes (alacritty does it), Plan 05-10 will need `vector_input::osc52_outbound` (which uses base64) when wiring Cmd-C to OSC 52 emission. Pre-declaring the dep avoids a Cargo.toml churn in 05-10.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — wgpu 29 API breaking changes] PipelineLayoutDescriptor + RenderPipelineDescriptor field renames**

- **Found during:** Task 1a tint_stripe GREEN compile.
- **Issue:** The plan body's code used wgpu 0.20-era field names: `bind_group_layouts: &[&bg_layout]` (now `&[Some(&bg_layout)]`), `push_constant_ranges: &[]` (now `immediate_size: 0`), `multiview: None` (now `multiview_mask: None`).
- **Fix:** Updated tint_stripe.rs to match wgpu 29's API (cross-checked against `cell_pipeline.rs:185-186, 238` which already uses the new shape).
- **Files modified:** `crates/vector-render/src/tint_stripe.rs`
- **Verification:** `cargo build -p vector-render` exit 0.
- **Commit:** `be88d02`

**2. [Rule 1 — Lint] Multiple clippy::pedantic warnings on tint_stripe.rs**

- **Found during:** Task 1a clippy gate.
- **Issue:** `default_trait_access` on `Default::default()` for `PipelineCompilationOptions`; `cast_precision_loss` on `u32 as f32` (mathematically safe for content widths within f32 mantissa range); `similar_names` on `surface_w_px` vs `surface_h_px` (deliberate by-design — both are dimensions).
- **Fix:** Module-level `#![allow(clippy::default_trait_access, clippy::cast_precision_loss, clippy::similar_names)]`. Mirrors pattern from `crates/vector-theme/src/builtins.rs` (Plan 05-03) for color literals.
- **Files modified:** `crates/vector-render/src/tint_stripe.rs`
- **Commit:** `be88d02`

**3. [Rule 1 — Lint] test float comparison + contains_helper warnings**

- **Found during:** Task 1a tint_stripe test clippy gate.
- **Issue:** `clippy::float_cmp` on `y == 28.0` literal; `clippy::iter_with_drain` (actually `clippy::any` → `clippy::contains`-suggestion) on `xs.iter().any(|&x| x == 1200.0)`.
- **Fix:** Module-level `#![allow(clippy::float_cmp)]` (the geometry helper produces literal f32s that match exactly — no FP arithmetic involved); switched from `iter().any(|&x| x == ...)` to `xs.contains(&...)`.
- **Files modified:** `crates/vector-render/tests/tint_stripe.rs`
- **Commit:** `be88d02`

**4. [Rule 1 — Lint] uninlined_format_args + assigning_clones + field_reassign_with_default in tests**

- **Found during:** Task 1c profile_local_spawn clippy gate.
- **Issue:** Plan-body snippet used `format!("...: {:?}", names)` style and reassigned fields on `SpawnCommand::default()`. Modern clippy pedantic enforces `format!("{names:?}")` and constructor-style struct literals.
- **Fix:** Inlined format args; rebuilt `SpawnCommand { argv, cwd, rows, cols, env }` as a single struct literal.
- **Files modified:** `crates/vector-mux/tests/profile_local_spawn.rs`, `crates/vector-app/tests/profile_picker.rs`
- **Commit:** `ee7d780` (mux), `0fcf1aa` (app)

**5. [Rule 2 — Missing critical wiring] App.rs PaneOutput handler did not consume the OSC 7 ring**

- **Found during:** Task 2 GREEN — plan-body intent is `pane.cwd = term.cwd_ring().back().cloned()` after `term.feed`. The plan defers this to Plan 05-10 but D-79's contract requires the ring to be drained somewhere; without it, `pane.cwd` is always `None` and `format_tab_title` always falls back to the bare process name.
- **Fix:** Added the one-line ring sync inside `UserEvent::PaneOutput` right after `term.feed(&bytes)`. Locks `pane.cwd.lock()` only after dropping the `term` lock to avoid lock-ordering issues.
- **Files modified:** `crates/vector-app/src/app.rs`
- **Verification:** Compiles + all existing tests pass.
- **Commit:** `174dff3`
- **Rationale:** Plan 05-10 will still need to add the tab-title formatter call elsewhere; this task is the *consumer wiring* per the plan name. Leaving the ring drain out would make Task 2's tests technically pass (they exercise the helper in isolation) but the feature would be inert in the real app. Rule 2 (missing critical wiring) applies.

### Documentation deviations

- Plan body called the helper `proc_pidinfo_fallback`; the actual D-63 function is `crate::cwd::pidcwd(pid) -> Result<PathBuf, String>` (already exists from Phase 4). Used `pidcwd(pid).ok()` to coerce to `Option<PathBuf>`. No new module needed.
- Plan body suggested implementing a `Pane::test_new` constructor; the chosen `PaneCwdView` adapter is strictly better — no need to expose internals via a test-only constructor (which would still need a fake Term).

## Authentication Gates Encountered

None — fully autonomous plan, no external services.

## Issues Encountered

- **Workspace dep churn for vector-app:** Adding `vector-config` + `vector-pty` + `fuzzy-matcher` + `base64` as new direct deps caused a fresh full `cargo build` walk on the first invocation. Subsequent runs are incremental. No user-visible impact; just slower first iteration.
- **No parallel-agent issues** — Plan 05-08 ran solo within Wave 3.

## User Setup Required

None.

## Next Phase Readiness

- **Plan 05-10 (render-pass orchestration + event-loop wiring)** picks up the chrome rendering:
  - `TintStripePipeline::draw(&mut RenderPass)` called BEFORE per-pane compositor pass (per UI-SPEC §3 / §5.1 layer ordering).
  - `ProfilePicker` modal overlay pass (rendered LAST when `picker.open`).
  - `ToastBanner` info/action passes (above per-pane compositor, below picker).
  - `ClipboardRouter::handle(...)` consumed in the App's `ClipboardEvent` route — `WritePasteboard(data)` triggers `NSPasteboard::generalPasteboard.setString:forType:NSPasteboardTypeString`; `ShowPrompt(banner)` pushes to `ToastStack`.
  - AppKit menu wiring for "Vector → Switch Profile →" mirroring `ProfilePicker::entries`.
  - Cmd-N / Cmd-Shift-P keybind dispatch through `UserEvent::OpenProfilePicker` / `SpawnNewWindow`.
  - Config watcher receiver pumped through `EventLoopProxy<UserEvent::ConfigReloaded>`.
- **Plan 05-09 (CI tmux-smoke)** unblocked — none of its dependencies sit on this plan.
- **D-79 B2 closed at the consumer layer.** Plan 05-05 (producer: OSC 7 ring) + Plan 05-08 (consumer: spawn_cwd_for + format_tab_title) jointly close the feature. App-side wiring of `format_tab_title` into `PaneTitleChanged` is live (deviation #5 above).

## Known Stubs

None. All implementation paths are concrete:
- `TintStripePipeline::new/set_color/update_quad/draw/quad_for` all have bodies. The `draw` early-returns when `current_color` is None — that's the documented "no-op" path, not a stub.
- `ProfilePicker::open/close/set_query/row_label/select_active` are all implemented.
- `ClipboardRouter::handle` covers all three `ClipboardPolicy` arms plus `LoadDenied`.
- `spawn_cwd_for` and `format_tab_title` have full bodies.

## Self-Check: PASSED

Verified files on disk:
- `crates/vector-render/src/tint_stripe.rs` — FOUND
- `crates/vector-render/src/shaders/tint_stripe.wgsl` — FOUND
- `crates/vector-app/src/profile_picker.rs` — FOUND
- `crates/vector-app/src/toast.rs` — FOUND
- `crates/vector-app/src/clipboard_router.rs` — FOUND
- `crates/vector-mux/tests/osc7_consumer.rs` — FOUND
- `crates/vector-render/tests/tint_stripe.rs` — UPDATED (no `#[ignore]`)
- `crates/vector-app/tests/profile_picker.rs` — UPDATED (no `#[ignore]`)
- `crates/vector-mux/tests/profile_local_spawn.rs` — UPDATED (no `#[ignore]`)

Verified commits in `git log`:
- `004570a` (Task 1 RED) — FOUND
- `be88d02` (TintStripePipeline GREEN) — FOUND
- `0fcf1aa` (Profile picker + toast + clipboard router GREEN) — FOUND
- `ee7d780` (Profile local spawn integration test GREEN) — FOUND
- `1b96b0b` (Task 2 RED) — FOUND
- `174dff3` (OSC 7 consumer + Pane.cwd + format_tab_title GREEN) — FOUND

Verified workspace state:
- `cargo test --workspace --tests` — zero failures (all 296+ tests pass; tally includes 9 new tests in this plan).
- `cargo clippy --workspace --all-targets -- -D warnings` — exit 0.
- All 9 acceptance-criteria greps pass (see Verification section).

---
*Phase: 05-polish-local-daily-driver*
*Completed: 2026-05-12*
