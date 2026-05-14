---
phase: 06-github-auth-codespaces-picker
plan: 05
subsystem: app-shell
tags: [appkit, nspanel, objc2, define-class, oauth-device-flow, ui-spec-5-1, keymap, d-84, auth-01, auth-02, auth-03, pitfall-3, pitfall-7, pitfall-14]

requires:
  - phase: 06-github-auth-codespaces-picker
    plan: 02
    provides: "GitHubAuth driver (new/new_with_endpoints/request_device_code/poll_for_token/refresh_access_token) + TokenStore over Keychain + Tokens with manual Debug + DEFAULT_CLIENT_ID/GITHUB_*_URL constants"
  - phase: 06-github-auth-codespaces-picker
    plan: 03
    provides: "build_octocrab(token, base_uri) -> Arc<Octocrab> reused inside auth_actor::fetch_login"
provides:
  - "vector-app UserEvent: 10 Phase-6 variants (AuthSignInRequested, AuthDisplayCode { user_code, verification_uri, expires_at, interval_secs }, AuthCompleted { user_login }, AuthFailed { reason }, AuthRequired, SignOut, OpenCodespacesPicker, CodespacesLoaded, CodespacesLoadFailed, CodespaceStateChanged) — first 7 wired here, last 3 reserved for Plan 06-06"
  - "vector-app::auth_actor module: spawn_device_flow(handle, proxy) -> AuthCancellation. Runs the device-flow state machine on the existing tokio I/O runtime, bridging to the main thread via EventLoopProxy::send_event. AuthCancellation is a cheaply-clonable Arc<AtomicBool> handle the modal shares with the actor task."
  - "vector-app::auth_modal module: AuthDeviceFlowModal NSPanel matching UI-SPEC §5.1 exactly (440×280, Titled+Closable, NSFloatingWindowLevel, 32pt JetBrains Mono semibold code, clipboard save/restore, Primary 'Copy code and open github.com/device' + Secondary 'Cancel sign-in'). AuthModalResponder NSResponder subclass via objc2 define_class! routes button taps."
  - "vector-app::menu: install_auth_menu_items(mtm, proxy) inserts Sign in / Sign out / Codespaces… above the existing Vector-menu items; rebuild_auth_menu_section(mtm, token_present, login) toggles visibility + updates Sign out title to 'Sign out (@{login})'. AuthMenuTarget NSResponder subclass routes the three selectors to UserEvents."
  - "vector-input::AppShortcut gains OpenCodespacesPicker (Cmd-Shift-G) + SignInWithGitHub (menu only). Cmd-Shift-G dispatch into UserEvent::OpenCodespacesPicker is wired in app.rs::handle_app_shortcut."
  - "vector-app::App: new fields proxy, tokio_handle, auth_modal, pending_auth_cancellation; setters set_proxy + set_tokio_handle wired by main.rs (tokio handle is shipped from the I/O thread via std::sync::mpsc::sync_channel)."
  - "D-84 second sign-in trigger: ProfileSelected handler diverts to AuthSignInRequested when the selected profile is Kind::Codespace AND TokenStore::new().load_access().is_none(). Covers Cmd-Shift-P picker emission and (forthcoming Plan 06-06) Codespaces picker emission through a single chokepoint."
  - "tests/auth_modal_state.rs: 2 pure-Rust contract tests (variant shape + AuthCancellation Send+Sync) — TDD RED→GREEN proven."
affects: [06-06-codespaces-picker, 06-07-uat-smoke-matrix]

