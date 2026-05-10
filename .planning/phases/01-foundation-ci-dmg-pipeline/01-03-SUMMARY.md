---
phase: 01-foundation-ci-dmg-pipeline
plan: 03
subsystem: ui
tags: [rust, winit, tokio, objc2-app-kit, appkit, threading, d-08, d-10, d-12, d-14, d-15, d-16, d-32]

requires:
  - "01-01-SUMMARY (14-crate workspace, vector-app stub, workspace deps pinned for winit 0.30.13, tokio 1.52.3, objc2 0.6.4, objc2-app-kit 0.3, objc2-foundation 0.3, raw-window-handle 0.6, tracing 0.1, tracing-subscriber 0.3, anyhow 1, thiserror 1)"
  - "01-02-SUMMARY (workspace lints with unsafe_code=deny + await_holding_lock=deny, per-crate tests/no_tokio_main.rs with BLOCK_ON_ALLOWLIST = [\"src/main.rs\"] for vector-app)"
provides:
  - "vector-app binary that opens a 1024×640 NSWindow titled `Vector`, ticking title `Vector — tick {n}` (U+2014) every 500ms via cross-thread EventLoopProxy::send_event"
  - "Triple-loop threading skeleton: winit EventLoop on main thread, dedicated `tokio-io` std::thread hosting a multi_thread Tokio runtime, EventLoopProxy<UserEvent> as the sole cross-thread signal"
  - "Native AppKit menu bar (Vector / File / Edit / View / Window / Help) wired via objc2-app-kit 0.3 — functional Cmd-Q / Cmd-M / Cmd-W / Cmd-Ctrl-F / Cmd-Opt-H, system-populated Services + Window + Help menus, disabled Cmd-N / Cmd-T + Edit group + Vector Help"
  - "NSTextField version overlay anchored bottom-right (autoresizingMask ViewMinXMargin | ViewMaxYMargin) reading `Vector v2026.05.10 (build {short-sha})` in 11pt monospaced system font, #9A9A9A on #2A2A2A 4px-rounded CALayer plate"
  - "build.rs emitting VECTOR_BUILD_SHA at compile time via `git rev-parse --short HEAD` (falls back to `unknown`), with rerun-if-changed on `.git/HEAD` and `.git/refs/heads`"
  - "Resource files for cargo-bundle/create-dmg pipeline: resources/icon.svg (1024² right-leaning 3-line chevron, #7B61FF on #1A1A1A plate, 22.4% corner radius), resources/Info.plist.partial (LSMinimumSystemVersion=13.0, NSHighResolutionCapable=true, CFBundleVersion=2026.05.10), resources/dmg-background.png (1280×800 placeholder, finalized in 01-04)"
  - "Architecture-lint test (Plan 01-02 BLOCK_ON_ALLOWLIST) now actually enforced — fixed allowlist entry from `src/main.rs` to `main.rs` because the scanner strips the `src/` prefix before matching; intent unchanged"
affects:
  - "01-04-PLAN (xtask DMG pipeline): consumes resources/icon.svg (→ .icns via iconutil), resources/Info.plist.partial (appended into the bundle's Info.plist), resources/dmg-background.png (DMG background). vector-app binary is the bundle payload."
  - "01-05-PLAN (CI): `cargo build -p vector-app --release` is the matrix-build target; tracing output on stdout is the smoke-test signal."
  - "Phase 3 (GPU renderer): inherits the EventLoop<UserEvent> + tokio-io thread pattern. UserEvent enum extends with renderer-driven variants. set_title-from-main-thread rule and the user_event handler are the load-bearing contract."
  - "Phase 9 (reconnect): inherits the EventLoopProxy::send_event-as-only-cross-thread-signal rule (Anti-Pattern 5 mitigation). Domain::reconnect() state transitions flow through the same proxy."

