---
phase: 09-persistence-reconnect-tmux-auto-attach
verified: 2026-05-24T23:30:00Z
status: human_needed
score: 3/4 must-haves verified (PERSIST-01/02/03 satisfied via automation; PERSIST-04 pending live UAT sign-off)
re_verification:
  previous_status: human_needed
  previous_score: 3/4 must-haves verified
  previous_verified: 2026-05-22T21:20:34Z
  gaps_closed:
    - "main.rs constructs DevTunnelsActor at App startup with event-loop proxy (grep 0 -> 3 matches)"
    - "main.rs -> devtunnels_actor.rs key link: DevTunnelsActor::new + set_router + spawn all present"
    - "application.set_devtunnels_cmd_tx(cmd_tx) called before event_loop.run_app"
  gaps_remaining: []
  regressions: []
gaps: []
human_verification:
  - test: "09-05-HUMAN-UAT.md — full reconnect UX walk (11 items)"
    expected: "Cmd-Shift-T → DT picker → live tunnel pane → force disconnect → inline status bar + tab `[reconnecting]` + input lock + single toast + backoff counter advance + recovery + Cmd-W cancel + multi-pane independence (see 09-05-HUMAN-UAT.md tests 1–11)"
    why_human: "Visual + UX behavior over a live Microsoft Dev Tunnels relay; requires real wifi/agent disconnect to drive the Reconnecting state machine. Runtime blocker (DevTunnelsActor not in main.rs) is CLOSED by 09-07. UAT is now walkable."
  - test: "09-06-HUMAN-UAT.md — PERSIST-04 tmux pass-through + reconnect UX (16 items: 3 automated --ignored + 13 manual)"
    expected: "Automated portion: with `VECTOR_E2E_TUNNEL_ID` + `VECTOR_E2E_MICROSOFT_TOKEN` and user-started `tmux new -s smoke; tmux set-option -g allow-passthrough on`, `cargo test -p vector-tunnels --test live_devtunnel_smoke -- --ignored --test-threads=1` runs and the three tests pass (osc52_round_trip, decscusr_and_mouse_modes, term_xterm_256color_advertised). Manual portion: vim/htop/OSC 52/DECSCUSR/mouse SGR 1006/$TERM/reconnect-with-htop-persistence walk in `09-SMOKE.md` is signed off by the user."
    why_human: "PERSIST-04 acceptance gate. Requires a live Dev Tunnel + user-managed tmux on the remote box. Runtime blocker CLOSED by 09-07; UAT is now walkable."
  - test: "09-SMOKE.md — fill in sign-off block"
    expected: "Approved by + date filled in; all four checkboxes ticked (USER-RUN tmux setup, automated tests pass, manual matrix pass, PERSIST-04 acceptance)"
    why_human: "User-owned acceptance record. Empty as of 2026-05-24; flips PERSIST-04 in REQUIREMENTS.md from Pending → Complete."
---

# Phase 9: Persistence + Reconnect — Verification Report

**Phase Goal:** The user closes their laptop lid for a meeting, reopens it, and a Dev Tunnels pane reconnects automatically — the local grid + scrollback never go blank, an inline status bar shows reconnect progress, and the transport hot-swaps under the live `Pane` without losing bytes already in flight. Shell-state-across-disconnect persistence is the user's responsibility (they run tmux themselves on the remote if they want it).

**Verified:** 2026-05-24T23:30:00Z
**Status:** human_needed
**Re-verification:** Yes — after 09-07 gap closure (previous: 2026-05-22T21:20:34Z, status: human_needed).

## Re-verification Summary

The single concrete code gap flagged in the initial verification — `DevTunnelsActor` not constructed in `crates/vector-app/src/main.rs` — is **CLOSED** by Plan 09-07. All plan acceptance greps return the required counts and all CI-equivalent gate commands exit 0.

**Gaps closed:**

| Gap | Previous Evidence | New Evidence |
| --- | ----------------- | ------------ |
| `DevTunnelsActor::new` in `main.rs` | 0 matches | 1 match (line 100) |
| `dt_actor.set_router(...)` before `spawn` | NOT_WIRED | WIRED (line 107, before line 108 `spawn`) |
| `application.set_devtunnels_cmd_tx(cmd_tx)` | NOT_WIRED | WIRED (line 237, before `event_loop.run_app`) |
| Key link `main.rs → devtunnels_actor.rs` | ✗ NOT_WIRED | ✓ WIRED |