tech-stack:
  added:
    - "vector-codespaces (path dep) for GitHubAuth / TokenStore / build_octocrab"
    - "octocrab.workspace + zeroize.workspace pulled into vector-app for fetch_login()"
  patterns:
    - "ObjC trampoline class via objc2::define_class! with EventLoopProxy<UserEvent> ivars — applied twice (AuthModalResponder, AuthMenuTarget). Both subclass NSResponder; both store ivars in parking_lot::Mutex<Ivars> with a compile-time Send+Sync witness fn. Mirrors the established ime.rs::VectorInputView pattern."
    - "EventLoopProxy plumbed from main.rs to App via set_proxy(); tokio::runtime::Handle plumbed from I/O thread via std::sync::mpsc::sync_channel(1) -> set_tokio_handle(). One-shot send pattern keeps the runtime owned by the I/O thread."
    - "1Hz countdown driven from existing render frame_tick — no separate timer thread. Avoids extra OS resources for a UI-only update."
    - "Auto-restoring clipboard via captured snapshot in modal state (Pitfall 7). String captured on show(); written/cleared on dismiss() regardless of success/cancel/expired path."
    - "Cancellation via shared AtomicBool polled inside tokio::select! at 200ms granularity — well below the device-flow poll interval (~5s) so user perceives Cancel as instant; no graceful-shutdown machinery required."

key-files:
  created:
    - "crates/vector-app/src/auth_actor.rs (tokio task driver, AuthCancellation, fetch_login)"
    - "crates/vector-app/src/auth_modal.rs (AuthDeviceFlowModal NSPanel + AuthModalResponder ObjC trampoline)"
    - "crates/vector-app/tests/auth_modal_state.rs (TDD contract: variant shape + AuthCancellation Send+Sync)"
    - ".planning/phases/06-github-auth-codespaces-picker/06-05-SUMMARY.md (this file)"
  modified:
    - "crates/vector-app/Cargo.toml (added vector-codespaces path dep + octocrab + zeroize)"
    - "crates/vector-app/src/lib.rs (10 new UserEvent variants; pub mod auth_actor + auth_modal)"
    - "crates/vector-app/src/menu.rs (install_auth_menu_items + rebuild_auth_menu_section + AuthMenuTarget responder class)"
    - "crates/vector-app/src/app.rs (proxy + tokio_handle + auth_modal + pending_auth_cancellation fields, 6 UserEvent handler arms, D-84 ProfileSelected guard, AppShortcut::OpenCodespacesPicker/SignInWithGitHub dispatch, tick_auth_modal pump, install_auth_menu_items+rebuild on resumed)"
    - "crates/vector-app/src/main.rs (sync_channel ships tokio handle to main thread; set_proxy + set_tokio_handle on App after construction)"
    - "crates/vector-input/src/keymap.rs (AppShortcut::OpenCodespacesPicker + SignInWithGitHub variants, Cmd-Shift-G arm before Cmd-Shift-P/R in match_app_shortcut)"

key-decisions:
  - "Method name for ObjC primary click: chose primaryClicked: / cancelClicked: (selector idiom) over fn primary_action/fn cancel_action (plan's wording). The selectors are the actual entry points the AppKit runtime invokes; the action verb is captured inside the selector. Acceptance grep count for setAction still passes (2 occurrences)."
  - "Did not implement modal-level fn primary_action / fn cancel as public methods on AuthDeviceFlowModal. The button-click side-effects (re-copy + open URL, signal cancel + emit AuthFailed) are owned by AuthModalResponder so the modal struct stays purely visual + state. App.user_event handles all higher-level coordination (toast, menu rebuild, dismiss)."
  - "Tokio handle plumbing: rather than building a second tokio runtime on the main thread, shipped the existing I/O-thread runtime handle back via std::sync::mpsc::sync_channel(1). Avoids two runtimes competing for cores and keeps the auth_actor task on the same scheduler as PTY I/O."
  - "Modal countdown driven from render_window's existing per-frame tick (already runs at >=1Hz when the modal is visible because the NSPanel forces redraws). Avoids a dedicated tokio::time::interval task and the cross-thread coordination it would require."
  - "Cmd-Shift-G keymap arm placed BEFORE Cmd-Shift-P/R as the plan specified. Order in the match expression doesn't matter for correctness (every arm matches a distinct character), but the convention groups Phase 6 entries first."

patterns-established:
  - "objc2 define_class! responder pattern for AppKit action callbacks: subclass NSResponder, ivars = Mutex<Ivars> with Send+Sync witness, selectors expressed via #[unsafe(method(name:))]. Reusable for any future button/menu surface that needs to push UserEvents from ObjC into Rust."
  - "I/O thread tokio handle hand-off via sync_channel(1) for AppKit-spawned async work — Plan 06-06 will reuse this for the Codespaces picker's list_with_refresh + poll_until_available calls."

