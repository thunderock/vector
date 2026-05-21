---
phase: 08-vs-code-remote-tunnels-connect
plan: 06
subsystem: distribution
tags: [cargo-deb, github-actions, debian-amd64, debian-arm64, xtask, agent-distribution]

requires:
  - phase: 08-vs-code-remote-tunnels-connect
    plan: 03
    provides: vector-tunnel-agent binary (CLI, device flow, RelayTunnelHost, JSON protocol pump) buildable via `cargo build -p vector-tunnel-agent --release`

provides:
  - DT-01 distribution half: vector-tunnel-agent ships as Debian .deb for amd64 + arm64 on every v* tag push
  - .github/workflows/agent-release.yml — dual-trigger CI workflow (tag push OR release: published)
  - xtask agent-dist subcommand — local Linux smoke; macOS no-op with CI hint
  - cargo-deb metadata in crates/vector-tunnel-agent/Cargo.toml
  - crates/vector-tunnel-agent/debian/{postinst,prerm} (mode 0755, v1 no-op per D-02)
  - crates/vector-tunnel-agent/README.md — install + first-run + reauth + tmux persistence + connect-from-Vector docs (96 lines)
  - Root README.md "Remote machines" section linking to the agent README

affects: [08-07-uat-smoke-matrix]

tech-stack:
  added:
    - "cargo-deb (CI dep, installed via `cargo install --locked cargo-deb`) — Rust→.deb tool, latest at execution; locked in CI lockfile after first run"
    - "gcc-aarch64-linux-gnu (apt, ubuntu-22.04 runner only) — cross-linker for the arm64 .deb"
  patterns:
    - "[package.metadata.deb] in the agent's Cargo.toml drives cargo-deb output; license-file path relative to the CRATE manifest, not workspace root (`../../LICENSE`)"
    - "Dual-trigger workflow shape: `on: push.tags ['v*']` AND `on: release: published`, with `concurrency.group` keyed off `tag_name || ref_name` so the two triggers do not double-fire"
    - "Concurrency group prefix `agent-release-` distinct from release.yml's `release-` — DMG + .deb workflows run in parallel on the same tag"
    - "Filename pinning via `cargo deb --output target/{target}/debian/vector-tunnel-agent_{ver}_{arch}.deb` strips the leading `v` from the tag so apt accepts the version"
    - "macOS-friendly xtask: `cfg!(not(target_os = \"linux\"))` short-circuits with CI hint, exit 0 — never blocks the dev loop"

key-files:
  created:
    - crates/vector-tunnel-agent/debian/postinst
    - crates/vector-tunnel-agent/debian/prerm
    - crates/vector-tunnel-agent/README.md
    - xtask/src/agent_dist.rs
    - .github/workflows/agent-release.yml
  modified:
    - crates/vector-tunnel-agent/Cargo.toml ([package.metadata.deb] section appended)
    - xtask/src/main.rs (agent_dist module + AgentDist subcommand + dispatch arm)
    - README.md (Remote machines section + link to agent README)

key-decisions:
  - "license-file path is `../../LICENSE` (relative to the agent's Cargo.toml, not the workspace root) — cargo-deb resolves license-file paths relative to the crate manifest. Verified by inspecting cargo-deb docs and the fact that the agent's manifest lives at crates/vector-tunnel-agent/Cargo.toml."
  - "Asset paths: `target/release/vector-tunnel-agent` (relative to repo root for cargo-deb) → `/usr/bin/vector-tunnel-agent` (0755); `README.md` (crate-relative) → `/usr/share/doc/vector-tunnel-agent/README.md` (0644). Plan called the second path as `crates/vector-tunnel-agent/README.md`; cargo-deb 2.x asset paths are crate-manifest-relative, so the bare `README.md` is correct (and the file lives next to the manifest at crates/vector-tunnel-agent/README.md)."
  - "Workflow concurrency group keyed off `github.event.release.tag_name || github.ref_name` (not `github.ref`) to match release.yml's pattern — handles both push-tag and release-published triggers without splintering."
  - "Filename pinning strategy: tag `v2026.5.10` → unprefixed `.deb` version `2026.5.10`. Done at the workflow level via `${TAG#v}` shell expansion + `cargo deb --output` flag so cargo-deb's default filename collation doesn't clash on download-artifact's path layout."
  - "`fail-fast: false` on the matrix so an arm64 cross-link failure doesn't abort the amd64 build mid-flight (independent diagnostic signals)."
  - "Two-stage workflow: build-deb matrix first, then attach-to-release fan-in. The attach job re-determines the tag from either trigger so it works for push events that don't carry `release.tag_name`."
  - "xtask uses subprocess invocation of `cargo deb` rather than a library dep (cargo-deb is not a library) — keeps xtask's Cargo.toml unchanged; on Linux without cargo-deb installed, prints actionable error instead of bailing silently."