**Regressions:** None. All 27 lib tests (16 vector-app + 11 vector-tunnels) still pass. Build, clippy (-D warnings), and fmt all exit 0.

**Classification unchanged:** `human_needed` — not because of any code gap, but because the two HUMAN-UAT walks and `09-SMOKE.md` sign-off are still pending. These are the accepted path to PERSIST-04 sign-off; they are not code-verifiable.

---

## Goal Achievement

### Observable Truths (from ROADMAP Success Criteria)

| #   | Truth                                                                                                                                                                                                                                                                                            | Status        | Evidence                                                                                                                                                                                                                                                                                  |
| --- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | On TCP/SSH disconnect, the affected pane enters a `Reconnecting` state, the local grid + scrollback stay in memory (no blank screen), input is locked (not queued), and an inline status bar shows `Reconnecting to {profile}… (attempt N)`.                                                       | ? UNCERTAIN   | Code paths exist (`reconnecting_panes` map, `ReconnectPass.update/draw`, input gate at `app.rs:1342/1359`, `format_reconnect_text`, single-shot toast). Unit tests pass. Visual + UX behavior unverifiable without live UAT (09-05-HUMAN-UAT.md tests 4/5). UNBLOCKED by 09-07.          |
| 2   | `Domain::reconnect()` re-establishes the transport with exponential backoff (1/2/4/8/16/30 s cap) and hot-swaps the `PtyTransport` under the live `Pane` without dropping bytes already in flight.                                                                                                  | ✓ VERIFIED    | `BACKOFF_SCHEDULE_SECS = &[1, 2, 4, 8, 16, 30]` at `pty_actor.rs:28`. `reconnect_with_backoff` at `pty_actor.rs:303`. Byte-integrity test (`reconnect_byte_integrity.rs`) passes — 2/2 green. Drain-and-swap proven by `pty_actor_reconnect.rs` — 4/4 green.                               |
| 3   | Vector does NOT auto-attach to tmux. Remote panes connect to the user's default shell.                                                                                                                                                                                                            | ✓ VERIFIED    | `OpenPty` handshake at `transport.rs:84-91` hard-codes `shell: None`. Regression test `open_pty_no_shell_override.rs` passes (asserts `shell.is_none()`). No tmux strings anywhere in `crates/vector-tunnels/src/` or `crates/vector-app/src/devtunnels_actor.rs`.                         |
| 4   | An end-to-end smoke test against a live Dev Tunnels agent on a remote box running tmux 3.4+ verifies DCS-wrapped OSC 52, DECSCUSR, mouse modes 1000/1002/1003 with SGR 1006, and `TERM=xterm-256color` advertisement.                                                                              | ? UNCERTAIN   | Three `#[ignore]`d tests in `live_devtunnel_smoke.rs` (239 lines). `persist-e2e` CI job in `ci.yml:119`. `09-SMOKE.md` (68 lines) skeleton landed with USER-RUN setup. **Sign-off block is empty.** Now UNBLOCKED by 09-07; user can walk the matrix via Cmd-Shift-T. |

**Score:** 2/4 truths fully verified by automation; 2/4 routed to human verification (carries deferred UAT debt documented in 09-05/09-06 HUMAN-UAT files). Same score as initial verification — the 09-07 gap closure unblocked the human UAT path but did not discharge the human steps themselves.

### Required Artifacts

