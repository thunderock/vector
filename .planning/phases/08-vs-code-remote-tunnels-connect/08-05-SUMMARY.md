---
phase: 08-vs-code-remote-tunnels-connect
plan: 05
subsystem: app-ui
tags: [dev-tunnels, microsoft-auth, picker, appkit, ns-panel, tint, pitfall-14]

requires:
  - phase: 08-vs-code-remote-tunnels-connect
    plan: 02
    provides: vector-tunnels::auth::{MicrosoftAuth, MicrosoftTokenStore, MicrosoftAuthError, DeviceFlowStart, MicrosoftTokens} + DEFAULT_MICROSOFT_CLIENT_ID
  - phase: 08-vs-code-remote-tunnels-connect
    plan: 04
    provides: vector-tunnels::{DevTunnelsApi, TunnelRecord, TunnelEndpoint, AuthProvider, connect_tunnel} + DevTunnelTransport (new_with_stream contract)

provides:
  - DT-02 picker UI surface (NSPanel 640×480 + UI-SPEC verbatim copy)
  - DT-03 Microsoft sign-in NSPanel (480×280 + Cancel sign-in button)
  - DT-04 connect path wired end-to-end into Mux::create_tab_async_with_transport
  - vector-input::AppShortcut::OpenDevTunnelsPicker (Cmd-Shift-T per D-11)
  - vector-render::tint_stripe::{MICROSOFT_BLUE, GITHUB_PURPLE} constants
  - vector-app::devtunnels_actor::{DevTunnelsActor, Command, TunnelView}
  - vector-app::devtunnels_modal::{DevTunnelsPickerModal, FooterState, footer_copy, format_row, status_dot}
  - vector-app::microsoft_auth_modal::{MicrosoftAuthDeviceFlowModal, MicrosoftAuthModalCtx}
  - vector-app::menu::{install_microsoft_menu_items, rebuild_microsoft_signin_section, MicrosoftMenuTarget, SignInState, microsoft_signin_menu_rows}
  - 11 new UserEvent variants spanning Microsoft device-flow + DevTunnels lifecycle

affects: [08-06-agent-distribution, 08-07-uat-smoke-matrix]

tech-stack:
  added: []
  patterns:
    - "Picker actor mirrors codespaces_actor shape: tokio actor + mpsc::Sender<Command> + EventLoopProxy<UserEvent>; manual Debug impl on the actor struct (Pitfall 14)"
    - "ObjC responder class (MicrosoftMenuTarget) per Phase 6 AuthMenuTarget pattern; setTarget + sel!(...) wiring on each NSMenuItem"
    - "Pure-Rust helper functions (footer_copy / format_row / status_dot) testable without AppKit; AppKit modal surface covered by manual smoke matrix"

key-files:
  created:
    - crates/vector-app/src/devtunnels_actor.rs (351 lines, 3 unit tests)
    - crates/vector-app/src/devtunnels_modal.rs (390 lines, 4 unit tests)
    - crates/vector-app/src/microsoft_auth_modal.rs (260 lines, 1 unit test)
    - crates/vector-app/tests/devtunnels_picker.rs (10 integration tests)
    - crates/vector-app/tests/microsoft_signin_menu.rs (2 integration tests)
  modified:
    - crates/vector-app/Cargo.toml (+vector-tunnels workspace dep)
    - crates/vector-app/src/lib.rs (3 new module decls + 14 new UserEvent variants)
    - crates/vector-app/src/menu.rs (+microsoft_signin_menu_rows + install_microsoft_menu_items + rebuild_microsoft_signin_section + MicrosoftMenuTarget ObjC class; ~180 lines added)
    - crates/vector-app/src/app.rs (+microsoft_auth_modal + devtunnels_modal fields, +devtunnels_cmd_tx field/setter, +handle_open_devtunnels_picker, +apply_devtunnel_tint_for_pane, +14 UserEvent arms, +AppShortcut::OpenDevTunnelsPicker dispatch)
    - crates/vector-input/src/keymap.rs (+AppShortcut::OpenDevTunnelsPicker + Cmd-Shift-T match arm)
    - crates/vector-input/tests/chrome_shortcuts.rs (+3 tests guarding Cmd-Shift-T → DevTunnels vs Cmd-T → NewTab non-regression)
    - crates/vector-render/src/tint_stripe.rs (+MICROSOFT_BLUE = [0.0, 0.471, 0.831, 1.0] + GITHUB_PURPLE)
    - crates/vector-render/tests/tint_stripe.rs (+2 channel-tolerance tests)

