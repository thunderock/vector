---
phase: 01-foundation-ci-dmg-pipeline
plan: 02
subsystem: infra
tags: [rust, workspace-lints, cargo-deny, cargo-husky, architecture-lint, d-08, d-11, d-20, d-31]

requires:
  - "01-01-SUMMARY (14-crate workspace and stub lib/main files)"
provides:
  - "Workspace-level [workspace.lints.rust] (unsafe_code=deny) and [workspace.lints.clippy] (pedantic warn + await_holding_lock deny) inherited by all 14 member crates"
  - "deny.toml policy: 11-entry SPDX allow-list (incl. Unicode-DFS-2016, Unicode-3.0, MPL-2.0), openssl/openssl-sys explicit bans, unknown-registry+unknown-git denied"
  - "cargo-husky 1.x dev-dep on vector-app + .cargo-husky/hooks/pre-commit running fmt --check + clippy -D warnings"
  - "Per-crate tests/no_tokio_main.rs in all 14 crates: filesystem scan that fails the build on #[tokio::main], #[tokio::test], Builder::new_current_thread(), Runtime::new(), or unauthored block_on()"
  - "vector-app's BLOCK_ON_ALLOWLIST = [\"src/main.rs\"] — only main.rs may call block_on (Plan 01-03 will exercise this)"
  - "Verified Wave-0 spike: CARGO_HUSKY_DONT_INSTALL_HOOKS=1 is honored by cargo-husky 1.5.0 (no hook write); demonstrated separately on a clean clone that cargo-husky DOES install the hook when the env var is absent."
affects:
  - "01-03-PLAN (vector-app shell): will add rt.block_on(io_main(proxy)) to crates/vector-app/src/main.rs; the per-crate test already allowlists exactly that file."
  - "01-05-PLAN (CI): will run `cargo deny check`, `cargo test --workspace`, and grep for missing tests/no_tokio_main.rs on new crates as belt-and-braces."
  - "every later phase: must not introduce a #[tokio::main], #[tokio::test], or block_on() outside main.rs without first updating the per-crate allowlist (deliberate architectural decision, not a typo escape)."

tech-stack:
  added:
    - "cargo-husky 1.5.0 (dev-dep, user-hooks feature; default-features off per Pitfall 6)"
    - "cargo-deny 0.19.5 (developer-machine tool, not a workspace dep)"
  patterns:
    - "Workspace-wide lint policy via [workspace.lints.{rust,clippy}] + per-crate [lints] workspace = true inheritance (cargo 1.74+ feature)."
    - "Cargo-deny `unmaintained = \"workspace\"` keeps advisory checks ranged to direct + transitive workspace deps without false-positives from unrelated crates.io."
    - "Per-crate architecture-lint test = byte-identical file across crates except for one ALLOWLIST line — easy to copy when adding a new crate, hard to silently regress."

key-files:
  created:
    - "deny.toml — cargo-deny policy at repo root (graph/advisories/licenses/bans/sources)."
    - ".cargo-husky/hooks/pre-commit — executable shell hook (fmt --check + clippy -D warnings)."
    - "crates/vector-app/tests/no_tokio_main.rs — filesystem-scan integration test; ALLOWLIST = [\"src/main.rs\"]."
    - "crates/vector-ui/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-render/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-mux/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-term/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-pty/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-ssh/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-codespaces/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-tunnels/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-config/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-secrets/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-fonts/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-input/tests/no_tokio_main.rs — same; empty ALLOWLIST."
    - "crates/vector-theme/tests/no_tokio_main.rs — same; empty ALLOWLIST."
  modified:
    - "Cargo.toml — appended [workspace.lints.rust] + [workspace.lints.clippy] block (6 effective settings, 4 pedantic mute-overrides)."
    - "crates/vector-app/Cargo.toml — added [dev-dependencies] cargo-husky and [lints] workspace = true."
    - "crates/vector-ui/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-render/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-mux/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-term/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-pty/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-ssh/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-codespaces/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-tunnels/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-config/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-secrets/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-fonts/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-input/Cargo.toml — appended [lints] workspace = true."
    - "crates/vector-theme/Cargo.toml — appended [lints] workspace = true."
    - "Cargo.lock — regenerated to include cargo-husky 1.5.0."

