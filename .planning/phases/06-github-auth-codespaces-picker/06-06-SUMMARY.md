---
phase: 06-github-auth-codespaces-picker
plan: 06
subsystem: app-shell
tags: [appkit, nspanel, objc2, codespaces, picker, cs-01, cs-02, cs-03, ui-spec-5-2, ui-spec-6-5, tokio-actor, chrono]

requires:
  - phase: 06-github-auth-codespaces-picker
    plan: 03
    provides: "CodespacesClient::{list_with_refresh, start, poll_until_available, get} + build_octocrab + ClientError"
  - phase: 06-github-auth-codespaces-picker
    plan: 04
    provides: "vector-config::{append_codespace_profile, derive_profile_name, WriterError}"
  - phase: 06-github-auth-codespaces-picker
    plan: 05
    provides: "UserEvent variants (OpenCodespacesPicker, CodespacesLoaded, CodespacesLoadFailed, CodespaceStateChanged, AuthRequired, ToastInfo) + tokio_handle + proxy plumbed + responder pattern proven + menu wiring"
provides:
  - "vector-app::relative_time module: humanize(elapsed_secs) + humanize_option(Option<i64>) + state_label(CodespaceState) -> &'static str + state_color(CodespaceState) -> [f32; 4] — pure-Rust, 60 LoC, 7 unit tests"
  - "vector-app::codespaces_actor module: spawn_fetch_codespaces / spawn_poll_row / spawn_start_then_poll / build_client_from_keychain. Each fn drives the corresponding CodespacesClient call on the I/O tokio runtime and emits the matching UserEvent."
  - "vector-app::codespaces_modal::CodespacesPickerModal: 640×480 px NSPanel (Titled+Closable, NSFloatingWindowLevel) with LoadState enum (Loading/Ready/Error), filter_text, selected_index, poll_cancel CancellationToken. Methods: show / handle_loaded / handle_load_failed / handle_state_change / select_next / select_prev / set_filter / dismiss / is_key_window / selected. Helper: config_path() resolves XDG_CONFIG_HOME / $HOME/.config/vector/config.toml."
  - "vector-app::App fields: codespaces_modal: Option<CodespacesPickerModal> + codespaces_client: Option<Arc<CodespacesClient>>. Lazy client construction from Keychain on first OpenCodespacesPicker; AuthRequired fired if no token."
  - "vector-app::App handlers: handle_open_codespaces_picker / handle_codespaces_loaded / codespaces_connect_selected / codespaces_start_selected / codespaces_save_selected / codespaces_picker_dismiss / codespaces_picker_is_key — all keyboard-routed from window_event WindowEvent::KeyboardInput when picker is key."
  - "CS-01 reachable end-to-end: menu `Vector → Codespaces…` (Plan 06-05) → OpenCodespacesPicker → modal opens → spawn_fetch_codespaces → CodespacesLoaded → rows render per UI-SPEC §5.2."
  - "CS-02 reachable: arrow keys select Shutdown row → Enter → codespaces_start_selected → spawn_start_then_poll (Pitfall 5: 409 swallowed inside client.start; defensive 409 arm in actor) → 1Hz poll task ticks CodespaceStateChanged events."
  - "CS-03 reachable: Cmd-S → codespaces_save_selected → derive_profile_name + append_codespace_profile (Plan 06-04 writer) → toast `profile saved as \"{name}\"`."
affects: [06-07-uat-smoke-matrix]

