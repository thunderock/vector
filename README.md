# Vector

Fast native macOS terminal with first-class GitHub Codespaces and Dev Tunnels support.

## Install

1. Download the latest `Vector-{version}-universal.dmg` from [GitHub Releases](https://github.com/colligo/vector/releases/latest).
2. Open the DMG and drag `Vector.app` to `/Applications`.
3. The first time you launch, macOS will block the unsigned app. Run this once in Terminal:

```sh
xattr -dr com.apple.quarantine /Applications/Vector.app
```

4. Open Vector from `/Applications` (or Launchpad). You should see a window titled `Vector` with a small build identifier in the bottom-right corner.

## Status

Phase 1 (Foundation & CI/DMG Pipeline) — early bootstrap. Phases 2–10 fill in
the terminal core, GPU renderer, mux, polish, GitHub auth, Codespaces SSH,
Dev Tunnels, persistence, and hardening. See `.planning/ROADMAP.md`.

## Build from source

Requires Rust 1.88.0+ (auto-pinned via `rust-toolchain.toml`) and Xcode CLI tools.

```sh
brew install create-dmg librsvg git-cliff
cargo install cargo-bundle@0.10.0
cargo xtask dmg --universal     # produces target/dmg/Vector-{version}-universal.dmg
```

## License

MIT. See `LICENSE`.
