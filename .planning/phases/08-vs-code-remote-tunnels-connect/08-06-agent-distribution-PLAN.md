---
phase: 08-vs-code-remote-tunnels-connect
plan: 06
type: execute
wave: 3
depends_on: [03]
files_modified:
  - .github/workflows/agent-release.yml
  - crates/vector-tunnel-agent/Cargo.toml
  - crates/vector-tunnel-agent/debian/postinst
  - crates/vector-tunnel-agent/debian/prerm
  - crates/vector-tunnel-agent/README.md
  - xtask/src/agent_dist.rs
  - xtask/src/main.rs
autonomous: false
requirements:
  - DT-01
user_setup:
  - service: linux-cross-compile-toolchain
    why: "CI must produce x86_64 and aarch64 Debian .deb binaries; local dev machine (Mac) cannot ship .deb without cross toolchain"
    env_vars: []
    dashboard_config:
      - task: "Verify GitHub Actions ubuntu-22.04 runners are available (free tier covers this); confirm gh CLI is logged in for the local `cargo xtask agent-dist` smoke test"
        location: "https://github.com/colligo/vector/settings/actions"
must_haves:
  truths:
    - "Pushing a v* tag triggers a CI workflow that builds the vector-tunnel-agent binary for Linux x86_64 + aarch64 and produces a .deb package per architecture"
    - "Both .deb files attach to the GitHub Release alongside Vector.dmg"
    - "Running `cargo xtask agent-dist` locally builds at least one .deb artifact end-to-end (for the host arch when on Linux; skipped with clear message on macOS)"
    - "README.md at the repo root documents the agent install path: `wget {release.url}/vector-tunnel-agent_X.Y.Z_amd64.deb && sudo apt install ./vector-tunnel-agent_X.Y.Z_amd64.deb && vector-tunnel-agent`"
  artifacts:
    - path: ".github/workflows/agent-release.yml"
      provides: "CI workflow that triggers on v* tags, cross-compiles for linux/x86_64 + linux/aarch64, runs cargo-deb, attaches both .deb to the release"
      min_lines: 60
    - path: "xtask/src/agent_dist.rs"
      provides: "cargo xtask agent-dist subcommand: builds host-arch .deb locally"
    - path: "crates/vector-tunnel-agent/README.md"
      provides: "install + first-run instructions; manual smoke matrix is the source of UAT truth"
  key_links:
    - from: ".github/workflows/agent-release.yml"
      to: "cargo-deb"
      via: "cargo install cargo-deb + cargo deb --target x86_64-unknown-linux-gnu"
      pattern: "cargo deb"
    - from: ".github/workflows/agent-release.yml"
      to: "gh release upload"
      via: "after .deb produced, attach to the release that triggered the workflow"
      pattern: "gh release upload"
---

<objective>
Distribute `vector-tunnel-agent` on Linux x86_64 and aarch64 as Debian/Ubuntu `.deb` packages (D-01). Attach both to GitHub Releases on `v*` tag pushes. Document the install path for the user.

Purpose: closes the "open the app → install agent → connect" loop. Without distribution, the agent can only be built from source.
Output: a new CI workflow `agent-release.yml`, an `xtask agent-dist` local entry point, and `crates/vector-tunnel-agent/debian/` packaging metadata. Manual UAT smoke matrix is the gate (covered by Plan 08-07).
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/phases/08-vs-code-remote-tunnels-connect/08-CONTEXT.md
@.planning/phases/08-vs-code-remote-tunnels-connect/08-RESEARCH.md
@.github/workflows/ci.yml
@.github/workflows/release.yml
@xtask/Cargo.toml
@xtask/src/main.rs
@README.md
@crates/vector-tunnel-agent/Cargo.toml

<interfaces>
Existing CI workflows (Phase 1):
- `.github/workflows/ci.yml` — PR DAG (7 jobs: lint, commitlint, test, deny, build-arm64, build-x86_64, package)
- `.github/workflows/release.yml` — triggers on v* tag push + release published; builds Universal Vector.dmg, attaches to release