tech-stack:
  added:
    - "objc2-quartz-core 0.3 (workspace dep; features = [\"CALayer\", \"objc2-core-foundation\", \"objc2-core-graphics\"]) — needed for setCornerRadius / setBackgroundColor on the overlay's CALayer plate. Pulled in objc2-core-foundation + objc2-core-graphics transitively."
  patterns:
    - "Triple-loop threading: winit EventLoop owns the macOS main thread; a single dedicated std::thread named `tokio-io` spawns a multi_thread Tokio runtime (`tokio-worker` threads); `EventLoopProxy::send_event(UserEvent)` is the only cross-thread signal. `rt.block_on(io_main(proxy))` lives at exactly one call site (src/main.rs inside the io-thread closure)."
    - "AppKit-from-Rust via MainThreadMarker: every `unsafe` AppKit call site (`menu::install_main_menu`, `overlay::install`) is gated by `MainThreadMarker::new().expect(\"main thread\")` which panics off-thread, providing a runtime safety net beyond the file-scoped `#![allow(unsafe_code)]`."
    - "Compile-time SHA stamping without vergen: a 16-line build.rs running `git rev-parse --short HEAD` + `cargo:rerun-if-changed` directives. `env!(\"VECTOR_BUILD_SHA\")` flows into the overlay text and the startup tracing line."
    - "NSWindow + objc2-app-kit native menu wiring without re-implementing window-list / help-search / services-menu: `NSApplication::setWindowsMenu`, `setHelpMenu`, `setServicesMenu` hand the relevant submenus to AppKit which auto-populates them."

key-files:
  created:
    - "crates/vector-app/build.rs — compile-time VECTOR_BUILD_SHA emission via git rev-parse, with rerun directives on .git/HEAD and .git/refs/heads."
    - "crates/vector-app/src/main.rs — entry point: tracing init, EventLoop<UserEvent>::with_user_event, dedicated tokio-io thread, rt.block_on(tick::io_main(proxy)), run_app. File-scoped #![allow(unsafe_code)]."
    - "crates/vector-app/src/tick.rs — async io_main with 500ms tokio::time::interval emitting UserEvent::Tick(n); exits cleanly when proxy.send_event returns Err (event loop closed)."
    - "crates/vector-app/src/app.rs — ApplicationHandler<UserEvent> impl. resumed creates 1024×640 NSWindow, installs menu + overlay. user_event mutates window title to `Vector — tick {n}` (U+2014). window_event handles CloseRequested → exit and Resized → overlay.relayout (no-op; autoresizingMask handles layout)."
    - "crates/vector-app/src/menu.rs — Native NSMenu install. Six submenu factories (app/file/edit/view/window/help) + five helpers (add, add_disabled, add_with_modifiers, add_disabled_with_modifiers, add_services). Wires setMainMenu, setWindowsMenu, setHelpMenu, setServicesMenu."
    - "crates/vector-app/src/overlay.rs — NSTextField version banner. 11pt NSFont::monospacedSystemFontOfSize_weight, #9A9A9A text on #2A2A2A 4px-rounded CALayer plate, AutoresizingMask ViewMinXMargin | ViewMaxYMargin anchors bottom-right. Pulls SHA from env!(\"VECTOR_BUILD_SHA\")."
    - "crates/vector-app/resources/icon.svg — 1024×1024 right-leaning chevron of 3 motion lines, #7B61FF stroke at width 64 on #1A1A1A rounded plate (229/1024 ≈ 22.4% corner radius). Three polylines with descending opacity (1.0, 0.75, 0.5)."
    - "crates/vector-app/resources/Info.plist.partial — LSMinimumSystemVersion=13.0, NSHighResolutionCapable=true, CFBundleVersion=2026.05.10, CFBundleShortVersionString=2026.05.10."
    - "crates/vector-app/resources/dmg-background.png — 1280×800 placeholder (4165 bytes). Finalized in Plan 01-04 by xtask rasterization."
  modified:
    - "Cargo.toml — added workspace dep `objc2-quartz-core = { version = \"0.3\", features = [\"CALayer\", \"objc2-core-foundation\", \"objc2-core-graphics\"] }`."
    - "Cargo.lock — regenerated to pull in winit 0.30.13, tokio 1.52.3, objc2 0.6.4 + objc2-app-kit + objc2-foundation + objc2-quartz-core, raw-window-handle 0.6, tracing-subscriber 0.3, and their transitives."
    - "crates/vector-app/Cargo.toml — appended 10 workspace deps under [dependencies]: anyhow, thiserror, tracing, tracing-subscriber, tokio, winit, objc2, objc2-app-kit, objc2-foundation, raw-window-handle, plus objc2-quartz-core for the overlay CALayer."
    - "crates/vector-app/src/lib.rs — trimmed to a single doc-comment (binary uses module declarations in main.rs; lib.rs is intentionally empty)."
    - "crates/vector-app/tests/no_tokio_main.rs — BLOCK_ON_ALLOWLIST entry fixed from `src/main.rs` → `main.rs` so the scanner (which strips the `src/` prefix before matching) actually allowlists the intended file. The architectural intent — block_on permitted only in main.rs — is unchanged and is now enforced for the first time."

