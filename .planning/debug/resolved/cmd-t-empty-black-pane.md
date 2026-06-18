---
status: resolved
trigger: "Cmd-T opens new tab but pane is empty/black — no shell prompt; surfaced during Phase 9 UAT re-run after Phase 9.1 landed"
created: 2026-05-27
updated: 2026-05-28
resolved: 2026-05-28
---

## Current Focus

hypothesis: CONFIRMED — input routing in main.rs:147-152 selects the target pane via `Mux::any_active_pane_id()`, which iterates `RwLock<HashMap<WindowId, Window>>` in non-deterministic order. With 2 windows the order happens to put bootstrap first → tab 2 keystrokes silently hit the (hidden) bootstrap pane. With 3+ windows the bucket layout differs and the newest window wins.
test: Add an explicit `active_window_id` to Mux, set it on `create_window` (newest wins, matches macOS makeKeyAndOrderFront) + on `WindowEvent::Focused(true)` + on `NewTabReady` arm. `any_active_pane_id` honors it first, falls back to HashMap scan.
expecting: Tab 2 typing now reaches the new pane (visible echo). Tab 1 still works (bootstrap window's pane_id is the initial active_window_id). Tab 3+ still works (each new window claims focus). Closing a focused window falls back to an arbitrary remaining one.
next_action: Awaiting human verification of the freshly-built target/release/vector-app — Cmd-T tab 2 should now accept input.

## Symptoms

expected: |
  Pressing Cmd-T opens a new tab. The new tab's pane shows a live local shell prompt (zsh by default) in the same way the initial cold-start pane does. Typing echoes characters. Tab title shows the local hostname/cwd.
actual: |
  Cmd-T opens a new tab (visible in the macOS native tab group — Plan 04-04 set_tabbing_identifier path works). The pane region inside the new tab is empty/black — no shell prompt rendered, no characters appear when typing. The initial cold-start pane in the original tab keeps working normally.
errors: |
  No panic in stderr per user-visible behavior. Screenshot shows the affected new-tab pane appears as a dark/empty surface.
reproduction: |
  1. Launch Vector (cargo run --release -p vector-app, or the bundled Vector.app)
  2. The initial pane displays a working zsh prompt — this is fine.
  3. Press Cmd-T.
  4. A new tab opens; its pane is empty/black with no prompt.
started: 2026-05-27 Phase 9 UAT re-run, after Phase 9.1 commits 21767ee/2701942/5ffa51e/2a50c7f/622d3fe/7a24131/8ccc0aa.

## Eliminated

- hypothesis: Stale binary (user running build from before Phase 9.1)
  evidence: `target/release/vector-app` mtime is 2026-05-27 10:02, AFTER all Phase 9.1 commits (latest 622d3fe May 26). Screenshot timestamp 10:03:22 matches that build.
  timestamp: 2026-05-27

- hypothesis: Commit 21767ee accidentally modified Cmd-T / spawn path while sweeping in 09.1-02 deletions
  evidence: `git show 21767ee -- crates/vector-app/src/app.rs` shows ONLY the PaneExited arm changed (lines 1872-1888). No edits to handle_new_tab or any spawn-path code. The "swept-in deletions" were entire files (auth_actor.rs, codespaces_actor.rs, etc.), not in-place edits.
  timestamp: 2026-05-27

- hypothesis: Plan 09.1-02 Codespaces removal entangled with Cmd-T / NewTab routing
  evidence: `grep MuxCommand::NewTab / NewTab` shows it routes through vector-input/keymap.rs:160 (`"t" => MuxCommand::NewTab`) → app.rs:1344 → handle_new_tab. None of the deleted modules touch this path. The deleted UserEvent variants were Codespaces/auth-specific.
  timestamp: 2026-05-27

- hypothesis: `active_pane_id` / focus wiring not updated when new tab activates
  evidence: Moot — there is no new pane to focus. handle_new_tab inserts the AppWindow with `compositors: HashMap::new()` and `active_pane_id: None`, and never spawns a Pane via Mux. The compositor map remains empty forever.
  timestamp: 2026-05-27

## Evidence (new — input-routing investigation)

- timestamp: 2026-05-28
  checked: crates/vector-app/src/app.rs:1915-1966 (WindowEvent::KeyboardInput arm) and :1046-1061 (try_send_pty_bytes) and crates/vector-app/src/input_bridge.rs
  found: KeyboardInput → `self.try_send_pty_bytes(bytes)` (does NOT take WindowId). try_send_pty_bytes only checks `Mux::any_active_pane_id()` for the *reconnecting* gate, then calls `self.input_bridge.send_bytes(bytes)`. InputBridge is a single mpsc::Sender — bytes are not tagged with any pane/window ID.
  implication: Input is NOT routed by focused winit window. There is one global write channel.

- timestamp: 2026-05-28
  checked: crates/vector-app/src/main.rs:142-154 (write_rx consumer task on the I/O thread)
  found: The consumer does `let target = Mux::try_get().and_then(|m| m.any_active_pane_id()).unwrap_or(pane_id /* bootstrap */); router_w.lock().send_write(target, bytes);`. So pane selection happens here, via `Mux::any_active_pane_id()`.
  implication: Pane-of-input is entirely controlled by `Mux::any_active_pane_id()`. Comment in code: "Until per-pane selection lands, fall back to the bootstrap pane." This is the unfinished feature.

- timestamp: 2026-05-28
  checked: crates/vector-mux/src/mux.rs:109-123 (any_active_pane_id) and field declaration at line 28
  found: `windows: RwLock<HashMap<WindowId, Window>>`. `any_active_pane_id` iterates `windows.values()` and returns the FIRST window's active_tab_id's active_pane_id. HashMap iteration order is NOT deterministic — uses RandomState SipHash seed per HashMap.
  implication: Which window's pane gets the input is essentially undefined. Why tab 2 is broken vs tab 3+ working is a consequence of HashMap bucket layout with 2 vs 3 entries — fragile coincidence, not a design.

- timestamp: 2026-05-28
  checked: grep for `WindowEvent::Focused` across vector-app/src
  found: No handler at all. The App never reacts to NSWindow key-status changes.
  implication: No "current focused window" state exists in the App. Even if we wanted to route by focus, there's nothing tracking it.

## Resolution (revised)

root_cause (updated): |
  Two distinct root causes wrapped in the same Cmd-T symptom.

  (Already-fixed half) `handle_new_tab` never spawned a backing Mux Window+Tab+Pane. Fixed by the NewTabReady channel.

  (Remaining half) Input routing in main.rs:147-152 selects the destination pane via `Mux::any_active_pane_id()`, which iterates a `HashMap<WindowId, Window>` in non-deterministic order and returns the first window's active pane. With one mux window (just bootstrap) this trivially returns the bootstrap pane. With two windows (bootstrap + first Cmd-T tab) the iteration order happens to still yield bootstrap first on Apple Silicon Rust 1.88's hashbrown bucket layout — so the first Cmd-T tab's keystrokes are silently routed to the (hidden) bootstrap pane. With three+ windows, the bucket layout changes and the most-recently-created window happens to come first — so input routes correctly to the visible tab. There is no per-window focus state and no `WindowEvent::Focused` handler.

## Evidence

- timestamp: 2026-05-27
  checked: build mtime + commit dates
  found: target/release/vector-app dated May 27 10:02; all Phase 9.1 commits dated May 26 (latest 622d3fe). Binary is current.
  implication: Bug is in source, not a stale build.

- timestamp: 2026-05-27
  checked: `git show 21767ee -- crates/vector-app/src/app.rs`
  found: Diff is exactly the 19-line PaneExited replacement at app.rs:1870-1888. No other code paths in app.rs touched.
  implication: Cmd-T regression theory disproved. Look elsewhere.

- timestamp: 2026-05-27
  checked: crates/vector-app/src/app.rs lines 1281-1335 (handle_new_tab)
  found: Function creates NSWindow + RenderHost + ChromePipelines + overlay; inserts `AppWindow { compositors: HashMap::new(), active_pane_id: None, ... }`; maps the new winit window id to the existing bootstrap mux WindowId so resize routes; enables IME. THAT IS ALL. There is no `mux.create_tab_async` call, no `router.spawn_pane` call, no UserEvent dispatch to the I/O thread. The TODO at line 1322-1324 explicitly defers Mux Tab+Pane spawn to a never-completed follow-up.
  implication: ROOT CAUSE. Cmd-T creates window chrome only; no PTY/Pane/Compositor for the new tab.

- timestamp: 2026-05-27
  checked: `git log -G "fn handle_new_tab" --oneline -- crates/vector-app/src/app.rs`
  found: Single result `b080b18 need to test (#1)` (Phase 4 land commit). handle_new_tab has not been modified since.
  implication: This is a pre-existing incomplete feature from Phase 4, NOT a Phase 9.1 regression. The bug only surfaced now because Phase 9 UAT exercised Cmd-T explicitly.

- timestamp: 2026-05-27
  checked: crates/vector-app/src/main.rs lines 111-130 (bootstrap pane spawn)
  found: The working bootstrap pane is created by the I/O thread via: `let window_id = mux.create_window(); let (_tab_id, pane_id) = mux.create_tab_async(window_id, None, 24, 80).await?; if let Some(pane) = mux.pane(pane_id) { if let Some(transport) = pane.take_transport() { router_io.lock().spawn_pane(pane_id, transport, local_domain_dyn, "", CancellationToken::new()); } }`
  implication: This exact sequence must run on the I/O thread when Cmd-T fires. Pattern to mirror: split_req_tx (main.rs:43-44 declaration + 186-217 consumer + app.rs ~1376 dispatch).

- timestamp: 2026-05-27
  checked: crates/vector-app/src/app.rs lines 387-436 (Cmd-N SpawnNewWindow arm)
  found: Same incomplete pattern — creates an ungrouped NSWindow + RenderHost + ChromePipelines, inserts AppWindow with empty compositors/None active_pane_id, then stops. No mux create_window / create_tab_async / spawn_pane.
  implication: Cmd-N (new untabbed window) is broken the same way as Cmd-T. The fix should cover both: a unified "spawn-fresh-pane-for-window" dispatch. Scope decision below.

## Resolution

root_cause: |
  `handle_new_tab` (crates/vector-app/src/app.rs) was a Phase-4 scaffold that only created the macOS NSWindow + RenderHost + ChromePipelines for a Cmd-T tab. It never called `mux.create_window` / `mux.create_tab_async` / `router.spawn_pane` to allocate a backing Mux Window+Tab+Pane and start a PTY actor. The new AppWindow was inserted with an empty `compositors: HashMap` and `active_pane_id: None`, then mapped to the EXISTING bootstrap mux WindowId — but there was no second pane in that window for the new NSWindow to render. The render_window legacy fallback at app.rs:617 painted the bootstrap pane's `self.term` into the new tab when the compositor map was empty, which would have looked OK; but `winit_to_mux_window` mapping to the bootstrap window plus `mux.active_tab_id` always pointing at the bootstrap tab meant the per-pane render path also had nothing real to draw. Net visible effect: the new tab's content area stayed black.
  Not a Phase 9.1 regression — the TODO at line 1322-1324 ("phase-5: per-NSWindow mux WindowId allocation when Cmd-T spawns a fresh Mux Tab+Pane") explicitly deferred this. The follow-up was never written. The bug only surfaced now because Phase 9 UAT exercised Cmd-T for the first time post-Phase-9.1.

fix: |
  Two-patch fix.

  Patch 1 (rendering — already landed): wire a Cmd-T new-tab request channel
  that mirrors split_req_tx; lib.rs gets a `UserEvent::NewTabReady` variant;
  app.rs gets `new_tab_req_tx` + `set_new_tab_req_tx` + a NewTabReady arm in
  user_event that inserts `winit_to_mux_window` + sets `aw.active_pane_id` +
  calls `ensure_compositors_for_pane`.

  Patch 2 (input — this commit): explicit "active mux window" tracking.
  - vector-mux/src/mux.rs: add `active_window_id: RwLock<Option<WindowId>>`
    field; `create_window` sets it (newest window wins, matches AppKit's
    makeKeyAndOrderFront semantics); add `set_active_window(WindowId)`;
    rewrite `any_active_pane_id` to consult `active_window_id` first and
    fall back to HashMap scan; `close_pane` clears it (or hands off to any
    remaining window) when the focused window is removed.
  - vector-app/src/app.rs: handle `WindowEvent::Focused(true)` — look up
    the mux WindowId via `winit_to_mux_window` and call
    `Mux::set_active_window`; in the existing `NewTabReady` arm, also call
    `Mux::set_active_window(mux_window_id)` so the very first Cmd-T tab
    receives input even if macOS does not fire `Focused(true)` for it
    (the new NSWindow comes up already-key as part of the tab-group
    activation, and winit doesn't always emit a Focused event in that
    case).

  Cmd-N (`SpawnNewWindow`) still has the incomplete-feature bug; out of
  scope for this session.

verification: |
  Patch 2 self-verified:
  - `cargo check -p vector-mux -p vector-app` clean
  - `cargo clippy --all-targets --all-features -- -D warnings` clean
  - `cargo fmt --all -- --check` clean
  - `cargo test -p vector-mux --lib` — 12/12 passed
  - `cargo test -p vector-app --lib --bins` — 16/16 passed
  - `cargo build --release -p vector-app` — succeeds

  HUMAN-VERIFY CONFIRMED (2026-05-28): User reported "this works now... I am
  able to open different tabs and then type in them." Both halves verified:
  - Patch 1: Cmd-T new tabs render a live shell prompt.
  - Patch 2: Typing works in every tab including tab 2 (HashMap-order
    input-routing fix).
  Session RESOLVED.

out_of_scope_followup: |
  Cmd-N (`SpawnNewWindow` at crates/vector-app/src/app.rs:387-436) has the
  IDENTICAL incomplete-feature pattern as the original handle_new_tab: it
  creates an ungrouped NSWindow + RenderHost + ChromePipelines and inserts an
  AppWindow with empty `compositors` / `active_pane_id: None`, but never calls
  mux.create_window / create_tab_async / router.spawn_pane. As a result Cmd-N
  windows will show the same empty/black-pane defect and (since they go through
  the same `any_active_pane_id` routing) likely the same input-routing defect.
  This was NOT fixed in this session. Action: file as a backlog item / gap in a
  future phase — apply the same "spawn-backing-mux-pane + set_active_window"
  pattern that fixed Cmd-T.

files_changed:
  - crates/vector-app/src/lib.rs (Patch 1 — UserEvent::NewTabReady variant)
  - crates/vector-app/src/main.rs (Patch 1 — new_tab_req channel + I/O consumer)
  - crates/vector-app/src/app.rs (Patch 1 — new_tab_req_tx field/setter; handle_new_tab dispatch; NewTabReady arm; Patch 2 — WindowEvent::Focused(true) handler + set_active_window call inside NewTabReady arm)
  - crates/vector-mux/src/mux.rs (Patch 2 — active_window_id field; create_window sets it; set_active_window; any_active_pane_id consults it; close_pane clears/reassigns)