Existing xtask (Phase 1):
- `xtask/src/main.rs` — subcommands: dmg, release, ...
- Lives in a separate workspace (xtask/Cargo.toml has empty [workspace] table per D-04)

cargo-deb 2.x is the standard Rust→.deb tool. Generates Debian package from `[package.metadata.deb]` in Cargo.toml.
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Cargo.toml metadata + xtask agent-dist subcommand + agent README</name>
  <files>crates/vector-tunnel-agent/Cargo.toml, crates/vector-tunnel-agent/debian/postinst, crates/vector-tunnel-agent/debian/prerm, crates/vector-tunnel-agent/README.md, xtask/src/agent_dist.rs, xtask/src/main.rs, xtask/Cargo.toml</files>
  <read_first>
    - crates/vector-tunnel-agent/Cargo.toml (Plan 08-01 + 08-03 — current dep list; add metadata.deb section)
    - xtask/src/main.rs (entire — must understand current subcommand dispatch before adding a new arm)
    - xtask/Cargo.toml (xtask is a separate workspace per D-04; confirm)
    - README.md (root — add agent install section without breaking existing Vector.dmg install copy)
  </read_first>
  <action>
    Step 1 — crates/vector-tunnel-agent/Cargo.toml: append `[package.metadata.deb]` section with these fields (verify with `cargo deb --help` semantics on cargo-deb 2.x):
    ```toml
    [package.metadata.deb]
    name = "vector-tunnel-agent"
    maintainer = "Vector contributors <noreply@github.com>"
    copyright = "2026, Vector contributors. MIT licensed."
    license-file = ["LICENSE", "0"]   # path relative to repo root + skip-N-leading-lines
    extended-description = """
    Vector Tunnel Agent. Hosts a Microsoft Dev Tunnel and serves PTY shells
    to the Vector terminal app on macOS. Sign in once with GitHub or
    Microsoft; the agent then accepts incoming relay connections from
    Vector for the duration it runs.
    """
    section = "net"
    priority = "optional"
    depends = "$auto"
    assets = [
        ["target/release/vector-tunnel-agent", "usr/bin/vector-tunnel-agent", "755"],
        ["crates/vector-tunnel-agent/README.md", "usr/share/doc/vector-tunnel-agent/README.md", "644"],
    ]
    conf-files = []
    maintainer-scripts = "crates/vector-tunnel-agent/debian/"
    ```

    Step 2 — crates/vector-tunnel-agent/debian/postinst (NEW): minimal post-install script (chmod 0755):
    ```sh
    #!/bin/sh
    set -e
    # Nothing to do — agent runs as the invoking user, no system service in v1 (D-02).
    echo "vector-tunnel-agent installed. Run it with: vector-tunnel-agent"
    echo "On first run, complete the device-code sign-in printed to stdout."
    exit 0
    ```

    Step 3 — crates/vector-tunnel-agent/debian/prerm (NEW): minimal pre-remove script (chmod 0755):
    ```sh
    #!/bin/sh
    set -e
    # No service to stop in v1. If a user has the agent running, they killed it themselves.
    exit 0
    ```

    Both scripts must be `chmod 0755`. Cargo-deb honors maintainer-scripts directory.

    Step 4 — crates/vector-tunnel-agent/README.md (NEW): user-facing install + first-run docs. Sections:
    - **Install (Debian/Ubuntu, amd64):** `wget https://github.com/colligo/vector/releases/latest/download/vector-tunnel-agent_$VERSION_amd64.deb && sudo apt install ./vector-tunnel-agent_$VERSION_amd64.deb`
    - **Install (Debian/Ubuntu, arm64):** same with `_arm64.deb`
    - **Run:** `vector-tunnel-agent` — prints device-code URL + code to stdout; complete in any browser.
    - **Re-auth:** `vector-tunnel-agent --reauth` (clears stored token, re-runs device flow).
    - **Status:** `vector-tunnel-agent --status`.
    - **Persistence:** "v1 is manual-run only (D-02). To survive SSH disconnect, run under `tmux` or `nohup`: `tmux new -d -s vector-agent vector-tunnel-agent`."
    - **From Vector on macOS:** "After the agent is registered, open Vector → Cmd-Shift-T → pick your machine."
    - **Out of scope (v1.x):** systemd auto-start, rpm/yum, snap/flatpak, Windows host.

    No marketing voice. No emoji. Terse, verb-first per UI-SPEC §Copywriting Contract.

    Step 5 — xtask/Cargo.toml: confirm xtask deps include something equivalent to `cargo-deb` invocation capability. Since xtask invokes `cargo deb` as a subprocess (NOT as a library), no new Cargo dep is needed. If cargo-deb is not installed locally, the xtask subcommand prints `error: cargo-deb not found — run \`cargo install cargo-deb\``.

    Step 6 — xtask/src/agent_dist.rs (NEW):
    ```rust
    use anyhow::{bail, Context, Result};
    use std::path::PathBuf;

    /// `cargo xtask agent-dist` — builds the vector-tunnel-agent .deb for the host architecture.
    /// On macOS, this is a smoke check only (cross-compile to Linux is not supported locally).
    pub fn run() -> Result<()> {
        if cfg!(not(target_os = "linux")) {
            eprintln!("agent-dist: cross-compile to Linux is not supported locally.");
            eprintln!("agent-dist: invoke `.github/workflows/agent-release.yml` via a v* tag push.");
            return Ok(());   // not an error — just a no-op for the user's dev machine
        }

        // 1. Verify cargo-deb is installed.
        let status = std::process::Command::new("cargo").args(["deb", "--version"]).status();
        if status.map(|s| !s.success()).unwrap_or(true) {
            bail!("cargo-deb not installed. Run `cargo install cargo-deb`.");
        }

        // 2. Build release binary.
        let st = std::process::Command::new("cargo")
            .args(["build", "--release", "-p", "vector-tunnel-agent"])
            .status().context("cargo build")?;
        if !st.success() { bail!("cargo build failed"); }

        // 3. Run cargo-deb. --no-build because we just built.
        let st = std::process::Command::new("cargo")
            .args(["deb", "-p", "vector-tunnel-agent", "--no-build"])
            .status().context("cargo deb")?;
        if !st.success() { bail!("cargo deb failed"); }

        // 4. Report path.
        let deb_dir = PathBuf::from("target/debian");
        eprintln!("agent-dist: .deb artifact(s) in {}", deb_dir.display());
        Ok(())
    }
    ```

    Step 7 — xtask/src/main.rs: add `agent-dist` subcommand arm to the existing dispatcher. Mirror the shape of the existing `dmg` / `release` arms.

    Step 8 — README.md (root): add a brief section "## Remote machines (Phase 8 Dev Tunnels)" linking to crates/vector-tunnel-agent/README.md. Keep it short — the agent README is the source of truth. Mirror Phase 1 BUILD-05 xattr-block formatting style.
  </action>
  <verify>
    <automated>cargo build -p vector-tunnel-agent --release 2>&amp;1 | tail -3 &amp;&amp; test -f crates/vector-tunnel-agent/debian/postinst &amp;&amp; test -f crates/vector-tunnel-agent/debian/prerm &amp;&amp; test -x crates/vector-tunnel-agent/debian/postinst &amp;&amp; test -x crates/vector-tunnel-agent/debian/prerm &amp;&amp; grep -q "package.metadata.deb" crates/vector-tunnel-agent/Cargo.toml &amp;&amp; grep -q "agent-dist" xtask/src/main.rs &amp;&amp; test -f crates/vector-tunnel-agent/README.md &amp;&amp; grep -q "vector-tunnel-agent" README.md</automated>
  </verify>
  <acceptance_criteria>
    - cargo build -p vector-tunnel-agent --release exit 0
    - test -f crates/vector-tunnel-agent/debian/postinst (and is executable: mode 0755)
    - test -f crates/vector-tunnel-agent/debian/prerm (and is executable: mode 0755)
    - grep -c "package.metadata.deb" crates/vector-tunnel-agent/Cargo.toml >= 1
    - grep -c "assets" crates/vector-tunnel-agent/Cargo.toml >= 1
    - grep -c "usr/bin/vector-tunnel-agent" crates/vector-tunnel-agent/Cargo.toml >= 1
    - grep -c "agent-dist\\|agent_dist" xtask/src/main.rs >= 1
    - test -f crates/vector-tunnel-agent/README.md (>= 30 lines)
    - grep -c "vector-tunnel-agent --reauth" crates/vector-tunnel-agent/README.md >= 1
    - grep -c "tmux new" crates/vector-tunnel-agent/README.md >= 1 (manual persistence per D-02)
    - grep -c "vector-tunnel-agent" README.md >= 1 (root README links to agent)
  </acceptance_criteria>
  <done>Cargo.toml has cargo-deb metadata, debian/ scripts ship, xtask exposes agent-dist, README documents the install path.</done>