patterns-established:
  - "Sub-workflow for non-DMG release artifacts: copy release.yml's trigger + concurrency shape, use a `name:`-distinct concurrency group, fan-in to a separate `attach-to-release` job for upload"
  - "License file path in cargo-deb metadata: use crate-relative path with `../../LICENSE` for workspace projects, not absolute or workspace-root-relative"

requirements-completed: [DT-01]

metrics:
  duration: "8min (executor side, excluding human UAT)"
  completed: 2026-05-21
  tasks: "2 of 3 (Task 3 is checkpoint:human-verify)"
  files: 8
---

# Phase 8 Plan 06: Agent Distribution Summary (PARTIAL — Task 3 awaiting human UAT)

**Vector Tunnel Agent now ships as Debian/Ubuntu `.deb` packages for amd64 + arm64 on every `v*` tag push. CI workflow `.github/workflows/agent-release.yml` cross-compiles both arches on `ubuntu-22.04` with `cargo-deb` and attaches the artifacts to the GitHub Release. `cargo xtask agent-dist` is wired for local Linux dev (macOS no-ops with a CI hint). Install path documented in `crates/vector-tunnel-agent/README.md` (apt-install verbatim per plan must-have). Task 3 (human-verify smoke on real Linux env) is the open gate.**

## Performance

- **Duration:** ~8 min (executor side)
- **Started:** 2026-05-21T21:32Z
- **Completed (Tasks 1 + 2):** 2026-05-21T21:40Z
- **Tasks:** 2 of 3 committed (Task 3 = checkpoint:human-verify; pending UAT)
- **Files modified/created:** 8 (5 created + 3 modified)

## Task Commits

1. **Task 1: Cargo.toml metadata + debian scripts + agent README + xtask agent-dist** — `757bd2d`
   `feat(08-06): cargo-deb metadata + xtask agent-dist + agent README`
2. **Task 2: GitHub Actions agent-release.yml workflow** — `e2d1029`
   `feat(08-06): agent-release.yml CI workflow (linux x86_64 + arm64 .deb)`
3. **Task 3: Manual smoke (cargo-deb build + dpkg-deb inspect)** — checkpoint:human-verify; not committed; awaiting user

## Cargo-deb Version

Plan called for recording the cargo-deb version that landed. Because the local Mac has no `cargo-deb` installed (and the xtask correctly no-ops on macOS), the version is determined at CI runtime via `cargo install --locked cargo-deb` (latest from crates.io, then locked into `~/.cargo/.crates2.json` for the duration of that CI job). Once Task 3 lands a real .deb in CI logs, this section will be updated with the resolved version.

## .deb Filename Format

```
vector-tunnel-agent_<ver>_<arch>.deb
```

where `<ver>` is the tag with leading `v` stripped (`v2026.5.10` → `2026.5.10`) and `<arch>` is `amd64` or `arm64`. Pinned explicitly via `cargo deb --output …` to avoid collisions on `actions/download-artifact@v4`'s path layout.

## aarch64 Cross-Compile Gotchas

1. **Linker override required.** `dtolnay/rust-toolchain` installs the rustc target but doesn't wire the C linker. We `apt-get install -y gcc-aarch64-linux-gnu` then append to `~/.cargo/config.toml`:
   ```toml
   [target.aarch64-unknown-linux-gnu]
   linker = "aarch64-linux-gnu-gcc"
   ```
2. **`cargo-deb --no-build` is mandatory** when cross-compiling because cargo-deb's default build invocation doesn't honor `--target`. We build via plain `cargo build --release -p vector-tunnel-agent --target ${{ matrix.target }}` first, then `cargo deb -p vector-tunnel-agent --no-build --target …`.
3. **`fail-fast: false`** so an arm64 link failure doesn't kill the amd64 build mid-flight.
4. **No QEMU needed.** Cross-compile only; we don't try to *run* the arm64 binary during the workflow (UAT covers that on real hardware via Task 3).