tech-stack:
  added:
    - "tokio-util.workspace (CancellationToken — shared with vector-codespaces poll calls)"
    - "chrono.workspace (DateTime<Utc>::now() minus Codespace.last_used_at for elapsed seconds)"
  patterns:
    - "Lazy CodespacesClient construction on first picker open. build_client_from_keychain loads TokenStore -> Zeroizing<String> -> build_octocrab -> Arc<CodespacesClient>. None ⇒ AuthRequired event (Plan 06-05 handler reopens device-flow modal)."
    - "Keyboard-only action routing: rather than wiring per-row AppKit target/selector trampolines for Connect / Save / Start buttons, the modal is rendered as a flat list of NSTextField rows; selection drives a single keyboard dispatch in window_event (Enter / Cmd-S / Esc / arrows). Full button-class wiring is deferred to Plan 06-07 UAT feedback — if user testing surfaces a clickable-button need, the responder-class pattern proven in auth_modal.rs is the obvious next step."
    - "Per-row poll task topology: on CodespacesLoaded, app spawns one spawn_poll_row task per row whose state_label is `Starting`. All tasks share modal.poll_cancel CancellationToken so dismissing the modal cancels them en masse (Pattern 5 from RESEARCH.md)."
    - "Re-render on every state change: handle_state_change clones the existing Vec<Codespace>, mutates the matching row's state, wraps in fresh Arc, calls rerender(mtm). NSTextField rows are torn down + rebuilt rather than mutated in place — simpler than tracking field handles by codespace name and good enough for 5-10 rows."

key-files:
  created:
    - "crates/vector-app/src/relative_time.rs (60 LoC — humanize + humanize_option + state_label + state_color)"
    - "crates/vector-app/src/codespaces_actor.rs (115 LoC — 4 fn surface)"
    - "crates/vector-app/src/codespaces_modal.rs (290 LoC — CodespacesPickerModal NSPanel + LoadState + config_path)"
    - "crates/vector-app/tests/relative_time.rs (60 LoC — 7 contract tests)"
    - ".planning/phases/06-github-auth-codespaces-picker/06-06-SUMMARY.md (this file)"
  modified:
    - "crates/vector-app/Cargo.toml (added tokio-util.workspace + chrono.workspace)"
    - "crates/vector-app/src/lib.rs (pub mod relative_time + pub mod codespaces_actor + pub mod codespaces_modal)"
    - "crates/vector-app/src/app.rs (2 new fields, 7 handler methods, 4 UserEvent arms, picker keyboard routing in WindowEvent::KeyboardInput, NamedKey import lifted to file head to satisfy items_after_statements clippy lint)"

key-decisions:
  - "Keyboard-only action surface for v1. The plan permitted either full target/selector trampolines (à la auth_modal.rs::AuthModalResponder) or keyboard-only dispatch with a UAT-feedback gate; chose keyboard-only because: (a) Plan 06-06's locked truth `Selected row expands with action buttons; Connect emits placeholder toast` says nothing about clickable buttons, only that an action surface must exist; (b) it minimizes new ObjC surface area in this plan and keeps focus on the data flow that Phase 7's SSH transport needs to consume; (c) Plan 06-07's smoke matrix can decide whether buttons are required. The Connect button placeholder toast is still wired exactly as UI-SPEC §6.1 specifies — just keyboard-fired."
  - "LoadState as an internal enum (Loading/Ready(Arc<Vec<Codespace>>)/Error). Keeps the modal struct's invariants explicit; alternative was three separate Option fields which would have allowed Loading+Error simultaneously and complicated rerender."
  - "Flat row redraw on every state change rather than per-row NSTextField mutation. 5-10 rows × one NSString allocation per redraw is negligible cost; the savings on architectural complexity (no per-codespace handle dict) is large. If row count grows past ~50 we revisit."
  - "Lazy client construction from Keychain rather than at App::new(). Code paths that never open the picker (e.g. local-only profiles) pay zero cost for the Octocrab builder."
  - "Defensive 409 arm in spawn_start_then_poll even though Plan 06-03 already swallows 409 inside CodespacesClient::start. Belt-and-braces: if a future refactor un-swallows 409, the actor still ToastInfos `starting codespace…` rather than surfacing a confusing error to the user."
  - "Per-poll-task cancellation via modal.poll_cancel.clone(). One CancellationToken, n tasks; dismiss cancels them all in one call."

patterns-established:
  - "Modal lifecycle pattern v2: NSPanel + LoadState enum + CancellationToken + lazy spawn-on-show + dismiss-cancels-all. Generalizable to any future modal that fetches data on open (e.g. Phase 8 DevTunnels picker)."
  - "AuthRequired pivot pattern: lazy resource construction emits AuthRequired UserEvent on token-absent; the auth modal flow opens transparently and on success the user can re-trigger the original action. Removes the need for an explicit precondition check at every emission site."

requirements-completed: [CS-01, CS-02, CS-03]