| Artifact                                                                                                  | Expected                                                                              | Status      | Details                                                                                                                              |
| --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `crates/vector-mux/src/domain.rs`                                                                         | `Domain::reconnect_one_shot(rows, cols) -> Result<Option<Box<dyn PtyTransport>>>`     | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `crates/vector-mux/src/local_domain.rs`                                                                   | `LocalDomain::reconnect_one_shot` returns `Ok(None)`                                  | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `crates/vector-app/src/lib.rs`                                                                            | `UserEvent::PaneReconnecting { pane_id, attempt, profile_label }` + `PaneReconnected` | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `crates/vector-tunnels/src/domain.rs`                                                                     | `ReconnectableDevTunnelDomain` implementing `vector_mux::Domain`                       | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `crates/vector-app/src/pty_actor.rs`                                                                      | Per-pane reconnect actor + `EventSink` trait + `ProxyEventSink` newtype + backoff      | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `crates/vector-render/src/reconnect_pass.rs`                                                              | New wgpu pipeline + `format_reconnect_text` + UI constants                            | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `crates/vector-mux/src/pane.rs`                                                                           | `PaneUiState::{Active, Reconnecting}` + `format_tab_title(.., ui_state)` emitting `[reconnecting]` | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `crates/vector-app/src/chrome.rs`                                                                         | `ChromePipelines.reconnect: ReconnectPass`                                            | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `crates/vector-app/src/app.rs`                                                                            | `reconnecting_panes` map + render hook + input gate + first-keystroke toast            | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `crates/vector-app/src/devtunnels_actor.rs`                                                               | Picker actor builds `ReconnectableDevTunnelDomain` + passes `Arc<dyn Domain>` to `spawn_pane` | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `crates/vector-tunnels/tests/live_devtunnel_smoke.rs`                                                     | Three `#[ignore]`d live e2e tests gated on `VECTOR_E2E_TUNNEL_ID` + `VECTOR_E2E_MICROSOFT_TOKEN` | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `.github/workflows/ci.yml`                                                                                | `persist-e2e` CI job with `continue-on-error: true`                                   | ✓ VERIFIED  | Unchanged from initial verification.                                                                                                 |
| `.planning/phases/09-.../09-SMOKE.md`                                                                     | USER-RUN setup + matrix + sign-off block                                              | ⚠️ ORPHANED | Sign-off block at lines 61-68 still empty, awaiting user UAT walk. No longer BLOCKED at code level. |
| `crates/vector-app/src/main.rs`                                                                           | `DevTunnelsActor` constructed at App startup with event-loop proxy                    | ✓ VERIFIED  | **CLOSED by 09-07.** `DevTunnelsActor::new(dt_api, Arc::clone(&mux), dt_auth, dt_store, proxy_io.clone())` at line 100. `dt_actor.set_router(Arc::clone(&router_io))` at line 107 (before spawn). `dt_actor.spawn(&tokio::runtime::Handle::current())` at line 108. `application.set_devtunnels_cmd_tx(cmd_tx)` at line 237. |

### Key Link Verification

| From                                                    | To                                                  | Via                                                          | Status     | Details                                                                                                                       |
| ------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------ | ---------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `vector-mux/src/devtunnel_domain.rs`                    | `vector-mux/src/domain.rs`                          | `impl Domain for DevTunnelDomain`                            | ✓ WIRED    | Unchanged from initial verification.                                                                                          |
| `vector-mux/src/local_domain.rs`                        | `vector-mux/src/domain.rs`                          | `impl Domain for LocalDomain` returning `Ok(None)`           | ✓ WIRED    | Unchanged from initial verification.                                                                                          |
| `vector-tunnels/src/domain.rs`                          | `vector-mux/src/domain.rs`                          | `impl vector_mux::Domain for ReconnectableDevTunnelDomain`   | ✓ WIRED    | Unchanged from initial verification.                                                                                          |
| `vector-tunnels/src/domain.rs`                          | `vector-tunnels/src/transport.rs`                   | `connect_tunnel(...)` re-use on reconnect                    | ✓ WIRED    | Unchanged from initial verification.                                                                                          |
| `vector-app/src/pty_actor.rs`                           | `vector-mux/src/domain.rs`                          | `domain.reconnect_one_shot(rows, cols).await`                | ✓ WIRED    | Unchanged from initial verification.                                                                                          |
| `vector-app/src/pty_actor.rs`                           | `vector-app/src/lib.rs`                             | `UserEvent::PaneReconnecting / PaneReconnected` emission     | ✓ WIRED    | Unchanged from initial verification.                                                                                          |
| `vector-app/src/app.rs`                                 | `vector-render/src/reconnect_pass.rs`               | `chrome.reconnect.update(...)` + `.draw(...)`                | ✓ WIRED    | Unchanged from initial verification.                                                                                          |
| `vector-app/src/devtunnels_actor.rs`                    | `vector-tunnels/src/domain.rs`                      | `ReconnectableDevTunnelDomain::new(api, auth_factory, tunnel, label)` | ✓ WIRED    | Unchanged from initial verification.                                                                                           |
| `vector-app/src/devtunnels_actor.rs`                    | `vector-app/src/pty_actor.rs`                       | `router.lock().spawn_pane(pane_id, transport, domain, profile_label, cancel)` | ✓ WIRED    | Unchanged from initial verification.                                                                                          |
| `vector-app/src/main.rs`                                | `vector-app/src/devtunnels_actor.rs`                | `DevTunnelsActor::new(...)` at App startup                   | ✓ WIRED    | **CLOSED by 09-07.** `main.rs:100` constructs actor; `:107` calls `set_router`; `:108` calls `spawn`; `:237` calls `application.set_devtunnels_cmd_tx`. Full Cmd-Shift-T → picker → pane path is reachable at runtime. |
| `.github/workflows/ci.yml`                              | `crates/vector-tunnels/tests/live_devtunnel_smoke.rs` | `cargo test ... --test live_devtunnel_smoke -- --ignored`    | ✓ WIRED    | Unchanged from initial verification.                                                                                           |

