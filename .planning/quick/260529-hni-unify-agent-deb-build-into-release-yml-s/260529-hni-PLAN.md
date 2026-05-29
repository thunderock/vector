---
phase: quick
plan: 260529-hni
type: execute
wave: 1
depends_on: []
files_modified:
  - .github/workflows/release.yml
  - .github/workflows/agent-release.yml
autonomous: true
requirements: [BUILD-04, BUILD-05]
must_haves:
  truths:
    - "A single `v*` tag triggers ONE workflow that produces the Universal .dmg AND both agent .debs"
    - "The release job uploads Vector-*-universal.dmg AND vector-tunnel-agent_*_{amd64,arm64}.deb to the same GitHub Release"
    - "agent-release.yml no longer exists (its tag-triggered work is now redundant)"
    - "release.yml is valid YAML and parses"
  artifacts:
    - path: ".github/workflows/release.yml"
      provides: "Unified release workflow: macOS DMG jobs + Linux .deb jobs + single release attach"
      contains: "build-deb"
  key_links:
    - from: ".github/workflows/release.yml (release job needs:)"
      to: "build-deb matrix jobs"
      via: "needs: [build-arm64, build-x86_64, build-deb]"
      pattern: "build-deb"
    - from: "release job gh release create/upload"
      to: ".deb artifacts downloaded into artifacts/"
      via: "download-artifact + gh release upload glob"
      pattern: "vector-tunnel-agent_.*\\.deb"
---

<objective>
Unify the Linux agent `.deb` build into the macOS `release.yml` workflow so a single `v*` tag (or published Release) produces BOTH the Universal `Vector-*-universal.dmg` and the two agent `.deb`s (amd64 + arm64) on ONE GitHub Release.

Purpose: Today two separate workflows (`release.yml` and `agent-release.yml`) fire on the same `v*` tag, racing to attach to the same Release. Folding the `.deb` build into `release.yml` makes one tag = one workflow = one complete Release, removing the race and the duplicate trigger.

Output: An extended `.github/workflows/release.yml` containing a matrixed `build-deb` job (ubuntu-22.04) plus DMG-and-deb upload in the `release` job; `agent-release.yml` deleted.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
</execution_context>

<context>
@./CLAUDE.md

<interfaces>
<!-- Exact artifact/path contracts the executor must preserve. Do NOT invent new names. -->

Existing release.yml (macOS) — current shape:
- Jobs: build-arm64 (macos-14) -> uploads artifact `vector-aarch64` (binary at target/aarch64-apple-darwin/release/vector-app)
- build-x86_64 (macos-15-intel) -> uploads artifact `vector-x86_64`
- release (macos-14, needs: [build-arm64, build-x86_64]) -> downloads both, builds Universal DMG via `cargo xtask dmg --universal`, output glob `target/dmg/Vector-*-universal.dmg`,
  then `gh release create|upload "$TAG" ... target/dmg/Vector-*-universal.dmg`.
- Tag resolved as: TAG="${{ github.event.release.tag_name || github.ref_name }}" (step id `tag`, output `tag`).
- Triggers: push tags ['v*'] AND release types [published].

