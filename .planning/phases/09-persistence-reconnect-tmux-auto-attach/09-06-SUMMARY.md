---
phase: 09-persistence-reconnect-tmux-auto-attach
plan: 06
subsystem: testing
tags: [persist-04, live-e2e, tmux-passthrough, devtunnels, ci-smoke, osc52, decscusr, mouse-1006]

requires:
  - phase: 09-persistence-reconnect-tmux-auto-attach
    provides: "09-01..05 reconnect machinery + ReconnectableDevTunnelDomain + App-side reconnect state + render hook + tab-title flip + per-pane CancellationToken plumbing"
  - phase: 08-vs-code-remote-tunnels-connect
    provides: "vector-tunnels DevTunnelTransport + connect_tunnel helper + vector-tunnel-agent Linux binary advertising TERM=xterm-256color"
provides:
  - "Three #[ignore]d + env-gated live e2e tests in crates/vector-tunnels/tests/live_devtunnel_smoke.rs honoring the user-managed tmux contract (CONTEXT D-04/D-05)"
  - "persist-e2e CI job in .github/workflows/ci.yml (continue-on-error: true; no-op without VECTOR_E2E_TUNNEL_ID + VECTOR_E2E_MICROSOFT_TOKEN repository secrets)"
  - "09-SMOKE.md skeleton with USER-RUN tmux setup section, automated + manual matrices, and sign-off block (skeleton only — user fills + signs at the gated checkpoint)"
  - "09-06-HUMAN-UAT.md (status: partial) preserving the smoke matrix as 16 pending items, all blocked_by: prior-phase pending main.rs DevTunnelsActor wiring"
affects: [10-hardening-release]

tech-stack:
  added: []
  patterns:
    - "Live e2e tests gate on TWO env vars (VECTOR_E2E_TUNNEL_ID + VECTOR_E2E_MICROSOFT_TOKEN) and early-return via eprintln rather than panic when a var is unset — matches the Phase 5 osc52_tmux.rs shape but adds the second env var for Microsoft auth-token injection"
    - "CI job mirrors the existing tmux-smoke pattern (continue-on-error: true; gated on repository secrets; never blocks merges)"
    - "User-managed tmux contract is enforced at the test layer by a `! grep -E 'tmux (new|set-option|attach|kill-session)'` acceptance check + a fail-fast `$TMUX` pre-check in osc52_round_trip (CONTEXT D-04/D-05 absence assertion)"

key-files:
  created:
    - .planning/phases/09-persistence-reconnect-tmux-auto-attach/09-SMOKE.md
    - .planning/phases/09-persistence-reconnect-tmux-auto-attach/09-06-HUMAN-UAT.md
    - .planning/phases/09-persistence-reconnect-tmux-auto-attach/09-06-SUMMARY.md
  modified:
    - crates/vector-tunnels/tests/live_devtunnel_smoke.rs
    - .github/workflows/ci.yml

key-decisions:
  - "Task 3b (UAT sign-off in 09-SMOKE.md) is DEFERRED, not failed. Same root cause as Plan 09-05 Task 3 deferral: DevTunnelsActor is not yet constructed in main.rs, so Cmd-Shift-T cannot route to a live ReconnectableDevTunnelDomain pane. The smoke matrix can only be walked end-to-end after the main.rs wiring lands."
  - "Smoke matrix is preserved verbatim in 09-06-HUMAN-UAT.md (16 items: 3 automated + 13 manual) so /gsd:audit-uat surfaces it alongside 09-05-HUMAN-UAT.md as joint debt. Both UATs share the same blocking gap (DevTunnelsActor main.rs wiring) — that gap is intentionally duplicated across both UAT files so each surfaces independently."
  - "PERSIST-04 in REQUIREMENTS.md is NOT marked complete by this plan. The automated tests are `#[ignore]`d + env-gated and structurally green; the manual UAT is pending. PERSIST-04 flips to Complete only after 09-06-HUMAN-UAT.md is signed off."

patterns-established:
  - "User-managed tmux absence assertion at the test layer: `! grep -E 'tmux (new|set-option|attach|kill-session)' crates/vector-tunnels/tests/live_devtunnel_smoke.rs` — mirrors the Phase 9 absence-grep pattern at the runtime/source layer (`grep -RIn 'tmux new -A' crates/` returns empty)"
  - "Two-env-var gating for live e2e tests that require both a target identifier (tunnel id) AND a pre-minted auth token; CI provides both as repository secrets, local devs export the token from Keychain"

