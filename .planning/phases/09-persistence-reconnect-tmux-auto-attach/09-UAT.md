---
status: testing
phase: 09-persistence-reconnect-tmux-auto-attach
source: [09-01-SUMMARY.md, 09-02-SUMMARY.md, 09-03-SUMMARY.md, 09-04-SUMMARY.md, 09-05-SUMMARY.md, 09-06-SUMMARY.md, 09-07-SUMMARY.md]
started: 2026-05-26T00:00:00Z
updated: 2026-05-28T12:50:00Z
notes: "Resumed after Phase 9.1 closed the 5 blocking gaps. Test 2 surfaced + fixed a Cmd-T regression (empty pane + tab-2 input misroute) via /gsd:debug (commit ad1b19d) — now passing. Cmd-N has the same latent bug → backlogged as Phase 999.2. Continuing from Test 3."
---

## Current Test

number: 3
name: Cmd-Shift-T Opens Dev Tunnels Picker
expected: |
  Cmd-Shift-T opens the Dev Tunnels picker modal. If not signed into Microsoft, the picker footer shows a clickable "Sign in with Microsoft" button (no "GitHub or" text). The Vector app icon (not a black placeholder) shows in the dock. After clicking sign-in and completing the device flow, the picker lists reachable tunnels.
awaiting: user response

## Tests

### 1. Cold Start Smoke Test
expected: Kill any running Vector.app. Run `cargo run --release` (or launch the bundled .app) from a clean shell. Window opens, native menu visible, no panic in stderr, prompt appears in the default pane and accepts input.
result: pass
note: "Side-finding logged in Gaps — stale `Codespaces…` menu item (Cmd-Shift-G) still installed despite Phase 7 pivot away from Codespaces."

### 2. Local Pane Regression
expected: Open a new tab (Cmd-T) with a local shell. Typing echoes correctly, tab title shows the local hostname/cwd (no `[remote]` or `[reconnecting]` suffix). `exit` closes the pane cleanly with no reconnect spinner.
result: pass
note: "Initially failed: Cmd-T opened a tab but the pane was empty/black (no prompt), then after the render fix, tab 2 rendered but accepted no typing. Both diagnosed and fixed via /gsd:debug session cmd-t-empty-black-pane (commit ad1b19d). Root cause: handle_new_tab was a Phase-4 chrome-only scaffold that never spawned a backing Mux pane/PTY; plus non-deterministic HashMap iteration in any_active_pane_id misrouted tab-2 input to the hidden bootstrap pane. User confirmed 2026-05-28: 'I am able to open different tabs and then type in them.' Phase 9.1's PaneExited handler (Plan 09.1-01) works in concert with this."

### 3. Cmd-Shift-T Opens Dev Tunnels Picker
expected: Cmd-Shift-T opens the Dev Tunnels picker modal. If not yet signed into Microsoft, the picker shows a clickable "Sign in with Microsoft" button in the footer (or auto-presents the device-flow modal). After auth, the picker lists reachable tunnels (name, host, last-seen). The Vector app icon (not a black placeholder) is visible in the dock.
result: [pending]
prior_result: issue (resolved by Plans 09.1-03 menu wiring + 09.1-04 modal button + 09.1-05 app icon)

### 4. Connect to a Dev Tunnel
expected: From the picker, selecting a tunnel with `vector-tunnel-agent` running opens a new tab. Tab title contains `[remote]`. Typing in the pane reaches the remote shell and echoes back. Resize works (Cmd-+ / Cmd-- / window resize).
result: [pending]
prior_result: blocked_by prior-phase (unblocked by Phase 9.1)

### 5. Reconnect Status Bar on Disconnect
expected: With a live remote pane, kill `vector-tunnel-agent` on the remote (or briefly disable wifi). Within ~30 s a status bar appears at the top of the pane (dark surface band). Tab title flips from `…[remote]` to `…[reconnecting]`. (Known gap: bar text glyph row not yet composited — only the background band renders. Cursor dim + fade-out animation also deferred per UI-SPEC v1 fallbacks.)
result: [pending]
prior_result: blocked_by prior-phase (unblocked by Phase 9.1)

### 6. Input Lock + Toast During Reconnect
expected: While the Reconnecting state is active, typing produces NO characters in the pane. Exactly ONE info toast appears at the bottom: `Input ignored — reconnecting`. Continued typing does NOT spawn more toasts. Mouse selection and Cmd-Up scrollback still work; grid + scrollback do not blank.
result: [pending]
prior_result: blocked_by prior-phase (unblocked by Phase 9.1)

### 7. Recovery on Reconnect
expected: Restore connectivity (restart agent / re-enable wifi). Within one backoff slot: status bar disappears, tab title returns to `…[remote]`, cursor resumes normal blink, typing reaches the (possibly fresh) remote shell.
result: [pending]
prior_result: blocked_by prior-phase (unblocked by Phase 9.1)

### 8. Cmd-W During Reconnect Cancels Promptly
expected: Trigger a disconnect again, wait for the reconnect bar to appear, hit Cmd-W. Pane closes immediately (within ~50 ms) — does NOT hang for the next backoff slot. No further reconnect attempts after close.
result: [pending]
prior_result: blocked_by prior-phase (unblocked by Phase 9.1)