## Race-Condition Check: release.yml vs agent-release.yml

The two workflows trigger on the same `v*` tag push. Concurrency group keys are **distinct**:

| Workflow | Group | Cancel-in-progress |
| -------- | ----- | ------------------ |
| release.yml | `release-${{ github.event.release.tag_name || github.ref_name }}` | `false` |
| agent-release.yml | `agent-release-${{ github.event.release.tag_name || github.ref_name }}` | `false` |

Distinct prefixes → the two jobs run in parallel. `cancel-in-progress: false` on both → a follow-up release: published event doesn't kill an in-flight push trigger. Both workflows upload to the same GitHub Release object via `gh release upload --clobber` so they cooperate idempotently regardless of ordering.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] `license-file` path crate-relative, not repo-root-relative**

- **Found during:** Task 1 Cargo.toml authoring
- **Issue:** The plan template specified `license-file = ["LICENSE", "0"]`, implying repo-root resolution. cargo-deb 2.x resolves license-file paths relative to the **crate manifest**, not the workspace root. With `LICENSE` only, cargo-deb would look at `crates/vector-tunnel-agent/LICENSE` and fail.
- **Fix:** Used `license-file = ["../../LICENSE", "0"]` so cargo-deb finds the repo-root LICENSE from the agent crate's location.
- **Files modified:** crates/vector-tunnel-agent/Cargo.toml
- **Committed in:** 757bd2d

**2. [Rule 3 — Blocking] Asset paths crate-manifest-relative**

- **Found during:** Task 1 Cargo.toml authoring
- **Issue:** Plan template specified `["crates/vector-tunnel-agent/README.md", …]` for the doc asset. cargo-deb resolves asset source paths relative to the crate manifest too, not the repo root — so the prefix path would have failed to copy.
- **Fix:** Used bare `README.md` for the doc asset (file lives at `crates/vector-tunnel-agent/README.md`, which IS the crate manifest dir).
- **Files modified:** crates/vector-tunnel-agent/Cargo.toml
- **Committed in:** 757bd2d

**3. [Rule 2 — Critical Functionality] `${TAG#v}` version stripping in workflow**

- **Found during:** Task 2 workflow authoring
- **Issue:** Plan template used `${{ github.ref_name }}` directly inside the `--output` filename. That interpolates the **tag** (e.g. `v2026.5.10`), but cargo-deb's metadata `version` derives from Cargo.toml (`2026.5.10`). Filename `vector-tunnel-agent_v2026.5.10_amd64.deb` is non-standard for Debian (no leading `v`) and would also mismatch the .deb's internal Version field.
- **Fix:** Added a `Resolve tag` step that computes `ver=${TAG#v}` and uses `${{ steps.tag.outputs.ver }}` in the `--output` path. Now the filename matches the cargo-deb internal Version exactly.
- **Files modified:** .github/workflows/agent-release.yml
- **Committed in:** e2d1029

**4. [Rule 1 — Documentation completeness] Root README.md "Remote machines" section**

- **Found during:** Task 1 root README update
- **Issue:** Plan called for "a brief section linking to crates/vector-tunnel-agent/README.md". The current root README only documents Vector.dmg install. Without a section, users have no entry point to the agent at all.
- **Fix:** Added a 9-line "Remote machines (Phase 8 Dev Tunnels)" section between Install and Status, with the verbatim apt-install one-liner. Keeps the agent README as source of truth.
- **Files modified:** README.md
- **Committed in:** 757bd2d

---