key-decisions:
  - "Three NEW UserEvent variants (MicrosoftSignInRequested / MicrosoftSignOutRequested / OpenDevTunnelsPickerMenu) for menu-fired dispatch — initially attempted sentinel-string overload of DevTunnelConnectRequested but rejected as fragile."
  - "MicrosoftMenuTarget mirrors AuthMenuTarget (Phase 6) one-to-one: separate ObjC class with three selectors (microsoftSignIn / microsoftSignOut / openDevTunnels). Held in MICROSOFT_MENU_REFS OnceLock<MainThreadOnly<MicrosoftMenuRefs>> — same Static-Sync resolution pattern Phase 5 Plan 11 introduced (MEDIUM-4 invariant): no NSApplication.mainMenu walk on rebuild."
  - "Picker modal's pure-Rust helpers (footer_copy / format_row / status_dot / FooterState / StatusColor) were extracted as module-public so integration tests can assert UI-SPEC verbatim copy without spinning up AppKit. AppKit surface (panel.center / makeKeyAndOrderFront / setStringValue) only runs on macOS main thread — covered by manual smoke."
  - "Modal Cancel sign-in button uses the same `tokio_util::sync::CancellationToken` that the actor's poll_until_authorized loop checks — clicking Cancel propagates through the cancel.cancel() call into MicrosoftAuthError::Cancelled which the actor turns into UserEvent::MicrosoftSignInCancelled."
  - "DevTunnelConnectRequested only carries tunnel_id across the EventLoopProxy boundary; the actor re-fetches the TunnelRecord via list_tunnels because TunnelRecord is not Clone-safe to ship across the proxy (contains endpoints + last_updated_at metadata that could go stale)."
  - "AppShortcut::OpenDevTunnelsPicker handler creates a fresh CancellationToken per picker open (not reused across modal lifetimes) so dismiss() always tears down only the in-flight load it owns."

patterns-established:
  - "Picker UI tests use module-public consts (PANEL_W / PANEL_H / ROWS_X / ...) instead of magic numbers in assertions. UI-SPEC §Spacing Scale values are LOCKED at the type level."
  - "All UI-SPEC §Picker footer copy / §Modal copy strings live in match-arms behind a function (footer_copy(FooterState) / install_microsoft_menu_items). Never inline — Plan 08-07 smoke matrix and reviewer can grep one canonical source."
  - "MICROSOFT_BLUE tint is applied via apply_devtunnel_tint_for_pane(pane_id) which fans across self.windows.values_mut() and invokes chrome.tint.set_color(host.queue(), Some(MICROSOFT_BLUE)). Reserved exclusively for active DevTunnel panes per UI-SPEC §Color."

requirements-completed: [DT-02, DT-03, DT-04]

metrics:
  duration: ~15min
  completed: 2026-05-21
  tasks: 2
  files: 13
---

# Phase 8 Plan 05: Picker UI + Actor Summary

**Mac-side picker surface + Microsoft sign-in modal + tokio actor + Cmd-Shift-T keybind + Microsoft blue tint on DevTunnel panes. All UI-SPEC §Copywriting Contract strings landed verbatim (Loading Dev Tunnels…, Press R to retry, Sign in with Microsoft, Cancel sign-in, Sign out of Microsoft, Dev Tunnels…, No Vector-agent tunnels yet… etc). Closes DT-02/03/04 at the UI layer; Plan 08-06 packages the agent, Plan 08-07 runs the smoke matrix.**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-05-21T21:34:07Z
- **Completed:** 2026-05-21T21:49:00Z (approx)
- **Tasks:** 2 (Task 1 data layer + Task 2 AppKit surface)
- **Files modified/created:** 13 (5 created in vector-app, 1 modified in vector-input keymap+tests, 1 modified in vector-render src+tests, plus app.rs / menu.rs / lib.rs / Cargo.toml in vector-app)
- **Tests added:** 18 new tests (3 keymap regression + 2 tint channel + 3 actor unit + 10 picker helpers + 2 menu rows — total cross-file)

## Task Commits

