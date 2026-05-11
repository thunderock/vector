# 0006. CI runner labels + main branch protection

- Status: accepted
- Date: 2026-05-10
- Deciders: solo (user)
- Tags: phase-1, ci, gating, security

## Context and Problem Statement

(a) D-21 of CONTEXT.md specified the previous Intel runner as the x86_64
target. That runner image was retired by GitHub on 2025-12-04 (per RESEARCH.md
§Constraint Drift). We need a documented amendment.

(b) D-34 + D-35 specify branch protection on `main`: required status checks,
linear history, force-push disabled, PR review required (with 0 reviewers
for now). GitHub branch protection is configured in the GitHub UI / API,
not in repo files. We need an audit trail.

## Decision Drivers

- Use a runner image that's actually available
- Document the EOL window so we have advance warning before the next change
- Make the branch protection state inspectable + reproducible

## Considered Options

- Runner: previous Intel image (rejected — retired)
- Runner: `cargo-zigbuild` cross-compile on `macos-14` (rejected — extra
  complexity; we don't need it because `macos-15-intel` exists)
- Runner: `macos-15-intel` (chosen — Intel macOS 15, available until Aug 2027)
- Branch protection via probot/settings (rejected — extra GitHub App install)
- Branch protection via GitHub UI manual setup, documented in `docs/setup.md`

## Decision Outcome

(a) Runner: `macos-15-intel` for x86_64 builds in both `ci.yml` and
`release.yml`. The `cargo-zigbuild` fallback is no longer needed for v1.
**EOL warning: macos-15-intel is GitHub's last Intel runner, available until
August 2027.** Set a calendar reminder in 2027 Q1 to revisit.

(b) Branch protection configured via GitHub UI per `docs/setup.md`. Required
status checks: `lint`, `commitlint`, `test`, `deny`, `build-arm64`,
`build-x86_64`, `package`. Linear history required. Force-push disabled on
`main`. PR review = 0 reviewers (the gate exists; just not requiring approvals
while solo). Required-status checks list matches the seven ci.yml job names
verbatim per Plan 01-05's branch-protection contract.

## Pros and Cons of the Options

- **Previous Intel runner:** familiar; retired.
- **cargo-zigbuild on arm64:** removes Intel runner dep; new tooling surface.
- **macos-15-intel (chosen):** drop-in replacement; documented EOL.
- **probot/settings:** declarative; requires an additional GitHub App and
  permission to install on the repo.
- **Manual UI + `docs/setup.md` (chosen):** zero extra surface area; trade-off
  is the periodic-audit responsibility documented below.

## Consequences

- A future `macos-15-intel` retirement (post-Aug 2027) needs a migration plan.
  `cargo-zigbuild` on `macos-14` is the documented v2 fallback.
- Branch protection isn't enforced by source control; periodic audit via
  `gh api repos/{owner}/vector/branches/main/protection` is the only way to
  verify it stayed configured. Add an entry to a future `docs/release-checklist.md`.
- Commit signing is NOT required in v1 (solo dev). A v2 ADR will add it.
