---
status: partial
phase: 09-persistence-reconnect-tmux-auto-attach
source: [09-06-SUMMARY.md, 09-SMOKE.md]
started: 2026-05-22T21:00:00Z
updated: 2026-05-22T21:00:00Z
---

## Current Test

[testing paused — automated portion (3 items) + manual portion (13 items) outstanding; blocked on `DevTunnelsActor` construction wiring in `main.rs` (same root cause as 09-05-HUMAN-UAT.md Gap #5; separate follow-up plan)]

## Tests

### Automated portion (live e2e via `cargo test --ignored`)

### 1. osc52_round_trip
expected: |
  With `VECTOR_E2E_TUNNEL_ID` + `VECTOR_E2E_MICROSOFT_TOKEN` exported and the user
  already running `tmux new -s smoke; tmux set-option -g allow-passthrough on` on
  the remote, `cargo test -p vector-tunnels --test live_devtunnel_smoke -- --ignored
  --test-threads=1 --nocapture` runs `osc52_round_trip` and PASSES. The test
  pre-checks `$TMUX` is non-empty on the remote and round-trips a 200-byte OSC 52
  payload via DCS-wrapped form through user-managed tmux, asserting multi-chunk
  reassembly through the relay.
result: [pending]
blocked_by: prior-phase
reason: "Picker actor → ReconnectableDevTunnelDomain path requires DevTunnelsActor to be constructed in main.rs; same blocker as 09-05-HUMAN-UAT.md Gap #5. Without that wiring, the runtime connect path the test exercises end-to-end (Cmd-Shift-T → DT picker → tunnel) cannot be reached for live UAT-grade validation of the surrounding feature. The test itself is `#[ignore]`d + env-gated and is structurally green (compiles + lists), but signing PERSIST-04 off requires the wiring."

### 2. decscusr_and_mouse_modes
expected: |
  Same env + USER-RUN tmux setup as test 1. Sends DECSCUSR cursor-shape escapes
  (`\\e[3 q`), enables mouse modes 1000/1002/1003 with SGR 1006, then asserts
  `stty -a | head -1` contains `rows 24` and `columns 80` — proving the window
  size propagated through `open_pty` on connect. No hang, no error.
result: [pending]
blocked_by: prior-phase
reason: "Same DevTunnelsActor main.rs wiring gap; cannot sign off PERSIST-04 end-to-end."

### 3. term_xterm_256color_advertised
expected: |
  Same env + USER-RUN tmux setup. Reads `$TERM` from the remote shell and asserts
  exact equality with `xterm-256color` (the value `vector-tunnel-agent` sets per
  Phase 8 contract).
result: [pending]
blocked_by: prior-phase
reason: "Same DevTunnelsActor main.rs wiring gap."

### Manual portion (full reconnect UX + tmux pass-through, end-to-end)

### 4. vim inside user's tmux session on remote
expected: |
  Inside the user's `smoke` tmux session on the remote, run `vim hello.txt`. vim
  opens, cursor visible.
result: [pending]
blocked_by: prior-phase
reason: "Cannot reach a live remote pane without DevTunnelsActor main.rs wiring."

### 5. Write + save a file with :wq
expected: |
  Inside the same tmux session, type some text and save with `:wq`. File is
  written.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 6. htop renders correctly w/ colors + smooth scrolling
expected: |
  Inside the same tmux session, run `htop`. htop renders correctly with 256-color
  output and ProMotion-smooth scrolling on the Vector side.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 7. OSC 52 inside tmux — small payload
expected: |
  `printf '\\e]52;c;%s\\a' "$(echo hello | base64)"` inside the user's tmux session
  puts `hello` on the macOS clipboard.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 8. OSC 52 inside tmux — 200-byte payload (Pitfall 7)
expected: |
  `printf '\\e]52;c;%s\\a' "$(head -c 200 /dev/urandom | base64)"` inside tmux
  puts 200 bytes of base64 on the macOS clipboard. Verify with
  `pbpaste | base64 -d | wc -c` → reports 200. Validates 58-byte outbound chunking
  + multi-chunk DCS reassembly through user-managed tmux.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 9. DECSCUSR cursor shapes (visual)
expected: |
  `printf '\\e[1 q'` (blink block), `printf '\\e[3 q'` (blink underline),
  `printf '\\e[5 q'` (blink bar) each visibly change the cursor shape in Vector.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 10. Mouse mode SGR 1006
expected: |
  After `printf '\\e[?1000h\\e[?1006h'`, clicking inside the terminal area causes
  Vector to send SGR 1006 sequences to the remote (visible via `cat` if the shell
  echoes them).
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 11. `tput cols && tput lines` reports actual viewport size
expected: |
  Reports the real viewport size of the Vector pane (not 80×24 default).
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 12. `echo $TERM` reports `xterm-256color`
expected: |
  `echo $TERM` inside the remote shell reports exactly `xterm-256color` (Phase 8
  agent contract).
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 4."

### 13. Reconnect: force a disconnect — Vector flips to [reconnecting]
expected: |
  With htop running inside the user's tmux session, in a separate SSH session run
  `pkill -f vector-tunnel-agent`. Vector inline status bar appears; tab title
  flips to `[reconnecting]`; cursor stops blinking. (See 09-05-HUMAN-UAT.md
  Gap #1–#4 for known visual limitations carried forward.)
result: [pending]
blocked_by: prior-phase
reason: "Reconnect flow exercises the same App-side state map as 09-05; both UATs share the DevTunnelsActor main.rs wiring blocker."

### 14. Input lock during reconnect — single toast
expected: |
  After step 13, wait ~8 s, then try typing. Toast `Input ignored — reconnecting`
  appears once; subsequent keystrokes silent-drop.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 13."

### 15. Recovery: agent restart → reconnect within next backoff slot
expected: |
  Restart `vector-tunnel-agent` on the remote. Vector reconnects within the next
  backoff slot; tab title returns to `[remote]`. Reattaching to the user's tmux
  session via `tmux attach -t smoke` shows htop STILL RUNNING (user's tmux
  persisted across the disconnect, which is the whole point of PERSIST-03's
  revised user-managed-tmux contract).
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 13."

### 16. Cmd-W during reconnect cancels promptly
expected: |
  Re-trigger a disconnect (pkill the agent again). With the status bar visible,
  hit Cmd-W. The pane closes immediately — does NOT wait for the next backoff
  slot. Per-pane CancellationToken fires and the reconnect loop exits.
result: [pending]
blocked_by: prior-phase
reason: "Depends on test 13."

## Summary

total: 16
passed: 0
issues: 0
pending: 16
skipped: 0
blocked: 16

## Gaps

<!-- Pre-recorded so /gsd:audit-uat surfaces the carried-forward blockers and limitations. -->
<!-- The DevTunnelsActor main.rs wiring gap is shared with 09-05-HUMAN-UAT.md Gap #5; intentionally duplicated here so both UATs surface it independently. -->

- truth: "DevTunnelsActor is constructed in main.rs and the picker actor can route UserEvent::DevTunnelPaneCancelToken back to the App, enabling Cmd-Shift-T → DT picker → live tunnel pane end-to-end"
  status: failed
  reason: "BLOCKING for the 09-06 live UAT. Same root cause as 09-05-HUMAN-UAT.md Gap #5: the picker actor's `handle_connect` is correctly wired to build `ReconnectableDevTunnelDomain` and emit `UserEvent::DevTunnelPaneCancelToken`, but the App-level constructor that owns `DevTunnelsActor` (in `crates/vector-app/src/main.rs`) is not yet calling the new construction path. Without that wiring, Cmd-Shift-T does not flow into a live `ReconnectableDevTunnelDomain`, so neither the automated `--ignored` portion nor the manual reconnect matrix can be exercised end-to-end against a running app + live tunnel. The follow-up plan must wire `DevTunnelsActor` construction in main.rs with the App's event-loop proxy before this UAT (and 09-05's) can be signed off."
  severity: blocker
  test: 1
  root_cause: ""
  artifacts:
    - path: "crates/vector-app/src/main.rs"
      issue: "DevTunnelsActor not constructed / not connected to the event-loop proxy (same gap noted in 09-05-HUMAN-UAT.md Gap #5)"
  missing:
    - "Construct DevTunnelsActor in main.rs with the same event-loop proxy used by the App; route UserEvent::DevTunnelPaneCancelToken back to App.pane_cancel_tokens; verify 09-05-HUMAN-UAT.md AND 09-06-HUMAN-UAT.md can be walked end-to-end before closing Phase 9."
  debug_session: ""
