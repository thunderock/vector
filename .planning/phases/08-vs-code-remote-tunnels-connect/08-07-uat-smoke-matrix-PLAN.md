---
phase: 08-vs-code-remote-tunnels-connect
plan: 07
type: execute
wave: 4
depends_on: [05, 06]
files_modified:
  - .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md
autonomous: false
requirements:
  - DT-01
  - DT-02
  - DT-03
  - DT-04
user_setup:
  - service: live-linux-remote-box
    why: "Manual UAT requires a real Linux machine to install the .deb agent on (per CONTEXT.md: Adobe corporate Linux box or personal EC2 / home server)"
    env_vars: []
    dashboard_config:
      - task: "Ensure user has SSH or console access to at least ONE Linux remote box (Debian/Ubuntu) for installing the .deb"
        location: "user's own infrastructure"
  - service: github-and-microsoft-accounts
    why: "Two providers tested per D-03; user must have a Microsoft account (any: personal MSA or Adobe Entra) AND a GitHub account"
    env_vars: []
    dashboard_config: []
must_haves:
  truths:
    - "DT-01 spike decision document (committed by Plan 08-01 Task 1 Step 0) verified to exist at `.planning/research/spikes/dev-tunnels-decision.md`"
    - "All 9 smoke matrix items pass on real hardware against a real live agent"
    - "DT-01..04 marked Complete in REQUIREMENTS.md after user signs off"
  artifacts:
    - path: ".planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md"
      provides: "9-item smoke matrix with pass/fail boxes and user-signoff line"
      min_lines: 80
  key_links:
    - from: ".planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md Item 1"
      to: ".planning/research/spikes/dev-tunnels-decision.md"
      via: "Item 1 verifies the spike doc (committed by 08-01) exists"
      pattern: "dev-tunnels-decision\\.md"
    - from: "Sign-off"
      to: ".planning/REQUIREMENTS.md DT-01..04"
      via: "user flips boxes after 9/9 PASS"
      pattern: "DT-0[1-4]"
---

