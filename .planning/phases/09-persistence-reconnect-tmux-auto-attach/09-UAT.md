---
status: partial
phase: 09-persistence-reconnect-tmux-auto-attach
source: [09-01-SUMMARY.md, 09-02-SUMMARY.md, 09-03-SUMMARY.md, 09-04-SUMMARY.md, 09-05-SUMMARY.md, 09-06-SUMMARY.md, 09-07-SUMMARY.md]
started: 2026-05-26T00:00:00Z
updated: 2026-05-26T20:30:00Z
---

## Current Test

[testing paused — 6 items blocked on prior-phase gap closure]

## Tests

### 1. Cold Start Smoke Test
expected: Kill any running Vector.app. Run `cargo run --release` (or launch the bundled .app) from a clean shell. Window opens, native menu visible, no panic in stderr, prompt appears in the default pane and accepts input.
result: pass
note: "Side-finding logged in Gaps — stale `Codespaces…` menu item (Cmd-Shift-G) still installed despite Phase 7 pivot away from Codespaces."

### 2. Local Pane Regression
expected: Open a new tab (Cmd-T) with a local shell. Typing echoes correctly, tab title shows the local hostname/cwd (no `[remote]` or `[reconnecting]` suffix). `exit` closes the pane cleanly with no reconnect spinner.
result: issue
reported: "exit obviously doesnt seem to work"
severity: major
note: "Tab title labelling appears correct (no [remote] suffix). Failure is that local shell `exit` does not close/free the pane — PTY actor emits PaneExited correctly but the App handler at app.rs:1872-1874 is a logging-only stub (`Plan 04-05 will render sentinel`). Pre-existing Phase 4 incompletion surfaced during Phase 9 verification."

### 3. Cmd-Shift-T Opens Dev Tunnels Picker
expected: Cmd-Shift-T opens the Dev Tunnels picker modal. If not yet signed into Microsoft, a device-flow modal appears first; after auth, the picker lists reachable tunnels (name, host, last-seen).
result: issue
reported: "this is what I see when I press cmd + shift + T [screenshot: modal opens, footer says 'Sign in with GitHub or Microsoft to list Dev Tunnels.', no sign-in button visible]. also app looks black exec ... instead of vector thumbnail"
severity: major
note: "Cmd-Shift-T binding works, modal renders, footer copy is correct (FooterState::NotSignedIn at devtunnels_modal.rs:54-56). FAILURE: modal has no sign-in affordance — user cannot recover from NotSignedIn state without a separate menu path; no auto-route to device-flow either. Blank dock icon logged separately."

### 4. Connect to a Dev Tunnel
expected: From the picker, selecting a tunnel with `vector-tunnel-agent` running opens a new tab. Tab title contains `[remote]`. Typing in the pane reaches the remote shell and echoes back. Resize works (Cmd-+ / Cmd-- / window resize).
result: blocked
blocked_by: prior-phase
reason: "Cannot reach a live remote pane: picker has no sign-in affordance (Test 3 gap) and Microsoft menu items are never installed in main menu (install_microsoft_menu_items defined but uncalled). Will retest after gap-closure phase."

### 5. Reconnect Status Bar on Disconnect
expected: With a live remote pane, kill `vector-tunnel-agent` on the remote (or briefly disable wifi). Within ~30 s a status bar appears at the top of the pane (dark surface band). Tab title flips from `…[remote]` to `…[reconnecting]`. (Known gap: bar text glyph row not yet composited — only the background band renders. Cursor dim + fade-out animation also deferred per UI-SPEC v1 fallbacks.)
result: blocked
blocked_by: prior-phase
reason: "Cannot reach a live remote pane: picker has no sign-in affordance (Test 3 gap) and Microsoft menu items are never installed in main menu (install_microsoft_menu_items defined but uncalled). Will retest after gap-closure phase."

### 6. Input Lock + Toast During Reconnect
expected: While the Reconnecting state is active, typing produces NO characters in the pane. Exactly ONE info toast appears at the bottom: `Input ignored — reconnecting`. Continued typing does NOT spawn more toasts. Mouse selection and Cmd-Up scrollback still work; grid + scrollback do not blank.
result: blocked
blocked_by: prior-phase
reason: "Cannot reach a live remote pane: picker has no sign-in affordance (Test 3 gap) and Microsoft menu items are never installed in main menu (install_microsoft_menu_items defined but uncalled). Will retest after gap-closure phase."