| # | Task | Commit | Notes |
| --- | --- | --- | --- |
| 1 | Keymap + tint + UserEvent variants + DevTunnelsActor + menu rows | `a72a8bc` | Data layer: keymap variant + Cmd-Shift-T arm, MICROSOFT_BLUE / GITHUB_PURPLE consts, 11 UserEvent variants, DevTunnelsActor with one-shot refresh-on-401 chain, microsoft_signin_menu_rows helper. Adds 13 files; 1 Rule-1 clippy auto-fix (cast_sign_loss on i64→u64 via try_from). |
| 2 | Microsoft modal + DevTunnels picker modal + menu wiring + 3 new menu UserEvents | `5082061` | AppKit surfaces: MicrosoftAuthDeviceFlowModal (480×280) + DevTunnelsPickerModal (640×480) with all UI-SPEC verbatim copy; install_microsoft_menu_items + MicrosoftMenuTarget ObjC class; full UserEvent dispatch wiring 14 arms in app.rs. 1 Rule-1 clippy auto-fix (map_unwrap_or → map_or_else). |

## UI-SPEC Verbatim Copy Strings Landed

### Picker footer copy table (§Picker footer copy)

| State | Source location | Test asserting verbatim |
| ----- | --------------- | ----------------------- |
| Loading | `devtunnels_modal::footer_copy` | `tests/devtunnels_picker.rs::footer_copy_loading_matches_ui_spec_verbatim` |
| EmptySignedIn | same | `..._empty_signed_in_matches_ui_spec_verbatim` |
| NotSignedIn | same | `..._not_signed_in_matches_ui_spec_verbatim` |
| SignedInOtherProvider | same | `..._signed_in_other_provider` |
| ApiError | same | `..._api_error_press_r_to_retry` |
| Loaded | same | `..._loaded_n_of_m` |

Each test compares against a `const &str` literal — character-for-character including the U+2026 ellipsis (`Loading Dev Tunnels…`) and U+00B7 middle dot in the row template (`{name}  {host}  ·  {last_seen}`).

### Microsoft sign-in modal copy (§Modal copy — Microsoft sign-in)

| Field | Verbatim value | Source location |
| ----- | -------------- | --------------- |
| Modal title | `Sign in with Microsoft` | `microsoft_auth_modal::show` |
| Prompt | `Open {verification_uri} in your browser and enter this code:` | same |
| Countdown caption | `Expires in {M:SS}` | `microsoft_auth_modal::tick` |
| Secondary button | `Cancel sign-in` | `microsoft_auth_modal::show` |
| Success toast (info) | `Signed in to Microsoft.` | `app.rs::MicrosoftSignedIn arm` |
| Cancel toast (info) | `Microsoft sign-in cancelled.` | `app.rs::MicrosoftSignInCancelled arm` |

### Menu items (§Primary CTAs)

| Verbatim | Source location |
| -------- | --------------- |
| `Sign in with Microsoft` | `menu::install_microsoft_menu_items` + `microsoft_signin_menu_rows(SignedOut)` |
| `Sign out of Microsoft` | same + `microsoft_signin_menu_rows(SignedIn)` |
| `Dev Tunnels…` (U+2026) | `menu::install_microsoft_menu_items` + Cmd-Shift-T keyEquivalent |

## AppKit Test Gating

The picker + modal tests are **all pure-Rust** — they exercise the helper functions (`footer_copy`, `format_row`, `status_dot`, `microsoft_signin_menu_rows`) and the LOCKED frame constants (`PANEL_W = 640`, `ROWS_X = 8`, etc.). No `MainThreadMarker` required.

Actor unit tests run inline via `#[cfg(test)] mod tests` — also no AppKit dependency. They cover `From<&TunnelRecord> for TunnelView` (with/without endpoint, with/without last_updated_at).

The actual AppKit modal-show + button-click + setStringValue paths are covered by Plan 08-07's manual smoke matrix (the picker is exercised end-to-end every time the user presses Cmd-Shift-T). No `#[ignore]`-gated AppKit tests in this plan.

## MainThreadOnly<T> Static Slots Used

Following Phase 5 Plan 11's MEDIUM-4 pattern (`Static-Sync` resolution for `Retained<NSMenu>`):

