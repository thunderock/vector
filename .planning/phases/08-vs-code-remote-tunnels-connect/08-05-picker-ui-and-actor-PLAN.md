---
phase: 08-vs-code-remote-tunnels-connect
plan: 05
type: execute
wave: 3
depends_on: [02, 04]
files_modified:
  - crates/vector-app/src/microsoft_auth_modal.rs
  - crates/vector-app/src/devtunnels_modal.rs
  - crates/vector-app/src/devtunnels_actor.rs
  - crates/vector-app/src/menu.rs
  - crates/vector-app/src/app.rs
  - crates/vector-app/src/lib.rs
  - crates/vector-app/Cargo.toml
  - crates/vector-input/src/keymap.rs
  - crates/vector-render/src/tint_stripe.rs
  - crates/vector-app/tests/devtunnels_picker.rs
  - crates/vector-app/tests/microsoft_signin_menu.rs
autonomous: true
requirements:
  - DT-02
  - DT-03
  - DT-04
user_setup: []
must_haves:
  truths:
    - "Cmd-Shift-T opens DevTunnelsPickerModal (UI-SPEC Interaction Contract)"
    - "Vector menu shows 'Sign in with Microsoft' / 'Sign out of Microsoft' / 'Dev Tunnels...' items (UI-SPEC Surfaces S3)"
    - "Picker lists tunnels from vector-tunnels::api::list_tunnels filtered to vector-agent label, rows formatted per UI-SPEC Row copy template"
    - "Selecting a row + Enter calls vector_tunnels::domain::connect_tunnel and installs the transport via Mux::create_tab_async_with_transport"
    - "On successful connect: pane shows [remote] badge AND tab tint stripe is Microsoft blue #0078d4 for the active DevTunnel pane"
    - "Footer copy matches UI-SPEC Picker footer copy table verbatim"
    - "On 401 Unauthorized from API: actor auto-refreshes Microsoft token once; if still 401, surfaces 'Token rejected by Dev Tunnels API. Re-authenticate.' sticky toast"
  artifacts:
    - path: "crates/vector-app/src/microsoft_auth_modal.rs"
      provides: "MicrosoftAuthDeviceFlowModal NSPanel 480x280 32pt mono user_code countdown Cancel sign-in"
      min_lines: 80
    - path: "crates/vector-app/src/devtunnels_modal.rs"
      provides: "DevTunnelsPickerModal NSPanel 640x480 monospaced 13pt rows footer states"
      min_lines: 120
    - path: "crates/vector-app/src/devtunnels_actor.rs"
      provides: "tokio actor load_tunnels connect_tunnel emits UserEvent variants"
      min_lines: 100
    - path: "crates/vector-render/src/tint_stripe.rs"
      provides: "MICROSOFT_BLUE constant and helper to set tint on DevTunnel pane focus"
  key_links:
    - from: "vector-input::keymap"
      to: "EncodedKey::App(AppShortcut::OpenDevTunnelsPicker)"
      via: "Cmd-Shift-T match arm"
      pattern: "OpenDevTunnelsPicker"
    - from: "DevTunnelsPickerModal on_enter"
      to: "devtunnels_actor ConnectTunnel command"
      via: "EventLoopProxy UserEvent::DevTunnelConnectRequested"
      pattern: "DevTunnelConnectRequested|DevTunnelConnectStarted"
    - from: "devtunnels_actor handle_connect"
      to: "vector_mux::Mux::create_tab_async_with_transport"
      via: "Box dyn PtyTransport from vector_tunnels::domain::connect_tunnel"
      pattern: "create_tab_async_with_transport"
    - from: "TintStripePipeline::set_color"
      to: "MICROSOFT_BLUE 0.0 0.471 0.831 1.0"
      via: "applied when active pane has TransportKind::DevTunnel"
      pattern: "0\\.471"
---

<objective>
Wire the Mac-side picker UI, Microsoft sign-in modal, devtunnels_actor, keymap, menu items, and Microsoft-blue tint into the running app. Mirror Phase 6 CodespacesPickerModal + AuthDeviceFlowModal one-to-one against Phase 8 endpoints + copy per UI-SPEC.md.