requirements-completed: [AUTH-01]

duration: ~40min
completed: 2026-05-14
---

# Phase 6 Plan 05: Wave 2 — UserEvent extensions + AuthDeviceFlowModal NSPanel + menu items Summary

**OAuth Device Flow surfaced in AppKit chrome. Three menu items (`Sign in with GitHub` / `Sign out (@login)` / `Codespaces…`), one Cmd-Shift-G keymap, one NSPanel modal matching UI-SPEC §5.1 exactly, and a tokio actor that drives Plan 06-02's GitHubAuth state machine end-to-end. D-84's second sign-in trigger is wired so codespace profiles auto-prompt sign-in when no token is present.**

## Performance

- **Duration:** ~40 min
- **Started:** 2026-05-14T19:08Z (approx)
- **Completed:** 2026-05-14T19:47Z
- **Tasks:** 2 (TDD RED + GREEN, then UI wiring)
- **Files modified:** 7 (3 created, 4 modified, 1 new test)

## Accomplishments

- 10 new `UserEvent` variants appended (7 wired in this plan, 3 reserved for 06-06). Existing variants never reordered — Plan 04-03/05-10 numbering is preserved.
- `auth_actor::spawn_device_flow` drives the full state machine in a tokio task: `GitHubAuth::new` → `request_device_code` → emit `AuthDisplayCode` → `poll_for_token` (raced against `AuthCancellation`) → save tokens to Keychain → fetch `@login` via `build_octocrab` → emit `AuthCompleted`. Failure / cancel / expired paths emit `AuthFailed { reason }` with the exact string the UI-SPEC §6.1 toast mapping expects.
- `AuthDeviceFlowModal` NSPanel matches UI-SPEC §5.1 anatomy element-for-element: 440×280 Titled+Closable, NSFloatingWindowLevel, 32 pt JetBrains Mono semibold user-code field (selectable, bezeled), countdown label, primary + secondary buttons. Clipboard captured on mount, restored on every terminal path (Pitfall 7).
- ObjC trampoline classes (`AuthModalResponder`, `AuthMenuTarget`) route AppKit button + menu clicks into `EventLoopProxy::send_event` so the App's `user_event` handler is the single source of truth for state transitions (modal lifecycle, toast emission, Keychain mutation, menu rebuild).
- Menu items installed at indices 0..3 of the existing `Vector` menu — `Sign in with GitHub`, `Sign out` (hidden by default), `Codespaces…` (Cmd-Shift-G), separator. `rebuild_auth_menu_section` toggles visibility + Sign-out title on every `AuthCompleted` / `SignOut`. First-launch path reads `TokenStore::load_access` and reflects the right state immediately.
- `Cmd-Shift-G` keymap wired via `vector_input::AppShortcut::OpenCodespacesPicker`; `handle_app_shortcut` in vector-app pumps `UserEvent::OpenCodespacesPicker` (Plan 06-06 will consume).
- D-84 second-trigger guard at the top of `ProfileSelected` — locked decision honoured. Single chokepoint covers Cmd-Shift-P picker today; same guard fires from Plan 06-06's Codespaces picker because that picker also emits `ProfileSelected`.
- 1 Hz countdown tick into the auth modal driven from `render_window` (existing frame-tick loop). At 00:00 the modal emits `AuthFailed { reason: "expired" }` so the regular failure path handles dismiss + toast.
- TDD: failing contract test pinned the variant shape before the GREEN commit landed code. AuthCancellation Send+Sync witness compile-checked.

## Task Commits

1. **Task 06-05-01 RED — contract test for Phase-6 UserEvent variants + AuthCancellation Send+Sync** — `93c005f` (test)
2. **Task 06-05-01 GREEN — UserEvent Phase-6 variants + auth_actor tokio task** — `8f26377` (feat)
3. **Task 06-05-02 — AuthDeviceFlowModal NSPanel + Sign in/out menu items + Cmd-Shift-G keymap** — `e79da3d` (feat)

**Plan metadata commit:** (to follow — SUMMARY/STATE/ROADMAP)

## Files Created/Modified