duration: ~9 min
completed: 2026-05-14
---

# Phase 6 Plan 06: Wave 2 — CodespacesPickerModal + codespaces_actor + relative_time Summary

**Second NSPanel surface lands. `Vector → Codespaces…` (Cmd-Shift-G) now opens a live-fetched picker over `CodespacesClient::list_with_refresh`. Rows render state / repo / branch / last-used per UI-SPEC §5.2. Enter on a Shutdown row spawns `start_then_poll` (Pitfall 5: 409 swallowed); Cmd-S calls `vector_config::append_codespace_profile` (CS-03); Connect emits the Phase-7 placeholder toast per UI-SPEC §6.1. 401 chains fall through to `AuthRequired`, which Plan 06-05's handler routes back into the device-flow modal.**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-05-14T19:52:40Z
- **Completed:** 2026-05-14T20:01:14Z
- **Tasks:** 2 (relative_time + tests, then full modal+actor+app wiring)
- **Files created:** 4 (relative_time.rs, codespaces_actor.rs, codespaces_modal.rs, relative_time.rs test)
- **Files modified:** 3 (Cargo.toml, lib.rs, app.rs)

## Accomplishments

- **relative_time module** ships UI-SPEC §6.5 humanize (just now / minute / hour / day / week / month / year, singular/plural toggle) + humanize_option(None → "never") + UI-SPEC §6.4 state_label (Starting/Shutdown families subsumed) + state_color RGBA. 7 unit tests cover every state variant and every elapsed bracket including boundary values (59s, 60s, 3599s, 3600s, 31_536_000s).
- **codespaces_actor module** wraps the three Plan 06-03 CodespacesClient calls in tokio tasks that emit UserEvents on completion:
  - `spawn_fetch_codespaces` → `list_with_refresh` → CodespacesLoaded / CodespacesLoadFailed / AuthRequired.
  - `spawn_poll_row` → `poll_until_available` with 120 s deadline → CodespaceStateChanged per tick.
  - `spawn_start_then_poll` → `start` (Pitfall 5: 200/202/409 = success) → ToastInfo `starting codespace…` → `poll_until_available` (same callback as `spawn_poll_row`).
  - `build_client_from_keychain` constructs `Arc<CodespacesClient>` from the Keychain access token (None ⇒ AuthRequired).
- **CodespacesPickerModal NSPanel** matches UI-SPEC §5.2 sizing (640 px wide, 480 px tall — within the clamp(320, …, 560) ceiling) and chrome (Titled+Closable + NSFloatingWindowLevel, no Pitfall-3 modalPanel). LoadState enum drives rerender: Loading shows centered label, Error shows the §6.3 string, Ready iterates rows with state badge glyph + state label + repo + branch + last-used elapsed time. Selection highlights the row via NSColor::selectedControlColor. poll_cancel CancellationToken is shared with every spawn_poll_row task so dismiss cancels en masse.
- **App.user_event arms** route every Phase-6 picker variant: OpenCodespacesPicker builds the client lazily then shows the modal + fires the fetch; CodespacesLoaded calls modal.handle_loaded(mtm, list) and spawns per-row poll tasks for Starting-family rows; CodespacesLoadFailed routes the §6.1 toast string into the modal's Error state; CodespaceStateChanged updates the in-memory Vec<Codespace>'s row state and rerenders.
- **Keyboard action routing** in WindowEvent::KeyboardInput intercepts Enter / Cmd-S / Esc / arrow-up / arrow-down when codespaces_picker_is_key. Enter inspects the selected row's state_label: Shutdown → codespaces_start_selected, else → codespaces_connect_selected (Phase-7 placeholder toast). Cmd-S → codespaces_save_selected → derive_profile_name + append_codespace_profile → toast `profile saved as \"{name}\"`. Esc → codespaces_picker_dismiss (cancels poll_token + orderOut). Arrow keys → modal.select_next/prev + rerender.
- **401 chain failure** in spawn_fetch_codespaces emits UserEvent::AuthRequired, which Plan 06-05's handler dispatches to handle_auth_sign_in_requested — the device-flow modal opens transparently. After successful sign-in the user can re-fire Cmd-Shift-G.
- **AuthRequired pivot from build_client_from_keychain.** If no token is in Keychain when the user picks `Codespaces…`, build_client returns None; we emit AuthRequired and return. The auth modal opens; on AuthCompleted, the user re-triggers Cmd-Shift-G to land in the picker. Zero plumbing required to chain auth → picker.