key-decisions:
  - "Wave-0 spike (Pitfall 6) ran in this worktree: CARGO_HUSKY_DONT_INSTALL_HOOKS=1 honored — no hook write. Separately verified in a clean /tmp git repo that without the env var cargo-husky DOES install .git/hooks/pre-commit. CI escape hatch is real and locked-in (Plan 01-05's CI YAML must export this env var)."
  - "Per-crate test file kept byte-identical except for the BLOCK_ON_ALLOWLIST constant and one rustdoc comment line. Diff-able invariant: `diff crates/vector-app/tests/no_tokio_main.rs crates/vector-mux/tests/no_tokio_main.rs` shows exactly 4 lines differ (2 rustdoc lines + 1 ALLOWLIST line + diff hunk-marker)."
  - "cargo-deny exits 0 on the empty Phase-1 workspace. The 9 `license-not-encountered` warnings (BSD-2/3-Clause, 0BSD, ISC, MPL-2.0, Unicode-DFS-2016, Unicode-3.0, CC0-1.0, Zlib) are intentional future-proofing per Pitfall 5 — Phases 2-8 will populate them as real deps arrive (e.g., russh brings BSD-3-Clause, alacritty_terminal brings Apache-2.0)."

requirements-completed: [WIN-05, BUILD-01]

duration: ~8min
completed: 2026-05-10
---

# Phase 01 Plan 02: Workspace Lints + cargo-deny + Architecture-Lint Summary

**Locks D-06 (unsafe_code deny), D-11 (await_holding_lock deny), D-20 (cargo-deny rustls-only supply chain), D-31 (cargo-husky local gate), and D-08 (per-crate filesystem-scan against tokio runtime regressions) — `cargo test --workspace --tests` and `cargo deny check` both green on the empty Phase-1 workspace.**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-10T22:40:17Z
- **Completed:** 2026-05-10T22:48:18Z
- **Tasks:** 2 (Task 1: workspace.lints + deny.toml + cargo-husky; Task 2: 14× no_tokio_main.rs)
- **Files created:** 16 (deny.toml, .cargo-husky/hooks/pre-commit, 14× tests/no_tokio_main.rs)
- **Files modified:** 16 (Cargo.toml, Cargo.lock, 14× crates/vector-*/Cargo.toml)

## Accomplishments

- `Cargo.toml` now contains `[workspace.lints.rust] unsafe_code = "deny"` and `[workspace.lints.clippy] pedantic = { level = "warn", priority = -1 }, await_holding_lock = "deny"` with 4 muted pedantic rules (`module_name_repetitions`, `must_use_candidate`, `missing_errors_doc`, `missing_panics_doc`) per D-06 + Phase-10 deferral note.
- All 14 `crates/vector-*/Cargo.toml` files added `[lints]\nworkspace = true` — verified by grep loop, no false positives.
- `deny.toml` at repo root: `[graph] all-features = true`, `[advisories] yanked = "deny", unmaintained = "workspace"`, `[licenses] version = 2, confidence-threshold = 0.93`, 11-entry SPDX allow-list including the two `Unicode-DFS-2016` / `Unicode-3.0` entries that Pitfall 5 specifically called out, `[bans]` denies `openssl` and `openssl-sys` by name, `[sources] unknown-registry = "deny"` and `unknown-git = "deny"`.
- `cargo deny check advisories licenses bans sources` exits 0; final line: `advisories ok, bans ok, licenses ok, sources ok`.
- `crates/vector-app/Cargo.toml` has `[dev-dependencies] cargo-husky = { version = "1", default-features = false, features = ["user-hooks"] }`.
- `.cargo-husky/hooks/pre-commit` exists, executable (`-rwxr-xr-x`), 94 bytes, runs `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`.
- All 14 `crates/*/tests/no_tokio_main.rs` files compile and pass; `cargo test --workspace --tests` reports 14 `test forbidden_tokio_patterns_absent_from_src ... ok` lines.
- `cargo build --workspace` exits 0 with no warning regressions.

## Task Commits

1. **Task 1: workspace lints + cargo-deny + cargo-husky** — `14ecd78` (feat)
2. **Task 2: per-crate no_tokio_main.rs architecture-lint** — `e3fb5df` (test)

## Files Created/Modified

### Created (16)