requirements-completed: []  # PERSIST-04 NOT marked complete — see "Status" below. Verified once 09-06-HUMAN-UAT signs off.

duration: ~18min (Tasks 1 + 2 + 3a; Task 3b deferred)
completed: 2026-05-22
---

# Phase 9 Plan 06: Live e2e + CI + smoke skeleton (PERSIST-04 verification surface) Summary

**Three `#[ignore]`d live e2e tests honoring the user-managed tmux contract + `persist-e2e` CI job + `09-SMOKE.md` skeleton with USER-RUN setup — all landed and structurally green; PERSIST-04 sign-off deferred pending the same `DevTunnelsActor` main.rs wiring blocker that deferred Plan 09-05 Task 3.**

## Status: SUBSTANTIALLY COMPLETE — Task 3b DEFERRED (not failed)

3 of 4 tasks landed and verified green:

- **Task 1 (test bodies)** — `d1fb3c8` — three real test bodies (`osc52_round_trip`, `decscusr_and_mouse_modes`, `term_xterm_256color_advertised`) replace the Wave-0 stubs in `crates/vector-tunnels/tests/live_devtunnel_smoke.rs`. Each is `#[ignore]`d, gated on `VECTOR_E2E_TUNNEL_ID` + `VECTOR_E2E_MICROSOFT_TOKEN`, wrapped in `tokio::time::timeout(Duration::from_secs(30), ...)`, and the `osc52_round_trip` test pre-checks `$TMUX` on the remote and fails fast if empty (CONTEXT D-04/D-05 — Vector NEVER bootstraps tmux). The file contains zero `tmux new` / `tmux set-option` / `tmux attach` / `tmux kill-session` strings (absence assertion holds).
- **Task 2 (CI job)** — `2ec55fe` — `persist-e2e` job appended to `.github/workflows/ci.yml`, mirroring the existing `tmux-smoke` shape: `continue-on-error: true`, gated on `VECTOR_E2E_TUNNEL_ID` + `VECTOR_E2E_MICROSOFT_TOKEN` repository secrets, runs `cargo test -p vector-tunnels --test live_devtunnel_smoke -- --ignored --test-threads=1` when both are present and emits a no-op skip log otherwise. Never blocks merges.
- **Task 3a (smoke skeleton)** — `e5968ed` — `.planning/phases/09-persistence-reconnect-tmux-auto-attach/09-SMOKE.md` created with the USER-RUN setup section instructing the user to run `tmux new -s smoke; tmux set-option -g allow-passthrough on` on the remote BEFORE running any test, the automated portion (3 rows), the manual portion (13 rows mirroring UI-SPEC + UI-VALIDATION matrix), and a blank Sign-off block. No checkboxes are pre-filled.
- **Task 3b (user smoke UAT sign-off)** — **DEFERRED** to a follow-up plan. See "Deferred from plan" below. The smoke matrix is preserved verbatim in `09-06-HUMAN-UAT.md` (status: partial; 16 pending items, all blocked_by: prior-phase).

Automated guards at plan close all pass:

- `! grep -RIn 'tmux new -A' crates/` — empty (PERSIST-03 absence assertion holds through phase close).
- `! grep -E 'tmux (new|set-option|attach|kill-session)' crates/vector-tunnels/tests/live_devtunnel_smoke.rs` — empty (CONTEXT D-04/D-05 absence at the test layer).
- `cargo test -p vector-tunnels --test live_devtunnel_smoke` (no env vars) — `0 passed; 0 failed; 3 ignored`.
- `cargo test -p vector-tunnels --test live_devtunnel_smoke -- --list` — three tests listed (`osc52_round_trip`, `decscusr_and_mouse_modes`, `term_xterm_256color_advertised`).
- `cargo clippy -p vector-tunnels --tests -- -D warnings` — clean.
- `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` — YAML parses cleanly.
- `grep -n "persist-e2e:" .github/workflows/ci.yml` — exactly one match.
- `grep -c "continue-on-error: true" .github/workflows/ci.yml` — at least two matches (tmux-smoke + persist-e2e).

## Performance

- **Duration:** ~18 min (Tasks 1 + 2 + 3a; Task 3b deferred)
- **Started:** 2026-05-22
- **Completed:** 2026-05-22 (executor tasks)
- **Tasks executed:** 3 of 4 (Task 3b deferred — see "Deferred" section below)
- **Files modified:** 5 (2 source + 3 docs created)