**Total deviations:** 4 (3 blocking dep-resolution fixes the plan's pseudocode couldn't have foreseen without exercising cargo-deb's path semantics + 1 doc completeness Rule-1). No scope creep; the workflow shape matches the plan's intent.

**Notable plan content not deviated:** No Cargo.toml changes to xtask (Step 5 noted "no new Cargo dep is needed" — confirmed; xtask shells out to `cargo deb` as subprocess). No `[[bin]]` changes to the agent. No CHANGELOG/version bump.

## Auth Gates

None. All work was code/docs/workflow — no external service auth required to commit. The only auth-bearing step is the CI workflow's `GITHUB_TOKEN` which GitHub Actions provides automatically; nothing for the executor to obtain.

## Self-Check: PASSED

**Files verified to exist:**

- FOUND: crates/vector-tunnel-agent/Cargo.toml (with `[package.metadata.deb]` block + `assets` + `maintainer-scripts`)
- FOUND: crates/vector-tunnel-agent/debian/postinst (mode 0755)
- FOUND: crates/vector-tunnel-agent/debian/prerm (mode 0755)
- FOUND: crates/vector-tunnel-agent/README.md (96 lines)
- FOUND: xtask/src/agent_dist.rs (44 lines)
- FOUND: xtask/src/main.rs (with `mod agent_dist;` + `Cmd::AgentDist` + dispatch)
- FOUND: .github/workflows/agent-release.yml (123 lines)
- FOUND: README.md (with "Remote machines" section + 5 mentions of `vector-tunnel-agent`)

**Commits verified in git log:**

- FOUND: 757bd2d (Task 1 — feat: cargo-deb metadata + xtask agent-dist + agent README)
- FOUND: e2d1029 (Task 2 — feat: agent-release.yml workflow)

**Acceptance gates verified (Task 1):**

- `cargo build -p vector-tunnel-agent --release` → `Finished release profile … in 31.61s` (exit 0)
- `cargo xtask agent-dist` (on macOS host) → prints "cross-compile to Linux is not supported locally" + CI hint, exits 0
- `grep -c "package.metadata.deb" crates/vector-tunnel-agent/Cargo.toml` = 1
- `grep -c "usr/bin/vector-tunnel-agent" crates/vector-tunnel-agent/Cargo.toml` = 1
- `grep -c "assets" crates/vector-tunnel-agent/Cargo.toml` = 1
- `grep -c "AgentDist\|agent_dist" xtask/src/main.rs` = 3
- `wc -l crates/vector-tunnel-agent/README.md` = 96 (≥ 30 required)
- `grep -c "vector-tunnel-agent --reauth" crates/vector-tunnel-agent/README.md` = 1
- `grep -c "tmux new" crates/vector-tunnel-agent/README.md` = 1
- `grep -c "vector-tunnel-agent" README.md` = 5

**Acceptance gates verified (Task 2):**

- `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/agent-release.yml'))"` → exit 0 (YAML parses)
- `grep -c "x86_64-unknown-linux-gnu" .github/workflows/agent-release.yml` = 1
- `grep -c "aarch64-unknown-linux-gnu" .github/workflows/agent-release.yml` = 3
- `grep -c "gcc-aarch64-linux-gnu" .github/workflows/agent-release.yml` = 1
- `grep -c "cargo deb" .github/workflows/agent-release.yml` = 1
- `grep -c "gh release upload" .github/workflows/agent-release.yml` = 1
- `grep -c "dpkg-deb --contents" .github/workflows/agent-release.yml` = 1
- `grep -c "concurrency:" .github/workflows/agent-release.yml` = 1
- `grep -c "published" .github/workflows/agent-release.yml` = 1
- `grep -c "GITHUB_TOKEN" .github/workflows/agent-release.yml` = 1

## Checkpoint Status — Task 3 (Manual Smoke on Linux)

**State:** Awaiting user UAT. The executor cannot run dpkg-deb / sudo apt install on the dev Mac. Plan called for either a Linux box or Docker container:

```sh
docker run -it --rm -v $PWD:/work -w /work rust:1.88-bookworm bash -c '
  cargo install cargo-deb &&
  cargo build --release -p vector-tunnel-agent &&
  cargo deb -p vector-tunnel-agent --no-build &&
  dpkg-deb --info target/debian/vector-tunnel-agent_*.deb &&
  dpkg-deb --contents target/debian/vector-tunnel-agent_*.deb
'
```

Plus, ideally on a fresh Ubuntu VM:
- `sudo apt install ./target/debian/vector-tunnel-agent_*.deb` — install succeeds, postinst prints "installed"
- `vector-tunnel-agent --version` — prints version, exits 0
- `sudo apt remove vector-tunnel-agent` — uninstall clean

**macOS smoke (item #2 in plan's how-to-verify):** PASSED. `cargo xtask agent-dist` on Mac dev host printed "cross-compile to Linux is not supported locally." + CI hint, exited 0.

## Known Stubs

None this plan. All artifacts are real:

- `debian/postinst` / `debian/prerm` are intentionally no-op (D-02: v1 has no system service; user-managed lifecycle). The `echo "installed"` line in postinst is a user-affordance message, not a stub.
- `xtask agent-dist` on macOS is a no-op by *design* — cross-compiling Rust to Linux from a Mac without a Linux toolchain is unsupported; the workflow shoulders that load via `ubuntu-22.04` runners. The function explicitly prints the CI hint and exits 0.

## Issues Encountered

- **License-file path resolution.** cargo-deb 2.x documentation specifies all paths are crate-manifest-relative. The plan's template `license-file = ["LICENSE", "0"]` would have failed on the first CI run (where cargo-deb walks the path); fixed pre-emptively with `../../LICENSE`. Plan 08-07's UAT smoke matrix should also verify the .deb's `dpkg-deb --info` output lists the correct copyright/license.
- **Cross-link toolchain absence on dtolnay/rust-toolchain.** Installing the rustc target via `targets:` does not provide a C linker. Added the apt install + `~/.cargo/config.toml` linker override step explicitly. This is the documented workaround for arm64 cross-compile on x86_64 GitHub-hosted runners.

## Next Phase / Plan Readiness

- **Plan 08-07 (UAT smoke matrix):** Inherits the .deb-on-tag distribution loop. Smoke matrix should include:
  - `wget` the .deb from a tagged release page (after the next `v*` push)
  - `sudo apt install ./vector-tunnel-agent_*.deb`
  - First-run device-flow completes end-to-end (Plan 08-03's auth path)
  - Mac client picker (Plan 08-05) sees the registered tunnel labeled `vector-agent`
  - Open a tab → remote shell works (Plan 08-04 transport + Plan 08-03 protocol)
  - `sudo apt remove vector-tunnel-agent` clean

## Deferred — Awaiting Human Verification

**Phase 8 execution proceeds without blocking on Task 3.** The orchestrator has elected to DEFER Task 3 (Debian package manual smoke on Linux) rather than block Wave 4. No Linux env is available on the dev Mac; the .deb smoke is tracked as a pending human-verification item and will be exercised by Plan 08-07's UAT smoke matrix (or sooner, ad-hoc, once a Linux box is at hand).

**See:** `08-06-HUMAN-UAT.md` (status: partial, pending: 1).

### Task 3 — Manual .deb smoke (Linux)

**Status:** Pending — human UAT.

**Verification steps** (verbatim from `08-06-agent-distribution-PLAN.md` §`<how-to-verify>`):

1. On a Linux dev box (or Ubuntu VM), or Docker container `docker run -it --rm -v $PWD:/work -w /work rust:1.88-bookworm`:
   ```sh
   cargo install cargo-deb
   cargo build --release -p vector-tunnel-agent
   cargo deb -p vector-tunnel-agent --no-build
   dpkg-deb --info target/debian/vector-tunnel-agent_*.deb        # metadata + maintainer + section: net
   dpkg-deb --contents target/debian/vector-tunnel-agent_*.deb    # lists /usr/bin/vector-tunnel-agent
   sudo apt install ./target/debian/vector-tunnel-agent_*.deb     # installs cleanly; postinst prints "installed"
   vector-tunnel-agent --version                                   # prints version, exits 0
   sudo apt remove vector-tunnel-agent                             # uninstalls cleanly; prerm runs
   ```
2. On Mac dev box: `cargo xtask agent-dist` should print "cross-compile to Linux is not supported locally." and exit 0. **(Already PASSED — see Self-Check section.)**
3. After next `v*` tag push: verify `.github/workflows/agent-release.yml` runs end-to-end in the GitHub Actions tab and that both `vector-tunnel-agent_<ver>_amd64.deb` and `_arm64.deb` attach to the release.

**Resume signal:** Type "approved" with brief notes on `dpkg-deb` output. If install fails on the test box, paste the error.

**Impact while deferred:** None on Wave 4. The .deb distribution path is implementation-complete (Cargo.toml metadata + debian/ scripts + agent-release.yml workflow + xtask agent-dist) and validated structurally (cargo-deb path semantics + workflow YAML + arch matrix). Only the live `dpkg-deb`/`apt install`/`apt remove` round-trip on a real Linux host is unproven. DT-01 remains Complete (already closed by Plan 08-03's agent-binary half; Plan 08-06 closes the distribution half at the implementation layer).

---
*Phase: 08-vs-code-remote-tunnels-connect*
*Tasks 1-2 completed: 2026-05-21*
*Task 3 (human UAT) status: deferred — tracked in 08-06-HUMAN-UAT.md (will be exercised by Plan 08-07 smoke matrix or ad-hoc on next Linux access)*