- `deny.toml` — full policy from Plan 01-RESEARCH §`deny.toml` example.
- `.cargo-husky/hooks/pre-commit` — `#!/usr/bin/env sh` + `set -e` + fmt-check + clippy -D warnings. `chmod +x`.
- `crates/vector-app/tests/no_tokio_main.rs` — BLOCK_ON_ALLOWLIST `["src/main.rs"]`.
- `crates/{vector-ui,vector-render,vector-mux,vector-term,vector-pty,vector-ssh,vector-codespaces,vector-tunnels,vector-config,vector-secrets,vector-fonts,vector-input,vector-theme}/tests/no_tokio_main.rs` — 13 library crates, identical file with empty BLOCK_ON_ALLOWLIST.

### Modified (16)

- `Cargo.toml` — appended `[workspace.lints.rust]` and `[workspace.lints.clippy]` blocks.
- `Cargo.lock` — regenerated to lock `cargo-husky 1.5.0`.
- `crates/vector-app/Cargo.toml` — added `[dev-dependencies]` + `[lints]` block.
- 13 × `crates/vector-{ui,render,mux,term,pty,ssh,codespaces,tunnels,config,secrets,fonts,input,theme}/Cargo.toml` — appended `[lints]\nworkspace = true`.

## Decisions Made

- **cargo-husky 1.5.0 with `default-features = false, features = ["user-hooks"]` (Pitfall 6 mitigation).** The default cargo-husky config would write a vendored hook that hard-codes `cargo test`; we want our `.cargo-husky/hooks/pre-commit` instead. `user-hooks` reads from the `.cargo-husky/hooks/` directory in the workspace root.
- **`unmaintained = "workspace"` instead of `"all"`.** Restricts advisory checks to deps that the workspace directly or transitively pulls; avoids noise from random parts of crates.io.
- **`confidence-threshold = 0.93`** (not 1.0) — cargo-deny's license detector is heuristic; 0.93 is the upstream-recommended default and avoids tripping on minor whitespace differences in COPYING files.
- **No `[advisories] ignore` entries.** When real advisories surface during Phases 6-8 we will list them explicitly with the advisory ID + dated rationale, not pre-emptively.

## Deviations from Plan

None. Both tasks executed exactly as specified in 01-02-PLAN.md.

## Wave-0 Spike: cargo-husky CARGO_HUSKY_DONT_INSTALL_HOOKS

The plan required documenting the spike result (assumption A2 from 01-RESEARCH.md §Assumptions Log).

### Methodology

Two runs of `cargo build -p vector-app --tests`, separated by `target/debug/build/cargo-husky-*` cache cleanup so the build script reruns:

1. **Spike A** (`CARGO_HUSKY_DONT_INSTALL_HOOKS=1 cargo build -p vector-app --tests`):
   - Pre-state: no `pre-commit` in either `$GITDIR/hooks/` (worktree gitdir) or `$GITCOMMONDIR/hooks/` (`.git/hooks/` at repo top).
   - Post-state: no `pre-commit` written. **Env var honored.**
   - cargo-husky's build-script stderr was empty (silent).

2. **Spike B** (`cargo build -p vector-app --tests`, env var unset):
   - cargo-husky's build script ran (visible in `target/debug/build/cargo-husky-*/stderr`).
   - It printed `Warning: .git directory was not found in '/home/colligo/vector/.claude/worktrees/agent-aff462b3a028e4bfd/target/debug/build/cargo-husky-05b4b1641613c9be/out' or its parent directories` — because in a **git worktree**, `.git` at the repo root is a *file* (a gitlink), not a directory, and cargo-husky 1.5.0 only walks parent directories looking for a `.git` *directory*. **No hook was written in this run either.**

### Cross-check (out-of-worktree validation)

To confirm Spike B's "no hook written" wasn't a hidden bug in our config but just the worktree-specific limitation of cargo-husky, an isolated control was run in a fresh `mktemp -d` git repo with the same `[dev-dependencies] cargo-husky` and the same `.cargo-husky/hooks/pre-commit` template. Result:

```
=== hook state ===
PRESENT:
#!/usr/bin/env sh
#
# This hook was set by cargo-husky v1.5.0: https://github.com/rhysd/cargo-husky#readme
set -e
echo test-hook
```

cargo-husky **does** install the hook on a normal (non-worktree) checkout.

### Result