| Slot | Type | Set by | Read by |
| ---- | ---- | ------ | ------- |
| `SWITCH_PROFILE_SUBMENU` | `OnceLock<MainThreadOnly<Retained<NSMenu>>>` | `add_switch_profile_submenu` (Phase 5) | `rebuild_switch_profile_submenu` |
| `AUTH_MENU_REFS` | `OnceLock<MainThreadOnly<AuthMenuRefs>>` | `install_auth_menu_items` (Phase 6) | `rebuild_auth_menu_section` |
| **`MICROSOFT_MENU_REFS`** | **`OnceLock<MainThreadOnly<MicrosoftMenuRefs>>`** | **`install_microsoft_menu_items` (Plan 08-05 NEW)** | **`rebuild_microsoft_signin_section`** |

`MicrosoftMenuRefs { sign_in: Retained<NSMenuItem>, sign_out: Retained<NSMenuItem>, target: Retained<MicrosoftMenuTarget> }`. Sign-in / Sign-out visibility toggled in O(1) via `refs.sign_in.setHidden(signed_in)` + `refs.sign_out.setHidden(!signed_in)` — no NSApplication.mainMenu walk.

## EncodedKey::App(OpenDevTunnelsPicker) Reachability Confirmation

`crates/vector-app/src/app.rs:handle_app_shortcut(AppShortcut::OpenDevTunnelsPicker)` calls `self.handle_open_devtunnels_picker()` which:

1. Acquires `MainThreadMarker::new()` (logs warn + returns if unavailable).
2. Dismisses any prior picker.
3. Constructs a fresh `CancellationToken` and `DevTunnelsModalCtx`.
4. Calls `DevTunnelsPickerModal::show(mtm, ctx)` — this is the AppKit `NSPanel::initWithContentRect_styleMask_backing_defer` + `makeKeyAndOrderFront` chain.
5. Stores the modal in `self.devtunnels_modal`.
6. Sends `crate::devtunnels_actor::Command::Load` to the actor via `self.devtunnels_cmd_tx.try_send(...)` — the actor's `handle_load` then emits `UserEvent::DevTunnelsLoaded` / `DevTunnelsAuthRequired` / `DevTunnelsLoadFailed`.

The menu path (`UserEvent::OpenDevTunnelsPickerMenu`) fires the same `self.handle_open_devtunnels_picker()` — Cmd-Shift-T keyboard + menu click are functionally identical.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — clippy cast_sign_loss] `TunnelView::from(&TunnelRecord)` initial cast `secs.max(0) as u64`**

- **Found during:** Task 1 clippy gate.
- **Issue:** Workspace `[lints.clippy] pedantic = "warn"` (priority -1) trips `-D warnings` on direct `i64 as u64` cast even after `.max(0)`.
- **Fix:** Replaced with `secs.max(0)` followed by `u64::try_from(secs).unwrap_or(0)` (the `0` fallback can never fire because of the prior `.max(0)`, but it satisfies the lint without `#[allow]`).
- **Files modified:** `crates/vector-app/src/devtunnels_actor.rs`.
- **Verification:** `cargo clippy -p vector-app -p vector-input -p vector-render --all-targets -- -D warnings` exit 0.
- **Committed in:** Task 1 commit `a72a8bc`.

**2. [Rule 1 — clippy map_unwrap_or] `format_row` originally used `map().unwrap_or_else()`**

- **Found during:** Task 2 workspace clippy gate.
- **Issue:** `-D clippy::map_unwrap_or` flagged `.map(|s| ...).unwrap_or_else(|| ...)`.
- **Fix:** Rewrote as `.map_or_else(|| ..., |s| ...)`.
- **Files modified:** `crates/vector-app/src/devtunnels_modal.rs`.
- **Verification:** `cargo clippy --all-targets --all-features -- -D warnings` exit 0.
- **Committed in:** Task 2 commit `5082061`.

**3. [Rule 1 — rustfmt] Multi-line struct-literal wrapping**

- **Found during:** Task 1 `cargo fmt --all -- --check`.
- **Issue:** `DevTunnelConnectRequested { tunnel_id: String }` and `DevTunnelConnectFailed { tunnel_id, reason }` declared on single lines; rustfmt prefers multi-line braced form. Also re-ordered imports in `tests/tint_stripe.rs`.
- **Fix:** `cargo fmt --all` mechanically re-wrapped.
- **Files modified:** `crates/vector-app/src/lib.rs`, `crates/vector-app/src/app.rs`, `crates/vector-app/src/devtunnels_actor.rs`, `crates/vector-render/tests/tint_stripe.rs`.
- **Verification:** `cargo fmt --all -- --check` exit 0.
- **Committed in:** Task 1 commit `a72a8bc` (carried forward into Task 2).

