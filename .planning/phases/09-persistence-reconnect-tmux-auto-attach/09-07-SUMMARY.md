---
phase: 09-persistence-reconnect-tmux-auto-attach
plan: 07
subsystem: app
tags: [devtunnels, actor, main-wiring, gap-closure, microsoft-auth, mpsc, sync_channel]

# Dependency graph
requires:
  - phase: 08-vs-code-remote-tunnels-connect
    provides: DevTunnelsActor type + DevTunnelsApi + MicrosoftAuth/TokenStore
  - phase: 09-persistence-reconnect-tmux-auto-attach
    provides: ReconnectableDevTunnelDomain (09-03), PtyActorRouter reconnect path (09-05), DevTunnelPaneCancelToken plumbing (09-05/06), App.set_devtunnels_cmd_tx hook (08-05)
provides:
  - main.rs constructs DevTunnelsActor inside the io-thread tokio runtime
  - DevTunnelsActor::set_router(router_io) called before spawn
  - sync_channel ships mpsc::Sender<Command> from io-thread to main thread
  - application.set_devtunnels_cmd_tx(cmd_tx) wired before event_loop.run_app
  - Cmd-Shift-T -> DevTunnelsPickerModal -> live actor -> ReconnectableDevTunnelDomain end-to-end reachable
affects: [10-hardening-release, future devtunnels UX work]

# Tech tracking
tech-stack:
  added: []  # no new deps
  patterns:
    - "Two parallel sync_channel hand-offs (tokio Handle + actor cmd_tx) from io-thread to main thread"
    - "Actor configured (set_router) BEFORE spawn so the running task never sees a None router"

key-files:
  created:
    - .planning/phases/09-persistence-reconnect-tmux-auto-attach/09-07-SUMMARY.md
  modified:
    - crates/vector-app/src/main.rs (29 net-added lines: import extension, sync_channel decl, actor construction inside rt.block_on, recv + set_devtunnels_cmd_tx on main thread)

key-decisions:
  - "Lump the pre-existing workspace-wide rustfmt drift + one clippy::redundant_pattern_matching fix into a single chore commit ahead of Task 1 — the cargo-husky pre-commit hook (cargo fmt --all --check + cargo clippy --all-targets -D warnings) blocked ANY commit otherwise. Rule 3 (blocking)."
  - "Use vector_tunnels::auth::device_flow_microsoft::DEFAULT_MICROSOFT_CLIENT_ID directly — the const is pub and reachable through the module path, no literal-string fallback needed."
  - "Call Handle::current() inside rt.block_on (the canonical pattern from inside an async block) rather than capturing rt; equivalent to rt.handle().clone()."

patterns-established:
  - "actor cmd_tx hand-off via sync_channel mirrors the existing handle_tx/handle_rx pattern at main.rs:67"
  - "Phase 9 follow-up comment cadence `// Plan 09-07 — ...` matches surrounding `// Plan NN-NN —` style"

requirements-completed: [PERSIST-01, PERSIST-04]

# Metrics
duration: 12min
completed: 2026-05-24
---

# Phase 09 Plan 07: DevTunnelsActor Main-Wiring Summary

**Constructs DevTunnelsActor inside the io-thread tokio runtime and ships its cmd_tx to App via a sync_channel — unblocking Cmd-Shift-T -> picker -> ReconnectableDevTunnelDomain end-to-end.**

## Performance

- **Duration:** ~12 min
- **Started:** 2026-05-24T23:04:00Z (approx)
- **Completed:** 2026-05-24T23:16:20Z
- **Tasks:** 2 (1 source edit + 1 verification-only)
- **Files modified:** 1 in-scope (main.rs); plus 10 pre-existing-drift files cleaned by `cargo fmt --all` + 1 clippy nit (out-of-scope but required to satisfy the cargo-husky pre-commit hook)

## Accomplishments

- Closed the single Phase 9 verifier-flagged code gap: `grep "DevTunnelsActor" crates/vector-app/src/main.rs` now matches (3 lines; previously 0).
- DevTunnelsActor is constructed with `DevTunnelsApi::new()` + `MicrosoftAuth::new(DEFAULT_MICROSOFT_CLIENT_ID)` + `MicrosoftTokenStore::for_vector()` + `Arc::clone(&mux)` + `proxy_io.clone()`.
- `dt_actor.set_router(Arc::clone(&router_io))` is called BEFORE `dt_actor.spawn(&tokio::runtime::Handle::current())` — guarantees tunnel pane spawns reach the per-pane PTY router with the reconnectable domain + cancel token.
- A new `sync_channel::<tokio::sync::mpsc::Sender<devtunnels_actor::Command>>(1)` ships the cmd_tx from the io-thread back to the main thread; the main thread calls `application.set_devtunnels_cmd_tx(cmd_tx)` before `event_loop.run_app`.
- `cargo build --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test -p vector-app --lib`, and `cargo test -p vector-tunnels --lib` all exit 0.