</task>

<task type="auto">
  <name>Task 2: GitHub Actions agent-release.yml workflow</name>
  <files>.github/workflows/agent-release.yml</files>
  <read_first>
    - .github/workflows/release.yml (entire file — Phase 1 BUILD-04; mirror the trigger + concurrency + checkout patterns)
    - .github/workflows/ci.yml (entire file — Phase 1 BUILD-02; mirror cross-compile + matrix patterns)
    - crates/vector-tunnel-agent/Cargo.toml (Task 1 output — cargo-deb metadata to consume)
  </read_first>
  <action>
    Create .github/workflows/agent-release.yml. Goals: triggers on v* tag push (mirroring release.yml dual-trigger pattern); matrix-builds vector-tunnel-agent for linux/x86_64 + linux/aarch64; runs cargo-deb per architecture; attaches both .deb to the GitHub Release.

    Concrete contents (verbatim YAML):
    ```yaml
    name: agent-release
    on:
      push:
        tags: ['v*']
      release:
        types: [published]
    concurrency:
      group: agent-release-${{ github.ref }}
      cancel-in-progress: false
    permissions:
      contents: write   # release upload
    jobs:
      build-deb:
        name: Build .deb (${{ matrix.target }})
        runs-on: ubuntu-22.04
        strategy:
          matrix:
            include:
              - target: x86_64-unknown-linux-gnu
                deb-arch: amd64
              - target: aarch64-unknown-linux-gnu
                deb-arch: arm64
        steps:
          - uses: actions/checkout@v4

          - name: Install Rust 1.88
            uses: dtolnay/rust-toolchain@1.88.0
            with:
              targets: ${{ matrix.target }}

          - name: Install cross-compile toolchain (aarch64 only)
            if: matrix.target == 'aarch64-unknown-linux-gnu'
            run: |
              sudo apt-get update
              sudo apt-get install -y gcc-aarch64-linux-gnu
              mkdir -p ~/.cargo
              cat >> ~/.cargo/config.toml << EOF
              [target.aarch64-unknown-linux-gnu]
              linker = "aarch64-linux-gnu-gcc"
              EOF

          - name: Install cargo-deb
            run: cargo install --locked cargo-deb

          - name: Build release binary
            run: cargo build --release -p vector-tunnel-agent --target ${{ matrix.target }}

          - name: Build .deb
            run: |
              cargo deb -p vector-tunnel-agent \
                --no-build \
                --target ${{ matrix.target }} \
                --output target/${{ matrix.target }}/debian/vector-tunnel-agent_${{ github.ref_name }}_${{ matrix.deb-arch }}.deb

          - name: Sanity-check .deb
            run: |
              dpkg-deb --info target/${{ matrix.target }}/debian/*.deb
              dpkg-deb --contents target/${{ matrix.target }}/debian/*.deb | grep -q usr/bin/vector-tunnel-agent

          - name: Upload artifact (CI staging)
            uses: actions/upload-artifact@v4
            with:
              name: agent-deb-${{ matrix.deb-arch }}
              path: target/${{ matrix.target }}/debian/*.deb

      attach-to-release:
        name: Attach .deb to GitHub Release
        needs: build-deb
        runs-on: ubuntu-22.04
        steps:
          - uses: actions/checkout@v4

          - name: Download artifacts
            uses: actions/download-artifact@v4
            with:
              path: dist/

          - name: Determine tag
            id: tag
            run: |
              if [ "${{ github.event_name }}" = "push" ]; then
                echo "name=${GITHUB_REF#refs/tags/}" >> "$GITHUB_OUTPUT"
              else
                echo "name=${{ github.event.release.tag_name }}" >> "$GITHUB_OUTPUT"
              fi

          - name: Upload .deb to release
            env:
              GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
            run: |
              for deb in dist/agent-deb-*/vector-tunnel-agent_*.deb; do
                echo "Uploading $deb to release ${{ steps.tag.outputs.name }}"
                gh release upload "${{ steps.tag.outputs.name }}" "$deb" --clobber --repo "${{ github.repository }}"
              done
    ```

    Notes:
    - The version interpolation `${{ github.ref_name }}` produces `v2026.5.10`; the `.deb` filename per cargo-deb convention uses the unprefixed version from Cargo.toml. Use `--output` flag to control the filename explicitly per architecture (so artifacts don't collide on download-artifact's path layout).
    - On `release: published` trigger (manual UI release), the workflow re-runs. The `--clobber` flag on `gh release upload` ensures idempotency.
    - The `aarch64-unknown-linux-gnu` cross-link configuration uses gcc-aarch64-linux-gnu (Debian package); standard pattern.
    - Don't add this to `ci.yml` PR DAG — agent builds are tagged-release only (matches BUILD-04 pattern).

    Critical: do NOT push tags. The user pushes asynchronously per CLAUDE.md. This workflow's existence is what gets committed; its first run happens on the user's next `git push --follow-tags`.
  </action>
  <verify>
    <automated>test -f .github/workflows/agent-release.yml &amp;&amp; grep -q "on:" .github/workflows/agent-release.yml &amp;&amp; grep -q "tags:.*'v\\*'" .github/workflows/agent-release.yml &amp;&amp; grep -q "x86_64-unknown-linux-gnu" .github/workflows/agent-release.yml &amp;&amp; grep -q "aarch64-unknown-linux-gnu" .github/workflows/agent-release.yml &amp;&amp; grep -q "cargo deb" .github/workflows/agent-release.yml &amp;&amp; grep -q "gh release upload" .github/workflows/agent-release.yml &amp;&amp; grep -q "dpkg-deb --contents" .github/workflows/agent-release.yml &amp;&amp; grep -q "concurrency:" .github/workflows/agent-release.yml</automated>
  </verify>
  <acceptance_criteria>
    - .github/workflows/agent-release.yml exists
    - grep -c "x86_64-unknown-linux-gnu\\|aarch64-unknown-linux-gnu" .github/workflows/agent-release.yml >= 2 (both targets)
    - grep -c "gcc-aarch64-linux-gnu" .github/workflows/agent-release.yml >= 1 (cross toolchain)
    - grep -c "cargo deb" .github/workflows/agent-release.yml >= 1
    - grep -c "gh release upload" .github/workflows/agent-release.yml >= 1
    - grep -c "dpkg-deb --contents.*vector-tunnel-agent" .github/workflows/agent-release.yml >= 1 (sanity-check installs binary at /usr/bin)
    - grep -c "concurrency:" .github/workflows/agent-release.yml >= 1
    - grep -c "release.*published\\|published" .github/workflows/agent-release.yml >= 1 (dual-trigger)
    - grep -c "GITHUB_TOKEN" .github/workflows/agent-release.yml >= 1
    - actionlint or yamllint (if available locally; otherwise skip) parses the file without errors
  </acceptance_criteria>
  <done>agent-release.yml ships; on v* tag push it cross-compiles both architectures, runs cargo-deb, sanity-checks .deb contents, attaches to release.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: Manual smoke — local cargo-deb build + dpkg-deb inspect</name>
  <what-built>
    - crates/vector-tunnel-agent/Cargo.toml with [package.metadata.deb] section
    - crates/vector-tunnel-agent/debian/postinst + prerm (mode 0755)
    - crates/vector-tunnel-agent/README.md install + first-run docs
    - xtask agent-dist subcommand
    - .github/workflows/agent-release.yml (will fire on next tag push by user)
  </what-built>
  <files>(verification only — no file writes)</files>
  <action>This is a checkpoint task. Claude pauses; the human runs the steps in <how-to-verify> and types the resume signal. No code changes in this task.</action>
  <how-to-verify>
    1) On a Linux dev box (or Ubuntu VM), or Docker container `docker run -it --rm -v $PWD:/work -w /work rust:1.88-bookworm`:
       - `cargo install cargo-deb`
       - `cargo build --release -p vector-tunnel-agent`
       - `cargo deb -p vector-tunnel-agent --no-build`
       - `dpkg-deb --info target/debian/vector-tunnel-agent_*.deb`     → should show metadata + maintainer + section: net
       - `dpkg-deb --contents target/debian/vector-tunnel-agent_*.deb` → should list /usr/bin/vector-tunnel-agent
       - `sudo apt install ./target/debian/vector-tunnel-agent_*.deb`  → should install cleanly, postinst prints "installed" message
       - `vector-tunnel-agent --version`                                → prints version, exits 0
       - `sudo apt remove vector-tunnel-agent`                          → uninstalls cleanly, prerm runs
    2) On Mac dev box: `cargo xtask agent-dist` should print "cross-compile to Linux is not supported locally" and exit 0.
    3) Inspect .github/workflows/agent-release.yml visually in the GitHub Actions tab if user pushes; verify the next `v*` tag push runs the workflow end-to-end.
  </how-to-verify>
  <verify>Manual — human executes the verification checklist above. No automated check.</verify>
  <done>Human types the resume signal with approval notes (or paste failure details).</done>
  <resume-signal>Type "approved" with brief notes on dpkg-deb output. If install fails on the test box, paste the error.</resume-signal>
</task>

</tasks>

<verification>
- cargo build -p vector-tunnel-agent --release exit 0 on host arch
- .github/workflows/agent-release.yml lints (manual visual inspection acceptable in absence of actionlint)
- Manual smoke matrix item 1 PASSES (dpkg-deb install/remove works end-to-end on a real Linux env)
</verification>

<success_criteria>
- vector-tunnel-agent ships as .deb for amd64 and arm64 on every v* tag
- Local `cargo xtask agent-dist` works on Linux dev boxes
- README documents the apt-install path verbatim
- Manual UAT confirms the .deb installs cleanly + binary runs --version
</success_criteria>

<output>
After completion, create .planning/phases/08-vs-code-remote-tunnels-connect/08-06-SUMMARY.md documenting:
- The exact cargo-deb version pinned (`cargo install --locked cargo-deb` resolves; record the version that landed)
- The .deb filename format produced (`vector-tunnel-agent_X.Y.Z_amd64.deb` vs cargo-deb default)
- Any aarch64 cross-compile gotchas (e.g., the linker config workaround above)
- Confirmation that release.yml + agent-release.yml don't race each other on the same tag (concurrency keys are distinct: `release-${{ github.ref }}` vs `agent-release-${{ github.ref }}`)
</output>
