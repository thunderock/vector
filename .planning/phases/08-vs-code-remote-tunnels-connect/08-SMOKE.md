# Phase 8 — Manual Smoke Matrix

**Date:** _____ (fill at sign-off)
**Tester:** _____
**Vector commit:** `_______________` (`git rev-parse HEAD`)
**vector-tunnel-agent .deb version:** `_______________`

## Pre-conditions
- Vector.app launched from a built DMG or `cargo run --release -p vector-app`.
- Linux remote box (Debian/Ubuntu) accessible. Has `wget` + `apt`.
- User signed out of both GitHub and Microsoft inside Vector.

## Items

### Item 1 — DT-01: Spike doc exists and is committed (no live system needed)
- [ ] `test -f .planning/research/spikes/dev-tunnels-decision.md` exits 0.
- [ ] `grep "Path 2 Variant 2c"` of the file returns at least 1 hit.
- Expected: file contains the locked decision codified by Plan 08-01 Task 1 Step 0.

### Item 2 — DT-02: Sign in with Microsoft + list shows ONLY vector-agent tunnels
1. Vector menu → "Sign in with Microsoft".
2. Modal opens at 480x280 with 32pt mono user-code (UI-SPEC).
3. Complete the device flow in a browser.
4. Modal dismisses; toast appears: "Signed in to Microsoft." (UI-SPEC verbatim).
5. Cmd-Shift-T opens DevTunnelsPickerModal at 640x480.
6. Picker footer reads either "Loading Dev Tunnels…" (briefly) then "{N} of {M} tunnels." OR "No Vector-agent tunnels yet. Install vector-tunnel-agent on a remote machine and run it." (verbatim).
- [ ] PASS / [ ] FAIL
- Notes: ______________

### Item 3 — DT-01 cont.: Agent install + first-run device flow on Linux remote
1. On remote: `wget https://github.com/colligo/vector/releases/latest/download/vector-tunnel-agent_X.Y.Z_amd64.deb`
2. `sudo apt install ./vector-tunnel-agent_*.deb` — postinst prints the install message.
3. `vector-tunnel-agent` — prints "To sign in, open https://..." with a user-code.
4. Complete the device flow in a browser.
5. Agent prints something like: "tunnel 'vector-{hostname}' registered. Waiting for connections."
- [ ] PASS / [ ] FAIL
- Notes: ______________

### Item 4 — DT-02: Picker now shows the newly-registered tunnel
1. With agent running from Item 3, go back to Vector picker (Cmd-Shift-T, may need to refresh via 'R').
2. The tunnel appears in the picker with display_name = `{hostname}` (NOT `vector-{hostname}` per D-09 prefix-strip).
3. Row format: `●  {display_name}  {host}  ·  {last_seen}` per UI-SPEC.
4. Status dot is green (●) — agent reachable, last-seen recent.
- [ ] PASS / [ ] FAIL
- Notes: ______________

### Item 5 — DT-03: Connect → live remote shell with [remote] badge
1. With the row selected in the picker, press Enter.
2. Picker dismisses. A new tab opens.
3. Tab title contains `[remote]` per UI-SPEC §Surfaces S5 + format_tab_title.
4. Cursor lands on a remote prompt (`user@hostname:~$`).
5. Run `hostname` — output matches the Linux remote box's hostname, NOT the Mac's.
6. Run `echo $TERM` — outputs `xterm-256color`.
- [ ] PASS / [ ] FAIL
- Notes: ______________

### Item 6 — DT-04: Microsoft-blue tint on active DevTunnel pane
1. With the remote pane focused, observe the tab tint stripe.
2. Stripe is Microsoft-blue `#0078d4` — visually distinct from local panes (no tint) and from any Phase 6 codespace pane (purple).
3. Spawn a new local tab (Cmd-T). Tint disappears (local has no tint).
4. Switch back to the DevTunnel pane (Cmd-Shift-[). Tint reappears.
- [ ] PASS / [ ] FAIL
- Notes: ______________

### Item 7 — Resize → remote `tput cols`/`tput lines` matches
1. In the remote pane, run `tput cols && tput lines`. Note the values.
2. Resize the Vector window to a different shape.
3. Rerun `tput cols && tput lines`. Values must match the new pane size.
- [ ] PASS / [ ] FAIL
- Notes: ______________

### Item 8 — Token-leak audit (Pitfall 14)
1. With Vector still running and remote pane active, run:
   `RUST_LOG=trace cargo run --release -p vector-app 2>&1 | tee /tmp/vector.log` (in a fresh terminal, separate Vector instance OK — what matters is logs from a session that included Microsoft sign-in + tunnel list + connect).
   Or, for a previously-running session, dump existing tracing logs.
2. Run: `grep -E 'gho_|ghp_|eyJ|Bearer [A-Za-z0-9._-]{20,}' /tmp/vector.log` — must return ZERO hits.
3. Also check `~/Library/Application Support/com.vector.vector/` (or wherever Vector logs to) for any persistent log file containing token-shaped strings — should be zero.
- [ ] PASS (zero hits) / [ ] FAIL (any hit)
- Notes: ______________

### Item 9 — Sign out of Microsoft → live pane survives
1. With remote pane still open, Vector menu → "Sign out of Microsoft".
2. Toast appears: "Signed out of Microsoft." (matches UI-SPEC).
3. The live DevTunnel pane KEEPS RUNNING (per UI-SPEC §Destructive actions: existing tunnel access token is already issued; reconnect is Phase 9 territory).
4. Open the picker again (Cmd-Shift-T). Footer reads "Sign in with GitHub or Microsoft to list Dev Tunnels." (UI-SPEC verbatim).
5. Close the picker (Esc); the live pane still works (type `echo hi` → echo arrives).
- [ ] PASS / [ ] FAIL
- Notes: ______________

## Sign-off

All 9 items PASS: [ ] yes / [ ] no

**User signature:** _____________  **Date:** _____________

## Post-sign-off updates

After the user signs off:
1. Update `.planning/REQUIREMENTS.md` — flip DT-01..04 to "Complete".
2. Update `.planning/ROADMAP.md` — Phase 8 row to "Complete".
3. Update `.planning/STATE.md` — increment completed_phases; record smoke completion date.
4. Commit:
   ```
   docs(08): close Phase 8 — DT-01..04 manual UAT 9/9 PASS, smoke approved {DATE}
   ```