### 7. Recovery on Reconnect
expected: Restore connectivity (restart agent / re-enable wifi). Within one backoff slot: status bar disappears, tab title returns to `…[remote]`, cursor resumes normal blink, typing reaches the (possibly fresh) remote shell.
result: blocked
blocked_by: prior-phase
reason: "Cannot reach a live remote pane: picker has no sign-in affordance (Test 3 gap) and Microsoft menu items are never installed in main menu (install_microsoft_menu_items defined but uncalled). Will retest after gap-closure phase."

### 8. Cmd-W During Reconnect Cancels Promptly
expected: Trigger a disconnect again, wait for the reconnect bar to appear, hit Cmd-W. Pane closes immediately (within ~50 ms) — does NOT hang for the next backoff slot. No further reconnect attempts after close.
result: blocked
blocked_by: prior-phase
reason: "Cannot reach a live remote pane: picker has no sign-in affordance (Test 3 gap) and Microsoft menu items are never installed in main menu (install_microsoft_menu_items defined but uncalled). Will retest after gap-closure phase."

### 9. Tmux Auto-Attach Preserves Remote Shell State (PERSIST-04)
expected: In a remote pane, run something stateful (e.g. `export FOO=hello`, `cd /tmp`). Force a reconnect. After the bar disappears, in the same pane run `echo $FOO` + `pwd`. The variable and cwd are still set — the remote session was preserved by the agent's tmux auto-attach, not started fresh.
result: blocked
blocked_by: prior-phase
reason: "Cannot reach a live remote pane: picker has no sign-in affordance (Test 3 gap) and Microsoft menu items are never installed in main menu (install_microsoft_menu_items defined but uncalled). Will retest after gap-closure phase."

## Summary

total: 9
passed: 1
issues: 3
pending: 0
skipped: 0
blocked: 6

## Gaps

- truth: "Vector menu should not advertise GitHub Codespaces affordances after the Phase 7 pivot to Dev Tunnels"
  status: failed
  reason: "User reported: `Codespaces…` menu item (Cmd-Shift-G) still visible in app menu at cold start. Vector-codespaces crate, codespaces_actor, codespaces_modal, OpenCodespacesPicker UserEvent + install_phase6_items handler are all still wired (menu.rs:437-452 + lib.rs:19-20,108-117)."
  severity: minor
  test: 1
  artifacts: []
  missing: []

- truth: "Typing `exit` in a local pane should terminate the pane (close the tab or render an exited sentinel)"
  status: failed
  reason: "User reported: exit obviously doesnt seem to work. PTY actor emits UserEvent::PaneExited but the handler at crates/vector-app/src/app.rs:1872-1874 is a logging-only stub carrying a `Plan 04-05 will render sentinel` TODO. Pre-existing Phase 4 incompletion — not a Phase 9 regression, but surfaced during Phase 9 verification."
  severity: major
  test: 2
  artifacts: []
  missing: []

- truth: "Dev Tunnels picker must give the user a path to sign in when in the NotSignedIn state"
  status: failed
  reason: "User reported: opened picker via Cmd-Shift-T, modal shows only footer text 'Sign in with GitHub or Microsoft to list Dev Tunnels.' with no clickable affordance. devtunnels_modal.rs renders the state correctly but provides no sign-in button and does not auto-route to a device-flow modal; user is stranded unless they know to navigate a separate menu item."
  severity: major
  test: 3
  artifacts: []
  missing: []

- truth: "`Sign in with Microsoft` / `Sign out of Microsoft` / `Dev Tunnels…` menu items must be installed in the Vector app submenu so unauthenticated users have a sign-in path"
  status: failed
  reason: "`install_microsoft_menu_items` is defined in crates/vector-app/src/menu.rs:563 but is NEVER CALLED anywhere in the workspace (grep verified). Phase 8 Plan 05 SUMMARY claims it ships but the wiring was not finished. Without this, the only path to Microsoft sign-in is via the picker — which itself lacks an affordance (see prior gap). Combined effect: user cannot reach a signed-in state at all."
  severity: blocker
  test: 3
  artifacts: []
  missing: []

- truth: "Bundled Vector.app should display the Vector icon in the macOS dock, not a blank/black placeholder"
  status: failed
  reason: "User reported: app looks black exec ... instead of vector thumbnail. Likely missing CFBundleIconFile entry in Info.plist or .icns not bundled by cargo-bundle. Distribution polish leftover, unrelated to Phase 9."
  severity: cosmetic
  test: 3
  artifacts: []
  missing: []
