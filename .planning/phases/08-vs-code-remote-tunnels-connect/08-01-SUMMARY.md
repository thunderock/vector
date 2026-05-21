---
phase: 08-vs-code-remote-tunnels-connect
plan: 01
subsystem: infra
tags: [dev-tunnels, microsoft-tunnels, russh-patch, workspace-scaffold, arch-lint, pitfall-14]

requires:
  - phase: 07-ssh-transport-codespaces-connect
    provides: russh 0.60 scaffolding (vector-ssh) + TransportKind::DevTunnel variant + [remote] badge wiring
  - phase: 06-github-auth-codespaces-picker
    provides: vector-secrets Keychain surface + Pitfall-14 arch-lint baseline

provides:
  - DT-01 spike decision doc committed at canonical path (gates Phase 8 SC#1)
  - vector-tunnel-protocol crate (AgentMessage enum + JSON+base64 codec + PROTOCOL_VERSION)
  - vector-tunnel-agent crate (Linux user-space binary stub for Wave 1)
  - vector-tunnels crate filled out to module surface (api/auth/domain/model/transport)
  - workspace [patch.crates-io] russh family -> microsoft/vscode-russh (dormant)
  - tunnels SDK pinned at 64048c1409ff56cb958b879de7ea069ec71edc8b (workspace dep, not yet consumed)
  - Secrets::MICROSOFT_REFRESH_ACCOUNT constant
  - Pitfall-14 arch-lint extended to 4 paths (alphabetical) + agent_token + tunnel_access_token identifiers
  - 5 Wave-0 #[ignore] test stubs across vector-tunnels (3) + vector-tunnel-agent (2) for Waves 1-2 to flip green

affects: [08-02-microsoft-oauth, 08-03-tunnel-agent-binary, 08-04-mac-client-transport, 08-05-picker-ui-and-actor, 08-06-agent-distribution, 08-07-uat-smoke-matrix]

tech-stack:
  added:
    - "tunnels @ git rev 64048c1409ff56cb958b879de7ea069ec71edc8b (microsoft/dev-tunnels rs/) — workspace dep declared, not yet consumed"
    - "tokio-tungstenite 0.29 — pinned at workspace level for Wave 1 transitives"
    - "[patch.crates-io] russh family -> microsoft/vscode-russh branch=main — dormant until tunnels SDK is wired"
  patterns:
    - "Wave-0 module-surface scaffold: empty mod files with `pub fn _wave_0_placeholder() {}` to make cargo happy until Wave 2 fills bodies"
    - "Workspace path-deps must carry `version = X.Y` per existing path_deps_have_versions arch-lint"

key-files:
  created:
    - .planning/research/spikes/dev-tunnels-decision.md
    - crates/vector-tunnel-protocol/Cargo.toml
    - crates/vector-tunnel-protocol/src/lib.rs
    - crates/vector-tunnel-protocol/tests/messages.rs
    - crates/vector-tunnel-agent/Cargo.toml
    - crates/vector-tunnel-agent/src/main.rs
    - crates/vector-tunnel-agent/tests/protocol_codec.rs
    - crates/vector-tunnels/src/{api,domain,model,transport}.rs
    - crates/vector-tunnels/src/auth/{mod,device_flow_microsoft,token_store}.rs
    - crates/vector-tunnels/tests/list_tunnels.rs
    - crates/vector-tunnels/tests/fixtures/dev_tunnels_list.json
    - crates/vector-secrets/tests/microsoft_account.rs
  modified:
    - Cargo.toml (workspace members + workspace deps + patch.crates-io)
    - Cargo.lock (new crates resolved)
    - crates/vector-tunnels/Cargo.toml (filled from stub)
    - crates/vector-tunnels/src/lib.rs (module surface)
    - crates/vector-secrets/src/lib.rs (added MICROSOFT_REFRESH_ACCOUNT)
    - crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs (SCAN_PATHS list + Phase-8 identifiers)

key-decisions:
  - "Russh patch declared at workspace level but stays DORMANT because no crate actually depends on `tunnels` this wave (Wave-1 Plan 08-03 wires the agent dep; Wave-2 Plan 08-04 wires the Mac client). Cargo emitted a non-fatal `Patch ... was not used in the crate graph` warning. vector-ssh on russh 0.60 is unaffected — the very downgrade risk the plan warned about is deferred until the SDK is actually consumed."
  - "Phase-8 plan crates declared with `version = 2026.5.10` on path-deps to satisfy the existing path_deps_have_versions arch-lint. Same convention used in workspace.dependencies for vector-tunnels / vector-tunnel-protocol."
  - "vector-tunnel-agent declares oauth2 + keyring-core + reqwest as workspace deps in Wave 0 per plan, even though the stub doesn't use them — keeps Wave 1's first patch surface minimal."
  - "`tunnels = { workspace = true }` left commented in both vector-tunnels and vector-tunnel-agent Cargo.toml — explicit Wave-0 minimal-surface rule. Waves 1-2 uncomment when consuming the SDK."

patterns-established:
  - "Pitfall-14 SCAN_PATHS: const-list of relative crate src dirs, alphabetical; new crates added by appending to the const + a single guard test (scan_paths_include_new_phase_8_crates)"
  - "Phase-8 token identifier set adds agent_token + tunnel_access_token alongside Phase-6's access_token / refresh_token / device_code / client_secret / user_code"

requirements-completed: [DT-01, DT-02, DT-03, DT-04]

duration: 18min
completed: 2026-05-21
---

# Phase 8 Plan 01: Foundations & Scaffold Summary

**Vendored microsoft/dev-tunnels SDK (pinned SHA 64048c1) + russh patch (dormant) + 3 new crates (vector-tunnel-protocol, vector-tunnel-agent, vector-tunnels filled out) + Pitfall-14 arch-lint extended to all 4 token-handling crates, all behind a committed DT-01 spike decision doc that gates Phase 8 SC#1.**

## Performance

- **Duration:** ~18 min
- **Started:** 2026-05-21T20:42Z
- **Completed:** 2026-05-21T20:57Z
- **Tasks:** 2 (plus a Step-0 doc commit before Task 1's code)
- **Files modified/created:** 21 (5 new crate files in vector-tunnel-protocol, 3 in vector-tunnel-agent, 9 in vector-tunnels, 1 in vector-secrets test, 1 in vector-arch-tests, plus Cargo.toml/Cargo.lock + the spike doc)

## Accomplishments

- DT-01 spike decision document committed at `.planning/research/spikes/dev-tunnels-decision.md` (89 lines, codifies Path 2c LOCKED). **Gates Phase 8 SC#1.**
- Three crates wired into the workspace: `vector-tunnel-protocol` (real codec — 4 passing tests), `vector-tunnel-agent` (stub binary), `vector-tunnels` (filled out from a 1-line stub to a module surface).
- `Secrets::MICROSOFT_REFRESH_ACCOUNT = "microsoft_refresh_token"` constant added + 1 passing test.
- Pitfall-14 arch-lint refactored from inline `vector-codespaces/src`-only scan to a 4-path `SCAN_PATHS` const list with a guard test (`scan_paths_include_new_phase_8_crates`).
- Five Wave-0 `#[ignore]` test stubs landed across the three new crates — flipped green by Waves 1-2:
  - `vector-tunnels/tests/list_tunnels.rs::list_tunnels_filters_to_vector_agent_label` → Wave 2 Plan 08-04
  - `vector-tunnels/tests/list_tunnels.rs::list_tunnels_handles_401` → Wave 2 Plan 08-04
  - `vector-tunnels/tests/list_tunnels.rs::list_tunnels_strips_vector_prefix` → Wave 2 Plan 08-04
  - `vector-tunnel-agent/tests/protocol_codec.rs::agent_echoes_data_frames` → Wave 1 Plan 08-03
  - `vector-tunnel-agent/tests/protocol_codec.rs::agent_emits_exit_on_shell_exit` → Wave 1 Plan 08-03

## Task Commits

1. **Step 0: DT-01 spike decision doc** — `454618e` (docs: gate SC#1 before any integration code)
2. **Task 1: Workspace scaffold + russh patch + 3 crates + secrets const + fixture** — `44d35ba` (feat)
3. **Task 2: Pitfall-14 arch-lint extension** — `959b449` (test)

## Pinned SHAs

- `tunnels` (microsoft/dev-tunnels rs/): `64048c1409ff56cb958b879de7ea069ec71edc8b` (matches vscode CLI's own pin)
- `russh` / `russh-keys` / `russh-cryptovec` (microsoft/vscode-russh): `branch = "main"` (Cargo.lock pins to current main; resolved git rev `0bced230` as of 2026-05-21)

## Wave-0 #[ignore] Test Stubs Created

| Test | Crate | Flipped green by |
| ---- | ----- | ---------------- |
| `list_tunnels_filters_to_vector_agent_label` | vector-tunnels | Plan 08-04 |
| `list_tunnels_handles_401` | vector-tunnels | Plan 08-04 |
| `list_tunnels_strips_vector_prefix` | vector-tunnels | Plan 08-04 |
| `agent_echoes_data_frames` | vector-tunnel-agent | Plan 08-03 |
| `agent_emits_exit_on_shell_exit` | vector-tunnel-agent | Plan 08-03 |

## Phase 7 vector-ssh Status: SURVIVED

The plan called out the russh 0.37 patch as a Phase-7 break risk and instructed STOP-and-escalate if vector-ssh failed to compile. Result: **no escalation needed.** The patch is dormant because no workspace crate consumes the `tunnels` SDK this wave; Cargo printed `Patch ... was not used in the crate graph` and resolved russh at its workspace-pinned 0.60. `vector-ssh v2026.5.10` compiled clean against russh 0.60 with zero source changes. This deferral pushes the actual russh-0.37 downgrade decision to Wave 1 Plan 08-03 (agent depends on tunnels) and Wave 2 Plan 08-04 (Mac client depends on tunnels) — whoever lights up that path will face the real Phase-7 compatibility question.

## Decisions Made

- Committed DT-01 spike doc **first** as a standalone commit before any code, to make the SC#1 gate observable in `git log` independently of the scaffold churn (matches Step 0's intent).
- Kept `tunnels = { workspace = true }` commented in both new-crate Cargo.toml files. The workspace dep declaration alone resolves the SHA in Cargo.lock; commenting it out of crate-level deps keeps russh at 0.60 across Wave 0.
- Added `version = "2026.5.10"` to all new workspace path-deps to silence the existing `path_deps_have_versions` arch-lint (D-83 sub-item #2). Same fix applied to `vector-tunnels/Cargo.toml`'s newly-added vector-mux + vector-secrets path deps.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] path_deps_have_versions arch-lint required `version = "2026.5.10"` on Phase-8 path deps**
- **Found during:** Task 2 verification (cargo test -p vector-arch-tests --tests)
- **Issue:** Workspace-level `vector-tunnel-protocol`/`vector-tunnels` path deps were declared as `{ path = "..." }` only; the existing D-83 #2 arch-lint requires every path dep to also carry `version = "X.Y"` for cargo-publish / cargo-deny compatibility. vector-tunnels/Cargo.toml's vector-mux + vector-secrets path deps also needed pins.
- **Fix:** Added `version = "2026.5.10"` to all four path-dep declarations.
- **Files modified:** Cargo.toml, crates/vector-tunnels/Cargo.toml
- **Verification:** `cargo test -p vector-arch-tests --tests` → 3 + 1 + 2 + 1 passed across the 4 arch-lint test files; 0 failed.
- **Committed in:** 959b449 (Task 2 commit)

**2. [Rule 1 - rustfmt] cargo fmt rewrapped match arms in `AgentMessage::fmt` and tests**
- **Found during:** Task 1 `make lint` step
- **Issue:** Initial hand-written multi-arm match wrapping in `Debug for AgentMessage` and one inline assert message in tests did not match rustfmt's preferred wrap; `make lint` (which runs `cargo fmt --all -- --check`) failed.
- **Fix:** Ran `cargo fmt --all` and re-verified all tests stayed green.
- **Files modified:** crates/vector-tunnel-protocol/src/lib.rs, crates/vector-tunnel-protocol/tests/messages.rs, crates/vector-secrets/tests/microsoft_account.rs (one-line reflow)
- **Verification:** `make lint` exit 0; tests still 4 + 1 passed.
- **Committed in:** 44d35ba (Task 1 commit)

**3. [Rule 1 - rustfmt] cargo fmt rewrapped the `expected` array in `scan_paths_include_new_phase_8_crates`**
- **Found during:** Task 2 `make lint` step
- **Issue:** Inline `let expected = ["x", "y", "z"]` wrapping was wider than rustfmt's preferred limit; lint failed.
- **Fix:** Ran `cargo fmt --all` to multi-line the array.
- **Files modified:** crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs
- **Verification:** `make lint` exit 0; arch-tests 6 + 1 passed across all files.
- **Committed in:** 959b449 (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (1 blocking dep contract, 2 rustfmt reflows)
**Impact on plan:** All three are mechanical Rule-1/Rule-3 fixes the plan couldn't have predicted without running rustfmt + arch-lint locally. No semantic change to any acceptance criterion. No scope creep.

## Issues Encountered

- **Cargo emitted "Patch was not used in the crate graph" warning** on first build after adding `[patch.crates-io] russh = vscode-russh`. Investigated: this is expected because no workspace crate actually depends on the `tunnels` SDK in Wave 0 (the agent and the Mac client both have `tunnels` commented out per the plan's minimal-surface rule). The patch correctly sits dormant; russh resolves to 0.60.3 as before. Phase-7 vector-ssh compiled clean — the russh-0.37 downgrade question is deferred until Wave 1/2 plans actually consume `tunnels`. Documented in §"Phase 7 vector-ssh Status".

## Known Stubs

`vector-tunnels` ships 7 module files (`api.rs`, `domain.rs`, `model.rs`, `transport.rs`, `auth/mod.rs`, `auth/device_flow_microsoft.rs`, `auth/token_store.rs`) each containing only:

```rust
//! <module purpose>. Wave 2 Plan 08-04 fills in.

pub fn _wave_0_placeholder() {}
```

These are **intentional Wave-0 module-surface placeholders** explicitly mandated by Plan 08-01 Step 9 ("Each file body = a top-of-file comment 'Wave 2 Plan 08-04' plus minimal `pub fn _wave_0_placeholder() {}` to keep cargo silent. Module surface only; types live in Wave 2"). They unblock parallel planning of Waves 1-2 against a stable module surface. Plan 08-04 replaces every one with real types. No data is wired to UI in Wave 0 — these modules have no callers yet outside `vector-tunnels::lib.rs` re-exports.

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: .planning/research/spikes/dev-tunnels-decision.md (89 lines)
- FOUND: crates/vector-tunnel-protocol/src/lib.rs
- FOUND: crates/vector-tunnel-protocol/tests/messages.rs
- FOUND: crates/vector-tunnel-agent/src/main.rs
- FOUND: crates/vector-tunnel-agent/tests/protocol_codec.rs
- FOUND: crates/vector-tunnels/tests/list_tunnels.rs
- FOUND: crates/vector-tunnels/tests/fixtures/dev_tunnels_list.json (5 tunnels, 2 with vector-agent label)
- FOUND: crates/vector-secrets/tests/microsoft_account.rs

**Commits verified in git log:**
- FOUND: 454618e (docs(08-01) spike doc)
- FOUND: 44d35ba (feat(08-01) scaffold)
- FOUND: 959b449 (test(08-01) arch-lint)

**Acceptance gates verified:**
- `Path 2 Variant 2c` in spike doc: 2 matches
- `LOCKED` in spike doc: 1 match
- pinned SHA `64048c1409ff56cb958b879de7ea069ec71edc8b` in spike doc + Cargo.toml
- jq tunnel count in fixture: 5; vector-agent-labeled: 2
- `#[ignore` stubs: list_tunnels.rs=3, protocol_codec.rs=2

## Next Phase / Plan Readiness

- **Plan 08-02 (Microsoft OAuth Device Flow):** Inherits `Secrets::MICROSOFT_REFRESH_ACCOUNT`, `vector-tunnels/src/auth/{device_flow_microsoft,token_store}.rs` empty surfaces, and Pitfall-14 arch-lint live on `vector-tunnels/src`. Ready.
- **Plan 08-03 (vector-tunnel-agent binary):** Inherits `vector-tunnel-agent/src/main.rs` CLI stub + `vector-tunnel-protocol` AgentMessage + 2 ignored protocol_codec tests to flip green. Will need to either uncomment `tunnels = { workspace = true }` (forcing the russh patch live) or vendor a thinner subset; the russh-0.37 downgrade decision lands here.
- **Plan 08-04 (Mac client + transport):** Inherits `vector-tunnels/src` module surface + `tests/fixtures/dev_tunnels_list.json` + 3 ignored list_tunnels tests. Will hit the same russh-0.37 question. Recommend coordinating with Plan 08-03's resolution.

---
*Phase: 08-vs-code-remote-tunnels-connect*
*Completed: 2026-05-21*