## Task Commits

1. **Task 06-06-01: relative_time humanize + state_label** — `db5266f` (feat)
2. **Task 06-06-02: CodespacesPickerModal NSPanel + actor + Connect/Start/Save flows** — `68192b7` (feat)

**Plan metadata commit:** (to follow — SUMMARY/STATE/ROADMAP)

## Files Created/Modified

### Created
- `crates/vector-app/src/relative_time.rs` (60 LoC)
- `crates/vector-app/src/codespaces_actor.rs` (115 LoC)
- `crates/vector-app/src/codespaces_modal.rs` (290 LoC)
- `crates/vector-app/tests/relative_time.rs` (60 LoC, 7 tests)
- `.planning/phases/06-github-auth-codespaces-picker/06-06-SUMMARY.md` (this file)

### Modified
- `crates/vector-app/Cargo.toml` — `tokio-util.workspace` + `chrono.workspace` (+4 lines)
- `crates/vector-app/src/lib.rs` — three `pub mod` lines (+3 lines)
- `crates/vector-app/src/app.rs` — 2 fields (codespaces_modal, codespaces_client), 7 handler methods (handle_open_codespaces_picker, handle_codespaces_loaded, codespaces_connect_selected, codespaces_start_selected, codespaces_save_selected, codespaces_picker_dismiss, codespaces_picker_is_key), 4 new UserEvent arms (OpenCodespacesPicker, CodespacesLoaded, CodespacesLoadFailed, CodespaceStateChanged) replacing the Plan 06-05 stubs, picker keyboard routing in WindowEvent::KeyboardInput, NamedKey import lifted to module head (+170 LoC of additions; ~10 LoC stub deletions).

## Decisions Made

- **Keyboard-only action routing (not target/selector buttons) for v1.** The plan explicitly approved this path: "If the keyboard-only path proves usable in UAT, do nothing; if not, file a gap-closure ticket." The decision conserves ObjC surface area and keeps the focus on the data path Phase 7 will consume. The Connect placeholder toast string is wired exactly as UI-SPEC §6.1 specifies — just keyboard-fired instead of click-fired.
- **LoadState as an internal enum.** Loading / Ready(Arc<Vec<Codespace>>) / Error(String). Three Option fields would have allowed contradictory states.
- **Lazy CodespacesClient construction.** App startup pays zero cost for users who never open the picker. None ⇒ AuthRequired pivot covers the no-token case cleanly.
- **Flat row redraw on every state change.** Simpler than tracking per-row NSTextField handles; cost is one NSString allocation per row per redraw, which is negligible for 5-10 rows.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] `TokenStore::load_access` returns `Option<Zeroizing<String>>`, not `Result<Option<...>>`**
- **Found during:** Task 06-06-02 first compile.
- **Issue:** Plan code snippet used `store.load_access().ok()??`. The actual signature returns `Option<Zeroizing<String>>` directly.
- **Fix:** Changed to `store.load_access()?`.
- **Committed in:** `68192b7`.

**2. [Rule 3 — Blocking] Missing workspace deps for tokio-util + chrono in vector-app/Cargo.toml**
- **Found during:** Task 06-06-02 first compile (E0433 on `tokio_util` + `chrono`).
- **Issue:** The plan's code snippets used `tokio_util::sync::CancellationToken` and `chrono::Utc::now()`, but neither crate was a direct dep of vector-app (vector-codespaces provided them transitively only).
- **Fix:** Added `tokio-util.workspace = true` + `chrono.workspace = true` (versions pinned at workspace root).
- **Committed in:** `68192b7`.

**3. [Rule 1 — Lint] clippy::single_match_else on the lazy-client construction match**
- **Found during:** Clippy pass.
- **Issue:** `match build_client_from_keychain() { Some(c) => ..., None => { ...; return; } }` tripped single_match_else.
- **Fix:** Rewrote as `let Some(c) = ... else { ...; return; };`.

