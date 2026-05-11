# Setup — One-time manual configuration

Run these once on a fresh checkout.

## 1. Local development tools

```sh
# Xcode CLI (required for lipo, iconutil, hdiutil, codesign, file)
xcode-select --install

# Build tools used by xtask
brew install create-dmg librsvg git-cliff

# Cargo subcommands
cargo install cargo-bundle@0.10.0 --locked
cargo install cargo-deny --locked
```

## 2. Rust toolchain

`rust-toolchain.toml` pins channel `1.88.0` and both Apple Darwin targets.
`rustup` will auto-install on first `cargo build`. Verify:

```sh
rustup show
# active toolchain: 1.88.0-aarch64-apple-darwin (or x86_64)
```

## 3. GitHub branch protection (one-time per repo)

D-34 / D-35 require this. Configure in the GitHub UI at
`Settings → Branches → Branch protection rules → Add rule` for `main`:

- Require a pull request before merging
  - Required approving reviews: **0** (gate exists; solo dev, not blocking)
  - Dismiss stale pull request approvals when new commits are pushed
- Require status checks to pass before merging
  - Require branches to be up to date before merging
  - Required status checks (must match the job names in `.github/workflows/ci.yml`):
    - `lint`
    - `commitlint`
    - `test`
    - `deny`
    - `build-arm64`
    - `build-x86_64`
    - `package`
- Require linear history
- Do not allow force pushes
- Do not allow deletions (off — protect the branch)
- Do not require deployments to succeed (not used)
- Do not require signed commits (deferred to v2)

Verify via the API:

```sh
gh api repos/colligo/vector/branches/main/protection
```

Should report `required_status_checks.contexts` containing
`lint, commitlint, test, deny, build-arm64, build-x86_64, package`;
`required_linear_history: { enabled: true }`; `allow_force_pushes: { enabled: false }`.

## 4. Git hooks (cargo-husky auto-installs)

`cargo-husky` installs `.git/hooks/pre-commit` on first `cargo build`. The hook
runs `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings`. To
opt out (rare): set `CARGO_HUSKY_DONT_INSTALL_HOOKS=1` in your shell.

## 5. First build + DMG

```sh
cargo build --workspace
cargo test --workspace --tests
cargo deny check
cargo xtask dmg              # host-arch-only DMG, ~30s
cargo xtask dmg --universal  # Universal DMG, ~2 min cold
```

The output DMG is at `target/dmg/Vector-{version}-universal.dmg`.

## 6. First release (when ready to ship a tag)

```sh
cargo xtask release    # bumps CalVer, runs git-cliff, commits, tags v{date}
git push --follow-tags # triggers .github/workflows/release.yml
```

CI will publish the Universal DMG to GitHub Releases.