## Task Commits

1. **Pre-task chore (Rule 3 — blocking pre-commit hook): cargo fmt --all + one clippy fix** — `a388b2b` (chore). Touched 10 files. Required to allow any subsequent commit through the cargo-husky pre-commit hook. Task 1's main.rs edit was carried inside this same commit (the staged main.rs and the fmt-only drift were squashed together once the chore commit succeeded; the actor wiring is fully present in this hash — verified via `git show a388b2b -- crates/vector-app/src/main.rs`).
2. **Task 2: verification gates (no source edits)** — no commit needed; gates ran clean.

**Plan metadata:** to be committed in the final docs commit (this SUMMARY + STATE + ROADMAP) per the workflow.

_Note: under normal circumstances Task 1 would have been its own `feat(09-07)` commit; the pre-commit hook's workspace-wide fmt+clippy gate made the chore unavoidable and indivisible. The substantive change is the 29 lines of main.rs additions and is fully auditable via `git show a388b2b -- crates/vector-app/src/main.rs`._

## Exact lines added to main.rs (post-edit line numbers)

- **Line 11 (modified):** `use vector_app::{app, devtunnels_actor, lpm, pty_actor, ske, UserEvent, DEFAULT_CONFIG_TOML};` — added `devtunnels_actor` to the import list.
- **Lines 69-71 (added):** new sync_channel decl with the `// Plan 09-07 — ship DevTunnelsActor cmd_tx ...` comment.
- **Lines 94-110 (added):** actor construction block inside `rt.block_on` body — `DevTunnelsApi::new()`, `MicrosoftAuth::new(DEFAULT_MICROSOFT_CLIENT_ID)`, `MicrosoftTokenStore::for_vector()`, `DevTunnelsActor::new(...)`, `dt_actor.set_router(Arc::clone(&router_io))`, `dt_actor.spawn(&tokio::runtime::Handle::current())`, `dt_tx.send(dt_cmd_tx)`.
- **Lines 235-240 (added):** main-thread `dt_rx.recv()` + `application.set_devtunnels_cmd_tx(cmd_tx)` block mirroring the existing `handle_rx.recv()` fallback pattern.

## Files Created/Modified

- `crates/vector-app/src/main.rs` — DevTunnelsActor construction + spawn + cmd_tx hand-off to App.
- `.planning/phases/09-persistence-reconnect-tmux-auto-attach/09-07-SUMMARY.md` — this file.

## Decisions Made

- **DEFAULT_MICROSOFT_CLIENT_ID via path, not literal:** the const is pub and reachable through `vector_tunnels::auth::device_flow_microsoft::DEFAULT_MICROSOFT_CLIENT_ID`. The plan authorized a literal-string fallback (`"aebc6443-996d-45c2-90f0-388ff96faa56"`) — not needed. Path form keeps the magic string in one place.
- **set_router before spawn:** intentional. The router_io Arc is cheap to clone and the actor's `router: Option<...>` field is only consumed inside `handle_connect`. Configuring before `spawn(...)` consumes `self` means the running task always sees `Some(router)`.
- **Pre-commit hook required workspace-wide fmt:** `.git/hooks/pre-commit` runs `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`. Pre-existing drift in 09 prior plans (app.rs, pty_actor.rs, several tests in vector-app and vector-tunnels) plus one `matches!(shell, None)` -> `shell.is_none()` clippy nit blocked the commit path. Running `cargo fmt --all` + the one-line clippy fix cleared the gate. This satisfies the plan's own success criteria (`cargo fmt --check` must pass).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] Workspace-wide rustfmt drift blocked pre-commit hook**
- **Found during:** Task 1 commit attempt.
- **Issue:** `cargo fmt --all -- --check` failed against 9 unrelated source files (app.rs reconnecting-state arm, pty_actor.rs drain helper signature, 4 vector-app tests, vector-render reconnect_pass, vector-tunnels live_devtunnel_smoke, open_pty_no_shell_override). All drift was pre-existing from Plans 09-03/05/06 and unrelated to this plan's scope.
- **Fix:** `cargo fmt --all` to normalize. No logic edits.
- **Files modified:** 9 files (out-of-scope) — see commit `a388b2b`.
- **Verification:** `cargo fmt --check` exits 0; the plan's success criterion is satisfied.
- **Committed in:** `a388b2b` (folded into the chore commit alongside the clippy fix; the main.rs Task 1 wiring is also inside this commit).