agent-release.yml (Linux) — the jobs to fold in:
- build-deb matrix:
    - target: x86_64-unknown-linux-gnu  deb-arch: amd64
    - target: aarch64-unknown-linux-gnu deb-arch: arm64
  runs-on: ubuntu-22.04
  steps:
    1. checkout
    2. dtolnay/rust-toolchain@1.88.0 with targets: ${{ matrix.target }}
    3. Swatinem/rust-cache@v2 shared-key: agent-${{ matrix.deb-arch }}
    4. (aarch64 only) apt-get install gcc-aarch64-linux-gnu + append to ~/.cargo/config.toml:
         [target.aarch64-unknown-linux-gnu]
         linker = "aarch64-linux-gnu-gcc"
    5. cargo install --locked cargo-deb
    6. cargo build --release -p vector-tunnel-agent --target ${{ matrix.target }}
    7. resolve tag -> VER="${TAG#v}" (strip leading v for .deb version field)
    8. cargo deb -p vector-tunnel-agent --no-build --target ${{ matrix.target }} \
         --output target/${{ matrix.target }}/debian/vector-tunnel-agent_${VER}_${{ matrix.deb-arch }}.deb
    9. dpkg-deb sanity check: contents must contain 'usr/bin/vector-tunnel-agent'
   10. upload-artifact name agent-deb-${{ matrix.deb-arch }}, path target/${{ matrix.target }}/debian/*.deb, retention-days: 90

cargo-deb metadata (crates/vector-tunnel-agent/Cargo.toml [package.metadata.deb]):
- name = "vector-tunnel-agent", assets place binary at usr/bin/vector-tunnel-agent (755)
- maintainer-scripts = "debian/" (postinst + prerm exist) — do NOT remove these
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Fold build-deb jobs into release.yml and attach .debs in the release job</name>
  <files>.github/workflows/release.yml</files>
  <action>
Edit `.github/workflows/release.yml` (do NOT change triggers, concurrency, or the existing build-arm64/build-x86_64/release jobs except where noted):

1. Add a NEW job `build-deb` (place it after `build-x86_64`, before `release`), copied verbatim from `agent-release.yml`'s `build-deb` job — same matrix (x86_64-unknown-linux-gnu/amd64, aarch64-unknown-linux-gnu/arm64), runs-on ubuntu-22.04, fail-fast: false, and ALL steps:
   - checkout@v4
   - dtolnay/rust-toolchain@1.88.0 (targets: ${{ matrix.target }})
   - Swatinem/rust-cache@v2 (shared-key: agent-${{ matrix.deb-arch }})
   - aarch64-only cross toolchain step (apt gcc-aarch64-linux-gnu + ~/.cargo/config.toml linker block) — copy EXACTLY, the heredoc `<< 'EOF'` must be preserved
   - cargo install --locked cargo-deb
   - cargo build --release -p vector-tunnel-agent --target ${{ matrix.target }}
   - tag resolve step (id: tag) emitting `tag` and `ver` where VER="${TAG#v}" (strip leading v)
   - cargo deb --no-build --target ... --output target/${{ matrix.target }}/debian/vector-tunnel-agent_${{ steps.tag.outputs.ver }}_${{ matrix.deb-arch }}.deb
   - dpkg-deb sanity check (info + contents, grep 'usr/bin/vector-tunnel-agent')
   - upload-artifact: name agent-deb-${{ matrix.deb-arch }}, path target/${{ matrix.target }}/debian/*.deb, retention-days: 90  (KEEP the 90-day retention — this preserves the "CI staging" value of agent-release.yml, which is the only non-redundant thing it did)

2. The repo-level `release.yml` has NO top-level `permissions:` block (the `release` job sets `permissions: contents: write` locally). The new `build-deb` job needs NO write permission (it only uploads workflow artifacts) — do not add one.

3. In the `release` job:
   a. Add `build-deb` to `needs:` -> `needs: [build-arm64, build-x86_64, build-deb]`.
   b. After the two existing `download-artifact` steps (vector-aarch64, vector-x86_64), add a third download step for the .debs:
        - uses: actions/download-artifact@v4
          with:
            pattern: agent-deb-*
            path: artifacts/deb
            merge-multiple: true
      (merge-multiple flattens both agent-deb-amd64 and agent-deb-arm64 into artifacts/deb/*.deb)
   c. Add a guard step BEFORE the publish step (mirrors the existing Pitfall-3 guards) that asserts both .debs landed:
        - name: Verify agent .debs present
          run: |
            set -e
            ls artifacts/deb/vector-tunnel-agent_*_amd64.deb
            ls artifacts/deb/vector-tunnel-agent_*_arm64.deb
   d. In the "Publish or update GitHub Release" step, add the .deb glob to BOTH the `gh release upload` (the `gh release view` branch) and the `gh release create` (the else branch) commands, alongside the existing DMG glob. Concretely:
      - upload branch: `gh release upload "$TAG" target/dmg/Vector-*-universal.dmg artifacts/deb/vector-tunnel-agent_*.deb --clobber`
      - create branch: append `artifacts/deb/vector-tunnel-agent_*.deb` as an additional positional asset arg after `target/dmg/Vector-*-universal.dmg`.

Do not alter the DMG build steps, release-notes generation, or the install footer.
  </action>
  <verify>
    <automated>cd /Users/ashutosh/personal/vector && python3 -c "import yaml,sys; d=yaml.safe_load(open('.github/workflows/release.yml')); j=d['jobs']; assert 'build-deb' in j, 'build-deb job missing'; assert j['release']['needs']==['build-arm64','build-x86_64','build-deb'], j['release']['needs']; assert any('aarch64-unknown-linux-gnu' in str(i) for i in j['build-deb']['strategy']['matrix']['include']); print('OK release.yml parses, build-deb present, needs wired')"</automated>
  </verify>
  <done>release.yml parses as valid YAML; contains a `build-deb` matrix job (amd64+arm64 on ubuntu-22.04) with the cargo-deb invocation and 90-day artifact upload; the `release` job lists `build-deb` in `needs:`, downloads `agent-deb-*` artifacts into artifacts/deb, guards their presence, and includes `artifacts/deb/vector-tunnel-agent_*.deb` in both the gh release upload and create commands next to `target/dmg/Vector-*-universal.dmg`.</done>
</task>

<task type="auto">
  <name>Task 2: Retire agent-release.yml and validate artifact globs end-to-end</name>
  <files>.github/workflows/agent-release.yml</files>
  <action>
Decision (state in commit body): `agent-release.yml` triggers ONLY on `push tags ['v*']` and `release types [published]` — the exact same triggers as `release.yml`. After Task 1, every artifact it produced (both `.deb`s, the 90-day CI-staging artifacts, and the release attachment) is produced by `release.yml`. It has NO independent trigger (no schedule, no workflow_dispatch, no PR/CI-staging trigger). Therefore it is fully redundant -> DELETE it.

Steps:
1. `git rm .github/workflows/agent-release.yml`.
2. Validate the artifact globs in the new release.yml match what the tools actually emit (no actionlint available locally — use the YAML + glob-consistency checks below). Confirm:
   - The cargo-deb `--output` path pattern `vector-tunnel-agent_<ver>_<deb-arch>.deb` matches the upload-artifact path `target/<target>/debian/*.deb` and the download/upload glob `vector-tunnel-agent_*.deb`.
   - The DMG glob `target/dmg/Vector-*-universal.dmg` (unchanged) still appears in the publish step.
   - The cargo-deb assets in crates/vector-tunnel-agent/Cargo.toml put the binary at `usr/bin/vector-tunnel-agent`, matching the dpkg-deb grep in the sanity step.

Do not modify Cargo.toml or the debian/ scripts.
  </action>
  <verify>
    <automated>cd /Users/ashutosh/personal/vector && test ! -f .github/workflows/agent-release.yml && python3 -c "import yaml,re; d=yaml.safe_load(open('.github/workflows/release.yml')); s=open('.github/workflows/release.yml').read(); assert 'Vector-*-universal.dmg' in s; assert re.search(r'vector-tunnel-agent_.*\.deb', s); assert 'usr/bin/vector-tunnel-agent' in s; cargo=open('crates/vector-tunnel-agent/Cargo.toml').read(); assert 'usr/bin/vector-tunnel-agent' in cargo; print('OK agent-release.yml removed; deb+dmg globs and deb asset path consistent')"</automated>
  </verify>
  <done>`agent-release.yml` no longer exists; `release.yml` retains the `Vector-*-universal.dmg` glob and adds `vector-tunnel-agent_*.deb`; the `.deb` output/upload globs are mutually consistent and the `usr/bin/vector-tunnel-agent` path matches both the dpkg-deb sanity grep and the cargo-deb asset map; commit body explains the retirement rationale.</done>
</task>

</tasks>

<verification>
- `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"` succeeds (valid YAML).
- `build-deb` job exists with both `amd64` and `arm64` matrix entries on ubuntu-22.04.
- `release` job `needs:` == `[build-arm64, build-x86_64, build-deb]`.
- Publish step references BOTH `target/dmg/Vector-*-universal.dmg` and `artifacts/deb/vector-tunnel-agent_*.deb`.
- `.github/workflows/agent-release.yml` is deleted.
- cargo-deb `--output` glob, upload-artifact path, download/upload glob, and the `usr/bin/vector-tunnel-agent` asset path are all mutually consistent.
</verification>

<success_criteria>
A single `v*` tag (or published Release) runs `release.yml` only, which builds the Universal DMG (macOS) and both agent `.deb`s (Linux), then attaches `Vector-*-universal.dmg` + `vector-tunnel-agent_*_amd64.deb` + `vector-tunnel-agent_*_arm64.deb` to one GitHub Release. `agent-release.yml` is gone. No push (per CLAUDE.md). Each task committed separately on master.
</success_criteria>

<output>
After completion, commit each task separately on `master` (do NOT push). Then report the unified workflow and the agent-release.yml retirement decision.
</output>