key-decisions:
  - "Keep lib.rs empty (single doc-comment) rather than re-exporting modules. Rationale: the binary declares modules inline in main.rs; a lib.rs that re-exports them would either duplicate the module tree or force main.rs to import from `crate::*` instead of declaring modules. The 01-03-PLAN <interfaces> block explicitly allowed either pattern; the simpler choice is the empty lib.rs."
  - "Add objc2-quartz-core 0.3 (workspace dep) for CALayer access. Rationale: NSTextField alone cannot render the rounded #2A2A2A background plate the UI-SPEC specifies; setWantsLayer + layer.setCornerRadius + layer.setBackgroundColor is the standard AppKit pattern and requires a typed CALayer binding. objc2-quartz-core is the canonical objc2-family crate for this and is already in the dep graph through objc2-app-kit transitively — making it a direct dep just lifts a `cargo update` ceiling."
  - "Use `#[allow(clippy::unused_self)]` on `Overlay::relayout` rather than removing the method or making it a free function. Rationale: autoresizingMask handles the resize for us today, so the method body is a no-op, but the WindowEvent::Resized handler in app.rs calls `overlay.relayout()` as a future-proof seam — when Phase 3 introduces CALayer-backed compositing that doesn't autoresize, the relayout method already exists at the call site. Removing it would force a future migration to find every Resized handler; keeping it costs one allow attribute."
  - "Fix BLOCK_ON_ALLOWLIST in-place during Task 2 rather than deferring to a follow-up plan. Rationale: the no-op allowlist was discovered as the architecture-lint test was being verified against the real main.rs; the fix is one line (`\"src/main.rs\"` → `\"main.rs\"`) and committed in the same commit as the threading skeleton (25c59e3) so the lint actually enforces what it was always supposed to enforce. Without this fix the test would have continued to pass for the wrong reason (the substring scan was checking for `src/main.rs` against a path string that the scanner had already stripped to `main.rs`)."

patterns-established:
  - "Pattern 1 (triple-loop threading): main thread = winit::EventLoop with custom UserEvent; one std::thread named `tokio-io` hosts a `Builder::new_multi_thread().enable_all().thread_name(\"tokio-worker\").build()` runtime; `EventLoopProxy::send_event(UserEvent)` is the only cross-thread signal. Every later phase that needs a new I/O task adds a new `UserEvent` variant + a `user_event` arm, never a new runtime."
  - "Pattern 2 (compile-time git SHA without vergen): a 16-line build.rs reading `git rev-parse --short HEAD` + rerun directives. `env!(\"VECTOR_BUILD_SHA\")` flows into both the overlay text and the startup tracing line. Falls back to `unknown` on any error (no .git, git not in PATH, dirty filesystem)."
  - "Pattern 3 (AppKit-from-Rust safety): every unsafe AppKit call site is gated by `MainThreadMarker::new().expect(\"main thread\")`. Runtime panic if called off-thread; no #[allow(unsafe_code)] proliferation beyond the file-scoped attribute in main.rs."
  - "Pattern 4 (native menu wiring without re-implementation): `NSApplication::setWindowsMenu` / `setHelpMenu` / `setServicesMenu` hand the relevant submenus to AppKit. No custom window-list management, no custom Help-search integration, no custom Services menu — AppKit owns all three."

requirements-completed: [WIN-05, BUILD-01]

duration: ~5min30s
completed: 2026-05-10
---

# Phase 01 Plan 03: Threading Skeleton + AppKit Window + Menu + Overlay Summary