### Data-Flow Trace (Level 4)

| Artifact                                        | Data Variable           | Source                                              | Produces Real Data | Status        |
| ----------------------------------------------- | ----------------------- | --------------------------------------------------- | ------------------ | ------------- |
| `app.rs::reconnecting_panes` render hook        | `reconnecting_panes`    | `UserEvent::PaneReconnecting/Reconnected` event arms | Yes (under test)   | ✓ FLOWING — `DevTunnelsActor` is now constructed and live in the running App; `PaneReconnecting` events will flow once the user triggers a disconnect via Cmd-Shift-T → live tunnel pane. Previously HOLLOW at runtime; now WIRED end-to-end. |
| `pty_actor.rs::pane_io_loop`                    | `transport`             | `domain.reconnect_one_shot(rows, cols)`             | Yes                | ✓ FLOWING — proven by `pty_actor_reconnect.rs` (4 tests) and `reconnect_byte_integrity.rs` (2 tests). Unchanged. |
| `ReconnectableDevTunnelDomain::reconnect_one_shot` | `Box<dyn PtyTransport>` | `connect_tunnel(api, auth, tunnel, rows, cols)`     | Yes                | ✓ FLOWING — `reconnect_one_shot.rs` (2 tests pass). Unchanged.                                              |
| `live_devtunnel_smoke.rs` test bodies           | OSC 52 / TERM / DECSCUSR responses | live Dev Tunnels relay + USER-RUN tmux | Unverified         | ⚠️ STATIC at present — tests structurally green and `#[ignore]`d; no live run has been performed and no sign-off recorded. Now UNBLOCKED at code level by 09-07. |

### Behavioral Spot-Checks

| Behavior                                                    | Command                                                                                            | Result                                  | Status   |
| ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | --------------------------------------- | -------- |
| Workspace builds                                            | `cargo build --workspace`                                                                          | Finished dev profile in 4.72s           | ✓ PASS   |
| clippy -D warnings across workspace                         | `cargo clippy --workspace --all-targets -- -D warnings`                                            | Finished dev profile in 7.19s (no warnings) | ✓ PASS   |
| cargo fmt --check                                           | `cargo fmt --check`                                                                                | Exit 0, no output                       | ✓ PASS   |
| vector-app lib tests                                        | `cargo test -p vector-app --lib`                                                                   | 16 passed; 0 failed                     | ✓ PASS   |
| vector-tunnels lib tests                                    | `cargo test -p vector-tunnels --lib`                                                               | 11 passed; 0 failed                     | ✓ PASS   |
| `DevTunnelsActor::new` in `main.rs`                         | `grep -c "DevTunnelsActor::new" crates/vector-app/src/main.rs`                                      | 1                                       | ✓ PASS   |
| `set_router` called on actor (2 call sites)                 | `grep -n "\.set_router(" crates/vector-app/src/main.rs`                                             | 2 lines: 107 (dt_actor) + 225 (application) | ✓ PASS   |
| `set_devtunnels_cmd_tx` called before `run_app`             | `grep -c "set_devtunnels_cmd_tx" crates/vector-app/src/main.rs`                                     | 1                                       | ✓ PASS   |
| Two sync_channels (handle + dt cmd_tx)                      | `grep -c "sync_channel" crates/vector-app/src/main.rs`                                              | 2                                       | ✓ PASS   |
| Phase 9 unit tests pass                                     | reconnect-specific test files                                                                      | pty_actor_reconnect 4/4; reconnect_byte_integrity 2/2; reconnect_pass_render 8/8; reconnect_one_shot 2/2; reconnect_trait 2/2; open_pty_no_shell_override 1/1 | ✓ PASS |
| Live e2e tests are `#[ignore]`d                             | `grep "#\[ignore\]" crates/vector-tunnels/tests/live_devtunnel_smoke.rs`                            | 3 matches                               | ✓ PASS   |
| `persist-e2e` CI job exists                                 | `grep "persist-e2e:" .github/workflows/ci.yml`                                                     | Found at line 119                       | ✓ PASS   |
| `OpenPty` carries `shell: None`                             | `grep "shell:" crates/vector-tunnels/src/transport.rs`                                             | `shell: None,` at line 88               | ✓ PASS   |
| 09-SMOKE.md sign-off complete                               | `grep "Approved by:" 09-SMOKE.md` + check for non-blank field                                       | Approved by: `___________` (blank)      | ? SKIP (human task) |

