---
status: partial
phase: 09-persistence-reconnect-tmux-auto-attach
source: [09-05-SUMMARY.md]
started: 2026-05-22T20:24:53Z
updated: 2026-05-22T20:24:53Z
---

## Current Test

[testing paused — 11 items outstanding; blocked on `DevTunnelsActor` construction wiring in `main.rs` (separate follow-up plan)]

## Tests

### 1. Build and launch Vector locally
expected: |
  `cargo run --release` builds and launches `Vector.app`. Window opens, native menu visible,
  no panic in stderr.
result: [pending]
blocked_by: prior-phase
reason: "DevTunnelsActor is not yet constructed in main.rs — picker UI cannot be invoked end-to-end."

### 2. Sign in to Microsoft
expected: |
  Cmd-Shift-T or the menu item triggers the Microsoft Device Flow modal. After completing
  device-flow in browser, the modal closes and tokens are persisted to Keychain
  (verify via `security find-generic-password -s vector.microsoft 2>/dev/null`).
result: [pending]
blocked_by: prior-phase
reason: "Picker actor not constructed; Microsoft auth modal not reachable from running app."

### 3. Open the Dev Tunnels picker and connect
expected: |
  Cmd-Shift-T opens the DevTunnelsPickerModal. Tunnels with `vector-tunnel-agent` running
  are listed with name, host machine, last-seen. Selecting a tunnel triggers connect.
  A new tab opens; tab title contains `[remote]`. Typing in the pane echoes correctly.
result: [pending]
blocked_by: prior-phase
reason: "DevTunnelsActor not wired in main.rs."

### 4. Force a disconnect — observe Reconnecting UI affordances
expected: |
  After killing `vector-tunnel-agent` on the remote (or briefly disabling wifi), within ~30 s:
  - Inline status bar appears at the top of the pane: `Reconnecting to {profile}… (attempt 1)`.
  - Tab title flips from `…  [remote]` to `…  [reconnecting]`.
  - Cursor stops blinking and dims (40% alpha) — see Gap #2 for known limitation.
  - Status bar fade-in is animated over 120 ms after the 250 ms debounce — see Gap #3.
  - Status bar surface color matches active ChromePalette — see Gap #1 for light-mode limitation.
result: [pending]
blocked_by: prior-phase
reason: "Cannot reach a live Reconnecting state without main.rs wiring."

### 5. Input lock during reconnect — single toast
expected: |
  Typing during the Reconnecting state drops all keystrokes (no characters reach the dead
  remote). Exactly ONE Info toast appears at the bottom: `Input ignored — reconnecting`.
  Continued typing produces no additional toasts. Mouse selection and Cmd-Up scrollback
  STILL WORK. The grid + scrollback do NOT blank.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 6. Backoff attempt counter advances
expected: |
  With connectivity still broken, wait through at least 3 reconnect attempts (~7 s cumulative).
  The status bar attempt counter advances: `(attempt 1)` → `(attempt 2)` → `(attempt 3)`.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 7. Recovery on reconnect