## Accomplishments

- PERSIST-04 verification surface is now in place: three live e2e tests + CI job + USER-RUN smoke matrix skeleton, all aligned with the user-managed tmux contract (CONTEXT D-04/D-05/D-06).
- Tests are structurally green and runnable: `cargo test -p vector-tunnels --test live_devtunnel_smoke -- --ignored --test-threads=1` will execute against any reachable tunnel + Microsoft token without further code changes — the only thing missing for sign-off is the main.rs wiring that unblocks the live runtime pane path used in the manual matrix portion.
- The CI job is a no-op until the user adds `VECTOR_E2E_TUNNEL_ID` + `VECTOR_E2E_MICROSOFT_TOKEN` repository secrets, at which point it begins running live e2e tests on every push without blocking merges.
- 09-SMOKE.md skeleton has the USER-RUN setup section called out prominently, fails-fast affordances documented (the `$TMUX` pre-check in `osc52_round_trip`), and a clean Sign-off block with no pre-filled checkboxes.
- Joint UAT debt with Plan 09-05 surfaced: 09-05-HUMAN-UAT.md (11 pending items) + 09-06-HUMAN-UAT.md (16 pending items) both block on the SAME `DevTunnelsActor` main.rs wiring gap. A follow-up plan that wires `DevTunnelsActor` in `main.rs` will unblock both UATs simultaneously.

## Task Commits

1. **Task 1: Live e2e test bodies (user-managed tmux contract)** — `d1fb3c8` (test)
2. **Task 2: persist-e2e CI job mirroring tmux-smoke pattern** — `2ec55fe` (ci)
3. **Task 3a: 09-SMOKE.md skeleton with USER-RUN tmux setup** — `e5968ed` (docs)
4. **Task 3b: User runs smoke matrix + signs off in 09-SMOKE.md** — **DEFERRED** to follow-up plan (see "Deferred" below)

**Plan metadata commit:** see final `docs(09-06)` commit (this SUMMARY + 09-06-HUMAN-UAT.md + STATE.md + ROADMAP.md + REQUIREMENTS.md updates).

## Files Created/Modified

- `crates/vector-tunnels/tests/live_devtunnel_smoke.rs` — three real `#[ignore]`d + env-gated test bodies; `$TMUX` pre-check in `osc52_round_trip`; zero Vector-managed tmux command strings.
- `.github/workflows/ci.yml` — `persist-e2e` job appended after `tmux-smoke`; `continue-on-error: true`; gated on the two repository secrets.
- `.planning/phases/09-persistence-reconnect-tmux-auto-attach/09-SMOKE.md` — skeleton created with USER-RUN setup, automated + manual matrices, blank Sign-off block.
- `.planning/phases/09-persistence-reconnect-tmux-auto-attach/09-06-HUMAN-UAT.md` — created (status: partial); 16 pending items mirroring the SMOKE matrix verbatim; pre-recorded `severity: blocker` Gap pointing to the same `DevTunnelsActor` main.rs wiring root cause as 09-05-HUMAN-UAT.md Gap #5.
- `.planning/phases/09-persistence-reconnect-tmux-auto-attach/09-06-SUMMARY.md` — this file.

## Decisions Made

- **Task 3b treated as DEFERRED (not failed)** — exact-match resolution to Plan 09-05 Task 3: the executor work (test bodies + CI job + SMOKE skeleton) is fully landed, but the sign-off step requires a runtime pane path that doesn't exist yet. Deferring rather than failing keeps the plan counter advancing and the gap captured as joint debt with 09-05.
- **Smoke matrix preserved as a pending UAT, not re-litigated in this SUMMARY** — `09-06-HUMAN-UAT.md` is the canonical record. This SUMMARY references it; it does not duplicate the matrix content (avoids drift).
- **PERSIST-04 explicitly NOT marked Complete** — REQUIREMENTS.md gets a "Verified once 09-06-HUMAN-UAT signs off" note; the checkbox stays unchecked. The automated portion is gated behind `#[ignore]` + env vars and does not count as live verification.
- **Joint UAT debt surfaced in both UAT files** — the `DevTunnelsActor` main.rs wiring blocker is intentionally documented in BOTH `09-05-HUMAN-UAT.md` Gap #5 AND `09-06-HUMAN-UAT.md` Gap (test 1). When `/gsd:audit-uat` runs, both will surface the gap independently, making the follow-up wiring plan's scope obvious.

