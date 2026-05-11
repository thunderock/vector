# 0004. DMG packaging pipeline

- Status: accepted
- Date: 2026-05-10
- Deciders: solo (user)
- Tags: phase-1, build, dmg, build-02, build-03, build-04, build-05

## Context and Problem Statement

Vector ships unsigned via `.dmg` for v1 (signing deferred to v2). We need a
DMG produced identically by CI and by a local `cargo xtask dmg` invocation
(per D-22), in both Universal (CI / tagged release) and host-arch-only
(local dev) modes.

## Decision Drivers

- BUILD-03 — local DMG must equal CI DMG bytewise per the same code path
- BUILD-02 — Universal binary on every push to main
- BUILD-04 — tagged releases auto-publish; tip + tagged pattern (ghostty model)
- BUILD-05 — xattr instruction in three places (D-26)
- Don't hand-roll: cargo-bundle for `.app`, create-dmg for styled DMG

## Considered Options

- Hand-rolled `mkdir Vector.app/Contents/{MacOS,Resources}` + Info.plist
- `tauri-bundler` (rejected — Tauri-shaped, webview deps)
- `cargo-bundle 0.10` + `create-dmg` shell script (chosen)

## Decision Outcome

Per D-25, D-22, D-17, D-18, D-19, D-26. Pipeline:
1. CI matrix builds per-arch binaries (`macos-14` for arm64, `macos-15-intel`
   for x86_64; ADR 0006 covers the runner-label amendment).
2. `package` (CI) or local `cargo xtask dmg --universal` runs `lipo -create`
   to merge per-arch binaries; Pitfall-3 guards verify the result is fat.
3. cargo-bundle reads the merged binary at `target/release/vector-app` (Wave-0
   spike confirmed this works) and produces `Vector.app`.
4. `iconutil` generates `.icns` from `icon.svg` via `rsvg-convert`.
5. `create-dmg` wraps `Vector.app` into a styled DMG with the xattr
   instruction rasterized into the background.
6. Tip pushes overwrite a pinned `tip` GitHub Release; tagged pushes (`v*`)
   create a permanent Release with `git-cliff`-generated notes + xattr footer.

## Pros and Cons of the Options

- **Hand-rolled bundle:** maximal control; high maintenance; reinvents
  cargo-bundle's Info.plist generation.
- **tauri-bundler:** assumes Tauri app shape; pulls webview deps.
- **cargo-bundle + create-dmg (chosen):** standard Rust tooling for both
  layers; xtask is the thin glue.

## Consequences

- DMG byte-identity local↔CI: only true if the same xtask version, the same
  `cargo-bundle@0.10.0`, the same `librsvg` rsvg-convert version, and the
  same git SHA are used. We accept "byte-identical content; possibly different
  metadata timestamps" as the practical guarantee.
- The xattr line appears in: README install block, DMG background image, tip
  release body (Plan 01-05), tagged release body (this plan). Four places —
  one more than D-26's three for added discoverability; harmless redundancy.
- Wave-0 spike (Plan 01-04 SUMMARY) documented the cargo-bundle universal-binary
  path behavior — fallback path (cargo-bundle --bin + post-process) is on the
  bench if cargo-bundle ever regresses.

## Amendment 2026-05-11: Assumption A5 invalidated; fallback now permanent

First real CI run on `master @ 8e540ea` confirmed cargo-bundle 0.10 re-invokes
`cargo build --release` host-arch only, overwriting any pre-merged universal
binary staged at `target/release/vector-app` before bundling. The Pitfall-3
guard caught a non-fat Mach-O after `cargo bundle` ran. The documented fallback
is now the default code path, not a contingency: `xtask::dmg::finalize`
post-processes by copying `target/universal-apple-darwin/release/vector-app`
over `Vector.app/Contents/MacOS/vector-app` after `cargo bundle` completes.
Conditional on the merged file existing — `dmg_local` (host-arch only) is
unaffected. Validated by the `tip` release `Vector-2026.5.10-tip-8e540ea-universal.dmg`
running on the user's Sequoia box (window titled "Vector — tick N", proving
D-09/D-10/D-14 live).