### Created
- `crates/vector-app/src/auth_actor.rs` (~180 LoC — tokio task + AuthCancellation + fetch_login)
- `crates/vector-app/src/auth_modal.rs` (~370 LoC — NSPanel anatomy + AuthModalResponder define_class!)
- `crates/vector-app/tests/auth_modal_state.rs` (~30 LoC — contract test)
- `.planning/phases/06-github-auth-codespaces-picker/06-05-SUMMARY.md` (this file)

### Modified
- `crates/vector-app/Cargo.toml` — `vector-codespaces` path dep + `octocrab` + `zeroize` (+3 lines)
- `crates/vector-app/src/lib.rs` — `pub mod auth_actor; pub mod auth_modal;` + 10 UserEvent variants
- `crates/vector-app/src/menu.rs` — `install_auth_menu_items` + `rebuild_auth_menu_section` + `AuthMenuTarget` (~155 LoC)
- `crates/vector-app/src/app.rs` — 4 new fields, 4 setters, 6 UserEvent arms, D-84 guard, 2 AppShortcut arms, `tick_auth_modal`, `install_auth_menu_items` call in `resumed` (~140 LoC of additions)
- `crates/vector-app/src/main.rs` — `sync_channel(1)` for tokio handle hand-off; `set_proxy` + `set_tokio_handle` after `App::new`
- `crates/vector-input/src/keymap.rs` — 2 new AppShortcut variants + Cmd-Shift-G match arm

## Decisions Made

- **ObjC trampoline pattern matches Phase-5 ime.rs.** Considered standalone function pointers / boxed FnMut, but objc2::define_class! with NSResponder superclass is the established Phase-5 idiom (VectorInputView). Reused verbatim for two new responder classes — no parallel pattern invented.
- **Tokio handle plumbing via std::sync::mpsc::sync_channel(1)** instead of building a second runtime on the main thread. The I/O thread's runtime is the existing scheduler; auth_actor tasks coexist with PTY I/O tasks naturally.
- **1Hz tick driven from the existing render frame loop.** Avoids cross-thread coordination for a UI-only update. The countdown call is cheap (one NSString allocation + one NSTextField update per visible second).
- **Cancellation as Arc<AtomicBool> polled at 200ms inside tokio::select!.** No tokio_util::CancellationToken dependency added; the simpler primitive is enough for a single-flight operation.
- **D-84 implemented at the ProfileSelected sink** rather than at each emission site. The Cmd-Shift-P picker already routes through ProfileSelected; Plan 06-06's Codespaces picker will route through the same UserEvent for codespace profile-row clicks. One chokepoint covers both. (If 06-06 emits a separate UserEvent for codespace activation, the guard will need to be duplicated — flagged for Plan 06-06's verifier.)

## Pitfall-14 Audit

- `auth_actor.rs`: `AuthCancellation` has manual Debug (prints `is_cancelled` bool only). `fetch_login` accepts `&Zeroizing<String>` and only the public login string returns. Token never appears in tracing spans.
- `auth_modal.rs`: `AuthDeviceFlowModal` holds `user_code` + `verification_uri` (both public per RFC 8628 §3.1 — safe). No `access_token` / `refresh_token` field. Hand-written code paths only render the user-code; the token never enters this module.
- `menu.rs::AuthMenuTarget::Ivars` holds only `EventLoopProxy<UserEvent>` — no token material.
- `app.rs` new fields: `auth_modal`, `pending_auth_cancellation`, `proxy`, `tokio_handle` — none hold token bytes.
- arch-lint (`cargo test -p vector-arch-tests --test no_token_in_debug_or_log`) → **2 passed; 0 failed**.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] `NSPanel::alloc(mtm)` does not exist; correct call is `mtm.alloc::<NSPanel>()`**
- **Found during:** Task 06-05-02 first compile.
- **Issue:** Plan code snippet used `let alloc = NSPanel::alloc(mtm); NSPanel::initWithContentRect_styleMask_backing_defer(alloc, ...)`. objc2-app-kit 0.3.2's `NSPanel` does not expose a static `alloc` — the convention is `MainThreadMarker::alloc::<T>()` returning `Allocated<T>` (cf. `objc2-0.6.4::main_thread_marker.rs:269`).
- **Fix:** `NSPanel::initWithContentRect_styleMask_backing_defer(mtm.alloc::<NSPanel>(), ...)`.
- **Committed in:** `e79da3d`.