<objective>
Author the manual UAT smoke matrix template (9 items) and execute it. The DT-01 spike decision document was committed in Plan 08-01 Task 1 Step 0 (per ROADMAP §Phase 8 SC#1 ordering — spike-doc-before-integration-code); Item 1 of the smoke matrix verifies its existence rather than creating it.

Purpose: the user's smoke matrix sign-off is the only verification gate for the Phase 8 manual surface (per VALIDATION.md §Manual-Only Verifications). Without this, the phase is not Complete.
Output: 08-SMOKE.md with 9 ticked boxes; user sign-off line dated.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/phases/08-vs-code-remote-tunnels-connect/08-CONTEXT.md
@.planning/phases/08-vs-code-remote-tunnels-connect/08-RESEARCH.md
@.planning/phases/08-vs-code-remote-tunnels-connect/08-VALIDATION.md
@.planning/phases/08-vs-code-remote-tunnels-connect/08-UI-SPEC.md
@.planning/REQUIREMENTS.md
@.planning/ROADMAP.md
@.planning/research/spikes/dev-tunnels-decision.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Author the 9-item smoke matrix template</name>
  <files>.planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md</files>
  <read_first>
    - .planning/phases/08-vs-code-remote-tunnels-connect/08-VALIDATION.md (§Manual-Only Verifications — 5 baseline items; this plan expands to 9)
    - .planning/phases/08-vs-code-remote-tunnels-connect/08-CONTEXT.md (D-01..D-17 cross-reference)
    - .planning/phases/08-vs-code-remote-tunnels-connect/08-UI-SPEC.md (verbatim copy strings to assert on)
    - .planning/research/spikes/dev-tunnels-decision.md (committed by Plan 08-01 — Item 1 verifies it exists)
    - .planning/phases/05-polish-local-daily-driver/05-SMOKE.md (if exists — Phase 5 precedent format) OR Phase 4 04-SMOKE equivalent
  </read_first>
  <action>
    Create `.planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md`. 9 items mapping to DT-01..04 + UI-SPEC verbatim copy + Pitfall 14 token-leak audit. Format mirrors Phase 5 / Phase 4 SMOKE.md precedent.

    Body:

    ```markdown
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
    ```

    Do NOT mark anything as PASS in this plan — that requires user UAT. The file ships with empty boxes.
  </action>
  <verify>
    <automated>test -f .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md &amp;&amp; grep -c "Item " .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md &amp;&amp; grep -c "PASS / \\[ \\] FAIL" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md &amp;&amp; grep -q "DT-01\\|DT-02\\|DT-03\\|DT-04" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md &amp;&amp; grep -q "Pitfall 14\\|token-leak" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md &amp;&amp; grep -q "Microsoft-blue\\|#0078d4" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md &amp;&amp; test -f .planning/research/spikes/dev-tunnels-decision.md</automated>
  </verify>
  <acceptance_criteria>
    - File exists at .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md
    - grep -c "### Item " .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md == 9 (exactly 9 items)
    - grep -c "PASS / \\[ \\] FAIL" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md >= 8 (Items 2-9; Item 1 has different format)
    - grep -c "DT-0[1-4]" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md >= 4 (each requirement referenced)
    - grep -c "vector-tunnel-agent" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md >= 3
    - grep -c "Sign in with Microsoft\\|Sign out of Microsoft" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md >= 2
    - grep -c "Cmd-Shift-T" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md >= 1
    - grep -c "#0078d4\\|Microsoft-blue\\|Microsoft blue" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md >= 1
    - grep -c "Pitfall 14\\|token-leak\\|gho_\\|ghp_" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md >= 1
    - grep -c "Sign-off\\|signature" .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md >= 1
    - test -f .planning/research/spikes/dev-tunnels-decision.md (spike doc from Plan 08-01 is on disk — Item 1 verifies, this task does not create)
  </acceptance_criteria>
  <done>08-SMOKE.md ships with 9 unticked items + sign-off line + post-sign-off update procedure. Spike doc (created by Plan 08-01) verified present.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 2: User executes 9-item manual smoke matrix and signs off</name>
  <what-built>
    - All Phase 8 implementation plans (08-01..06) shipped and committed.
    - vector-tunnel-agent .deb published to GitHub Releases (via Plan 08-06 agent-release.yml on user's tag push).
    - 9-item smoke matrix template ready at .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md.
    - DT-01 spike doc committed at .planning/research/spikes/dev-tunnels-decision.md (by Plan 08-01).
  </what-built>
  <files>(verification only — no file writes)</files>
  <action>This is a checkpoint task. Claude pauses; the human runs the steps in <how-to-verify> and types the resume signal. No code changes in this task.</action>
  <how-to-verify>
    Walk through every item in `.planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md`:
    1. Read Item 1, run the two `test -f` + `grep` commands, tick the box.
    2. Read Items 2-9 in order, follow each step, tick PASS or FAIL with notes.
    3. Sign + date at the bottom.
    4. After all 9 PASS:
       - Edit `.planning/REQUIREMENTS.md`: change DT-01..04 from `[ ]` to `[x]` and flip phase status under `## Traceability` from "Pending" to "Complete".
       - Edit `.planning/ROADMAP.md`: change Phase 8 row in `## Phase Progress` to "Complete" + completion date.
       - Edit `.planning/STATE.md`: increment `completed_phases`; add Phase 8 completion entry under `## Accumulated Context > Key Decisions` summarizing the 9/9 PASS.
       - Commit: `docs(08): close Phase 8 — DT-01..04 manual UAT 9/9 PASS, smoke approved {YYYY-MM-DD}` (no push per CLAUDE.md).
    5. If any item FAILS, do NOT update REQUIREMENTS/ROADMAP/STATE. Open a gap-closure plan via `/gsd:plan-phase 08 --gaps`.
  </how-to-verify>
  <verify>Manual — human executes the verification checklist above. No automated check.</verify>
  <done>Human types the resume signal with approval notes (or paste failure details).</done>
  <resume-signal>Type "approved 9/9 PASS" with the date, OR list which items FAILED and paste notes.</resume-signal>
</task>

</tasks>

<verification>
- `.planning/research/spikes/dev-tunnels-decision.md` exists (created by Plan 08-01) — Item 1 of SMOKE matrix asserts this
- `.planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md` exists with exactly 9 numbered items + sign-off line
- After Task 2: REQUIREMENTS.md flips DT-01..04 to complete, ROADMAP.md flips Phase 8 to Complete, STATE.md records the 9/9 PASS sign-off
</verification>

<success_criteria>
- DT-01 satisfied: spike doc committed (Plan 08-01) + verified by Item 1 of smoke matrix
- DT-02/03/04 satisfied: 9-item smoke matrix PASS by user
- REQUIREMENTS.md / ROADMAP.md / STATE.md updated to reflect Phase 8 completion
- Phase 8 closed without violating CLAUDE.md "do not push" — user pushes asynchronously
</success_criteria>

<output>
After completion, create `.planning/phases/08-vs-code-remote-tunnels-connect/08-07-SUMMARY.md` documenting:
- The 9/9 sign-off date + any per-item notes
- Any deltas from the locked decision (none expected; if any, flag clearly)
- Pointer to the post-sign-off REQUIREMENTS / ROADMAP / STATE update commit hash
</output>