Purpose: this is the last gate before DT-02/03/04 close. After this plan the user can Cmd-Shift-T then pick then see a remote shell with a blue tint and [remote] badge.
Output: 4 new files + 7 modified files; 10+ tests; Phase 8 manual smoke matrix becomes runnable.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/phases/08-vs-code-remote-tunnels-connect/08-CONTEXT.md
@.planning/phases/08-vs-code-remote-tunnels-connect/08-UI-SPEC.md
@crates/vector-app/src/codespaces_modal.rs
@crates/vector-app/src/codespaces_actor.rs
@crates/vector-app/src/auth_modal.rs
@crates/vector-app/src/menu.rs
@crates/vector-app/src/app.rs
@crates/vector-app/src/relative_time.rs
@crates/vector-app/src/toast.rs
@crates/vector-input/src/keymap.rs
@crates/vector-render/src/tint_stripe.rs
@crates/vector-tunnels/src/api.rs
@crates/vector-tunnels/src/model.rs
@crates/vector-tunnels/src/domain.rs
@crates/vector-tunnels/src/auth/device_flow_microsoft.rs

<interfaces>
From Phase 6 / vector-app:
- CodespacesPickerModal: NSPanel 640x480, monospaced 13pt rows, footer NSTextField, floating window level, CancellationToken for poll cancel.
- AuthDeviceFlowModal: NSPanel, 32pt mono user_code, countdown field, "Cancel sign-in" button.
- codespaces_actor::spawn_*: tokio actor + EventLoopProxy<UserEvent>.
- relative_time::humanize / state_label / state_color: last-seen rendering + status colors.
- toast::ToastBanner / ToastStack / ToastMode: Info (5s) vs Action (sticky).

From vector-input::keymap (Phase 5 Plan 05-13/14):
- AppShortcut enum currently has: SpawnNewWindow, ToggleSearch, OpenProfilePicker, ReloadConfig, OpenCodespacesPicker (Phase 6).
- EncodedKey: Pty(Vec<u8>), Mux(MuxCommand), App(AppShortcut).
- match_app_shortcut(key, mods) is the per-shortcut matcher. Phase 8 adds the Cmd-Shift-T arm AND the OpenDevTunnelsPicker variant.

From vector-tunnels (Plans 08-02 + 08-04):
- MicrosoftAuth::start_device_flow returns DeviceFlowStart with user_code, verification_uri, expires_in, interval.
- MicrosoftAuth::poll_until_authorized(dc, interval, expires_in, cancel) returns MicrosoftTokens.
- MicrosoftTokenStore: save, load, clear.
- DevTunnelsApi::list_tunnels(auth) returns Vec<TunnelRecord>.
- vector_tunnels::domain::connect_tunnel(api, auth, tunnel, rows, cols) returns Box<dyn PtyTransport>.
- TunnelRecord::display_name / is_vector_agent / last_updated.