**Triple-loop threading skeleton (winit main + tokio-io thread + EventLoopProxy<UserEvent>) wired to a native 1024×640 NSWindow with the standard six-menu AppKit menu bar, an NSTextField version overlay anchored bottom-right, and a build.rs that stamps `git rev-parse --short HEAD` into VECTOR_BUILD_SHA — D-08 / D-10 / D-12 / D-14 / D-15 / D-16 / D-32 all locked.**

## Performance

- **Duration:** ~5 min 30 s (across Task 1 + Task 2 commits)
- **Started:** 2026-05-10T23:02:17Z (Task 1 commit)
- **Completed:** 2026-05-10T23:07:54Z (Task 2 commit)
- **Tasks:** 2 implementation tasks + 1 human-verify checkpoint (approved)
- **Files created:** 9 (build.rs, app.rs, menu.rs, overlay.rs, tick.rs, icon.svg, Info.plist.partial, dmg-background.png; main.rs effectively rewritten from the 01-01 placeholder)
- **Files modified:** 5 (Cargo.toml workspace + crates/vector-app/Cargo.toml + Cargo.lock + crates/vector-app/src/lib.rs + crates/vector-app/tests/no_tokio_main.rs)

## Accomplishments

- `cargo run -p vector-app` opens a 1024×640 NSWindow titled `Vector`; within 1 second the title cycles `Vector — tick 1`, `Vector — tick 2`, … at 500 ms cadence with a U+2014 em-dash (UI-SPEC §Threading-visible surface).
- Native AppKit menu bar (Vector / File / Edit / View / Window / Help) renders with the full UI-SPEC item set: About Vector + Hide / Hide Others / Show All / Quit Vector (functional); Preferences… (disabled); File → Close (Cmd-W functional, New Window/Tab disabled); Edit group (all disabled in Phase 1); View → Enter Full Screen (Cmd-Ctrl-F functional); Window → Minimize / Zoom / Bring All to Front; Help → Vector Help (disabled). Services / Window list / Help search auto-populated by AppKit via `setServicesMenu` / `setWindowsMenu` / `setHelpMenu`.
- Bottom-right NSTextField overlay reads `Vector v2026.05.10 (build {short-sha})` in 11 pt monospaced system font, #9A9A9A on a #2A2A2A 4 px-rounded CALayer plate; anchored via autoresizingMask `ViewMinXMargin | ViewMaxYMargin` so it stays glued to the bottom-right on resize.
- `crates/vector-app/build.rs` emits `cargo:rustc-env=VECTOR_BUILD_SHA={short-sha}` at compile time via `git rev-parse --short HEAD`, with rerun directives on `.git/HEAD`, `.git/refs/heads`, and the `VECTOR_BUILD_SHA_OVERRIDE` env var. Falls back to `unknown` on any error (no vergen dep).
- `tracing::info!(version = "2026.05.10", sha = "<short-sha>", "vector starting")` fires on stdout before the I/O thread spawns (D-32: subscriber initialized first).
- The architecture-lint test from Plan 01-02 (`crates/vector-app/tests/no_tokio_main.rs`) now actually enforces its intent: the allowlist was a no-op pre-fix (`src/main.rs` never matched because the scanner strips the `src/` prefix); the corrected entry (`main.rs`) permits exactly the one `block_on` call in main.rs and forbids the pattern everywhere else in `src/`. `cargo test --workspace --tests` exits 0 with all 14 `forbidden_tokio_patterns_absent_from_src ... ok` lines.
- `cargo clippy -p vector-app --all-targets -- -D warnings` exits 0 with workspace pedantic lints in force (D-06 + D-11). `cargo fmt --all -- --check` exits 0. `cargo build -p vector-app --release` exits 0.
- User approved the checkpoint:human-verify task after running the binary on macOS and confirming the visual + functional contract (ticking title, version overlay, menu bar item-by-item, Cmd-Q / M / W / Ctrl-F functional, Cmd-N / T disabled, resize anchors overlay correctly).

## Task Commits

1. **Task 1: wire vector-app deps + build.rs + Info.plist.partial + icon.svg + dmg-background.png** — `a0d9027` (feat)
2. **Task 2: threading skeleton + AppKit window + menu + overlay (+ no_tokio_main.rs allowlist fix)** — `25c59e3` (feat)
3. **Checkpoint 3: human-verify** — resolved by user reply "approved" (no commit).

## Files Created/Modified

### Created (9)