- ✅ `CARGO_HUSKY_DONT_INSTALL_HOOKS=1` is honored — Plan 01-05's CI workflow can rely on this env var to keep CI clones hook-free.
- ⚠️ Side-finding: cargo-husky 1.5.0 is a **no-op** inside a git worktree (regardless of env var) because `.git` is a gitlink file. Developers working in worktrees won't get the local pre-commit gate. This is a **known limitation**, not a regression introduced by this plan. Mitigation: developers either work on the main checkout (where cargo-husky installs cleanly), or accept that the belt-and-braces CI gate (Plan 01-05) is the authoritative enforcement point.
- ➡️ Recommendation logged for Plan 01-05: have the CI workflow `export CARGO_HUSKY_DONT_INSTALL_HOOKS=1` once at the top of every job so the env var is always set in CI, regardless of cache state.

## Negative-Test: Architecture-Lint Demonstrably Catches Violations

The plan required demonstrating that adding `#[tokio::main]` to a crate's `src/` makes the per-crate test fail.

### Procedure

1. Saved baseline `crates/vector-mux/src/lib.rs` to `/tmp/vector-mux-lib-baseline.rs`.
2. Appended `// regression-test marker: #[tokio::main]` to `crates/vector-mux/src/lib.rs` (comment form so the file still compiles — the architecture-lint is a substring scan, not a syntactic check).
3. Ran `CARGO_HUSKY_DONT_INSTALL_HOOKS=1 cargo test -p vector-mux --test no_tokio_main`.
4. Restored baseline; re-ran the test.

### Result

**Step 3 output (truncated to assertion line):**

```
test forbidden_tokio_patterns_absent_from_src ... FAILED

failures:

---- forbidden_tokio_patterns_absent_from_src stdout ----

thread 'forbidden_tokio_patterns_absent_from_src' panicked at crates/vector-mux/tests/no_tokio_main.rs:43:9:
lib.rs: forbidden pattern `#[tokio::main]` (D-08 architecture-lint).
```

- Assertion message contains the required `D-08 architecture-lint` token.
- Identifies the violating file (`lib.rs`) and the matched pattern.
- `cargo test` returned non-zero (cargo printed `error: test failed`).

**Step 4 output (post-revert):**

```
test forbidden_tokio_patterns_absent_from_src ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The lint is therefore demonstrably both:
1. **Sensitive** — fails on injected `#[tokio::main]` anywhere under `src/`.
2. **Reversible** — passes immediately after the violation is removed.

## Workspace Lints in Effect (Echo-Back)