From UI-SPEC.md (locked):
- Picker frame: 640x480; rows container x=8 y=32 w=624 h=416; footer y=4 h=24.
- Row height: 22px, SF Mono 13pt.
- Microsoft sign-in modal: 480x280.
- Microsoft blue tint: #0078d4 = RGBA [0.0, 0.471, 0.831, 1.0].
- Footer copy table: verbatim strings (see UI-SPEC §Picker footer copy).
- Row template: status_dot, tunnel_display_name, host, "·", last_seen.
- Keybinds: Cmd-Shift-T toggle, Esc close, Enter connect, Cmd-S save profile, R retry, ↑↓ move.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Keymap shortcut + Microsoft tint + UserEvent variants + devtunnels_actor</name>
  <files>crates/vector-input/src/keymap.rs, crates/vector-render/src/tint_stripe.rs, crates/vector-app/src/devtunnels_actor.rs, crates/vector-app/src/lib.rs, crates/vector-app/Cargo.toml, crates/vector-app/src/app.rs, crates/vector-app/tests/microsoft_signin_menu.rs</files>
  <read_first>
    - crates/vector-input/src/keymap.rs (entire file — must understand AppShortcut + EncodedKey + match_app_shortcut layout before extending)
    - crates/vector-app/src/codespaces_actor.rs (entire file — devtunnels_actor mirrors structure)
    - crates/vector-app/src/app.rs (locate UserEvent enum — add new variants)
    - crates/vector-render/src/tint_stripe.rs (TintStripePipeline::set_color + draw — confirm RGBA[4] format)
    - crates/vector-app/Cargo.toml (current deps — add vector-tunnels + chrono if not present)
  </read_first>
  <behavior>
    - Test 1 (keymap): match_app_shortcut on Key::Character("T") with Mods { cmd:true, shift:true } returns Some(AppShortcut::OpenDevTunnelsPicker); same for lowercase "t".
    - Test 2 (keymap negative): Cmd-T alone (no shift) returns MuxCommand::NewTab not the new shortcut. Phase 4 binding must not regress.
    - Test 3 (tint constant): vector_render::tint_stripe::MICROSOFT_BLUE equals approximately [0.0_f32, 0.471, 0.831, 1.0] within 0.01 tolerance per channel.
    - Test 4 (actor: load): given an injected DevTunnelsApi (with_base_url=wiremock) returning 2 vector-agent tunnels, the actor receives Load and emits UserEvent::DevTunnelsLoaded(Vec<TunnelView>) with 2 entries; on Err(Unauthorized) emits UserEvent::DevTunnelsAuthRequired.
    - Test 5 (actor: connect — mocked): given an injected connector closure returning Box<dyn PtyTransport> (a vector_pty-like mock), the actor receives Connect(TunnelId) and calls Mux::create_tab_async_with_transport via injected adapter, then emits UserEvent::DevTunnelPaneReady with window/tab/pane ids.
    - Test 6 (Microsoft sign-in menu items present): vector_app::menu::microsoft_signin_menu_rows(state) returns rows containing "Sign in with Microsoft" when signed-out AND "Sign out of Microsoft" when signed-in.
  </behavior>
  <action>
    Step 1 — crates/vector-input/src/keymap.rs: extend AppShortcut and match_app_shortcut.
    Add variant `OpenDevTunnelsPicker` to the AppShortcut enum (after `OpenCodespacesPicker` if present, otherwise at the end before any `#[doc]` items). Add an arm in `match_app_shortcut`:
    "If mods.cmd && mods.shift && !mods.alt && !mods.ctrl AND the key character matches 't' or 'T', return Some(AppShortcut::OpenDevTunnelsPicker)."
    Critical: this arm must be AFTER the match_mux_command call but BEFORE encode_pty, AND must NOT match Cmd-T (no-shift) — that is MuxCommand::NewTab from Phase 4. Use the same `character_shortcut` helper Phase 5 Plan 05-13 used for Cmd-Shift-P / Cmd-Shift-R.

    Step 2 — crates/vector-render/src/tint_stripe.rs: add constants at the top of the file (or alongside existing constants):
    `pub const MICROSOFT_BLUE: [f32; 4] = [0.0, 0.471, 0.831, 1.0];` — Phase 8 D-17 / UI-SPEC. Reserved exclusively for active DevTunnel pane tint.
    `pub const GITHUB_PURPLE: [f32; 4] = [0.478, 0.227, 0.686, 1.0];` — Phase 6 legacy (dormant in v1).
    If these constants already exist elsewhere, do NOT duplicate; refer to the existing ones.
    Add ONE unit test in the existing tests module (or create one):
    Assert MICROSOFT_BLUE equals [0.0, 0.471, 0.831, 1.0] within 0.01 tolerance per channel (computed from #0078d4: 0x00=0.0, 0x78=120/255=0.4706, 0xd4=212/255=0.8314).

    Step 3 — crates/vector-app/Cargo.toml: add deps if missing:
    - `vector-tunnels = { workspace = true }` (added at workspace level by Plan 08-01)
    - `chrono = { workspace = true }` for last-seen humanize

    Step 4 — crates/vector-app/src/app.rs: locate the `enum UserEvent { ... }` (already has Phase 6 codespace variants). Add Phase 8 variants:
    - `MicrosoftDeviceFlowStarted { user_code: String, verification_uri: String, expires_in: Duration, cancel: CancellationToken }`
    - `MicrosoftSignedIn`
    - `MicrosoftSignInFailed(String)`
    - `MicrosoftSignInCancelled`
    - `DevTunnelsLoaded(Vec<vector_app::devtunnels_actor::TunnelView>)`
    - `DevTunnelsLoadFailed(String)`
    - `DevTunnelsAuthRequired`
    - `DevTunnelConnectRequested { tunnel_id: String }`
    - `DevTunnelConnectStarted(String)` — tunnel_id
    - `DevTunnelPaneReady { window_id: WindowId, tab_id: TabId, pane_id: PaneId }`
    - `DevTunnelConnectFailed { tunnel_id: String, reason: String }`

    Each variant follows the existing Phase 6 codespaces variant naming/shape.

    Step 5 — crates/vector-app/src/devtunnels_actor.rs (NEW): tokio actor that mirrors codespaces_actor.

    Define a `TunnelView` struct: `{ tunnel_id: String, display_name: String, host: String, last_seen_secs_ago: Option<u64> }`. Derive Debug + Clone (no token-bearing fields).

    Implement `From<&TunnelRecord> for TunnelView` that builds display_name via `t.display_name()`, host from first endpoint's host_id, last_seen_secs_ago via `(chrono::Utc::now() - t.last_updated())` clamped to >=0.

    Define `enum Command { Load, Connect { tunnel_id, rows, cols, window_id }, StartMicrosoftSignIn, SignOutMicrosoft }`.

    Define `struct DevTunnelsActor { api: DevTunnelsApi, microsoft_auth: MicrosoftAuth, token_store: MicrosoftTokenStore, proxy: EventLoopProxy<UserEvent>, mux: Arc<vector_mux::Mux>, signin_cancel: Option<CancellationToken> }`. Manual Debug impl required (no derived Debug — actor holds MicrosoftAuth which holds an http client).

    Implement `DevTunnelsActor::spawn(api, mux, microsoft_auth, token_store, proxy) -> mpsc::Sender<Command>`. Spawns a tokio task; loops `while let Some(cmd) = rx.recv().await { match cmd { ... } }`.

    Implement `async fn auth_provider(&self) -> Option<AuthProvider>`: tries `token_store.load().ok().flatten()`; on Some(t), returns `Some(AuthProvider::Microsoft(t.access_token))`; on None returns None.

    Implement `async fn handle_load(&mut self)`:
    - If auth_provider returns None, send UserEvent::DevTunnelsAuthRequired and return.
    - Call api.list_tunnels(auth). On Ok, map to TunnelView and send DevTunnelsLoaded. On Err(Unauthorized), call try_refresh once. If refresh succeeds, retry list_tunnels once. If retry still fails or refresh failed, send DevTunnelsAuthRequired. On any other Err, send DevTunnelsLoadFailed(reason).

    Implement `async fn try_refresh(&mut self) -> Result<(), MicrosoftAuthError>`:
    - Load tokens; if None, return RefreshExpired.
    - If no refresh_token, return RefreshExpired.
    - Call microsoft_auth.refresh(rt); on Ok save the new tokens; on Err propagate.

    Implement `async fn handle_connect(&mut self, tunnel_id, rows, cols, window_id)`:
    - Emit DevTunnelConnectStarted(tunnel_id) first.
    - Resolve auth_provider; if None, emit DevTunnelsAuthRequired and return.
    - Call api.list_tunnels(auth) to find the matching TunnelRecord by tunnel_id (the picker only passes id across the EventLoopProxy boundary). On Err, emit DevTunnelConnectFailed.
    - Call vector_tunnels::domain::connect_tunnel(api, auth, tunnel, rows, cols). On Ok(transport), call mux.create_tab_async_with_transport(window_id, transport, rows, cols). On success emit DevTunnelPaneReady; on error emit DevTunnelConnectFailed.

    Implement `async fn handle_start_microsoft_signin(&mut self)`:
    - microsoft_auth.start_device_flow returns DeviceFlowStart.
    - Create a CancellationToken; store in self.signin_cancel.
    - Emit MicrosoftDeviceFlowStarted with user_code, verification_uri, expires_in, cancel.
    - Spawn an inner task that calls poll_until_authorized; on Ok save tokens + emit MicrosoftSignedIn; on Err(Cancelled) emit MicrosoftSignInCancelled; on other Err emit MicrosoftSignInFailed(reason).

    Implement `async fn handle_sign_out(&mut self)`: token_store.clear().

    Step 6 — crates/vector-app/src/lib.rs: add module declarations `pub mod devtunnels_actor;`, `pub mod devtunnels_modal;`, `pub mod microsoft_auth_modal;`. The latter two are populated in Task 2.

    Step 7 — crates/vector-app/tests/microsoft_signin_menu.rs (NEW): land Test 6.
    Add a function in crates/vector-app/src/menu.rs (or extend if it exists) named `microsoft_signin_menu_rows(state: SignInState) -> Vec<(String, bool)>` where SignInState is `SignedIn | SignedOut`. SignedOut returns `[("Sign in with Microsoft", true)]`. SignedIn returns `[("Sign out of Microsoft", true)]`. Pure data, no AppKit. Test asserts these labels match UI-SPEC §Copywriting Contract verbatim.
  </action>
  <verify>
    <automated>cargo build -p vector-app -p vector-input -p vector-render &amp;&amp; cargo test -p vector-input keymap &amp;&amp; cargo test -p vector-render tint_stripe &amp;&amp; cargo test -p vector-app --tests 2>&amp;1 | tail -10 &amp;&amp; cargo clippy -p vector-app -p vector-input -p vector-render --all-targets -- -D warnings &amp;&amp; grep -q "OpenDevTunnelsPicker" crates/vector-input/src/keymap.rs &amp;&amp; grep -q "MICROSOFT_BLUE" crates/vector-render/src/tint_stripe.rs &amp;&amp; grep -q "DevTunnelsLoaded" crates/vector-app/src/app.rs &amp;&amp; grep -q "DevTunnelPaneReady" crates/vector-app/src/app.rs</automated>
  </verify>
  <acceptance_criteria>
    - cargo build -p vector-app -p vector-input -p vector-render exit 0
    - cargo test -p vector-input keymap reports the new test passes; existing keymap tests still pass
    - cargo test -p vector-render tint_stripe reports >= 1 new test passes (microsoft_blue_is_0078d4)
    - cargo test -p vector-app --tests reports actor + menu tests pass
    - grep -c "OpenDevTunnelsPicker" crates/vector-input/src/keymap.rs >= 2 (variant declaration + match arm)
    - grep -c "MICROSOFT_BLUE" crates/vector-render/src/tint_stripe.rs >= 1
    - grep -c "0\\.471" crates/vector-render/src/tint_stripe.rs >= 1
    - grep -c "DevTunnelsLoaded\\|DevTunnelPaneReady\\|DevTunnelConnectStarted\\|DevTunnelsAuthRequired" crates/vector-app/src/app.rs >= 4
    - grep -c "impl std::fmt::Debug for DevTunnelsActor" crates/vector-app/src/devtunnels_actor.rs >= 1
    - cargo test -p vector-arch-tests --tests 0 failed (Pitfall 14 holds with new actor file scanned)
    - cargo clippy --workspace --all-targets -- -D warnings exit 0
  </acceptance_criteria>
  <done>Keymap, tint, UserEvent variants, and actor all land; menu rows function exists. The actor is testable in isolation via injection; downstream Task 2 wires it to live AppKit panels.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Microsoft sign-in modal + DevTunnels picker modal + menu wiring</name>
  <files>crates/vector-app/src/microsoft_auth_modal.rs, crates/vector-app/src/devtunnels_modal.rs, crates/vector-app/src/menu.rs, crates/vector-app/src/app.rs, crates/vector-app/tests/devtunnels_picker.rs</files>
  <read_first>
    - crates/vector-app/src/auth_modal.rs (entire file — Microsoft modal mirrors verbatim)
    - crates/vector-app/src/codespaces_modal.rs (entire file — DevTunnels picker mirrors verbatim)
    - crates/vector-app/src/menu.rs (entire file — add the new menu items + rebuild logic)
    - crates/vector-app/src/relative_time.rs (humanize + state_label + state_color)
    - .planning/phases/08-vs-code-remote-tunnels-connect/08-UI-SPEC.md (Surfaces S1, S2, S3, S6; Spacing Scale; Typography; Copywriting Contract; Visual Diff vs Phase 6)
  </read_first>
  <behavior>
    - Test 1 (picker frame constants): DevTunnelsPickerModal panel frame is 640x480; rows container x=8 y=32 w=624 h=416; footer y=4 h=24. Match UI-SPEC verbatim.
    - Test 2 (picker footer copy table): footer_copy(state) returns the exact strings from UI-SPEC §Picker footer copy verbatim — Loading: "Loading Dev Tunnels…", Empty: "No Vector-agent tunnels yet. Install vector-tunnel-agent on a remote machine and run it.", NotSignedIn: "Sign in with GitHub or Microsoft to list Dev Tunnels.", ApiError(reason): formatted as "Could not load tunnels: {reason}. Press R to retry."
    - Test 3 (picker row format): format_row(view) returns a single-line string matching the template "● {display_name}  {host}  ·  {last_seen}" where status_dot defaults to ● (color applied via NSTextField setTextColor on a leading sub-range). Verify no leading "vector-" prefix appears (use display_name not name).
    - Test 4 (microsoft modal frame): MicrosoftAuthDeviceFlowModal panel frame is 480x280. Match UI-SPEC.
    - Test 5 (microsoft modal copy): modal title is "Sign in with Microsoft"; secondary button is "Cancel sign-in"; prompt is "Open {verification_uri} in your browser and enter this code:" with the placeholder format-string preserved.
    - Test 6 (menu items present): vector_app::menu::install_microsoft_signin_items + install_devtunnels_picker_item add NSMenuItems with EXACT titles: "Sign in with Microsoft", "Sign out of Microsoft", "Dev Tunnels…" (with ellipsis character).
    - Test 7 (Cmd-Shift-T menu key equivalent): the "Dev Tunnels…" menu item has keyEquivalent="T" and keyEquivalentModifierMask=Cmd|Shift.
  </behavior>
  <action>
    Step 1 — crates/vector-app/src/microsoft_auth_modal.rs (NEW): mirror crates/vector-app/src/auth_modal.rs structure verbatim. Differences:
    - Frame size: 480x280 (UI-SPEC).
    - Title: "Sign in with Microsoft" (UI-SPEC Modal title).
    - Prompt: "Open {verification_uri} in your browser and enter this code:" (UI-SPEC Prompt).
    - User-code display: 32pt monospaced semibold (weight=0.6), exact value verbatim from DeviceFlowStart.user_code.
    - Countdown: "Expires in {M:SS}" (UI-SPEC Caption); ticks every 1s; cancels device flow on hit.
    - Secondary button label: "Cancel sign-in" (UI-SPEC). Clicking signals the CancellationToken passed in MicrosoftDeviceFlowStarted.
    - Floating window level; centered on active window's screen.
    - Use objc2-app-kit Retained<NSPanel> wrapped in MainThreadOnly<T> per Phase 5 Plan 11 pattern for any static-Sync needs.

    Public API:
    - `pub fn show(mtm: MainThreadMarker, ctx: MicrosoftAuthModalCtx) -> Retained<NSPanel>` — ctx contains verification_uri, user_code, expires_in, on_cancel callback.
    - `pub fn close(modal: &NSPanel)` — order out, cancels poll task via the cancellation token.

    Step 2 — crates/vector-app/src/devtunnels_modal.rs (NEW): mirror crates/vector-app/src/codespaces_modal.rs verbatim. Differences per UI-SPEC §Visual Diff vs Phase 6:
    - Panel title: "Dev Tunnels".
    - Keybind: Cmd-Shift-T (D-11). Installed via the menu item's keyEquivalent, NOT a custom NSEvent observer.
    - Row format: `{status_dot}  {display_name}  {host}  ·  {last_seen}`.
    - Status dots: ● green (live, recent), ● amber (stale > 5min ago), ● red (unreachable / API error).
    - Footer copy table (verbatim from UI-SPEC):
      - Loading: "Loading Dev Tunnels…"
      - Empty signed-in: "No Vector-agent tunnels yet. Install vector-tunnel-agent on a remote machine and run it."
      - Not signed in: "Sign in with GitHub or Microsoft to list Dev Tunnels."
      - Signed in to other provider: "No tunnels under your {provider} account. Switch providers or register one."
      - API error: "Could not load tunnels: {reason}. Press R to retry."
      - Loaded: "{N} of {M} tunnels."

    Helper functions to land as plain Rust (testable without AppKit):
    - `pub fn footer_copy(state: FooterState) -> String` where FooterState is an enum.
    - `pub fn format_row(view: &TunnelView, now: chrono::DateTime<Utc>) -> String` — uses relative_time::humanize.
    - `pub fn status_dot(last_seen_secs_ago: Option<u64>) -> (char, StatusColor)` — `●` always; color = Live (green) if < 5min, Stale (amber) if 5min - 24h, Unreachable (red) if older or None.

    Keybinds (per UI-SPEC Interaction Contract):
    - Enter: connect — sends Command::Connect to actor via the panel's stored mpsc Sender.
    - Cmd-S: save as profile — calls vector_config::writer::append_devtunnel_profile or equivalent (mirror Phase 6 append_codespace_profile; if writer is missing, defer to a follow-up patch task and document).
    - Esc: close — cancels in-flight load via stored CancellationToken; orders out.
    - ↑/↓: move selection; rerender.
    - R: retry — only effective in error state; resends Command::Load.
    - Typing alphanumerics: search-as-you-type filter on tunnel name + host substring (case-insensitive); reuse Phase 6 fuzzy-matcher OR plain `to_lowercase().contains()` for v1.

    Public API:
    - `pub fn show(mtm: MainThreadMarker, ctx: DevTunnelsModalCtx)` — ctx contains the actor command Sender + initial state.
    - `pub fn close(...)`.

    Step 3 — crates/vector-app/src/menu.rs: extend to register the new items. Mirror the Phase 6 Codespaces menu installation. Items:
    - "Sign in with Microsoft" — visible when not signed in; sends Command::StartMicrosoftSignIn.
    - "Sign out of Microsoft" — visible when signed in; sends Command::SignOutMicrosoft.
    - "Dev Tunnels…" with keyEquivalent "T" + Cmd+Shift modifier — opens DevTunnelsPickerModal.

    Add `pub fn rebuild_microsoft_signin_section(mtm, state: SignInState)` that dynamically swaps "Sign in" vs "Sign out" item, following Phase 5 Plan 11 `rebuild_switch_profile_submenu` MainThreadOnly<OnceLock<...>> pattern. NEVER walk NSApplication.mainMenu by index — store the menu reference at install time.

    Step 4 — crates/vector-app/src/app.rs: handle the new UserEvent variants and the AppShortcut::OpenDevTunnelsPicker dispatch.
    - Add a field on `App` (or wherever Phase 6 codespaces are stored): `devtunnels_actor: Option<mpsc::Sender<devtunnels_actor::Command>>` set at startup.
    - Wire EncodedKey::App(AppShortcut::OpenDevTunnelsPicker) arm in app.rs key dispatch to call `devtunnels_modal::show` and send Command::Load.
    - Wire EncodedKey::App(AppShortcut::OpenCodespacesPicker) if missing (Phase 6 should have this — if not, surface as a separate gap fix).
    - Handle each new UserEvent variant: DevTunnelsLoaded updates the picker's row list; DevTunnelsAuthRequired closes picker + shows toast "Sign in with GitHub or Microsoft to list Dev Tunnels." (UI-SPEC); DevTunnelsLoadFailed updates picker footer; DevTunnelConnectStarted shows progress toast "Connecting to {tunnel}…"; DevTunnelPaneReady updates Mux active pane focus + applies MICROSOFT_BLUE tint via TintStripePipeline::set_color; DevTunnelConnectFailed shows toast "Could not connect to '{tunnel_name}': {reason}." (UI-SPEC).
    - On pane focus change, if the newly focused pane has TransportKind::DevTunnel, call `tint.set_color(queue, Some(MICROSOFT_BLUE))`; otherwise None (local) or GITHUB_PURPLE (codespace, Phase 6 dormant). Mirror Phase 7's already-shipped tint dispatch in app.rs.

    Step 5 — crates/vector-app/tests/devtunnels_picker.rs (NEW): land Tests 1, 2, 3 from <behavior>. Pure Rust — testing the helper functions (footer_copy, format_row, status_dot). Tests 4-7 require AppKit; either gate them `#[cfg(target_os = "macos")]` with MainThreadMarker, or skip and document as smoke-only.

    Footer copy test must use EXACT strings from UI-SPEC §Picker footer copy table — character-for-character including the `…` Unicode ellipsis. The test compares against a static const inside the test file.
  </action>
  <verify>
    <automated>cargo build -p vector-app &amp;&amp; cargo test -p vector-app --test devtunnels_picker &amp;&amp; cargo test -p vector-app --test microsoft_signin_menu &amp;&amp; cargo clippy -p vector-app --all-targets -- -D warnings &amp;&amp; grep -q "Dev Tunnels…" crates/vector-app/src/devtunnels_modal.rs &amp;&amp; grep -q "Sign in with Microsoft" crates/vector-app/src/microsoft_auth_modal.rs &amp;&amp; grep -q "Cancel sign-in" crates/vector-app/src/microsoft_auth_modal.rs &amp;&amp; grep -q "Loading Dev Tunnels" crates/vector-app/src/devtunnels_modal.rs &amp;&amp; grep -q "No Vector-agent tunnels yet" crates/vector-app/src/devtunnels_modal.rs &amp;&amp; grep -q "Press R to retry" crates/vector-app/src/devtunnels_modal.rs &amp;&amp; grep -q "MICROSOFT_BLUE" crates/vector-app/src/app.rs</automated>
  </verify>
  <acceptance_criteria>
    - cargo build -p vector-app exit 0
    - cargo test -p vector-app --tests >= 6 passed (Task 1 tests + Task 2 tests combined)
    - grep -c "640" crates/vector-app/src/devtunnels_modal.rs >= 1 (UI-SPEC width)
    - grep -c "480" crates/vector-app/src/devtunnels_modal.rs >= 1 (UI-SPEC height)
    - grep -c "480" crates/vector-app/src/microsoft_auth_modal.rs >= 1 (Microsoft modal width)
    - grep -c "280" crates/vector-app/src/microsoft_auth_modal.rs >= 1 (Microsoft modal height)
    - grep -c "Cancel sign-in" crates/vector-app/src/microsoft_auth_modal.rs >= 1 (UI-SPEC verbatim)
    - grep -c "Sign in with Microsoft" crates/vector-app/src/microsoft_auth_modal.rs >= 1
    - grep -c "Loading Dev Tunnels…" crates/vector-app/src/devtunnels_modal.rs >= 1 (Unicode ellipsis verbatim)
    - grep -c "No Vector-agent tunnels yet" crates/vector-app/src/devtunnels_modal.rs >= 1
    - grep -c "Sign in with GitHub or Microsoft to list Dev Tunnels" crates/vector-app/src/devtunnels_modal.rs >= 1
    - grep -c "Press R to retry" crates/vector-app/src/devtunnels_modal.rs >= 1
    - grep -c "Dev Tunnels…" crates/vector-app/src/menu.rs >= 1
    - grep -c "Sign in with Microsoft\\|Sign out of Microsoft" crates/vector-app/src/menu.rs >= 2
    - grep -c "MICROSOFT_BLUE" crates/vector-app/src/app.rs >= 1 (tint applied on DevTunnel pane focus)
    - cargo clippy --workspace --all-targets -- -D warnings exit 0
    - cargo test -p vector-arch-tests --tests 0 failed
  </acceptance_criteria>
  <done>Microsoft sign-in modal + DevTunnels picker modal + menu items all wired with UI-SPEC-verbatim copy. Tint applies on DevTunnel pane focus.</done>
</task>

</tasks>

<scope_note>
Plan 08-05 intentionally bundles UI surfaces (picker modal + Microsoft sign-in modal + menu wiring + keymap + tint + actor + UserEvent variants) into 2 tasks across 11 files. Rationale: these surfaces are tightly coupled — the actor needs UserEvent variants, the modals need actor mpsc handles, the menu needs the keymap variant, the tint needs pane focus events from the actor. Splitting would introduce sequential dependencies between sibling sub-plans without reducing per-task scope. Task 1 lands the data layer (actor + keymap + tint + UserEvent + menu rows); Task 2 lands the AppKit modal surfaces that consume it. Executor context budget is ~50% — within the 2-3 task target.
</scope_note>

<verification>
- `make lint` exit 0
- `make test` exit 0 (>= 15 new tests pass + zero regressions)
- `cargo test -p vector-arch-tests --tests` 0 failed (Pitfall 14 holds across new vector-app modules)
- All UI-SPEC verbatim copy strings present in modals/menu
</verification>

<success_criteria>
- Cmd-Shift-T opens DevTunnelsPickerModal with rows pulled from list_tunnels
- "Sign in with Microsoft" / "Sign out of Microsoft" menu items wired
- Picker footer copy table matches UI-SPEC verbatim
- Microsoft sign-in modal mirrors Phase 6 AuthDeviceFlowModal shape with Microsoft-specific labels
- On connect: pane installs via create_tab_async_with_transport, [remote] badge appears, tab tint becomes Microsoft blue
- 401 auto-refresh path tested; sticky toast surfaces on terminal failure
</success_criteria>

<output>
After completion, create .planning/phases/08-vs-code-remote-tunnels-connect/08-05-SUMMARY.md documenting:
- All UI-SPEC verbatim copy strings landed (footer table + Microsoft modal + menu items)
- Any AppKit gates around tests + which tests are smoke-only
- The exact MainThreadOnly<T> static slot used for menu items rebuild (mirroring Phase 5 Plan 11)
- Confirmation that EncodedKey::App(OpenDevTunnelsPicker) is reachable in app.rs key dispatch
</output>