- `crates/vector-app/build.rs` — 16-line build script: `git rev-parse --short HEAD` → `cargo:rustc-env=VECTOR_BUILD_SHA={sha}` + three rerun directives.
- `crates/vector-app/src/app.rs` — `App` struct + `ApplicationHandler<UserEvent>` impl. `resumed` creates the window and installs menu + overlay; `user_event` mutates window title on `Tick(n)`; `window_event` handles `CloseRequested` → `event_loop.exit()` and `Resized` → `overlay.relayout()`.
- `crates/vector-app/src/menu.rs` — full native menu install: 6 submenu factories + 5 helper functions + setMainMenu / setWindowsMenu / setHelpMenu / setServicesMenu wiring.
- `crates/vector-app/src/overlay.rs` — `Overlay::install(window: &dyn Window) -> Overlay` + `Overlay::relayout(&mut self)`. NSTextField via objc2-app-kit, CALayer via objc2-quartz-core, AutoresizingMask via objc2-app-kit's NSAutoresizingMaskOptions.
- `crates/vector-app/src/tick.rs` — 19-line async `io_main(proxy)` task: 500 ms tokio::time::interval, saturating_add tick counter, `proxy.send_event(UserEvent::Tick(n))`, exit on Err.
- `crates/vector-app/resources/icon.svg` — 1024² SVG, 3-line chevron at #7B61FF stroke on #1A1A1A 22.4 %-rounded plate.
- `crates/vector-app/resources/Info.plist.partial` — 8-line XML stanza for cargo-bundle's Info.plist merge.
- `crates/vector-app/resources/dmg-background.png` — 1280×800 placeholder PNG (4165 bytes).
- (`crates/vector-app/src/main.rs` was effectively rewritten from the 01-01 placeholder — listed under Modified.)

### Modified (5)

- `Cargo.toml` — appended workspace dep `objc2-quartz-core = { version = "0.3", features = ["CALayer", "objc2-core-foundation", "objc2-core-graphics"] }`.
- `crates/vector-app/Cargo.toml` — added 11 workspace `*.workspace = true` deps under `[dependencies]` (the 10 deps the plan specified + objc2-quartz-core).
- `crates/vector-app/src/main.rs` — replaced 01-01 placeholder with the triple-loop entry point: `#![allow(unsafe_code)]`, fmt+EnvFilter tracing init, `UserEvent` enum, `EventLoop::<UserEvent>::with_user_event().build()`, `EventLoopProxy` created and moved into the `tokio-io` `thread::Builder::spawn` closure where `rt.block_on(tick::io_main(proxy))` runs, then `event_loop.run_app(&mut App::new())` on the main thread.
- `crates/vector-app/src/lib.rs` — replaced 01-01 placeholder content with a single doc-comment.
- `crates/vector-app/tests/no_tokio_main.rs` — fixed BLOCK_ON_ALLOWLIST entry from `"src/main.rs"` to `"main.rs"` so the existing scanner logic actually allowlists the intended file. Architectural intent unchanged.
- `Cargo.lock` — regenerated to lock the resolved versions of winit 0.30.13, tokio 1.52.3, objc2 0.6.4, objc2-app-kit 0.3.x, objc2-foundation 0.3.x, objc2-quartz-core 0.3.x, raw-window-handle 0.6.x, tracing-subscriber 0.3.x, and their transitives.

## Decisions Made

See `key-decisions` in the frontmatter for the four substantive decisions. Brief summary:

