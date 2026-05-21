---
status: awaiting_human_verify
trigger: "terminal-partial-window"
created: 2026-05-18T00:00:00Z
updated: 2026-05-18T00:05:00Z
---

## Current Focus

hypothesis: tab.last_cols/last_rows is stuck at the initial 80×24 from main.rs create_tab_async. winit Resized(physical) fires BEFORE the lazy compositor exists, so host.cell_metrics_px() returns None and the cols/rows-from-pixels calc at app.rs:1965 is skipped → pending_resize is never set → mux.resize_window never runs → tab dims stay 80×24. compute_layout returns rect(0,0,80,24); per-pane viewport in pixels = 80*cell_w × 24*cell_h = 1360×792 out of 2048×1280 surface → top-left quadrant.
test: Read flow: window create → Resized → first paint → ensure_compositors_for_pane → confirm tab dims at render time
expecting: tab.last_cols=80, tab.last_rows=24 at first paint, with no subsequent resize_window call until user manually drags the window
next_action: Apply fix — when first compositor is built in ensure_compositors_for_pane, derive cols/rows from surface size and call mux.resize_window AND also re-trigger pending_resize on host.resize when no compositor exists by storing the latest physical size and replaying after compositor init

## Symptoms

expected: Terminal content fills the entire window area — the grid of cells should cover the full window width and height
actual: Terminal text and cursor appear only in the top-left quadrant; large black areas to the right and bottom remain unused
errors: No error logs observed for this specific symptom
reproduction: Run ./target/release/vector-app — always reproduces
started: After black-screen fix (single-acquire restructure). Terminal IS rendering now, but viewport/grid sizing is wrong.

## Eliminated

## Evidence

- timestamp: 2026-05-18T00:01:00Z
  checked: crates/vector-app/src/main.rs:87
  found: `mux.create_tab_async(window_id, None, 24, 80)` — initial tab created with hardcoded 24 rows × 80 cols
  implication: tab.last_rows=24, tab.last_cols=80 at startup; layout viewport will be 80×24 cells until a resize_window call updates it

- timestamp: 2026-05-18T00:01:05Z
  checked: crates/vector-app/src/app.rs:1954-1973 (WindowEvent::Resized handler)
  found: pending_resize is queued ONLY if `host.cell_metrics_px()` returns Some. cell_metrics_px returns None until the first Compositor is built (RenderHost::cell_metrics_px maps over self.compositor which is None pre-lazy-init).
  implication: At startup, winit fires Resized(physical_size) once when the window first shows. At that moment no compositor exists yet (lazy build happens on first PTY byte via ensure_compositors_for_pane). So the if-let returns None → pending_resize stays None → tab dims never updated.

- timestamp: 2026-05-18T00:01:10Z
  checked: crates/vector-app/src/app.rs:1163-1254 (ensure_compositors_for_pane)
  found: Builds the first compositor sized to the full surface, then reads cell_w/cell_h back. Uses `tab.last_cols/last_rows` (still 80×24) to compute pane layout, NEVER calls mux.resize_window or re-derives cols/rows from the actual surface_size.
  implication: Even after compositor exists, no one fixes up tab.last_cols/last_rows. The per-pane viewport stays sized to 80*cell_w × 24*cell_h pixels.