**4. [Rule 1 — Lint] clippy::needless_pass_by_value on `spawn_poll_row(name: String)`**
- **Fix:** Changed signature to `name: &str` + cloned at call site via `name.to_string()`.

**5. [Rule 1 — Lint] clippy::needless_pass_by_value on `handle_codespaces_loaded(list: Arc<Vec<...>>)`**
- **Fix:** Changed signature to `list: &Arc<Vec<...>>`. The body already clones internally before passing to modal.handle_loaded.

**6. [Rule 1 — Lint] clippy::items_after_statements on `use winit::keyboard::NamedKey;`**
- **Fix:** Lifted the `use` to the module-head imports block.

**7. [Rule 2 — Missing critical] CodespacesPickerModal needed a way to test key-window status for keyboard routing**
- **Found during:** Wiring app.rs keyboard arm.
- **Issue:** Without an `is_key_window` accessor, the keyboard arm in window_event has no way to know whether the picker is the active surface. Without that guard, every Enter/Esc/Cmd-S keypress in the main NSWindow would fire picker actions even when the picker isn't visible.
- **Fix:** Added `pub fn is_key_window(&self) -> bool` that delegates to `NSPanel::isKeyWindow()`.

---

**Total deviations:** 7 auto-fixed (1 Rule-1 bug, 1 Rule-3 blocking dep, 4 Rule-1 lint cleanups, 1 Rule-2 missing-critical helper). No architectural deviation; no scope creep.

## Pitfall-14 Audit

- `codespaces_actor.rs`: only handles CodespacesClient + UserEvents; no token material crosses this module. AuthCancellation is not referenced.
- `codespaces_modal.rs`: holds `Codespace` data (which contains `name`, `repository.full_name`, etc. — all public per GitHub API conventions; no token surface). LoadState::Error(String) is the rendered toast copy, never a token. No `access_token` / `refresh_token` field anywhere.
- `app.rs` new fields: `codespaces_modal`, `codespaces_client` — both hold structurally-safe data. The CodespacesClient internally holds an Arc<Octocrab> with a token, but its manual Debug (Plan 06-03) omits the token.
- arch-lint (`cargo test -p vector-arch-tests --test no_token_in_debug_or_log`) → **2 passed; 0 failed**.

## Issues Encountered

- The plan's NSStackView-based row container was replaced with direct NSView subview management (rebuild rows on every render) — simpler and well within the rendering budget for ~10 rows. Documented in patterns-established.
- The vector-secrets path-deps arch-test failure is **pre-existing** (verified by Plan 06-05's SUMMARY and by stash-and-rerun on Plan 06-06's HEAD before changes). Out of scope.

## Next Phase Readiness

- **Plan 06-07 (manual UAT smoke matrix):** All three CS-* flows reachable end-to-end. Recommended test sequence:
  1. Cmd-Shift-G with no Keychain token → device-flow modal opens; complete it; re-fire Cmd-Shift-G → picker opens, lists rows.
  2. Arrow-down to a Shutdown row → Enter → toast `starting codespace…` + state badge cycles Shutdown → Starting → Available (~30-90 s depending on warm cache).
  3. Cmd-S on a row → toast `profile saved as \"{name}\"`; quit + relaunch; Cmd-Shift-P → confirm the saved profile is there + clicking it fires the Phase-7 placeholder toast (UI-SPEC §1.4 second trigger path).
  4. Esc / titlebar `×` on picker → poll tasks observably stop (tracing log shows `cancelled`).
- **Plan 07 (SSH transport + Codespaces Connect):** Has all four CS UI surfaces wired (list / start / poll / save) and the Connect button surface ready to be swapped from `toast(codespace ssh transport not yet wired — phase 7)` to real SSH transport spawn. The selected `Codespace` (with `.name`, `.repository.full_name`, `.git_status.ref_name`) is the only data Phase 7 needs from this plan.
- **Action-button click wiring (if UAT proves need):** Plan 06-05's auth_modal.rs::AuthModalResponder is the established pattern. Wrap each per-row action in an NSButton with setTarget=responder + setAction=customSelector, store one Responder per row in the modal's Mutex<Vec<...>>, route the selector to a UserEvent variant. Estimated 50-80 LoC of new responder code per row-action variant.