1. lib.rs kept empty (single doc-comment) — binary declares modules in main.rs; lib.rs would be redundant.
2. objc2-quartz-core 0.3 added as a workspace dep — needed for typed CALayer access on the overlay's rounded background plate. Already in the dep graph transitively; promoting it to a direct dep just lifts a `cargo update` ceiling.
3. `#[allow(clippy::unused_self)]` on `Overlay::relayout` — autoresizingMask handles the resize today; keeping the method as a future-proof seam costs one allow attribute and saves a future migration when Phase 3's CALayer-backed compositing replaces autoresizing.
4. Fix `BLOCK_ON_ALLOWLIST` in-place during Task 2 — the no-op allowlist was discovered while verifying the architecture-lint test against the real main.rs; one-line fix landed in the same commit so the lint actually enforces what it always intended to enforce.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 — Missing Critical] Added `objc2-quartz-core 0.3` to the workspace**
- **Found during:** Task 2 (overlay implementation)
- **Issue:** The plan specified the UI-SPEC overlay requirements (4 px corner radius, #2A2A2A background plate) but did not list `objc2-quartz-core` in the workspace deps. `NSTextField` alone cannot render a rounded background plate; the standard AppKit pattern is `setWantsLayer(true)` + `layer.setCornerRadius(4.0)` + `layer.setBackgroundColor(...)`, which requires typed `CALayer` bindings.
- **Fix:** Added `objc2-quartz-core = { version = "0.3", features = ["CALayer", "objc2-core-foundation", "objc2-core-graphics"] }` to the workspace `[workspace.dependencies]`, and `objc2-quartz-core.workspace = true` to `crates/vector-app/Cargo.toml`.
- **Files modified:** `Cargo.toml`, `crates/vector-app/Cargo.toml`, `Cargo.lock`.
- **Verification:** `cargo build -p vector-app --release` exits 0; the overlay renders with the rounded #2A2A2A plate per UI-SPEC §Version overlay placement; the user-approved checkpoint confirms this visually.
- **Committed in:** `25c59e3` (Task 2 commit).

**2. [Rule 1 — Bug] Fixed BLOCK_ON_ALLOWLIST entry in `crates/vector-app/tests/no_tokio_main.rs`**
- **Found during:** Task 2 (verifying the architecture-lint test against the real main.rs)
- **Issue:** Plan 01-02 set `BLOCK_ON_ALLOWLIST = &["src/main.rs"]`, but the test scanner in `no_tokio_main.rs` strips the `src/` prefix from the path before matching against the allowlist (it walks `crates/vector-app/src/**.rs` and stores relative-to-`src/` paths like `main.rs`, `app.rs`, etc.). The allowlist entry `"src/main.rs"` never matched any path the scanner produced — meaning the allowlist was a no-op. The test happened to pass on the 01-01 stub because there were no `block_on` calls at all; introducing the real `rt.block_on(tick::io_main(proxy))` in main.rs would have caused the lint to fail for the wrong reason.
- **Fix:** Changed `BLOCK_ON_ALLOWLIST: &[&str] = &["src/main.rs"]` to `BLOCK_ON_ALLOWLIST: &[&str] = &["main.rs"]` in `crates/vector-app/tests/no_tokio_main.rs`. The architectural intent — `block_on` permitted only in `main.rs` and forbidden everywhere else under `src/` — is unchanged and is now actually enforced.
- **Files modified:** `crates/vector-app/tests/no_tokio_main.rs` (1 line).
- **Verification:** `cargo test --workspace --tests` exits 0 with all 14 `forbidden_tokio_patterns_absent_from_src ... ok` lines. Spot-injected `rt.block_on(...)` into `crates/vector-app/src/app.rs` (then reverted) to confirm the lint fires on a non-allowlisted file; the test correctly failed with the D-08 architecture-lint message.
- **Committed in:** `25c59e3` (Task 2 commit; bundled with the threading-skeleton landing so the corrected lint enforces the new code).

**3. [Rule 1 — Bug] `build.rs` clippy pedantic: `map_unwrap_or`**
- **Found during:** Task 1 (running `cargo clippy -p vector-app --all-targets -- -D warnings`)
- **Issue:** The `build.rs` source in 01-RESEARCH.md §Pattern 5 used `.map(|o| ...).unwrap_or_else(|| ...)`, which `clippy::map_unwrap_or` (pedantic, on per workspace lints) flags as redundant.
- **Fix:** Collapsed to `.map_or_else(|| "unknown".into(), |o| String::from_utf8_lossy(&o.stdout).trim().to_string())`. Semantics identical.
- **Files modified:** `crates/vector-app/build.rs` (1 expression).
- **Verification:** `cargo clippy -p vector-app --all-targets -- -D warnings` exits 0.
- **Committed in:** `25c59e3` (Task 2 commit — clippy was first run after Task 1 landed but before Task 2 was committed; the fix bundled with Task 2 to keep Task 1's commit a pure deps/resources change).

**4. [Rule 1 — Bug] `overlay::Overlay::relayout` clippy pedantic: `unused_self`**
- **Found during:** Task 2 (running `cargo clippy -p vector-app --all-targets -- -D warnings`)
- **Issue:** `Overlay::relayout(&mut self)` has an empty body — autoresizingMask handles the resize for us today. `clippy::unused_self` (pedantic) flags methods that don't use `self`.
- **Fix:** Added `#[allow(clippy::unused_self)]` on the method with a one-line comment explaining the future-proof intent (the method is kept as a seam for Phase 3's CALayer-backed compositing).
- **Files modified:** `crates/vector-app/src/overlay.rs` (2 lines: the attribute + the comment).
- **Verification:** `cargo clippy -p vector-app --all-targets -- -D warnings` exits 0.
- **Committed in:** `25c59e3` (Task 2 commit).

### Environment Setup (Not a Repo Change)

- **rustup install (non-interactive).** The macOS dev environment had no Rust toolchain installed at the start of this resume. Ran `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path` and `source "$HOME/.cargo/env"` to install the 1.88.0 stable toolchain matched by `rust-toolchain.toml`. This is a one-time environment bootstrap, not a repo change, and is documented here for future macOS resumes.

---

**Total deviations:** 4 auto-fixed (1 missing-critical workspace dep, 1 architecture-lint allowlist bug, 2 clippy pedantic fixes).
**Impact on plan:** All four auto-fixes are required for correctness (deviation 1: overlay would not render per UI-SPEC; deviation 2: the architecture-lint would silently fail to enforce; deviations 3+4: workspace `-D warnings` clippy gate from Plan 01-02 would fail the build). No scope creep — every change is in service of the plan's stated must-haves.

## Issues Encountered

None during planned task execution. The four deviations above were all auto-fixed via Rules 1–2 without requiring user input. The checkpoint:human-verify (Task 3) was approved on first inspection — no visual or functional discrepancies were reported by the user.

## User Setup Required

None at the repo level — this is a code-only plan. Note for future macOS resumes: the Rust toolchain (`rust-toolchain.toml` pins 1.88.0) is required and was installed via `rustup` during this plan's execution (one-time bootstrap; not part of subsequent plans' setup).

## Next Phase Readiness

- **Plan 01-04 (xtask DMG pipeline):** consumes all three `crates/vector-app/resources/*` files (`icon.svg` → `.icns` via `iconutil`, `Info.plist.partial` → merged into the bundle's full `Info.plist` by cargo-bundle, `dmg-background.png` → DMG background via create-dmg). The `vector-app` release binary (now ~3 MB after Task 2's deps) is the bundle payload. The plan should expect `cargo build -p vector-app --release --target {aarch64,x86_64}-apple-darwin` + `lipo -create` to produce the universal binary; the current build already exits 0 on the host architecture.
- **Plan 01-05 (CI):** `cargo build -p vector-app --release` is the matrix-build target. The `tracing::info!("vector starting" ...)` startup line on stdout can be used as a smoke-test signal: spawn the binary headless with `osascript` (or `caffeinate`) for ~1 s and grep for the line. The `CARGO_HUSKY_DONT_INSTALL_HOOKS=1` env var must be set per Plan 01-02's Wave-0 spike finding.
- **Phase 3 (GPU renderer):** inherits the `EventLoop<UserEvent>` + `tokio-io` thread pattern. New `UserEvent` variants will land for renderer events (frame requests, resize damage, GPU-context-lost). The `set_title`-from-main-thread rule and the `user_event` handler are the load-bearing contract — Phase 3 must add renderer-driven UI mutations through `user_event` arms, never directly from worker threads.
- **Phase 9 (reconnect):** the `EventLoopProxy::send_event`-as-only-cross-thread-signal rule is locked. `Domain::reconnect()` state transitions (Active → Reconnecting → Swapping → Active) flow through the same proxy. Anti-Pattern 5 (set_title from worker thread, OnceCell<Runtime>, block_on outside main.rs) is structurally enforced by the architecture-lint test corrected in this plan.

## Verification Checklist

- [x] `crates/vector-app/build.rs` exists; contains `cargo:rustc-env=VECTOR_BUILD_SHA=` and three rerun directives (`.git/HEAD`, `.git/refs/heads`, `VECTOR_BUILD_SHA_OVERRIDE`).
- [x] `crates/vector-app/resources/icon.svg` is a valid SVG with `viewBox="0 0 1024 1024"`, `fill="#1A1A1A"` plate, `stroke="#7B61FF"` strokes, exactly 3 polylines (chevron of 3 motion lines).
- [x] `crates/vector-app/resources/Info.plist.partial` contains `<key>LSMinimumSystemVersion</key><string>13.0</string>`, `<key>NSHighResolutionCapable</key><true/>`, `<key>CFBundleVersion</key><string>2026.05.10</string>`.
- [x] `crates/vector-app/resources/dmg-background.png` exists as a 1280×800 PNG (4165 bytes; final art in Plan 01-04).
- [x] `crates/vector-app/Cargo.toml` `[dependencies]` contains all 10 plan-specified workspace deps + `objc2-quartz-core` (deviation 1).
- [x] `cargo build -p vector-app --release` exits 0.
- [x] `cargo test --workspace --tests` exits 0; the architecture-lint test reports `forbidden_tokio_patterns_absent_from_src ... ok` for all 14 crates.
- [x] `cargo clippy -p vector-app --all-targets -- -D warnings` exits 0.
- [x] `cargo fmt --all -- --check` exits 0.
- [x] `crates/vector-app/src/main.rs` line 1 is `#![allow(unsafe_code)]`.
- [x] `crates/vector-app/src/main.rs` contains `EventLoop::with_user_event` and `rt.block_on(tick::io_main`.
- [x] `crates/vector-app/src/main.rs` does NOT contain `#[tokio::main]`.
- [x] `crates/vector-app/src/{app,tick,menu,overlay}.rs` do NOT contain `block_on`.
- [x] `crates/vector-app/src/app.rs` contains `set_title` and the format string `Vector \u{2014} tick {n}` (U+2014 em-dash).
- [x] `crates/vector-app/src/tick.rs` contains `pub async fn io_main(proxy: EventLoopProxy<UserEvent>)`, `interval(Duration::from_millis(500))`, `proxy.send_event(UserEvent::Tick(`.
- [x] `crates/vector-app/src/menu.rs` contains 6 submenu factories (app/file/edit/view/window/help), 5 helpers (add/add_disabled/add_with_modifiers/add_disabled_with_modifiers/add_services), and all 10 plan-specified `sel!(...)` references.
- [x] `crates/vector-app/src/menu.rs` wires `setWindowsMenu`, `setHelpMenu`, `setServicesMenu`, `setMainMenu`.
- [x] `crates/vector-app/src/overlay.rs` contains `NSTextField`, `monospacedSystemFontOfSize`, `setCornerRadius`, uses `env!("CARGO_PKG_VERSION")` + `env!("VECTOR_BUILD_SHA")`.
- [x] User-approved checkpoint:human-verify — running `cargo run -p vector-app` opens the 1024×640 window, title ticks at 500 ms, overlay reads `Vector v2026.05.10 (build {short-sha})`, menu bar matches UI-SPEC item-by-item, Cmd-Q / Cmd-M / Cmd-W / Cmd-Ctrl-F functional, Cmd-N / Cmd-T disabled, resize anchors the overlay correctly.

## Self-Check: PASSED

- Files asserted present on disk (Bash `[ -f ]`): `crates/vector-app/src/main.rs`, `crates/vector-app/src/tick.rs`, `crates/vector-app/src/app.rs`, `crates/vector-app/src/menu.rs`, `crates/vector-app/src/overlay.rs`, `crates/vector-app/build.rs`, `crates/vector-app/Cargo.toml`, `crates/vector-app/resources/icon.svg`, `crates/vector-app/resources/Info.plist.partial`, `crates/vector-app/resources/dmg-background.png`, `crates/vector-app/tests/no_tokio_main.rs`. All 11 confirmed present.
- Commits asserted present (Bash `git log --all --oneline | grep`): `a0d9027` (Task 1 — feat: wire vector-app deps + build.rs + resources), `25c59e3` (Task 2 — feat: land threading skeleton + AppKit window + menu + overlay). Both confirmed present on `master`.

---
*Phase: 01-foundation-ci-dmg-pipeline*
*Completed: 2026-05-10*