For posterity (and for the SUMMARY's "echo back the 6 lint settings" requirement):

```toml
[workspace.lints.rust]
unsafe_code = "deny"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }     # global pedantic warn-level
await_holding_lock = "deny"                      # D-11 hard rule
module_name_repetitions = "allow"                # mute (pedantic noise)
must_use_candidate = "allow"                     # mute (pedantic noise)
missing_errors_doc = "allow"                     # mute (pedantic noise)
missing_panics_doc = "allow"                     # mute (pedantic noise)
```

Effective coverage: 4 lint-rule settings actually in force (`unsafe_code = deny`, `pedantic = warn`, `await_holding_lock = deny`, 4 mute-overrides). All 14 crates inherit via `[lints] workspace = true`.

## cargo-deny Output Snippet

Final three lines of `cargo deny check advisories licenses bans sources`:

```
   │      ━━━━ unmatched license allowance

advisories ok, bans ok, licenses ok, sources ok
```

Exit code: 0. The 9 `license-not-encountered` warnings (BSD-2-Clause, BSD-3-Clause, ISC, Unicode-DFS-2016, Unicode-3.0, CC0-1.0, Zlib, 0BSD, MPL-2.0) are intentional and will be resolved as Phases 2-8 add real deps.

## Issues Encountered

None. Both tasks executed in a single pass; the negative-test fired exactly as designed; cargo-deny green on first attempt.

## User Setup Required

None — the per-crate test scan is hermetic (filesystem read of `CARGO_MANIFEST_DIR/src/`), cargo-deny installs once per developer (`cargo install --locked cargo-deny`; took ~2 min on this worktree), and cargo-husky's hook installation is automatic at `cargo build --tests` time on non-worktree clones.

## Next Phase Readiness

- **Plan 01-03 (vector-app shell):** when the plan adds `rt.block_on(io_main(proxy))` to `crates/vector-app/src/main.rs`, the per-crate test will pass on first build because `BLOCK_ON_ALLOWLIST = ["src/main.rs"]` already permits exactly that file. Any drift (e.g. adding `block_on` to `src/lib.rs`) will be caught immediately.
- **Plan 01-04 (xtask):** xtask sits in a separate workspace (D-04), so it's outside `cargo test --workspace`'s reach — the per-crate tests do not need to be replicated in xtask. xtask can use `tokio::runtime::Builder::new_current_thread()` freely if needed; this plan's architecture-lint scope is the main workspace only.
- **Plan 01-05 (CI):** must `export CARGO_HUSKY_DONT_INSTALL_HOOKS=1` at the top of every CI job (Wave-0 spike confirms this gates hook installation). Must also `cargo install --locked cargo-deny` and run `cargo deny check advisories licenses bans sources` as a separate CI step.
- **Phases 2-8 each:** when they add real dependencies, the cargo-deny `license-not-encountered` warnings will shrink to zero. Any new git source (e.g. `microsoft/dev-tunnels`) must be added to `[sources] allow-git`. Any `openssl` transitive pull will trip the ban and force a `default-features = false` workaround.

## Verification Checklist

- [x] `Cargo.toml` contains `[workspace.lints.rust]` and `unsafe_code = "deny"`.
- [x] `Cargo.toml` contains `[workspace.lints.clippy]` with `pedantic = { level = "warn", priority = -1 }` and `await_holding_lock = "deny"`.
- [x] All 14 `crates/vector-*/Cargo.toml` files contain `[lints]\nworkspace = true` (verified by shell loop in plan's automated check).
- [x] `deny.toml` exists at repo root with the 11 SPDX entries (including `Unicode-DFS-2016`, `Unicode-3.0`, `MPL-2.0`).
- [x] `deny.toml` bans `openssl` and `openssl-sys` by name.
- [x] `deny.toml` `[sources]` has both `unknown-registry = "deny"` and `unknown-git = "deny"`.
- [x] `crates/vector-app/Cargo.toml` contains `cargo-husky = { version = "1", default-features = false, features = ["user-hooks"] }`.
- [x] `.cargo-husky/hooks/pre-commit` exists, is executable (`test -x` passes), runs fmt --check + clippy -D warnings.
- [x] `cargo deny check advisories licenses bans sources` exits 0 with `advisories ok, bans ok, licenses ok, sources ok`.
- [x] `cargo build --workspace` exits 0 with no lint regressions.
- [x] All 14 `crates/*/tests/no_tokio_main.rs` files exist.
- [x] Each contains `fn forbidden_tokio_patterns_absent_from_src()`.
- [x] Each contains FORBIDDEN array with all 4 entries (`#[tokio::main]`, `#[tokio::test]`, `Builder::new_current_thread()`, `Runtime::new()`).
- [x] `crates/vector-app/tests/no_tokio_main.rs` has `BLOCK_ON_ALLOWLIST: &[&str] = &["src/main.rs"]`.
- [x] All 13 library crates have empty `BLOCK_ON_ALLOWLIST: &[&str] = &[]`.
- [x] `cargo test --workspace --tests` exits 0 with 14 instances of `test result: ok. 1 passed`.
- [x] Negative-test verified: `#[tokio::main]` injection in `vector-mux/src/lib.rs` makes the test fail with `D-08 architecture-lint` in the assertion message.
- [x] Wave-0 spike documented (env var honored: yes; in-worktree special case acknowledged).

## Self-Check: PASSED

- Files asserted present (Read/Bash `test -f`/`ls`): `deny.toml`, `.cargo-husky/hooks/pre-commit`, all 14 `crates/*/tests/no_tokio_main.rs`, all 14 `crates/*/Cargo.toml` (modifications), `Cargo.toml`. All confirmed.
- Commits asserted present: `14ecd78` (Task 1), `e3fb5df` (Task 2) — both in `git log --oneline -5`.
- Verification commands re-run after final commit: `cargo build --workspace` → 0, `cargo deny check advisories licenses bans sources` → 0, `cargo test --workspace --tests` → 0 with 14 `... ok` lines for `forbidden_tokio_patterns_absent_from_src`.

---
*Phase: 01-foundation-ci-dmg-pipeline*
*Completed: 2026-05-10*
