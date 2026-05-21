---
phase: 08-vs-code-remote-tunnels-connect
plan: 07
subsystem: docs/uat
status: partial — Task 1 complete; Task 2 (checkpoint:human-verify) awaiting user UAT
tags: [uat, smoke-matrix, dev-tunnels, manual-verification, checkpoint]
requires:
  - 08-01 (spike doc at .planning/research/spikes/dev-tunnels-decision.md)
  - 08-02 (Microsoft OAuth driver)
  - 08-03 (vector-tunnel-agent binary)
  - 08-04 (Mac client transport + list endpoint)
  - 08-05 (picker UI + actor + Microsoft sign-in modal)
  - 08-06 (.deb distribution via agent-release.yml)
provides:
  - .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md (9-item UAT matrix)
  - Sign-off gate for DT-01..04 marking Phase 8 Complete
affects:
  - REQUIREMENTS.md (DT-01..04 flip pending user sign-off — Task 2)
  - ROADMAP.md (Phase 8 row flip pending user sign-off — Task 2)
  - STATE.md (completed_phases increment pending user sign-off — Task 2)
tech-stack:
  added: []
  patterns:
    - "manual UAT smoke matrix template (9 items, PASS/FAIL boxes, user-signoff line) — mirrors Phase 4/5 SMOKE precedent format"
key-files:
  created:
    - .planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md
  modified: []
decisions:
  - "SMOKE matrix body authored verbatim from plan <action> spec — no embellishment, no fabricated PASS results"
  - "Task 2 paused as checkpoint:human-verify per autonomous: false + plan objective explicit no-fabrication clause"
metrics:
  tasks_completed: 1
  tasks_total: 2
  files_created: 1
  files_modified: 0
  duration: ~3min (Task 1 author + verify + commit)
  completed: 2026-05-21
---

# Phase 8 Plan 07: UAT Smoke Matrix Summary

**One-liner:** Authored the 9-item Phase 8 manual UAT smoke matrix template at `.planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md` covering DT-01..04 + UI-SPEC verbatim copy + Pitfall 14 token-leak audit; Task 2 (live walk-through + sign-off) paused as a human-verify checkpoint per `autonomous: false` and the plan's explicit no-fabrication clause.

## What Shipped

**Task 1 — 08-SMOKE.md authored (commit `b5d006e`):**
- 9 items, all unticked PASS/FAIL boxes — ships empty per plan acceptance ("Do NOT mark anything as PASS in this plan — that requires user UAT").
- Item 1 verifies the DT-01 spike decision document (`.planning/research/spikes/dev-tunnels-decision.md`) exists and contains the `Path 2 Variant 2c` decision string. **The spike doc was committed by Plan 08-01 Task 1 Step 0; this plan does not create it.** Verified on disk during execution: `test -f` returned 0, `grep "Path 2 Variant 2c"` returned 2 hits.
- Items 2-9 are live-system gates (Microsoft sign-in modal → picker → connect → tab tint → resize → token-leak grep → sign-out resilience). Each carries PASS/FAIL boxes + Notes line.
- Sign-off line + post-sign-off procedure (REQUIREMENTS/ROADMAP/STATE flips + final docs commit message template) inline at the bottom.
- File: 105 lines.

**Task 2 — Checkpoint:human-verify (NOT EXECUTED):**
- The plan's task body explicitly walks 9 items requiring real Microsoft device flow, real `.deb` install on a Linux box, real picker interaction, real GPU-rendered tint inspection, and a real log scrape. Per plan objective: "items that require human action (typing credentials, clicking, observing UI state), STOP at that item and return a structured human-action checkpoint per references/checkpoints.md. Do not fabricate UAT results."
- Returned as a human-verify checkpoint to the parent orchestrator.
- REQUIREMENTS.md / ROADMAP.md / STATE.md flips deferred until user signs off in `08-SMOKE.md`.

## Automated Verification Results

Acceptance criteria (Task 1):

| Check | Target | Actual | Result |
|-------|--------|--------|--------|
| File exists | yes | yes | OK |
| `### Item ` count | == 9 | 9 | OK |
| `PASS / [ ] FAIL` literal-string count | >= 8 | 7 | **see Deviation #1** |
| `DT-0[1-4]` refs | >= 4 | 8 | OK |
| `vector-tunnel-agent` refs | >= 3 | 5 | OK |
| `Sign in/out of Microsoft` | >= 2 | 4 | OK |
| `Cmd-Shift-T` | >= 1 | 3 | OK |
| `#0078d4` / `Microsoft-blue` | >= 1 | 2 | OK |
| `Pitfall 14` / `token-leak` | >= 1 | 2 | OK |
| `Sign-off` / `signature` | >= 1 | 2 | OK |
| Spike doc exists | yes | yes | OK |

The plan's `<automated>` verify block (boolean composite) passes cleanly — it requires non-zero on each grep, which all hit.

## Deviations from Plan

### Discrepancies (informational, not actioned)

**1. PASS/FAIL literal-string count mismatch (Item 8 is parenthesized in plan body)**

- **Found during:** Task 1 verify.
- **Issue:** Plan acceptance criterion #3 requires `grep -c "PASS / \\[ \\] FAIL" >= 8`, but the plan's own `<action>` body for Item 8 specifies `PASS (zero hits) / [ ] FAIL (any hit)` — a different literal string. So at most 8 items can match the exact `PASS / [ ] FAIL` pattern (Items 2-7, 9) → 7 matches; Item 8 has parenthetical text and is excluded from the literal grep.
- **Fix:** None — followed the plan body verbatim (8 items have PASS/FAIL gates, 1 of which has parenthetical detail). This is a planner-side acceptance-criterion vs. body inconsistency; the matrix is semantically correct.
- **Files modified:** none beyond the SMOKE template itself
- **Commit:** n/a (within Task 1's single commit `b5d006e`)

### Auto-fixed Issues

None — Task 1 was a clean template author per spec.

## Authentication Gates

Task 2 contains multiple auth gates by design:
- **Microsoft device flow** (Item 2) — user opens browser, enters user_code at microsoft.com/devicelogin.
- **Microsoft device flow on Linux agent** (Item 3) — same flow from the agent's CLI prompt.
- **GitHub device flow** (footer copy reference Item 9) — only the sign-out path is exercised here; sign-in is covered by Phase 6.

These are not bugs — they are the requirement under test (DT-01..04).

## Known Stubs

None. The SMOKE template is a deliberately empty matrix; "stub" here means an unticked checkbox, which is the contract.

## Self-Check

- [x] `.planning/phases/08-vs-code-remote-tunnels-connect/08-SMOKE.md` exists (verified via `test -f`)
- [x] Commit `b5d006e` exists (verified via `git log --oneline | grep b5d006e`)
- [x] Spike doc dependency holds (`test -f .planning/research/spikes/dev-tunnels-decision.md` exits 0; `grep "Path 2 Variant 2c"` returns 2 hits)

## Self-Check: PASSED

## Status

**Plan 08-07 is partial-complete.** Task 1 landed; Task 2 returned as a human-verify checkpoint. Phase 8 cannot flip to Complete until the user walks `08-SMOKE.md` end-to-end on real hardware and signs off. Post-sign-off, the user (or a follow-up `/gsd:execute-phase` resume) flips REQUIREMENTS DT-01..04, ROADMAP Phase 8, and STATE per the procedure embedded in `08-SMOKE.md §Post-sign-off updates`.