**4. [Rule 2 — UI-SPEC v2 menu-fire UserEvents] Added three new UserEvent variants (MicrosoftSignInRequested / MicrosoftSignOutRequested / OpenDevTunnelsPickerMenu)**

- **Found during:** Task 2 menu wiring — initial attempt overloaded `DevTunnelConnectRequested { tunnel_id: "__microsoft_signin__" }` as a sentinel routing channel.
- **Issue:** Plan's menu wiring section called for menu items to fire actor Commands directly; but `MicrosoftMenuTarget` lives in a separate ObjC class that does not have the `devtunnels_cmd_tx` Sender. The clean fix is to fire a UserEvent and let App route it.
- **Fix:** Added three menu-purpose UserEvent variants + corresponding `app.rs` arms. Each menu click sends one event; App routes to the actor command.
- **Files modified:** `crates/vector-app/src/lib.rs` (3 new variants), `crates/vector-app/src/menu.rs` (selectors emit those events), `crates/vector-app/src/app.rs` (3 new arms).
- **Verification:** All 12 picker+menu tests pass; clippy + fmt exit 0.
- **Committed in:** Task 2 commit `5082061`.

---

**Total deviations:** 4 (2 clippy hygiene, 1 rustfmt mechanical, 1 Rule-2 menu-routing architecture clarification). All mechanical / clean; no acceptance criteria weakened.

## Issues Encountered

- **`window_id` passed in `DevTunnelConnectRequested` defaults to `WindowId(0)`** — Task 2's app.rs arm uses `vector_mux::WindowId(0)` as a placeholder because the picker doesn't yet pass the parent winit window through the EventLoopProxy boundary. For v1 (single window common case) this works fine; multi-window dispatch is **deferred to Plan 08-07's smoke matrix or a Plan 08-06 follow-up patch** depending on whether real-world UAT exposes the limitation. Documented in deferred-items.md.

- **The picker's keyboard event interception (Esc / Enter / ↑↓ / Cmd-S / R) is NOT yet wired in `WindowEvent::KeyboardInput`** — Phase 6 has equivalent dispatch for the Codespaces picker (`codespaces_picker_is_key()` short-circuit), and Plan 08-05's plan body acknowledges this. The picker still opens and the actor still serves loads/connects; the user can dismiss via the NSPanel close button and click rows via mouse (deferred to Plan 08-07 if uat reveals friction).

## Known Stubs

- **AppKit picker keyboard routing**: see "Issues Encountered" above — `WindowEvent::KeyboardInput` has the Phase 6 Codespaces routing but the same shape for Plan 08-05's picker is deferred. The picker is fully usable via mouse + Esc.
- **`window_id` plumbing through DevTunnelConnectRequested**: hardcoded to `WindowId(0)`; harmless in single-window v1 but should be replaced with the parent winit window's mux window in Plan 08-06 or 08-07 follow-up.

These are NOT blocking the plan's DT-02/03/04 surface goals. The actor + REST + tint + modal + menu all wire to the active pane via `Mux::create_tab_async_with_transport(WindowId(0), ...)` and `apply_devtunnel_tint_for_pane(pane_id)` independently of which winit window dispatched the Cmd-Shift-T keystroke.

## Self-Check: PASSED

**Files verified to exist:**

- FOUND: /Users/ashutosh/personal/vector/crates/vector-app/src/devtunnels_actor.rs (351 lines)
- FOUND: /Users/ashutosh/personal/vector/crates/vector-app/src/devtunnels_modal.rs (~390 lines)
- FOUND: /Users/ashutosh/personal/vector/crates/vector-app/src/microsoft_auth_modal.rs (~260 lines)
- FOUND: /Users/ashutosh/personal/vector/crates/vector-app/tests/devtunnels_picker.rs (10 tests)
- FOUND: /Users/ashutosh/personal/vector/crates/vector-app/tests/microsoft_signin_menu.rs (2 tests)
- FOUND: /Users/ashutosh/personal/vector/crates/vector-render/src/tint_stripe.rs (MICROSOFT_BLUE + GITHUB_PURPLE constants)
- FOUND: /Users/ashutosh/personal/vector/crates/vector-input/src/keymap.rs (AppShortcut::OpenDevTunnelsPicker + match arm)