expected: |
  Re-enable connectivity (restart agent / re-enable wifi). Within one backoff window:
  - Status bar disappears (instant removal — fade-out polish deferred; see Gap #3).
  - Tab title returns to `…  [remote]`.
  - Cursor resumes normal blink.
  - Typing reaches the (possibly fresh) remote shell again.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 8. Cmd-W during reconnect cancels promptly
expected: |
  Re-trigger a disconnect, wait for the status bar to appear, hit Cmd-W. The pane closes
  immediately — does NOT hang for the next backoff slot. The per-pane CancellationToken
  fires and the reconnect loop exits.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 9. Byte-loss sanity check across reconnect
expected: |
  On remote: `cat /dev/urandom | head -c 10485760 > /tmp/test_payload`; in Vector
  `cat /tmp/test_payload | md5` — note checksum. Re-run with a forced reconnect mid-stream.
  Confirm SOMETHING gets through (perfect parity requires the offset-resume protocol that
  was explicitly rejected — this test merely confirms no panic / hang).
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 10. Status bar glyph row composited
expected: |
  The inline status bar shows the formatted text glyphs (`Reconnecting to {profile}…
  (attempt N)`) overlaid on the bar surface — composited via the cell pipeline like
  SearchBarPass does. See Gap #4: glyph compositing is NOT YET wired in the render hook;
  the bar background renders but the text row is currently missing.
result: [pending]
blocked_by: prior-phase
reason: "Glyph compositing TODO + main.rs wiring TODO."

### 11. Multi-pane independent reconnect
expected: |
  Open two Dev Tunnel panes (or split a remote pane). Force a disconnect on one only.
  The other pane keeps responding; only the affected pane shows the Reconnecting status
  bar + tab badge. Local panes are unaffected.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 3."

## Summary

total: 11
passed: 0
issues: 0
pending: 11
skipped: 0
blocked: 11

## Gaps

<!-- Known limitations carried forward from Plan 09-05 implementation. -->
<!-- These are pre-recorded so /gsd:audit-uat surfaces them when the UAT is eventually run. -->

- truth: "Reconnect status bar surface color matches the active ChromePalette in both light and dark mode"
  status: failed
  reason: "Implementation hardcodes the dark-mode chrome.surface RGBA; light-mode palette is not threaded through to ReconnectPass::update yet. Acceptable v1 limitation; bar is still visible in light mode but contrast may be off."
  severity: cosmetic
  test: 4
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "Cursor dims to 40% alpha and stops blinking during the Reconnecting state"
  status: failed
  reason: "Existing cursor pipeline does not currently accept a reconnecting/alpha-multiplier flag. UI-SPEC §Input-Lock Affordances calls for dimmed cursor; deferred as a v1 fallback per Plan 09-05 task 2 interfaces note. Status bar is the primary signal."
  severity: minor
  test: 4
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "Status bar animates in (120 ms fade) after the 250 ms debounce and animates out (200 ms fade) on PaneReconnected"
  status: failed
  reason: "Fade-in curve is implemented and active; fade-out is NOT — Plan 09-05 implementation removes the reconnecting_panes entry immediately on PaneReconnected so the bar disappears in a single frame instead of animating out over 200 ms. Documented as a backlog polish item in the plan."
  severity: minor
  test: 4
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "Inline status bar shows the formatted text glyphs overlaid on the bar surface (Reconnecting to {profile}… (attempt N))"
  status: failed
  reason: "GLYPH ROW NOT YET COMPOSITED. The bar background renders via chrome_pipelines.reconnect.render, but the text-overlay step (analogous to SearchBarPass::render_text via the cell pipeline) is not wired in the per-pane render hook. The bar will appear as a colored strip with no text until this lands."
  severity: major
  test: 10
  root_cause: ""
  artifacts:
    - path: "crates/vector-app/src/app.rs"
      issue: "Render hook calls chrome_pipelines.reconnect.update + .render but skips the cell-pipeline text composition that SearchBarPass uses"
  missing:
    - "Wire format_reconnect_text output through the cell-pipeline glyph rasterizer at pane_rect.x + 8, pane_rect.y + 4 (or equivalent)"
  debug_session: ""

- truth: "DevTunnelsActor is constructed in main.rs and the picker actor can route UserEvent::DevTunnelPaneCancelToken back to the App"
  status: failed
  reason: "BLOCKING for the live UAT. Plan 09-05 task 1 wired ReconnectableDevTunnelDomain inside devtunnels_actor.rs::handle_connect, but the App-level constructor that owns the DevTunnelsActor (in main.rs) is not yet calling the new construction path. Without this wiring, Cmd-Shift-T cannot reach a real ReconnectableDevTunnelDomain, so the entire reconnect smoke matrix is unreachable end-to-end. A follow-up plan must wire DevTunnelsActor construction in main.rs before Test 1 can run."
  severity: blocker
  test: 1
  root_cause: ""
  artifacts:
    - path: "crates/vector-app/src/main.rs"
      issue: "DevTunnelsActor not constructed / not connected to the event-loop proxy"
  missing:
    - "Construct DevTunnelsActor in main.rs with the same event-loop proxy used by the App; route UserEvent::DevTunnelPaneCancelToken back to App.pane_cancel_tokens"
  debug_session: ""