**2. [Rule 3 — Blocking] Single clippy::redundant_pattern_matching pre-existed in open_pty_no_shell_override.rs**
- **Found during:** Task 1 commit attempt (pre-commit hook runs `cargo clippy --all-targets -- -D warnings`).
- **Issue:** `assert!(matches!(shell, None));` triggered `clippy::redundant_pattern_matching` on line 54.
- **Fix:** Replaced with `assert!(shell.is_none());` — same assertion, no semantic change. The acceptance comment ("Belt-and-suspenders explicit comparison") still applies.
- **Files modified:** `crates/vector-tunnels/tests/open_pty_no_shell_override.rs` (one line; out-of-scope but required to allow commits).
- **Verification:** `cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- **Committed in:** `a388b2b`.

---

**Total deviations:** 2 auto-fixed (both Rule 3 — blocking).
**Impact on plan:** Both deviations were forced by the workspace-wide pre-commit hook; without them no commit would land. Neither changes runtime behavior. The plan's substantive deliverable (main.rs wiring) is unaffected and remains 29 net-added lines in `crates/vector-app/src/main.rs`, exactly as the plan specified.

## Issues Encountered

- **Pre-commit hook squashed two intended commits into one:** because `cargo fmt --all` rewrote files already staged including `main.rs`, the eventual `git commit` captured both the fmt drift cleanup AND the Task 1 wiring in a single hash (`a388b2b`). Not ideal — Task 1 would normally be its own `feat(09-07)` commit — but the wiring is fully present and auditable via `git show a388b2b -- crates/vector-app/src/main.rs`. Future Phase 9 follow-up plans should expect to face the same hook gate at commit-1 if any more drift accumulates.

## User Setup Required

None — this plan does not introduce new env vars, credentials, or external service config. The downstream picker still requires the user to run `gh auth login` / Microsoft device-flow sign-in when first using Cmd-Shift-T, but that is unchanged from Phase 8.

## Verification Gates (all exit 0)

```
cargo build --workspace                                       # ok
cargo clippy --workspace --all-targets -- -D warnings         # ok
cargo fmt --check                                             # ok
cargo test -p vector-app --lib                                # 16 passed; 0 failed
cargo test -p vector-tunnels --lib                            # 11 passed; 0 failed
grep -c "DevTunnelsActor::new" crates/vector-app/src/main.rs  # 1
grep -c ".set_router(" crates/vector-app/src/main.rs          # 2 (application + dt_actor)
grep -c "set_devtunnels_cmd_tx" crates/vector-app/src/main.rs # 1
grep -c "sync_channel" crates/vector-app/src/main.rs          # 2
```

## Self-Check: PASSED

- `[ -f crates/vector-app/src/main.rs ]` -> FOUND
- `[ -f .planning/phases/09-persistence-reconnect-tmux-auto-attach/09-07-SUMMARY.md ]` -> FOUND (this file)
- `git log --oneline | grep -q a388b2b` -> FOUND (commit `a388b2b` contains the chore + Task 1 wiring)
- All acceptance greps return the required counts.

## Next Phase Readiness

- **09-05-HUMAN-UAT.md:** all 11 picker/reconnect walk items are now reachable end-to-end at runtime. User should walk this matrix to flip PERSIST-04 from Pending -> Complete.
- **09-06-HUMAN-UAT.md:** all 16 tmux-auto-attach + reconnect items are now reachable.
- **09-SMOKE.md sign-off:** the USER-RUN block (lines 61-68) becomes fill-in-able after the two HUMAN-UATs are walked.
- **09-VERIFICATION.md gap #5 (DevTunnelsActor missing from main.rs):** CLOSED. UI gaps #1-#4 (cursor-dim, fade-out, glyph row, light-mode palette) remain deferred v1 cosmetic limitations and are explicitly OUT OF SCOPE per the plan's `<verification>` block.
- **Phase 10 (Hardening & Release):** unblocked from a code-completeness perspective — Phase 9 implementation is now end-to-end wired. Sign-off still pending user UAT walks.

---
*Phase: 09-persistence-reconnect-tmux-auto-attach*
*Completed: 2026-05-24*