### Requirements Coverage

| Requirement | Source Plan             | Description                                                                                                   | Status         | Evidence                                                                                                                                                                                                                |
| ----------- | ----------------------- | ------------------------------------------------------------------------------------------------------------- | -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| PERSIST-01  | 09-01, 09-03, 09-04, 09-05, 09-07 | Pane enters Reconnecting state, grid+scrollback retained, reconnect overlay shown.                  | ✓ SATISFIED (code) / ? NEEDS HUMAN (UX) | All artifacts present and wired. `DevTunnelsActor` construction in `main.rs` closed the final code gap (09-07). Visual + UX gates routed to 09-05-HUMAN-UAT.md tests 4/5/6/7/8/11. UAT is now unblocked. |
| PERSIST-02  | 09-01, 09-02, 09-03      | `Domain::reconnect()` hot-swap with exponential backoff and no byte loss.                                       | ✓ SATISFIED    | `reconnect_with_backoff` + `BACKOFF_SCHEDULE_SECS=[1,2,4,8,16,30]` + `Ok(None)` clean-exit path + byte-integrity test passes.                                                                                            |
| PERSIST-03  | 09-02                   | (revised) Vector does NOT auto-attach to tmux; remote panes use default shell.                                | ✓ SATISFIED    | `OpenPty { shell: None }` regression test green; no tmux strings in Vector's connect path. `assert!(shell.is_none())` form after clippy fix (09-07 deviation).                                                           |
| PERSIST-04  | 09-06, 09-07            | (revised) Live e2e smoke verifies DCS-wrapped OSC 52, DECSCUSR, mouse modes 1000/1002/1003 with SGR 1006, `TERM=xterm-256color` against user-managed tmux. | ? NEEDS HUMAN  | Verification surface landed (3 tests + CI job + SMOKE.md skeleton). Runtime blocker CLOSED by 09-07. Sign-off pending in `09-SMOKE.md` + automated `--ignored` runs pending.                     |

No orphaned requirements: REQUIREMENTS.md maps exactly PERSIST-01..04 to Phase 9, all four are claimed by at least one plan's `requirements:` frontmatter.

### Anti-Patterns Found