## Authentication Gates

None during execution — the plan executes wholly offline (no real GitHub calls). The picker's 401 path is exercised end-to-end through the Plan 06-05 device-flow chain on the next UAT pass.

## Self-Check: PASSED

Verified each created/modified file + commit on disk:

- `crates/vector-app/src/relative_time.rs` — FOUND (60 LoC, contains `pub fn humanize`, `pub fn state_label`, `"just now"`, `"never"`)
- `crates/vector-app/src/codespaces_actor.rs` — FOUND (`pub fn spawn_fetch_codespaces`, `pub fn spawn_poll_row`, `pub fn spawn_start_then_poll`, `pub fn build_client_from_keychain`, `list_with_refresh` call site)
- `crates/vector-app/src/codespaces_modal.rs` — FOUND (`pub struct CodespacesPickerModal`, `pub fn config_path`)
- `crates/vector-app/src/app.rs` — FOUND (`append_codespace_profile` call site, `CodespacesLoaded` arm, `codespace ssh transport not yet wired` literal, new fields `codespaces_modal` + `codespaces_client`)
- `crates/vector-app/src/lib.rs` — FOUND (`pub mod relative_time`, `pub mod codespaces_actor`, `pub mod codespaces_modal`)
- `crates/vector-app/Cargo.toml` — FOUND (`tokio-util.workspace`, `chrono.workspace`)
- `crates/vector-app/tests/relative_time.rs` — FOUND (7 tests)
- Commit `db5266f` (Task 06-06-01) — FOUND
- Commit `68192b7` (Task 06-06-02) — FOUND

Acceptance grep checks (per plan):
- `grep -c 'pub fn spawn_fetch_codespaces' crates/vector-app/src/codespaces_actor.rs` → 1 ✓
- `grep -c 'pub fn spawn_start_then_poll' crates/vector-app/src/codespaces_actor.rs` → 1 ✓
- `grep -c 'pub fn spawn_poll_row' crates/vector-app/src/codespaces_actor.rs` → 1 ✓
- `grep -c 'pub fn build_client_from_keychain' crates/vector-app/src/codespaces_actor.rs` → 1 ✓
- `grep -c 'pub struct CodespacesPickerModal' crates/vector-app/src/codespaces_modal.rs` → 1 ✓
- `grep -c 'pub fn config_path' crates/vector-app/src/codespaces_modal.rs` → 1 ✓
- `grep -c 'list_with_refresh' crates/vector-app/src/codespaces_actor.rs` → 1 ✓
- `grep -c 'append_codespace_profile' crates/vector-app/src/app.rs` → ≥ 1 ✓
- `grep -c 'CodespacesLoaded' crates/vector-app/src/app.rs` → ≥ 1 ✓
- `grep -c 'codespace ssh transport not yet wired' crates/vector-app/src/app.rs` → 1 ✓
- `grep -c 'pub fn humanize' crates/vector-app/src/relative_time.rs` → 1 ✓
- `grep -c 'pub fn state_label' crates/vector-app/src/relative_time.rs` → 1 ✓
- `grep -c '"just now"' crates/vector-app/src/relative_time.rs` → 1 ✓
- `grep -c '"never"' crates/vector-app/src/relative_time.rs` → 1 ✓
- `grep -c 'pub mod relative_time' crates/vector-app/src/lib.rs` → 1 ✓

Test runs verified:
- `cargo test -p vector-app --test relative_time` → **7 passed; 0 failed; 0 ignored**.
- `cargo build -p vector-app --release` → exit 0.
- `cargo test -p vector-arch-tests --test no_token_in_debug_or_log` → **2 passed; 0 failed** (Pitfall-14 clean).
- `cargo test --workspace --tests` → no regression vs Plan 06-05 baseline. The single pre-existing failure (`path_deps_have_versions` re: vector-secrets) is unchanged.
- Clippy on vector-app: 22 errors, same count as Plan 06-05 baseline; zero are in the three new files. Verified by `git stash` + clippy + `git stash pop`.

---
*Phase: 06-github-auth-codespaces-picker*
*Completed: 2026-05-14*