### 9. Tmux Auto-Attach Preserves Remote Shell State (PERSIST-04)
expected: In a remote pane, run something stateful (e.g. `export FOO=hello`, `cd /tmp`). Force a reconnect. After the bar disappears, in the same pane run `echo $FOO` + `pwd`. The variable and cwd are still set — the remote session was preserved by the agent's tmux auto-attach, not started fresh.
result: [pending]
prior_result: blocked_by prior-phase (unblocked by Phase 9.1)

## Summary

total: 9
passed: 2
issues: 0
pending: 7
skipped: 0
blocked: 0

## Gaps

- truth: "Vector menu should not advertise GitHub Codespaces affordances after the Phase 7 pivot to Dev Tunnels"
  status: resolved
  reason: "Closed by Plan 09.1-02 (commit 2701942): vector-codespaces crate deleted; OpenCodespacesPicker UserEvent + install_phase6_items handler removed; Codespaces menu item gone."
  severity: minor
  test: 1
  artifacts: []
  missing: []

- truth: "Typing `exit` in a local pane should terminate the pane (close the tab or render an exited sentinel)"
  status: resolved
  reason: "Closed by Plan 09.1-01 (commit 21767ee): UserEvent::PaneExited handler at app.rs:1872 now calls Mux::close_pane() and propagates CloseResult (pane/tab/window/app), matching the Cmd-W path."
  severity: major
  test: 2
  artifacts: []
  missing: []

- truth: "Dev Tunnels picker must give the user a path to sign in when in the NotSignedIn state"
  status: resolved
  reason: "Closed by Plan 09.1-04 (commits 2a50c7f, 622d3fe): DevTunnelsPickerModal now renders an NSButton 'Sign in with Microsoft' in FooterState::NotSignedIn that fires the same actor command as the menu item; picker auto-refreshes on MicrosoftSignedIn."
  severity: major
  test: 3
  artifacts: []
  missing: []

- truth: "`Sign in with Microsoft` / `Sign out of Microsoft` / `Dev Tunnels…` menu items must be installed in the Vector app submenu so unauthenticated users have a sign-in path"
  status: resolved
  reason: "Closed by Plan 09.1-03 (commit 5ffa51e): install_microsoft_menu_items(mtm, proxy) now called from App::resumed at app.rs:1453 after install_main_menu(). Menu items are installed at startup."
  severity: blocker
  test: 3
  artifacts: []
  missing: []

- truth: "Bundled Vector.app should display the Vector icon in the macOS dock, not a blank/black placeholder"
  status: resolved
  reason: "Closed by Plan 09.1-05 (commits 7a24131, 8ccc0aa): typographic V icon authored, xtask icon subcommand wired, CFBundleIconFile=icon set in Info.plist.partial. Bundled .app will show the Vector icon."
  severity: cosmetic
  test: 3
  artifacts: []
  missing: []

# --- NEW gap surfaced during Phase 9 UAT re-run (post-Phase 9.1) — RESOLVED ---
- truth: "Cmd-T should open a new tab with a working local shell — pane renders the prompt AND typing reaches that pane's shell"
  status: resolved
  reason: "Diagnosed + fixed via /gsd:debug (commit ad1b19d). Two-part root cause: (1) handle_new_tab was a Phase-4 chrome-only scaffold that created NSWindow chrome but never spawned a backing Mux Window+Tab+Pane/PTY → empty black pane; (2) input routing used Mux::any_active_pane_id() which iterated a HashMap in non-deterministic order → tab-2 keystrokes silently routed to the hidden bootstrap pane (tab 3+ worked by accident of bucket order). Fix: NewTabReady UserEvent channel spawns a real PTY-backed pane on the I/O thread; Mux gains active_window_id tracking (set on create_window, Focused(true), and NewTabReady) consulted first by any_active_pane_id. User confirmed working 2026-05-28."
  severity: blocker
  test: 2
  artifacts:
    - path: "~/Desktop/Screenshot 2026-05-27 at 10.03.22.png"
      issue: "Cmd-T new-tab pane renders black with no prompt (original symptom)"
  missing: []
  debug_session: ".planning/debug/resolved/cmd-t-empty-black-pane.md"

# --- Follow-up surfaced during the above debug session (NOT a Phase 9 blocker) ---
- truth: "Cmd-N (SpawnNewWindow) should open a new window with a working local shell pane"
  status: failed
  reason: "Found during cmd-t-empty-black-pane debugging: Cmd-N (SpawnNewWindow at app.rs:387-436) has the identical Phase-4 incomplete-feature pattern as the old Cmd-T bug — creates an ungrouped NSWindow with empty compositors / active_pane_id: None and never spawns a Mux pane. Cmd-N windows will show the same empty/black-pane + input-routing defect. NOT tested in this UAT (no test exercises Cmd-N). Recommend backlog item or future-phase gap: apply the same spawn-backing-pane + set_active_window pattern that fixed Cmd-T."
  severity: major
  test: 0
  artifacts:
    - path: "crates/vector-app/src/app.rs"
      issue: "SpawnNewWindow handler (lines 387-436) creates NSWindow without spawning a Mux pane"
  missing:
    - "Apply NewTabReady-style backing-pane spawn to Cmd-N SpawnNewWindow path"
  debug_session: ".planning/debug/resolved/cmd-t-empty-black-pane.md"