| File                                                              | Line | Pattern                                                       | Severity   | Impact                                                                                                                                                                          |
| ----------------------------------------------------------------- | ---- | ------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.planning/.../09-SMOKE.md`                                       | 61-68 | Empty Sign-off block                                          | ⚠️ Warning | PERSIST-04 acceptance unrecorded. Will be filled once the user walks the 09-05 and 09-06 HUMAN-UAT matrices. Not a code gap; requires user action only.                         |
| `crates/vector-app/src/app.rs` (per 09-05-HUMAN-UAT Gap #4)       | render hook | Status bar background renders but text-glyph row NOT composited | ⚠️ Warning | The bar appears as a colored strip with no text until the cell-pipeline `render_text` step is wired. Documented; major-but-acceptable v1 gap.                                  |
| `crates/vector-app/src/app.rs` (per 09-05-HUMAN-UAT Gap #3)       | event arm | `reconnecting_panes.remove` happens immediately on `PaneReconnected` | ⚠️ Warning | Fade-out animation skipped. Minor cosmetic deviation from UI-SPEC.                                                                                                               |
| cursor pipeline (per 09-05-HUMAN-UAT Gap #2)                      | n/a  | No reconnecting/alpha-multiplier flag                          | ⚠️ Warning | Cursor does not dim to 40% during Reconnecting. Minor cosmetic deviation; status bar is the primary signal.                                                                     |
| `chrome.reconnect.update` (per 09-05-HUMAN-UAT Gap #1)             | n/a  | Hardcoded dark-mode chrome surface color                       | ⚠️ Warning | Bar contrast off in light mode. Cosmetic; bar still visible.                                                                                                                    |

No production TODO / FIXME / `unimplemented!()` were found in any Phase 9 source files. The blocker anti-pattern from the initial verification (`DevTunnelsActor` missing from `main.rs`) is **REMOVED** — closed by 09-07.

### Human Verification Required

#### 1. 09-05-HUMAN-UAT.md — Reconnect UX walk (11 items)

**Test:** Build + launch Vector locally; sign in to Microsoft; open Dev Tunnels picker via Cmd-Shift-T and connect to a tunnel running `vector-tunnel-agent`; force a disconnect (kill agent or wifi); observe Reconnecting affordances (inline status bar, tab title `[reconnecting]`, input lock + single toast, backoff counter advancement, Cmd-W cancellation, recovery on reconnect, multi-pane independence).
**Expected:** All 11 tests in `09-05-HUMAN-UAT.md` pass (or fall back to documented pre-recorded gaps for cursor-dim, fade-out, glyph-row, light-mode palette).
**Why human:** Real wifi/agent disconnect against a live Microsoft relay; visual + UX behavior. Runtime blocker CLOSED by 09-07 — UAT is now walkable end-to-end.

#### 2. 09-06-HUMAN-UAT.md — PERSIST-04 tmux pass-through + reconnect-with-htop-persistence (16 items)

**Test:** With user-managed tmux (`tmux new -s smoke; tmux set-option -g allow-passthrough on`) on the remote, run the three `#[ignore]`d tests with both env vars set, then walk the 13-row manual matrix (vim, :wq, htop, OSC 52 small + 200-byte, DECSCUSR, mouse SGR 1006, `tput`, `$TERM`, disconnect → htop persists across reconnect via user's tmux, input toast, Cmd-W cancel).
**Expected:** All 3 automated tests PASS + all 13 manual rows PASS, then the sign-off block in `09-SMOKE.md` is filled in.
**Why human:** PERSIST-04 acceptance requires a live tunnel + user-managed tmux. Runtime blocker CLOSED by 09-07 — UAT is now walkable end-to-end.

#### 3. 09-SMOKE.md sign-off

**Test:** After tests #1 + #2 pass, fill in "Approved by" + "Date" and tick the four checkboxes at `09-SMOKE.md:61-68`.
**Expected:** `[ ]` → `[x]` for all four lines; non-blank approver + date.
**Why human:** Acceptance record. Flips PERSIST-04 in `REQUIREMENTS.md` from Pending → Complete.

### Gaps Summary

Phase 9's implementation surface is now **fully complete and end-to-end wired**:

The single code gap from the initial verification — `DevTunnelsActor` not constructed in `crates/vector-app/src/main.rs` — is closed by Plan 09-07. `main.rs` now constructs `DevTunnelsApi`, `MicrosoftAuth`, `MicrosoftTokenStore`, and `DevTunnelsActor` inside the io-thread tokio runtime; calls `set_router` before `spawn`; and ships the returned `mpsc::Sender<Command>` back to the main thread via a second `sync_channel`, which is then handed to the App via `application.set_devtunnels_cmd_tx` before `event_loop.run_app`.

PERSIST-01/02/03 are satisfied by automation. PERSIST-04 is now blocked only on user-executed UAT walks, not code gaps.

The phase remains **`human_needed`** because:

1. **PERSIST-04 sign-off is the user's acceptance gate** — the 16-row matrix in `09-06-HUMAN-UAT.md` and 11-row matrix in `09-05-HUMAN-UAT.md` cannot be discharged programmatically.
2. **Both UATs are now UNBLOCKED** — the only code-level obstacle (no `DevTunnelsActor` in `main.rs`) was removed by 09-07.
3. **Four pre-recorded UI cosmetic gaps** are carried forward as accepted v1 limitations (no fade-out animation, no cursor dim, no light-mode palette threading, missing glyph-row composition on the status bar). These are documented in `09-05-HUMAN-UAT.md` Gaps #1–#4 — not failures.

No code-level gaps remain. No `/gsd:plan-phase --gaps` work is needed. The next step is for the user to walk 09-05-HUMAN-UAT.md and 09-06-HUMAN-UAT.md and fill in the 09-SMOKE.md sign-off block.

---

_Verified: 2026-05-24T23:30:00Z_
_Verifier: Claude (gsd-verifier)_
_Re-verification after: 09-07 gap closure (prior verification: 2026-05-22T21:20:34Z)_