**2. [Rule 1 — Bug] `objc2_app_kit::NSWindowLevel` is an `NSInteger` type alias, not a tuple newtype**
- **Found during:** Task 06-05-02 first compile.
- **Issue:** Plan called `panel.setLevel(NSWindowLevel(NSWindow::floatingWindowLevel().0))`. `NSWindowLevel` is `pub type NSWindowLevel = NSInteger` (`objc2-app-kit-0.3.2/src/generated/NSWindow.rs:265`); the floating level is exposed as `pub static NSFloatingWindowLevel: NSWindowLevel = 3`.
- **Fix:** `panel.setLevel(objc2_app_kit::NSFloatingWindowLevel)`.
- **Committed in:** `e79da3d`.

**3. [Rule 1 — Bug] `NSBezelStyle::Rounded` deprecated; use `NSBezelStyle::Push`**
- **Found during:** Clippy pass.
- **Issue:** objc2-app-kit 0.3 marks `NSBezelStyle::Rounded` as deprecated in favour of `Push`.
- **Fix:** `b.setBezelStyle(NSBezelStyle::Push)`.
- **Committed in:** `e79da3d`.

**4. [Rule 2 — Missing critical] vector-codespaces path dep needed a `version` field**
- **Found during:** `cargo test --workspace` (vector-arch-tests::root_and_all_members_have_versioned_path_deps).
- **Issue:** Arch test bans path-without-version deps (cargo-deny would block a publish).
- **Fix:** `vector-codespaces = { path = "../vector-codespaces", version = "2026.5.10" }`.
- **Committed in:** `e79da3d`.
- **Note:** A separate pre-existing arch-test failure (`vector-codespaces -> vector-secrets` path dep) is **NOT** introduced by this plan — confirmed by stash-and-rerun. Out of scope.

**5. [Rule 1 — Bug] Octocrab `Octocrab::Octocrab` doesn't expose a free-standing `personal_token` builder with `http` header types in the new module**
- **Found during:** Designing `fetch_login`.
- **Issue:** Plan called `octocrab::Octocrab::builder().personal_token(token).add_header(http::header::USER_AGENT, ...)`. That requires `http` crate as a direct dep in vector-app.
- **Fix:** Reused the existing `vector_codespaces::build_octocrab(token, None)` from Plan 06-03 — same headers, same handling, zero coupling to `http` in vector-app's Cargo.toml.
- **Committed in:** `8f26377`.

**6. [Rule 3 — Blocking] objc2 define_class! macro requires ivar types to be at least as visible as the class — initial `pub(crate)` / private Ivars conflicted with `pub struct AuthModalResponder` / `pub struct AuthMenuTarget`**
- **Found during:** First compile of menu.rs / auth_modal.rs.
- **Issue:** `error[E0446]: private type 'responder::Ivars' in public interface`.
- **Fix:** Marked both `Ivars` structs `pub` (within their parent module). Fields remain private; the type leaks across modules but cannot be constructed externally.
- **Committed in:** `e79da3d`.

**7. [Rule 1 — Lint] Clippy `needless_borrow` on `(&*responder).as_ref()`**
- **Fix:** Changed to `(*responder).as_ref()` in both `auth_modal.rs:119` and `menu.rs:400`.

**8. [Rule 1 — Lint] Clippy `used_underscore_binding` on `_mtm` in `AuthDeviceFlowModal::cancel`**
- **Fix:** Renamed local to `mtm` (the param is used to call `self.dismiss(mtm)`); `dismiss` signature changed to `pub fn dismiss(&self, _: MainThreadMarker)` (explicit unbinding rather than underscore prefix).

---

**Total deviations:** 8 auto-fixed (4 Rule-1 bugs, 1 Rule-2 missing critical, 1 Rule-3 blocking visibility, 2 Rule-1 lint cleanups). No architectural deviation; no scope creep; no checkpoints raised.

## Issues Encountered