- timestamp: 2026-05-18T00:01:15Z
  checked: crates/vector-app/src/render_host.rs:171-187, crates/vector-fonts/src/loader.rs:37-61
  found: new_compositor_for_viewport calls FontStack::load_bundled(self.dpr, 14.0). At DPR=2 this pre-multiplies size_pt → CoreText pixel size = 28pt → cell metrics ARE physical pixels at the current DPR. Logged cell_w=17, cell_h=33 is consistent (JetBrainsMono Mono → Menlo fallback at 28px → ~17×33 physical).
  implication: cell_w/cell_h are physical pixels. The pixel math `80*17 × 24*33 = 1360 × 792` against a `1024*2 × 640*2 = 2048 × 1280` physical surface yields a top-left rectangle ~66% wide × ~62% tall (matches the partial-window symptom; the user's "40%" estimate is rough).

- timestamp: 2026-05-18T00:01:20Z
  checked: crates/vector-mux/src/mux.rs:469-496 (resize_window)
  found: Updates tab.last_cols/last_rows and emits (pane_id, rows, cols) tuples. This is the ONLY path that updates tab dims. Called only from flush_pending_resize_if_quiescent (app.rs:706+), which is gated on pending_resize being Some.
  implication: Confirms the dead-loop — no compositor at Resized time → no pending_resize → no resize_window call → tab dims stuck at 80×24.

- timestamp: 2026-05-18T00:01:25Z
  checked: WindowEvent::Resized fires with PhysicalSize per winit 0.30 contract (event.size returns inner_size in physical pixels). 
  found: `size.width / cell_w` is correct (both physical) — the math at line 1965 would compute correct cols/rows if it ran.
  implication: The pixel math is fine. The only bug is the gating on cell_metrics_px being available too early.

## Resolution

root_cause: |
  Initial tab dims (tab.last_cols=80, tab.last_rows=24) set by main.rs:87
  `mux.create_tab_async(window_id, None, 24, 80)` were never updated after
  window resize because of an ordering bug:

  1. Window is created with logical 1024×640 → at DPR=2 physical surface is 2048×1280
  2. winit fires WindowEvent::Resized(PhysicalSize{2048, 1280}) ONCE on first show
  3. The Resized handler (app.rs:1954) calls host.resize() which configures the
     surface but skips queuing pending_resize because host.cell_metrics_px()
     returns None — the lazy compositor isn't built yet
  4. First PTY byte triggers PaneSpawned → ensure_compositors_for_pane builds
     the first compositor at full surface size, getting cell metrics (17×33 at DPR=2)
  5. compute_layout uses still-stale tab.last_cols=80, tab.last_rows=24 → returns
     viewport rect (0,0,80,24) for the single pane
  6. Per-pane viewport rect in pixels = 80*17 × 24*33 = 1360×792 inside a 2048×1280
     surface → terminal renders in top-left ~66%×62% of the window, rest stays black
     (matches the "top-left ~40%" symptom, the user estimate was rough)

fix: |
  In ensure_compositors_for_pane, after the first compositor is built and cell
  metrics are known, sync tab dims to the actual surface by calling
  mux.resize_window(rows, cols) with cols=surface_w/cell_w, rows=surface_h/cell_h
  and fan out the resulting (pane_id, rows, cols) tuples through the router so
  PTY children receive SIGWINCH. Extracted into a sync_tab_dims_to_surface
  helper. Also extracted the lazy compositor build into resolve_or_init_cell_metrics
  to keep ensure_compositors_for_pane under clippy's too_many_lines threshold.

verification: |
  Self-verification:
  - cargo build --release -p vector-app: succeeds
  - cargo fmt --all --check: clean
  - cargo clippy --all-targets --all-features -- -D warnings: clean
  - cargo test --workspace --tests: all passing (no failed/error lines in summary)
  Code-level reasoning: at first paint, with sw=2048, sh=1280, cell_w=17, cell_h=33:
    cols = 2048/17 = 120, rows = 1280/33 = 38
  mux.resize_window updates tab.last_cols=120, last_rows=38; the re-snapshot reads
  the new dims; compute_layout returns rect(0,0,120,38); per-pane viewport pixels =
  120*17 × 38*33 = 2040 × 1254, ≈ full surface (small remainder from integer
  division — single trailing column / row of pixels). Router fans out PaneResize
  so PTY children + Term grid resize to 120×38.

files_changed:
  - crates/vector-app/src/app.rs (added sync_tab_dims_to_surface helper; extracted
    resolve_or_init_cell_metrics helper; ensure_compositors_for_pane now calls
    sync helper on first compositor build and re-snapshots layout)