## Deviations from Plan

### Auto-fixed Issues

None — the three executor tasks landed close to the plan. The deferral is a planning miss (carried from 09-05) surfaced at Task 3b's gate, not an in-task fix.

### Deferred from plan

**Task 3b (User runs smoke matrix + signs off in 09-SMOKE.md) — DEFERRED to follow-up plan**

- **Reason:** Same root cause as Plan 09-05 Task 3 deferral. `DevTunnelsActor` is not yet constructed in `crates/vector-app/src/main.rs`. The picker actor's `handle_connect` is correctly wired to build `ReconnectableDevTunnelDomain` and emit `UserEvent::DevTunnelPaneCancelToken`, but without the actor being instantiated by the App at startup with the event-loop proxy, Cmd-Shift-T does not flow into the live `ReconnectableDevTunnelDomain` path. The manual reconnect matrix (rows 13–16 in 09-06-HUMAN-UAT.md) cannot run end-to-end, and signing off PERSIST-04 requires the full live UX walk — running the `--ignored` tests alone is insufficient.
- **Impact:** PERSIST-04 cannot be flipped to Complete until the follow-up plan wires main.rs and both `09-05-HUMAN-UAT.md` AND `09-06-HUMAN-UAT.md` are walked. Phase 9 closes implementation-complete with UAT debt rather than fully Complete.
- **Mitigation:** Smoke matrix preserved verbatim in `09-06-HUMAN-UAT.md` (status: partial, 16 items, all blocked_by: prior-phase). `severity: blocker` Gap pre-recorded pointing to the same `crates/vector-app/src/main.rs` artifact + missing-step as 09-05-HUMAN-UAT.md Gap #5, so the follow-up plan that fixes one UAT fixes both. CI job + automated tests are structurally green — no code-level rework needed, only the wiring.
- **Tracking:** A follow-up plan (proposed: Phase 9 backlog, scoped jointly to unblock 09-05 + 09-06 UATs) must:
  1. Construct `DevTunnelsActor` in `crates/vector-app/src/main.rs` with the App's event-loop proxy.
  2. Verify `UserEvent::DevTunnelPaneCancelToken` round-trips correctly (covered by 09-05's first three UAT items).
  3. Re-open `09-05-HUMAN-UAT.md` and walk the 11-item matrix.
  4. Re-open `09-06-HUMAN-UAT.md` and walk the 16-item matrix (3 automated + 13 manual).
  5. Flip PERSIST-04 to Complete in REQUIREMENTS.md only after both UATs are signed off.

## Issues Encountered

None during implementation. The deferral surfaces the same planning miss carried out of Plan 09-05 — main.rs wiring was implicitly assumed to exist; it does not.

## Joint Debt with Plan 09-05

Both `09-05-HUMAN-UAT.md` (11 pending tests, status: partial) and `09-06-HUMAN-UAT.md` (16 pending tests, status: partial) are blocked by the SAME `DevTunnelsActor` main.rs wiring gap. They are intentionally tracked as separate UAT files so each surfaces independently in `/gsd:audit-uat`, but they share one root cause and will be unblocked by one follow-up plan.

Documented carried-forward Gaps from 09-05 (still in effect during the eventual 09-06 manual matrix walk):

- 09-05 Gap #1: Light-mode chrome.surface RGBA not threaded into `ReconnectPass::update` (cosmetic).
- 09-05 Gap #2: Cursor dim during Reconnecting state deferred (minor).
- 09-05 Gap #3: Status-bar fade-out animation deferred — bar disappears in one frame (minor).
- 09-05 Gap #4: Inline status-bar GLYPH ROW not yet composited (major — status bar background renders, text overlay does not).
- 09-05 Gap #5: `DevTunnelsActor` construction missing in main.rs (blocker — duplicated in 09-06-HUMAN-UAT.md).

The four polish-grade limitations (#1–#4) do not block the 09-06 UAT from running; they will surface as findings during the manual matrix walk and can be resolved as gap-closure tasks rather than as 09-06 prerequisites.

## User Setup Required

None for the executor tasks. To eventually run the deferred UAT (Task 3b):

- Microsoft account signed in (Cmd-Shift-T → Sign in with Microsoft) — pending main.rs wiring.
- A reachable Dev Tunnel with `vector-tunnel-agent` running (per Phase 8 install instructions).
- USER-RUN tmux on the remote: `tmux new -s smoke; tmux set-option -g allow-passthrough on` (per 09-SMOKE.md USER-RUN setup section).
- For local `cargo test --ignored` runs: export `VECTOR_E2E_TUNNEL_ID` + `VECTOR_E2E_MICROSOFT_TOKEN` (from Keychain via `security find-generic-password -s vector.microsoft -w` or whatever account string `crates/vector-tunnels/src/auth.rs` uses).
- For CI runs: add `VECTOR_E2E_TUNNEL_ID` + `VECTOR_E2E_MICROSOFT_TOKEN` as repository secrets — until then, the `persist-e2e` job logs a skip and exits clean.
- Main.rs `DevTunnelsActor` wiring (separate follow-up plan — joint blocker with 09-05).

## Verification Snapshot

Automated checks at task completion:

- `cargo build --workspace` — green.
- `cargo test -p vector-tunnels --test live_devtunnel_smoke` (default flags, no env vars) — `0 passed; 0 failed; 3 ignored`.
- `cargo test -p vector-tunnels --test live_devtunnel_smoke -- --list` — three tests listed.
- `cargo clippy -p vector-tunnels --tests -- -D warnings` — clean.
- `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"` — YAML parses cleanly.
- `grep -n "persist-e2e:" .github/workflows/ci.yml` — exactly one match.
- `grep -c "continue-on-error: true" .github/workflows/ci.yml` — at least two matches (tmux-smoke + persist-e2e).
- `grep -n "cargo test -p vector-tunnels --test live_devtunnel_smoke" .github/workflows/ci.yml` — exactly one match.
- `! grep -E 'tmux (new|set-option|attach|kill-session)' crates/vector-tunnels/tests/live_devtunnel_smoke.rs` — empty (CONTEXT D-04/D-05 absence at the test layer).
- `grep -n "tmux must be running on remote" crates/vector-tunnels/tests/live_devtunnel_smoke.rs` — present (fail-fast message in `osc52_round_trip`).
- `grep -n "USER-RUN setup" .planning/phases/09-persistence-reconnect-tmux-auto-attach/09-SMOKE.md` — present.
- `grep -n "tmux new -s smoke" .planning/phases/09-persistence-reconnect-tmux-auto-attach/09-SMOKE.md` — present (USER-RUN setup instructs the user, not Vector).
- `grep -RIn "tmux new -A" crates/` — empty (PERSIST-03 absence assertion still holds at phase close).

## Next Phase Readiness

Phase 9 closes implementation-complete with UAT debt. Phase 10 (Hardening & Release) can begin in parallel; the deferred UATs are joint debt with 09-05 and are tracked as a focused follow-up plan that wires `DevTunnelsActor` in `main.rs`. PERSIST-04 stays Pending in REQUIREMENTS.md until both `09-05-HUMAN-UAT.md` and `09-06-HUMAN-UAT.md` sign off.

## Self-Check: PASSED

- `crates/vector-tunnels/tests/live_devtunnel_smoke.rs` modified — FOUND
- `.github/workflows/ci.yml` modified (persist-e2e job appended) — FOUND
- `.planning/phases/09-persistence-reconnect-tmux-auto-attach/09-SMOKE.md` created — FOUND
- `.planning/phases/09-persistence-reconnect-tmux-auto-attach/09-06-HUMAN-UAT.md` created (status: partial, 16 pending, 1 blocker Gap) — FOUND
- Commit `d1fb3c8` (Task 1 test bodies) — FOUND in git log
- Commit `2ec55fe` (Task 2 CI job) — FOUND in git log
- Commit `e5968ed` (Task 3a smoke skeleton) — FOUND in git log
- Task 3b deferral explicitly documented in this SUMMARY — FOUND
- Joint debt with 09-05-HUMAN-UAT.md surfaced (shared `DevTunnelsActor` main.rs blocker) — FOUND in both UAT files
- PERSIST-04 NOT marked Complete in REQUIREMENTS.md — verified pending REQUIREMENTS.md update step

---
*Phase: 09-persistence-reconnect-tmux-auto-attach*
*Implementation completed: 2026-05-22*
*UAT deferred pending main.rs DevTunnelsActor wiring (joint follow-up plan with 09-05)*