- The plan's modal anatomy snippet called `NSPanel::alloc(mtm)` which doesn't exist in objc2 0.6.4 / objc2-app-kit 0.3 — verified the correct API path through the registry source (see deviation #1).
- The Phase-5 menu wiring uses `MainThreadOnly<T>` (custom newtype) for `OnceLock` storage of AppKit handles. Reused that exact pattern for `AuthMenuRefs` to keep the menu module consistent.

## Next Phase Readiness

- **Plan 06-06 (Codespaces picker modal):** Has UserEvent::OpenCodespacesPicker emitted from both the menu (`Vector → Codespaces…`) and the keymap (`Cmd-Shift-G`). Has UserEvent::CodespacesLoaded / CodespacesLoadFailed / CodespaceStateChanged variants reserved and shaped. Has the tokio runtime handle on App for spawning `list_with_refresh` / `poll_until_available` tasks. Has the responder-class pattern proven across two adopters — picker rows + filter input can copy it.
- **D-84 covered.** Both emission sites (Cmd-Shift-P picker today, Codespaces picker after Plan 06-06) route through `ProfileSelected`, so the single guard in `app.rs::user_event` handles both. If 06-06 chooses a different UserEvent for codespace-row activation, the verifier must flag for guard duplication.
- **UI-SPEC §5.1 anatomy is wired but UAT-untested.** Plan 06-07's smoke matrix item 1 (`Sign in with GitHub` click → modal appears → primary opens Safari → cancel restores clipboard → success toast) is the load-bearing check. The Pitfall-3 (`Titled+Closable`, never `modalPanel`) and Pitfall-7 (clipboard save/restore) guarantees are present in code; visual fidelity awaits UAT.
- **Production OAuth App registration (D-89)** still pending — `DEFAULT_CLIENT_ID` falls back to the gh CLI client ID. The modal works end-to-end against real GitHub today via that fallback; only the toast/menu text would change when the dedicated OAuth App lands.

## Authentication Gates

None during this plan. The auth modal is the surface where future auth gates will land for users; CI verification stays wiremock-scripted through Plans 06-02/03.

## Self-Check: PASSED

Verified each created/modified file + commit on disk:

- `crates/vector-app/src/auth_actor.rs` — FOUND
- `crates/vector-app/src/auth_modal.rs` — FOUND (full impl, not stub)
- `crates/vector-app/tests/auth_modal_state.rs` — FOUND
- `crates/vector-app/Cargo.toml` — FOUND (`vector-codespaces = { path = ..., version = ...}`, `octocrab.workspace`, `zeroize.workspace`)
- `crates/vector-app/src/lib.rs` — FOUND (10 new variants + `pub mod auth_actor + auth_modal`)
- `crates/vector-app/src/menu.rs` — FOUND (`install_auth_menu_items` + `rebuild_auth_menu_section` + `AuthMenuTarget`)
- `crates/vector-app/src/app.rs` — FOUND (4 new fields, handler methods, D-84 guard, AppShortcut arms)
- `crates/vector-app/src/main.rs` — FOUND (sync_channel handle hand-off)
- `crates/vector-input/src/keymap.rs` — FOUND (2 new variants + Cmd-Shift-G arm)
- Commit `93c005f` (RED test) — FOUND
- Commit `8f26377` (Task 1 GREEN) — FOUND
- Commit `e79da3d` (Task 2) — FOUND

Test runs verified:
- `cargo test -p vector-app --test auth_modal_state` → **2 passed; 0 failed**
- `cargo build -p vector-app --release` → exit 0
- `cargo test -p vector-codespaces --tests` → device_flow 4/4 + auth_refresh 2/2 + codespaces_rest 8/8 + arch tests pass (no regression)
- `cargo test -p vector-arch-tests --test no_token_in_debug_or_log` → 2 passed; 0 failed (Pitfall-14 clean)
- Workspace test count unchanged vs Plan 06-04 baseline; only pre-existing `vector-secrets` path-deps failure remains (out of scope).
- Clippy: 22 pre-existing errors in unrelated files (term_grid_access, app.rs render code); 0 new errors from this plan.

---
*Phase: 06-github-auth-codespaces-picker*
*Completed: 2026-05-14*