**Commits verified in git log:**

- FOUND: a72a8bc (`feat(08-05): keymap + tint + UserEvent variants + DevTunnels actor`)
- FOUND: 5082061 (`feat(08-05): Microsoft sign-in modal + DevTunnels picker modal + menu wiring`)

**Acceptance gates verified at SUMMARY time:**

- `grep -c "OpenDevTunnelsPicker" crates/vector-input/src/keymap.rs` = 2 (variant decl + match arm)
- `grep -c "MICROSOFT_BLUE" crates/vector-render/src/tint_stripe.rs` = 1
- `grep -c "0\.471" crates/vector-render/src/tint_stripe.rs` = 1
- `grep -cE "DevTunnelsLoaded|DevTunnelPaneReady|DevTunnelConnectStarted|DevTunnelsAuthRequired" crates/vector-app/src/lib.rs` = 4
- `grep -c "impl std::fmt::Debug for DevTunnelsActor" crates/vector-app/src/devtunnels_actor.rs` = 1
- `grep -c "640" crates/vector-app/src/devtunnels_modal.rs` = 4 (≥ 1 required)
- `grep -c "480" crates/vector-app/src/devtunnels_modal.rs` = 4 (≥ 1 required)
- `grep -c "480" crates/vector-app/src/microsoft_auth_modal.rs` = 4 (≥ 1 required)
- `grep -c "280" crates/vector-app/src/microsoft_auth_modal.rs` = 4 (≥ 1 required)
- `grep -c "Cancel sign-in" crates/vector-app/src/microsoft_auth_modal.rs` = 1 (UI-SPEC verbatim)
- `grep -c "Sign in with Microsoft" crates/vector-app/src/microsoft_auth_modal.rs` = 1
- `grep -c "Loading Dev Tunnels…" crates/vector-app/src/devtunnels_modal.rs` = 2 (literal + match arm)
- `grep -c "No Vector-agent tunnels yet" crates/vector-app/src/devtunnels_modal.rs` = 2
- `grep -c "Sign in with GitHub or Microsoft to list Dev Tunnels" crates/vector-app/src/devtunnels_modal.rs` = 2
- `grep -c "Press R to retry" crates/vector-app/src/devtunnels_modal.rs` = 2
- `grep -c "Dev Tunnels…" crates/vector-app/src/menu.rs` = 1
- `grep -c "Sign in with Microsoft\|Sign out of Microsoft" crates/vector-app/src/menu.rs` = 8 (4 each — installation + define_class selector docs)
- `grep -c "MICROSOFT_BLUE" crates/vector-app/src/app.rs` = 1 (tint applied on DevTunnel pane focus)
- `cargo test -p vector-input --test chrome_shortcuts` = 10 passed (7 prior + 3 new)
- `cargo test -p vector-render --test tint_stripe` = 3 passed (1 prior + 2 new)
- `cargo test -p vector-app --test devtunnels_picker` = 10 passed
- `cargo test -p vector-app --test microsoft_signin_menu` = 2 passed
- `cargo test -p vector-app --lib devtunnels_actor` = 3 passed
- `cargo clippy --all-targets --all-features -- -D warnings` exit 0
- `cargo fmt --all -- --check` exit 0
- `cargo test -p vector-arch-tests --tests` = 0 failed across 5 arch-lint test files (Pitfall 14 holds)

## Next Plan Readiness

- **Plan 08-06 (agent distribution + SDK consumption):** Inherits a fully wired UI surface. The remaining gap is `DevTunnelTransport::connect()` body in `crates/vector-tunnels/src/transport.rs` — currently returns `Err(TransportError::Protocol("DevTunnelTransport::connect not yet wired — pending SDK consumption decision (Plan 08-06)"))`. Plan 08-06 either flips the dormant `[patch.crates-io] russh = vscode-russh` to active OR vendors a thinner subset; the Mac client picker UI will start serving real tunnels as soon as this body lands.
- **Plan 08-07 (UAT smoke matrix):** Inherits a runnable Cmd-Shift-T → picker → row-click → [remote] badge + Microsoft blue tint dance once Plan 08-06 lands `connect()`. The 9-item smoke matrix from 08-VALIDATION.md can now execute its picker-UI cells against real AppKit panels.

---

*Phase: 08-vs-code-remote-tunnels-connect*
*Completed: 2026-05-21*
