---
status: awaiting_human_verify
trigger: "black-screen-render"
created: 2026-05-17T00:00:00Z
updated: 2026-05-17T00:02:00Z
---

## Current Focus

hypothesis: render_window acquired + presented the surface twice per frame. The second acquire for the chrome pass returned a fresh swapchain texture (with no terminal content) and presented it on top of the just-presented terminal frame.
test: Restructured render_window to acquire ONCE, render terminal + chrome into the same SurfaceTexture, then present ONCE
expecting: Terminal content now visible; build + tests pass
next_action: User confirms visible terminal in the running app

## Symptoms

expected: Terminal content (zsh prompt, cursor) should be visible
actual: Window is completely black — no glyph/background rendering visible
errors:
  - WARN crossfont::darwin: Unable to load specified font JetBrains Mono, falling back to Menlo
  - WARN vector_app::app: render_window: DBG about to render cell_w=17 cell_h=33 leaves=1
reproduction: Run ./target/release/vector-app
started: phase4 branch, after commits 342e717 and 830971a

## Eliminated

## Evidence

- timestamp: 2026-05-17T00:00:30Z
  checked: crates/vector-app/src/app.rs:865 (first acquire_frame in render_window)
  found: Acquires AcquiredFrame for per-pane compositor loop
  implication: This is the surface texture the terminal renders INTO

- timestamp: 2026-05-17T00:00:35Z
  checked: crates/vector-app/src/app.rs:956 (frame.present after per-pane loop)
  found: Presents the per-pane frame to the swapchain
  implication: Terminal content IS submitted and presented — this part is fine

- timestamp: 2026-05-17T00:00:40Z
  checked: crates/vector-app/src/app.rs:1035-1040 (chrome pass entry)
  found: `if let (Some(host), Some(chrome))` — chrome_pipelines is always initialized when render_host exists (app.rs:635), so this branch ALWAYS runs after the per-pane present
  implication: Chrome pass runs on every frame

- timestamp: 2026-05-17T00:00:45Z
  checked: crates/vector-app/src/app.rs:1037 (second acquire_frame for chrome)
  found: A SECOND `host.acquire_frame()` is called after the per-pane frame was already presented
  implication: This returns a DIFFERENT swapchain image (the next one in the chain), which has uninitialized/cleared contents

- timestamp: 2026-05-17T00:00:50Z
  checked: crates/vector-app/src/app.rs:1048-1124 (chrome rpass content)
  found: All chrome draws are conditionally gated (active_tint_rgba.is_some(), search_bar.open, current toast, profile_picker.open). At startup ALL of these are false/None, so the chrome render pass clears nothing (LoadOp::Load) and draws nothing.
  implication: Chrome pass on a fresh swapchain image produces a blank/black texture

- timestamp: 2026-05-17T00:00:55Z
  checked: crates/vector-app/src/app.rs:1127 (frame.present on chrome)
  found: Presents the blank chrome texture as the most-recent swapchain frame
  implication: User sees the blank chrome texture (black) instead of the terminal texture presented at line 956

## Resolution

root_cause: render_window called host.acquire_frame() twice per frame and called frame.present() twice. Per-pane block (app.rs:865) acquired surface texture A, terminal compositors rendered into A, A was presented. Chrome block (app.rs:1037) then acquired surface texture B (the next swapchain image), loaded its uninitialized contents with LoadOp::Load, drew nothing (all chrome elements were gated off at startup — no tint, search bar closed, no toast, picker closed), then presented B. B (blank) replaced A (terminal) on screen → user saw a black window.
fix: Restructured render_window to acquire the surface frame exactly once. The AcquiredFrame is now created in the per-pane block, carried out via the block's return value, and reused by the chrome pass. frame.present() is called once at the end (after chrome, or immediately if chrome_pipelines is missing). This collapses terminal + chrome into one presented swapchain image per redraw.
verification: cargo build --release -p vector-app succeeds. cargo clippy --release -p vector-app -- -D warnings clean. cargo test -p vector-app --tests --release: 7/7 pass.
files_changed:
  - crates/vector-app/src/app.rs (render_window restructured: single acquire/present per frame)
