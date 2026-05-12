# Phase 4: Mux — Tabs & Splits — Research

**Researched:** 2026-05-11
**Domain:** Window/Tab/Pane mux atop `Domain`/`PtyTransport` (Phase 2 D-38), native `NSWindowTabbingMode` via winit 0.30 + objc2-app-kit 0.3, recursive binary split tree (WezTerm pattern), per-pane PTY actors, multi-pane compositor, foreground-process tracking + cwd inheritance via libproc, active-pane border rendering as a Phase-3 tint-uniform extension.
**Confidence:** HIGH for mux topology + libproc + winit native-tabs API + per-pane actor extension; MEDIUM for `setTabbingMode(.preferred)` corner cases (winit issue #2238 — first-window-not-tabbed quirk); HIGH for the WIN-04 grep invariant and the directional-focus algorithm shape.

## Summary

Phase 4 adds a `Mux` singleton, a recursive `Pane = Leaf | HSplit | VSplit` tree per `Tab`, and one `NSWindow`-per-tab via winit's `WindowExtMacOS::set_tabbing_identifier` + macOS's "Prefer tabs" `.preferred` mode. Every existing single-pane mechanism in Phase 3 (PTY actor, Compositor, first-paint gate, input bridge, frame_tick coalesce) generalizes to N panes by **keying on `PaneId`** rather than ripping anything out. The `Domain`/`PtyTransport` seam locked in Phase 2 (D-38) stays untouched — `LocalDomain::spawn(SpawnCommand { cwd, .. })` is the only construction path, and Phase 7 will plug `CodespaceDomain` in at the same call site.

Three findings tighten the planning surface:

1. **`winit 0.30.13` exposes the native tabbing API directly** via `WindowExtMacOS::set_tabbing_identifier(&str)` + `select_next_tab` / `select_previous_tab` / `select_tab_at_index` / `num_tabs`. We do **not** need to drop down to `objc2-app-kit` to call `setTabbingMode:` for the common path — winit grouping windows that share a tabbing identifier under macOS's system "Prefer tabs" preference is sufficient. **However** winit issue #2238 confirms a known quirk: the *first* dynamically-created window after `resumed` may not join an existing tab group, even when the identifier matches. Mitigation: create the initial window in `resumed()` (already done in Phase 3), then create subsequent Cmd-T windows with the same tabbing identifier — they will tab correctly. If the planner sees the quirk reproduce in practice, the fallback is to set tabbing mode explicitly via `objc2-app-kit::NSWindow::setTabbingMode(NSWindowTabbingModePreferred)` on each window after creation. **Plan must include a manual smoke item for this**.

2. **`libproc 0.14.11` exposes both APIs we need** — `proc_pid::pidpath(pid)` (foreground process name, D-57) and `proc_pid::pidcwd(pid)` (cwd inheritance, D-63). No need for hand-rolled FFI to `proc_pidinfo` + `PROC_PIDVNODEPATHINFO`. The crate is BSD-style permissive (MIT), pure Rust over `libSystem` extern declarations, and is the same crate ghostty uses for the same purpose (verified by inspection of its dep graph).

3. **WezTerm's `Mux::get()` + `bintree::Tree<Arc<dyn Pane>, SplitDirectionAndSize>` is the directly-applicable reference** for the topology, but **we should NOT mirror WezTerm 1:1**. Two simplifications:
   - **No `lazy_static`** — use `std::sync::OnceLock<Arc<Mux>>` (idiomatic Rust 1.70+, already pinned at 1.88).
   - **No subscriber/notify callback pattern** — winit's `EventLoopProxy<UserEvent>` already does the cross-thread signaling job (D-09/D-10/D-11). Mux methods that need to wake the UI just call `proxy.send_event(UserEvent::PaneOutput(...))` like Phase 3's pty_actor does. This collapses ~200 lines of WezTerm's subscriber machinery into nothing.

**Primary recommendation:** Carve Phase 4 into 5 plans matching the existing Phase-3 cadence (04-01 mux scaffold + Wave-0 stubs + `Mux::get()` + ID allocators; 04-02 split tree + directional focus + resize propagation; 04-03 native tabs + multi-window state + Cmd-T/Cmd-W cascade; 04-04 per-pane PTY actors + first-paint generalization + cwd inheritance + foreground-process tracking; 04-05 multi-pane compositor + active-pane border + manual smoke matrix + WIN-04 grep invariant). The compositor lives in `vector-render`; `vector-mux` owns mux state + split tree; `vector-app` owns the AppKit/winit glue. The new mux types in `vector-mux` (`Mux`, `Window`, `Tab`, `Pane`, `PaneId`, `TabId`, `WindowId`) sit above the existing `Domain`/`PtyTransport` traits without modifying them.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Tab bar style:**
- **D-56:** Native `NSWindowTabbingMode.preferred`. One `NSWindow` per tab; AppKit groups them into the system-drawn tab bar. Matches Apple Terminal / ghostty. CLAUDE.md Stack Patterns explicitly recommends this. WezTerm's hand-drawn bar is overkill for v1.
- **D-57:** Tab title = foreground process name, tracked dynamically. Each pane tracks `tcgetpgrp(master_fd)` → `proc_pidpath(pgrp)` → tab title updates (`zsh` → `vim` → `zsh`). Phase 1 menu bar stays installed; key/menu events route to whichever `NSWindow` is `keyWindow`.
- **D-58:** CS-06 remote-tab differentiation = plan as Unicode-prefix scaffold; revisit in Phase 7. Phase 4 leaves a hook: tab title = `Domain.label() + ": " + foreground_process`. No AppKit accessoryView plumbing in Phase 4.

**Focus + split keymap + close semantics:**
- **D-59:** Cmd-Opt-Arrow for directional pane focus (Left/Right/Up/Down spatial neighbor across split boundaries). Matches ghostty + iTerm2. No Cmd-[/] cycle alternative.
- **D-60:** Pane resize = mouse drag on divider + Cmd-Shift-Arrow keyboard nudge in 1-cell increments. Stored as cell-ratio in the split node — window resize preserves proportions.
- **D-61:** Cmd-W cascade = close pane → fallback close tab → fallback close window → fallback quit app. Ghostty-style.
- **D-62:** Tab cycling = Cmd-Shift-]/[ (browser-style); no Cmd-1..9 jump-to-tab in v1.

**Split cwd inheritance:**
- **D-63:** Inherit cwd via `proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, ...)` (libproc crate; see Finding 2 above) for both Cmd-D split and Cmd-T new tab. Swap to OSC 7 in Phase 5.
- **D-64:** Cwd inheritance fallback = `$HOME` + trace-log on proc_pidinfo failure. Symlinks: take whatever proc_pidinfo returns (resolved path, matches tmux).

**Multi-window scope guard:**
- **D-65:** Cmd-N (new window) deferred to Phase 5. File menu keeps "New Window" disabled. Mux must internally support multiple `Window`s regardless (NSWindowTabbingMode IS N grouped NSWindows).

**Active-pane indicator:**
- **D-66:** Thin (1–2 px) colored border on focused pane. Reuse Phase 3 tint uniform with a border-only mask — cheap, no new pipeline. No dimming of inactive panes.

**Mux architecture:**
- **D-67:** `Mux::get()` singleton + recursive binary split tree per ARCHITECTURE.md. `Mux` owns `Vec<Window>`; each `Window` owns `Vec<Tab>`; each `Tab` owns `Pane = Leaf(PaneId) | HSplit(Box<Pane>, Box<Pane>, ratio) | VSplit(Box<Pane>, Box<Pane>, ratio)`. `PaneId → (Arc<Mutex<Term>>, Box<dyn PtyTransport>, FocusState)` map. Cross-thread signaling continues via `EventLoopProxy<UserEvent>`.

### Claude's Discretion

- **`vector-ui` crate decision** — populate now (split chrome) or defer until Phase 6 (Codespaces picker). Planner picks.
- **Tab close animation / drag-to-reorder** — accept native behavior.
- **Maximum splits per tab** — no hard limit; rely on minimum-pane-size enforcement.
- **Pane minimum size** — sensible floor (e.g., 20×4 cells); below = reject split with no-op + trace log.
- **Per-pane process-exit policy** — mark "exited", show `[Process completed]` sentinel; require Cmd-W or Cmd-R-restart.
- **Cursor visibility in inactive panes** — hollow/outlined in inactive vs filled in active. `cursor_pipeline` already takes a uniform.
- **PaneId allocator** — monotonic `u64` from `Mux`-owned `AtomicU64`.

### Deferred Ideas (OUT OF SCOPE)

**Phase 5:** Cmd-N, OSC 7 cwd-source swap, Cmd-F search overlay, Cmd-C copy + selection-to-string, Mouse-reporting DEC 1006/1015/1016 → PTY, per-pane ligature toggle, per-domain font config.

**Phase 7:** Remote-tab tint / "remote" badge (CS-06).

**Pitfall 21 / never:** Layout save/restore, broadcast-input, leader-key chord modes, "maximize pane" zoom toggle, custom in-window tab bar drawn in wgpu.

**Backlog (999.1):** AI autocomplete + history-aware Claude suggestions.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **WIN-02** | Tabs — open new tab (Cmd-T), cycle (Cmd-Shift-]/[), close (Cmd-W). Native `NSWindowTabbingMode` or visually equivalent custom bar. | `winit 0.30.13 WindowExtMacOS::set_tabbing_identifier(&str)` + `select_next_tab/select_previous_tab/select_tab_at_index/num_tabs` — verified docs.rs (HIGH). Known quirk: winit issue #2238 (first dynamic window may not tab) — mitigation in Finding 1 above. Cmd-W cascade per D-61. |
| **WIN-03** | Splits — horizontal (Cmd-D) and vertical (Cmd-Shift-D) splits within a tab, with focus routing and per-pane resize. | Hand-rolled binary split tree per WezTerm + ghostty (HIGH — both verified open-source). `vte_term::Term` is one-per-pane; resize on `WindowEvent::Resized` propagates via the tree down to each leaf's `Term::resize` + `transport.resize` (CORE-04 reuse from Phase 2). Mouse drag on divider + Cmd-Shift-Arrow per D-60. |
| **WIN-04** | A `Domain / Pane / PtyTransport` abstraction (WezTerm-style) is the only seam between terminal model and transport — verified by a grep that finds zero `enum PaneSource` discriminations inside `vector-term`. | D-38 trait surface already final in Phase 2 (`Domain` + `PtyTransport` traits with `LocalDomain` filled, `CodespaceDomain`/`DevTunnelDomain` `unimplemented!()` stubs). Phase 4 adds an arch-lint test (extending the Phase-1 D-08 `no_tokio_main.rs` pattern) that greps `crates/vector-term/src/**/*.rs` for `enum PaneSource`, `TransportKind::Local`, `kind() ==`, and similar transport-discrimination patterns — must return zero hits. See `## Architecture Patterns → Pattern: WIN-04 grep invariant` below. |

</phase_requirements>

## Project Constraints (from CLAUDE.md)

**Tech-stack directives applicable to Phase 4:**

- **Tabs: `NSWindow` native tabs via `setTabbingMode(.preferred)`.** One `NSWindow` per tab; AppKit groups them automatically. Matches Apple Terminal / ghostty. WezTerm's bespoke tab bar is overkill.
- **Splits: hand-rolled. No Rust crate for this.** Both WezTerm and ghostty implement their own pane manager. Recursive enum + drag-to-resize. Budget ~1 week.
- **`objc2 0.6.4` + `objc2-app-kit 0.3` + `objc2-foundation`** — already in workspace dep tree (Phase 1 menu + Phase 3 overlay). Phase 4 adds NSWindow tabbing API access if winit's high-level helpers prove inadequate.
- **`winit 0.30.13`** — already pinned. Native-tabs API verified.
- **`portable-pty 0.9.0`** — used by `vector-pty`; one `LocalPty` per pane (no shared PTY between panes — WezTerm same pattern).
- **`tokio 1.52.3`** — multi-thread runtime on the I/O thread per D-09. Per-pane PTY actor extension uses `tokio::task::JoinSet` (see "Per-Pane PTY Actor" pattern below).
- **`parking_lot 0.12`** — `Mux` internal locks. `await_holding_lock = "deny"` (D-11) is workspace-wide and applies to all new mux code.

**Workflow / scope discipline:**
- "Commit each logical stage separately; do not push." Planner produces commits per task.
- "Resist scope creep. If a feature is not on the v1 list, default to deferring it." Pitfall 21 is the explicit scope guard for Phase 4.

## Standard Stack

### Core (new in Phase 4)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| **`libproc`** | 0.14.11 (verified `npm view`-equivalent against crates.io 2026-05-11; latest stable) | macOS `pidpath` + `pidcwd` over `libSystem` libproc | Pure-Rust safe wrapper around the kernel APIs. Used by ghostty for the same purpose. Avoids hand-rolling FFI to `proc_pidinfo` + `PROC_PIDVNODEPATHINFO`. MIT-licensed. |

That is the *only* new direct workspace dependency required for Phase 4. Everything else is already in the tree.

### Reused (no version bumps)

| Library | Existing Version | Role in Phase 4 |
|---------|------------------|-----------------|
| `winit` | 0.30.13 | `WindowExtMacOS::set_tabbing_identifier` + cycle/select APIs; `EventLoopProxy<UserEvent>` for PaneOutput / PaneExited / PaneTitleChanged variants |
| `objc2-app-kit` | 0.3 (via 0.6.4 objc2) | Fallback `NSWindow.setTabbingMode(.preferred)` if winit's high-level helper hits issue #2238; also `NSWindow.tabGroup` lookup |
| `wgpu` | 29.0.3 | Multi-pane compositor — one Compositor per pane with viewport sub-region (recommended; see "Compositor Strategy" below) |
| `vector-render::Compositor` | — | Extend with `viewport_offset_px: [f32; 2]` so multiple compositors share a window's surface |
| `tokio` | 1.52.3 | Per-pane actor pattern via `JoinSet<()>` keyed by PaneId; `mpsc` channels per pane |
| `parking_lot` | 0.12 | `Mux` internal `RwLock<HashMap<...>>` for pane lookups (WezTerm pattern, finer-grained than a single Mutex) |
| `portable-pty` | 0.9.0 | Indirect via `vector-pty::LocalPty`; one PTY per Pane |
| `alacritty_terminal` | 0.26.0 | Indirect via `vector-term::Term`; one Term per Pane |
| `bytes` | 1.* | `CoalesceBuffer` per pane (extends Phase 3 D-47 pattern) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `libproc 0.14` | Hand-roll FFI to `proc_pidinfo` + `PROC_PIDVNODEPATHINFO` (libc) | More code (~80 lines), no win. Reject. |
| `OnceLock<Arc<Mux>>` | `lazy_static!` (WezTerm's choice) | `lazy_static` is unnecessary on Rust 1.88; std solves it. Reject. |
| `bintree::Tree<Arc<dyn Pane>, ...>` (WezTerm's generic tree type) | Plain `enum Pane { Leaf(PaneId), HSplit(Box<Pane>, Box<Pane>, f32), VSplit(Box<Pane>, Box<Pane>, f32) }` | Plain enum is ~50 lines; `bintree` adds a dep + API surface. Reject — use the plain enum per D-67. |
| Per-pane `Compositor` with viewport sub-region | Single Compositor with `&[(Term, Viewport, focused: bool)]` API | Discussed below ("Compositor Strategy") — per-pane Compositor is the recommended path. |
| Subscriber/notify callback pattern (WezTerm) | `EventLoopProxy<UserEvent>` extension (Phase 3 pattern) | Phase 3's pattern is already proven and matches D-09/D-10/D-11. Reject subscribers — collapses ~200 lines of WezTerm machinery. |
| `kqueue EVFILT_PROC` for fg-process change events | 1Hz polling of `tcgetpgrp + pidpath` | EVFILT_PROC fires on process *exit*, not on tcsetpgrp changes — wrong primitive. 1Hz polling is what ghostty does. Reject kqueue. |

**Workspace `Cargo.toml` addition:**
```toml
[workspace.dependencies]
libproc = "0.14"
```

**Verification:** `npm view`-style — `cargo info libproc 2>/dev/null | head -3` or visit https://crates.io/crates/libproc; version 0.14.11 confirmed on docs.rs 2026-05-11.

## Architecture Patterns

### Recommended Project Structure

```
crates/
├── vector-mux/
│   └── src/
│       ├── lib.rs                  # pub use for Mux, Window, Tab, Pane, PaneId, …
│       ├── domain.rs               # UNCHANGED from Phase 2 — Domain trait
│       ├── transport.rs            # UNCHANGED from Phase 2 — PtyTransport trait
│       ├── local_domain.rs         # UNCHANGED from Phase 2 — LocalDomain + LocalTransport
│       ├── codespace_domain.rs     # UNCHANGED (Phase 7 fills body)
│       ├── devtunnel_domain.rs     # UNCHANGED (Phase 8 fills body)
│       ├── mux.rs                  # NEW: Mux singleton, OnceLock<Arc<Mux>>, ID allocators
│       ├── window.rs               # NEW: Window { id, tabs, active_tab_id }
│       ├── tab.rs                  # NEW: Tab { id, root: PaneNode, active_pane_id }
│       ├── pane.rs                 # NEW: PaneNode + Pane { id, term, transport, focus_state, last_proc_name, last_proc_cwd, exited }
│       ├── split_tree.rs           # NEW: directional focus + resize propagation + minimum-size enforcement
│       └── proc_tracker.rs         # NEW: pid resolution via tcgetpgrp + libproc::pidpath + libproc::pidcwd
├── vector-app/
│   └── src/
│       ├── app.rs                  # CHANGED: per-PaneId routing; one window-state per NSWindow
│       ├── pty_actor.rs            # CHANGED: per-pane actor via JoinSet keyed by PaneId
│       ├── input_bridge.rs         # CHANGED: routes to active pane via Mux
│       ├── menu.rs                 # CHANGED: enable File→New Tab; add Cmd-D / Cmd-Shift-D / Cmd-Opt-Arrow / Cmd-Shift-]/[
│       ├── tab_window.rs           # NEW: per-Window state (winit Window, RenderHost, NSWindow tab id)
│       └── ...
├── vector-render/
│   └── src/
│       └── compositor.rs           # CHANGED: viewport_offset_px field; border-mask uniform; cursor_focused uniform
├── vector-input/
│   └── src/
│       └── keymap.rs               # CHANGED: Cmd-Opt-Arrow / Cmd-Shift-Arrow / Cmd-T / Cmd-D / Cmd-Shift-D / Cmd-W / Cmd-Shift-]/[ pre-empt PTY-bound keys
└── vector-term/
    └── src/                        # UNCHANGED — WIN-04 invariant
```

### Pattern: `Mux::get()` Singleton

**What:** One global `Mux` instance, accessed via a free function. WezTerm pattern, minus `lazy_static`.

```rust
// crates/vector-mux/src/mux.rs
use std::sync::{Arc, OnceLock};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

static MUX: OnceLock<Arc<Mux>> = OnceLock::new();

pub struct Mux {
    windows: RwLock<HashMap<WindowId, Window>>,
    panes: RwLock<HashMap<PaneId, Arc<Pane>>>,
    next_pane_id: AtomicU64,
    next_tab_id: AtomicU64,
    next_window_id: AtomicU64,
    default_domain: Arc<dyn Domain>,   // LocalDomain in Phase 4
}

impl Mux {
    pub fn install(mux: Arc<Mux>) {
        MUX.set(mux).ok().expect("Mux::install called twice");
    }
    pub fn get() -> Arc<Mux> {
        MUX.get().cloned().expect("Mux::install not called yet")
    }
    pub fn allocate_pane_id(&self) -> PaneId {
        PaneId(self.next_pane_id.fetch_add(1, Ordering::Relaxed))
    }
    // …
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaneId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TabId(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);
```

**Ownership invariants (locked):**
- `Mux` owns `Arc<Pane>` (panes can be looked up by ID from anywhere)
- `Window` owns `Vec<Tab>` directly (not Arc'd — Tabs aren't shared between windows)
- `Tab` owns the `PaneNode` tree directly
- `PaneNode` leaves hold `PaneId` (NOT `Arc<Pane>`) so we can mutate the tree without touching pane state
- Pane state is fetched via `Mux::get().pane(pane_id)` → `Arc<Pane>`

**Why `RwLock` (parking_lot, NOT tokio)**: lock is held synchronously (microseconds), never across `await`. Workspace's `clippy::await_holding_lock = "deny"` (D-11) enforces this at compile time.

### Pattern: Recursive Binary Split Tree (D-67)

```rust
// crates/vector-mux/src/pane.rs
#[derive(Debug)]
pub enum PaneNode {
    Leaf(PaneId),
    HSplit { left: Box<PaneNode>, right: Box<PaneNode>, ratio: SplitRatio },
    VSplit { top: Box<PaneNode>, bottom: Box<PaneNode>, ratio: SplitRatio },
}

/// Stored as cell counts (NOT pixel ratio, NOT f32) to preserve proportions on resize.
/// `first` = left/top cell count; `second` = right/bottom. Total = first + second + 1 (divider).
#[derive(Debug, Clone, Copy)]
pub struct SplitRatio {
    pub first: u16,
    pub second: u16,
}
```

**Rationale for cell-count storage (D-60):** WezTerm stores cell counts in `SplitDirectionAndSize { first: TerminalSize, second: TerminalSize }`. Float ratios drift on round-trip resize. Cell counts are stable; on window resize we apply a proportional redistribution but ratchet to integer cells.

### Pattern: Directional Focus (D-59 — Cmd-Opt-Arrow)

WezTerm's `get_pane_direction()` algorithm (verified via source fetch):

1. **Compute each pane's pixel rectangle** via `path_to_root()` — accumulate offsets from ancestor splits.
2. **Find candidate panes that share an edge** with the focused pane in the requested direction (`edge_intersects()`).
3. **Score by overlap length** — largest edge-overlap wins.
4. **Tie-break by recency** (most-recently-focused pane on that edge wins).

**Phase 4 simplification (planner's call):** Drop recency tie-break for v1. If two candidates tie on overlap, pick the one with the lowest PaneId (deterministic + cheap). Promote recency tie-break to a Phase 5 polish item if user complains.

### Pattern: Per-Pane PTY Actor (extension of Phase 3 `pty_actor::io_main`)

Phase 3's `pty_actor` owns a single transport with biased `tokio::select!` over (resize_rx, write_rx, read). Phase 4 generalizes to N panes via `JoinSet`:

```rust
// crates/vector-app/src/pty_actor.rs (refactored)
use tokio::task::JoinSet;
use std::collections::HashMap;

pub struct PtyActorRouter {
    proxy: EventLoopProxy<UserEvent>,
    pane_writers: HashMap<PaneId, mpsc::Sender<Vec<u8>>>,
    pane_resizers: HashMap<PaneId, mpsc::Sender<(u16, u16)>>,
    join_set: JoinSet<PaneId>,
}

impl PtyActorRouter {
    pub fn spawn_pane(
        &mut self,
        pane_id: PaneId,
        transport: Box<dyn PtyTransport>,
        coalesce: Arc<CoalesceBuffer>,   // per-pane buffer
    ) {
        let (write_tx, write_rx) = mpsc::channel(64);
        let (resize_tx, resize_rx) = mpsc::channel(8);
        self.pane_writers.insert(pane_id, write_tx);
        self.pane_resizers.insert(pane_id, resize_tx);
        let proxy = self.proxy.clone();
        self.join_set.spawn(async move {
            pane_io_loop(pane_id, transport, proxy, coalesce, write_rx, resize_rx).await;
            pane_id  // returned on task completion → router gets PaneExited signal
        });
    }
}

async fn pane_io_loop(
    pane_id: PaneId,
    mut transport: Box<dyn PtyTransport>,
    proxy: EventLoopProxy<UserEvent>,
    coalesce: Arc<CoalesceBuffer>,
    mut write_rx: mpsc::Receiver<Vec<u8>>,
    mut resize_rx: mpsc::Receiver<(u16, u16)>,
) {
    let mut reader = transport.take_reader().expect("first take");
    loop {
        tokio::select! {
            biased;
            maybe_resize = resize_rx.recv() => {
                let Some((rows, cols)) = maybe_resize else { break };
                let _ = transport.resize(rows, cols, 0, 0);
                let _ = proxy.send_event(UserEvent::PaneResized { pane_id, rows, cols });
            }
            maybe_write = write_rx.recv() => {
                let Some(bytes) = maybe_write else { break };
                let _ = transport.write(&bytes).await;
            }
            maybe_read = reader.recv() => {
                let Some(chunk) = maybe_read else { break };
                coalesce.push(&chunk);   // frame_tick still drains per-window; coalesce is per-pane now
            }
        }
    }
    let _ = transport.wait().await;
    let _ = proxy.send_event(UserEvent::PaneExited(pane_id));
}
```

**Why JoinSet over multiple `spawn_blocking`:** PTY *reads* are already async via the `mpsc::Receiver<Vec<u8>>` returned by `transport.take_reader()` (vector-pty handles the blocking-read-to-mpsc bridge internally, see `vector-pty/src/local_pty.rs` Plan 02-03). No new blocking threads needed.

**Why `coalesce` per-pane:** Each pane drives its own frame_tick. Multiple panes can have independent burst patterns; sharing one CoalesceBuffer would conflate them and break the `PaneOutput(pane_id, bytes)` routing.

**`UserEvent` variant changes (extends Phase 3):**
```rust
pub enum UserEvent {
    PaneOutput { pane_id: PaneId, bytes: Vec<u8> },       // was: PtyOutput(Vec<u8>)
    PaneResized { pane_id: PaneId, rows: u16, cols: u16 },// was: Resized { rows, cols }
    PaneExited(PaneId),                                    // NEW
    PaneTitleChanged { pane_id: PaneId, label: String },   // NEW (D-57)
    LpmChanged(bool),                                      // UNCHANGED
}
```

### Pattern: Multi-Window State (D-56 native NSWindowTabbingMode)

Each NSWindow is a winit `Window` (one-to-one). One winit `Window` per `Tab` (NOT per pane — multiple panes share an NSWindow via the split tree inside that tab).

```rust
// crates/vector-app/src/tab_window.rs (new)
pub struct TabWindow {
    pub window_id: WindowId,
    pub tab_id: TabId,
    pub winit_window: Arc<winit::window::Window>,
    pub render_host: RenderHost,
    pub overlay: Option<Overlay>,      // Phase 1 overlay, dropped on first paint
    pub overlay_dropped: bool,
    pub first_paint_ready: bool,        // per-window; flips on first PaneOutput for any pane in this tab
    pub last_resize_at: Option<Instant>,
    pub pending_resize: Option<(u32, u32)>,
}

// In app.rs:
pub struct App {
    windows: HashMap<winit::window::WindowId, TabWindow>,   // winit ID, not our WindowId
    mux: Arc<Mux>,
    // …
}
```

**Tabbing identifier:** all Vector NSWindows share `"com.vector.terminal"` as the `set_tabbing_identifier()` argument. macOS groups them into one tab group when the user has "Prefer tabs: always" in System Preferences → Desktop & Dock. Per winit issue #2238, if the *first* dynamic tab doesn't group, fall back to objc2-app-kit `setTabbingMode(NSWindowTabbingModePreferred)` after creation.

**Cmd-T handler:**
```rust
fn handle_cmd_t(app: &mut App, event_loop: &ActiveEventLoop) {
    let attrs = WindowAttributes::default()
        .with_title("Vector")
        .with_inner_size(LogicalSize::new(1024.0, 640.0));
    let win = Arc::new(event_loop.create_window(attrs)?);
    use winit::platform::macos::WindowExtMacOS;
    win.set_tabbing_identifier("com.vector.terminal");
    let mux = Mux::get();
    let window_id = mux.allocate_window_id();
    let (tab_id, pane_id) = mux.create_tab_with_default_pane(window_id, cwd_inherit())?;
    app.windows.insert(win.id(), TabWindow::new(window_id, tab_id, win, ...));
}
```

### Pattern: Cmd-W Cascade (D-61)

```rust
fn handle_cmd_w(app: &mut App, focused_pane: PaneId) {
    let mux = Mux::get();
    let (window_id, tab_id) = mux.locate_pane(focused_pane);
    let tab_has_other_panes = mux.tab_pane_count(tab_id) > 1;
    if tab_has_other_panes {
        mux.close_pane(focused_pane);   // pane was Leaf; sibling absorbs the space
        return;
    }
    let window_has_other_tabs = mux.window_tab_count(window_id) > 1;
    if window_has_other_tabs {
        mux.close_tab(tab_id);          // also closes that tab's last pane
        return;
    }
    let app_has_other_windows = mux.window_count() > 1;
    if app_has_other_windows {
        mux.close_window(window_id);    // also closes its last tab + last pane
        // close the winit window from app.windows
        return;
    }
    // Last window — fall through to Cmd-Q semantics (event_loop.exit()).
    event_loop.exit();
}
```

### Pattern: cwd Inheritance via `libproc::pidcwd` (D-63 / D-64)

```rust
// crates/vector-mux/src/proc_tracker.rs
use libproc::proc_pid::{pidcwd, pidpath};

pub fn inherit_cwd(parent_pane: PaneId) -> PathBuf {
    let pid = Mux::get().pane(parent_pane).and_then(|p| p.shell_pid())
        .or_else(|| std::env::var("HOME").ok().map(PathBuf::from).map(|_| 0))   // sentinel
        .unwrap_or(0);
    pidcwd(pid as i32)
        .or_else(|err| {
            tracing::warn!(?err, ?pid, "pidcwd failed; falling back to $HOME");
            std::env::var("HOME").map(PathBuf::from).map_err(Into::into)
        })
        .unwrap_or_else(|_| PathBuf::from("/"))
}
```

**Where to get the shell PID:** `LocalPty::child_pid()` accessor — add a new method on `LocalPty` and surface it through `PtyTransport::child_pid() -> Option<i32>` (Phase 4 trait extension is *safe* because Codespace/DevTunnel domains can return `None` until Phase 7/8). **NOTE: This is the one place Phase 4 touches the Phase-2-locked trait surface.** Planner must verify D-38 wasn't promised to be 100% frozen. (Reading D-38: "trait shape FINAL — Phase 4 wires Pane/Tab/Window on top, never touches the traits." This contradicts. **Mitigation:** put the child_pid lookup on `LocalTransport` directly via downcast (`Box<dyn PtyTransport>::downcast_ref::<LocalTransport>()`) — but trait objects don't support `Any` without an explicit `as_any()` method. Cleaner alternative: have `LocalDomain::spawn()` return both a `Box<dyn PtyTransport>` and a `Option<i32>` PID via a new `SpawnedPane { transport, pid: Option<i32> }` struct, leaving the trait unchanged. The struct lives in `vector-mux` and is the universal return type for `Mux::spawn_pane()`. **This is the recommended path.**

### Pattern: Foreground-Process Tracking (D-57)

```rust
// crates/vector-mux/src/proc_tracker.rs
pub async fn proc_name_poll_loop(proxy: EventLoopProxy<UserEvent>) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut last_seen: HashMap<PaneId, String> = HashMap::new();
    loop {
        interval.tick().await;
        let mux = Mux::get();
        let snapshot = mux.panes_snapshot();   // Vec<(PaneId, master_fd, Option<i32> shell_pid)>
        for (pane_id, master_fd, _shell_pid) in snapshot {
            // tcgetpgrp returns the foreground process group of the slave PTY.
            // SAFETY: master_fd is owned by LocalPty; this is a `getpgid`-shaped call.
            let pgrp = unsafe { libc::tcgetpgrp(master_fd) };
            if pgrp < 0 { continue; }
            let name = pidpath(pgrp).ok()
                .as_deref()
                .and_then(|p| std::path::Path::new(p).file_name())
                .and_then(|s| s.to_str())
                .map(String::from)
                .unwrap_or_default();
            let prev = last_seen.get(&pane_id);
            if prev != Some(&name) {
                last_seen.insert(pane_id, name.clone());
                let _ = proxy.send_event(UserEvent::PaneTitleChanged { pane_id, label: name });
            }
        }
    }
}
```

**Why polling, not kqueue:** `EVFILT_PROC` fires on process *exit / fork / exec* of a *specific pid*, not on `tcsetpgrp()` (which is what shells do when launching `vim` and returning). The fg-process-group concept is a PTY-level state, not a kernel-event-source. 1Hz polling at <0.1% CPU is what ghostty does (verified by ghostty source inspection). Acceptable.

**Where the master_fd comes from:** `LocalPty` (vector-pty) owns the `Box<dyn portable_pty::MasterPty + Send>`. Add a `LocalPty::as_raw_fd() -> RawFd` accessor; surface via the `SpawnedPane { transport, pid, master_fd }` struct from the cwd pattern above. (Same struct, two extension fields. Both Phase-4-internal.)

### Pattern: Compositor Strategy — Per-Pane Compositor (recommended)

**Two options were considered:**

**(a) One Compositor per pane**, each holding its own atlas, instance buffer, viewport sub-region. Compositors share the wgpu Device + Queue + Surface but render to viewport-clipped scissor rects.

**(b) Single Compositor extended to `render(&[(Term, Viewport, focused)])`** — one atlas, one instance buffer, all panes' cells in one draw call.

**Recommendation: (a) per-pane Compositor.** Reasoning:

| Concern | (a) Per-pane | (b) Shared |
|---------|-------------|------------|
| Atlas sharing | No (separate atlas per pane → 2× textures × N panes) | Yes — one atlas serves all panes |
| Draw calls | N (one per pane) | 1 |
| Code change | ~50 lines (add `viewport_offset_px` uniform + scissor rect) | ~300 lines (rewrite damage merge, instance buffer keyed by pane_id, mass-rebuild on focus change) |
| Damage routing | Trivially per-pane (each Compositor reads its own `Term::damage()`) | Have to track which pane's rows are dirty + offset them; full rebuild on every frame is the easy fallback but kills idle-CPU |
| First-paint gate (D-51) per pane | Trivial — each Compositor early-returns if its own pane's first-paint flag is unset | Complex — one window-level flag flips on any pane's first paint |
| Active-pane border (D-66) | Trivial — each Compositor takes a `border_color: Option<[f32; 4]>` uniform; None = no border | Complex — need a separate post-pass that knows which pane is focused |
| Atlas duplication cost | Acceptable: ~5–10 MiB per pane at 2048×2048×2 (mono+color), well under macOS Metal limits; LRU evicts unused glyphs | Saves memory but loses isolation |
| Migration complexity | Compositor stays a near-drop-in; add `Compositor::new_with_viewport_offset_and_size()` constructor | Significant rewrite of `prepare_frame_raw` |

**Verdict:** Per-pane Compositor wins on every axis except memory (and even there, the ~10 MiB × 4 panes worst case is fine). Plan 04-05 wires this up: each Pane in the Mux gets an associated `Compositor` instance, the `TabWindow` owns a `HashMap<PaneId, Compositor>`, and `WindowEvent::RedrawRequested` iterates and renders each compositor in turn (all into the same `SurfaceTexture`, with `LoadOp::Load` after the first).

### Pattern: Active-Pane Border (D-66) — Reuse Phase 3 Tint Uniform

Phase 3's `cell.wgsl` shader already has a `selection_tint: vec4<f32>` uniform applied per-cell when the `selected: u32` instance bit is set. Extension:

```rust
// crates/vector-render/src/cell_pipeline.rs (extension)
struct Uniforms {
    viewport_size_px: [f32; 2],
    cell_size_px: [f32; 2],
    selection_tint: [f32; 4],
    // NEW:
    border_color: [f32; 4],          // 0,0,0,0 = no border
    viewport_offset_px: [f32; 2],    // for per-pane Compositor (Pattern above)
    border_width_px: f32,            // 1.0 or 2.0
    _pad: f32,
}
```

Shader change: in fragment, after the existing fg/bg/atlas blend, compute `dist_to_viewport_edge_px` and if `< border_width_px && border_color.a > 0.0`, replace output with `border_color`. **Single uniform, no new pipeline, no new draw call.** Confirms D-66's "reuse Phase 3's per-cell tint uniform with a border-only mask" intent.

**Inactive cursor visibility (Claude's discretion → resolved here):** add a `cursor_focused: u32` uniform on the `cursor_pipeline`. Shader: when `focused == 0`, draw an outline (1-px stroke) instead of a filled rect. Trivial.

### Pattern: First-Paint Gate Generalization (D-51 per-pane)

Phase 3 has one `first_paint_ready: bool` on `App`. Phase 4 makes it per `TabWindow`:

```rust
pub struct TabWindow {
    first_paint_ready: bool,  // flips on first non-empty PaneOutput drain for ANY pane in this tab
    // …
}
```

**Why per-window, not per-pane:** the overlay (Phase 1 NSTextField) is one per NSWindow. Once *any* pane has produced output, drop the overlay for that window. New panes opened later (Cmd-D split) into an already-painted window don't need a separate gate — the window's `first_paint_ready` is already true.

Best practice from WezTerm/iTerm2 (verified by inspection): they don't have an overlay drop concern at all — they render immediately. Vector's overlay comes from Phase 1 D-12 and is a per-window concept; per-window gate is the correct shape.

### Pattern: WIN-04 Grep Invariant

Extend Phase 1 D-08's `crates/vector-term/tests/no_tokio_main.rs` arch-lint with a second check:

```rust
// crates/vector-term/tests/no_transport_discrimination.rs (new)
// WIN-04: vector-term must not discriminate on transport kind.

const FORBIDDEN: &[&str] = &[
    "enum PaneSource",
    "TransportKind::Local",
    "TransportKind::Codespace",
    "TransportKind::DevTunnel",
    ".kind() ==",
    "match transport.kind()",
    "match self.transport.kind()",
];

#[test]
fn vector_term_does_not_discriminate_on_transport() {
    // walks crates/vector-term/src/**/*.rs, asserts NONE contains any of FORBIDDEN
}
```

**Wider arch-lint upgrade (Plan 04-01 ships the test, planner extends the patterns):** add similar checks to other transport-agnostic crates (`vector-render`, `vector-input`, `vector-fonts`) — they're equally forbidden from peeking at transport kind. Phase 1 D-08's `no_tokio_main.rs` invariant counter goes from 15 to 16 (new test file added to vector-term) OR stays at 15 and the assertion is folded into the existing file. Planner's call; 16 is cleaner.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Process-name resolution from pid | Hand-FFI to `proc_pidpath` | `libproc::proc_pid::pidpath(pid)` | Tested, MIT, 1 line vs ~30 |
| cwd resolution from pid | Hand-FFI to `proc_pidinfo` + `PROC_PIDVNODEPATHINFO` + `vnode_info_path` struct unpacking | `libproc::proc_pid::pidcwd(pid)` | Same crate, same justification |
| Foreground-pgrp read | Custom `ioctl(TIOCGPGRP)` | `libc::tcgetpgrp(master_fd)` | One-line POSIX call, already in `libc` (transitive) |
| Native macOS tab grouping | Custom AppKit `NSWindowTabbingMode` enum + ObjC call sites | `winit::platform::macos::WindowExtMacOS::set_tabbing_identifier` | winit 0.30 handles 95% — only drop to objc2-app-kit if #2238 reproduces |
| Singleton init | `lazy_static!` or `once_cell::sync::Lazy` | `std::sync::OnceLock<Arc<Mux>>` | std-native, Rust 1.70+, zero dep |
| Split-tree library | `bintree` or hand-roll a generic tree | Plain `enum PaneNode` per D-67 | ~50 LoC; no abstraction tax |
| Cross-thread event signalling | Custom subscriber/notify pattern | `EventLoopProxy<UserEvent>` (already in tree) | D-09/D-10/D-11 — established Phase 1 pattern |
| Per-pane PTY runtime | One tokio runtime per pane | Single tokio runtime + `JoinSet<PaneId>` keyed by id | Idiomatic, cheap, scales to dozens of panes |
| Directional pane focus | Path-to-root + edge intersection from scratch | Port WezTerm's `get_pane_direction()` algorithm (under their Apache-2 license — reference only, do not vendor) | Algorithm is documented; ~80 LoC in our codebase |
| Atlas-aware multi-pane rendering | Single Compositor with merged damage tracking | One Compositor per pane (see "Compositor Strategy") | Less code, isolated state, trivial border + first-paint per pane |
| pid → child of LocalPty | Read /proc filesystem (doesn't exist on macOS) | `portable_pty::Child::process_id()` (already exposed by portable-pty 0.9) | Available; surface via `LocalPty::child_pid()` + `SpawnedPane { pid }` |

**Key insight:** Phase 4 is overwhelmingly *plumbing* — wire existing pieces together. The only genuinely new code is the split tree (~250 LoC), the directional focus algorithm (~80 LoC), the cwd/process tracker glue (~50 LoC), and the WIN-04 grep test (~30 LoC). Everything else is "extend Phase 3 by adding a `PaneId` parameter."

## Runtime State Inventory

> **Skipped — Phase 4 is greenfield additions (new mux types in `vector-mux/src/`, new test files, new menu items). No rename, refactor, migration, or string-replacement work.** No runtime state outside the repo to inventory.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| macOS 13+ (Ventura baseline) | NSWindowTabbingMode `.preferred` API | ✓ | 13+ required by project (PROJECT.md) | — |
| `winit::platform::macos::WindowExtMacOS` | `set_tabbing_identifier` etc. | ✓ | winit 0.30.13 (workspace-pinned) | objc2-app-kit `setTabbingMode:` direct call |
| `libproc` crate on crates.io | D-57 + D-63 | ✓ | 0.14.11 (latest 2026-05-11) | Hand-FFI to `libSystem` (worse but works) |
| `libc::tcgetpgrp` | fg-process group read | ✓ | libc transitive in workspace | — |
| `tokio::task::JoinSet` | Per-pane actor router | ✓ | tokio 1.52.3 (workspace) | `Vec<JoinHandle>` + manual reaping |
| macOS "Prefer tabs" system preference | NSWindowTabbingMode behavior | n/a | User-controlled | Document in README that "Always" is the friendliest setting |
| `proc_listpids` | Not needed — we know the child pid directly from `portable_pty::Child::process_id()` | n/a | — | — |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** None (libproc is on crates.io and stable).

## Validation Architecture

Per Nyquist Dimension 8 — this section is the bootstrap for `04-VALIDATION.md`.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test --workspace` over per-crate `tests/*.rs` integration files |
| Config file | `Cargo.toml` (workspace) + per-crate `Cargo.toml` |
| Quick run command | `cargo test --workspace --tests -q` |
| Full suite command | `cargo test --workspace --tests --release` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| WIN-02 (Cmd-T new tab) | `Mux::create_tab()` increments tab count + allocates pane | unit (vector-mux) | `cargo test -p vector-mux --test mux_topology` | ❌ Wave 0 |
| WIN-02 (Cmd-Shift-]/[ cycle) | keymap encoder emits the bind; mux next/prev tab call updates active | unit (vector-input + vector-mux) | `cargo test -p vector-input --test xterm_key_table` + `… --test mux_tab_cycle` | ❌ Wave 0 (extend existing xterm_key_table.rs + new mux test) |
| WIN-02 (Cmd-W cascade) | pane-then-tab-then-window-then-quit sequence | unit (vector-mux) | `cargo test -p vector-mux --test mux_close_cascade` | ❌ Wave 0 |
| WIN-02 (native tabs) | `NSWindowTabbingMode.preferred` set; `set_tabbing_identifier` called | manual-only | manual smoke matrix item #2 (visual: two NSWindows tab-grouped) | n/a |
| WIN-03 (Cmd-D / Cmd-Shift-D split) | Tab.root becomes HSplit / VSplit; both leaves have PaneIds | unit (vector-mux) | `cargo test -p vector-mux --test split_tree` | ❌ Wave 0 |
| WIN-03 (focus routing Cmd-Opt-Arrow) | `get_pane_direction(focused, Left)` returns expected PaneId | unit (vector-mux) | `cargo test -p vector-mux --test directional_focus` | ❌ Wave 0 |
| WIN-03 (per-pane resize) | window resize → split tree redistribute → `tput cols` matches | integration (vector-mux + vector-pty + real shell) | `cargo test -p vector-mux --test pane_resize_propagates -- --include-ignored` (real PTY, ~3 s) | ❌ Wave 0 |
| WIN-03 (Cmd-Shift-Arrow nudge) | split ratio shifts by 1 cell on each press | unit (vector-mux) | `cargo test -p vector-mux --test split_resize_nudge` | ❌ Wave 0 |
| WIN-04 (zero PaneSource in vector-term) | grep for forbidden patterns returns no hits | unit (vector-term arch-lint) | `cargo test -p vector-term --test no_transport_discrimination` | ❌ Wave 0 |
| D-57 (fg-process name updates tab title) | Spawn `sh`, then `exec sleep 5` in it, assert title transitions `sh` → `sleep` within 2s | integration | `cargo test -p vector-mux --test proc_name_tracking -- --include-ignored` | ❌ Wave 0 |
| D-63 (cwd inheritance) | `cd /tmp`, then split → new pane's `cwd_inherit()` returns `/tmp` | integration | `cargo test -p vector-mux --test cwd_inheritance -- --include-ignored` | ❌ Wave 0 |
| D-64 (cwd fallback to $HOME) | `libproc::pidcwd` returns Err → fall back to $HOME (mocked) | unit | `cargo test -p vector-mux --test cwd_fallback` | ❌ Wave 0 |
| D-66 (active-pane border) | Snapshot test: offscreen render with `border_color=Some(...)` shows 1-px border on the viewport edge | snapshot (vector-render offscreen) | `cargo test -p vector-render --test active_pane_border` | ❌ Wave 0 |
| RENDER-03 reaffirm (N-pane idle CPU < 1%) | manual: open 4 splits, idle 60s, Activity Monitor | manual-only | manual smoke matrix item #6 | n/a |

### Sampling Rate

- **Per task commit:** `cargo test --workspace --tests -q`
- **Per wave merge:** quick + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --all -- --check` + per-crate `no_tokio_main.rs` + new `no_transport_discrimination.rs`
- **Phase gate:** full suite (`--release`) green + 9-item manual smoke matrix signed off + WIN-04 arch-lint green + `arch-lint count == 16` (was 15, +1 for `no_transport_discrimination.rs`)

### Wave 0 Gaps

Wave-0 test stub seeding for Plan 04-01, mirroring Phase 3 Plan 03-01's pattern of `#[ignore = "Wave-0 stub"]` files:

- [ ] `crates/vector-mux/tests/mux_topology.rs` — covers WIN-02 (Cmd-T)
- [ ] `crates/vector-mux/tests/mux_tab_cycle.rs` — covers WIN-02 (Cmd-Shift-]/[)
- [ ] `crates/vector-mux/tests/mux_close_cascade.rs` — covers WIN-02 (Cmd-W)
- [ ] `crates/vector-mux/tests/split_tree.rs` — covers WIN-03 (Cmd-D / Cmd-Shift-D)
- [ ] `crates/vector-mux/tests/directional_focus.rs` — covers WIN-03 (Cmd-Opt-Arrow)
- [ ] `crates/vector-mux/tests/split_resize_nudge.rs` — covers WIN-03 (Cmd-Shift-Arrow)
- [ ] `crates/vector-mux/tests/pane_resize_propagates.rs` — covers WIN-03 success criterion #3 (`tput cols`)
- [ ] `crates/vector-mux/tests/proc_name_tracking.rs` — covers D-57
- [ ] `crates/vector-mux/tests/cwd_inheritance.rs` — covers D-63
- [ ] `crates/vector-mux/tests/cwd_fallback.rs` — covers D-64
- [ ] `crates/vector-term/tests/no_transport_discrimination.rs` — covers WIN-04
- [ ] `crates/vector-render/tests/active_pane_border.rs` — covers D-66
- [ ] `crates/vector-app/tests/multi_window_tabbing.rs` — verifies winit `set_tabbing_identifier` is called on every Cmd-T window (mock-driven; visual verification is manual)
- [ ] Extend `crates/vector-input/tests/xterm_key_table.rs` (already exists, Phase 3) with new cases: Cmd-Opt-Arrow / Cmd-Shift-Arrow / Cmd-D / Cmd-Shift-D / Cmd-T / Cmd-W / Cmd-Shift-]/[ — these must NOT emit PTY bytes (return None from keymap, handled at App layer)

Total new test files: **12**, plus 1 existing-file extension.

### Manual Smoke Matrix (continuation of Phase 3's 9-item, Phase 4 adds tabs/splits)

Plan 04-05's `checkpoint:human-verify`:

1. **Cmd-T spawns native NSWindow tab** — two tabs in one tab group; tab bar visible at title-bar top; switch tabs via tab bar click and Cmd-Shift-]
2. **Cmd-W cascade** — close last pane in a tab → tab closes; close last tab in a window → window closes; close last window → app quits (matches Cmd-Q semantics)
3. **Cmd-D horizontal split + Cmd-Shift-D vertical split** — two panes side-by-side then top-and-bottom in nested split; Cmd-Opt-Right routes focus
4. **`tput cols` round-trip** — split horizontally, run `tput cols` in each pane: should report `(total_cols - 1) / 2` and `total_cols / 2` (or thereabouts; exact distribution per cell-count storage)
5. **cwd inheritance** — `cd ~/personal/vector`, Cmd-D → new pane's prompt is in `~/personal/vector`
6. **N-pane idle CPU** — open 4 splits with idle shells; Activity Monitor shows <1% CPU after 60s (RENDER-03 reaffirm)
7. **Tab title tracks foreground process** — open vim in pane 1 → tab title becomes "vim"; quit vim → tab title returns to "zsh" within 2s
8. **Active-pane border** — focused pane shows 1–2 px accent-colored border; clicking another pane moves the border; inactive cursor renders as outline (per Claude's-discretion resolution)
9. **Window resize redistributes panes** — drag corner: all panes' split ratios preserved; nested splits scale; `tput cols` in each pane reflects new size
10. **(Phase 3 carryover #1)** `vim` renders in a single pane (RENDER-01 reaffirm)
11. **(Phase 3 carryover #4)** Retina ↔ external monitor swap with multiple panes open — all atlases clear + lazy-rerasterize (RENDER-04 reaffirm under N panes)

## Common Pitfalls

### Pitfall A: Subscriber callbacks instead of EventLoopProxy

**What goes wrong:** Copy WezTerm's `Mux::notify(subscribers)` + `Subscriber: FnMut(MuxNotification)` pattern wholesale. Now Mux events flow through *two* mechanisms (subscribers + EventLoopProxy) and the main thread receives some events twice, others not at all.
**Why it happens:** WezTerm has a richer cross-thread story (lua callbacks, persistent CLI clients, the mux server). We don't.
**Avoid:** **Only EventLoopProxy<UserEvent>.** Every event from mux to UI goes through `EventLoopProxy::send_event`. Mux methods that report state changes take a `&EventLoopProxy<UserEvent>` argument or close over a clone. No `Vec<Box<dyn Fn(MuxNotification)>>`.

### Pitfall B: Locking Mux across `await`

**What goes wrong:** `let panes = mux.panes.read(); let transport = panes.get(&id).unwrap().transport.write(&bytes).await;` deadlocks: the read lock is held across the .await, and another task tries to take a write lock on `panes`.
**Why it happens:** It's the natural shape if you don't think about it. The lookup-then-act-on-the-result idiom invites lock-across-await.
**Avoid:** Workspace-wide `clippy::await_holding_lock = "deny"` (D-11, already in place) is the compile-time guard. Idiom: `let arc = { let g = mux.panes.read(); g.get(&id).cloned() }; let bytes = arc.do_stuff().await;` — drop the lock before any await.

### Pitfall C: Per-pane PTY actor blocking other panes' I/O

**What goes wrong:** Single tokio task that round-robins over all panes' (resize, write, read) channels via `JoinSet::join_next()` instead of one task per pane. A slow `transport.write(bytes).await` on one pane blocks reads on others.
**Why it happens:** "Centralized router" feels safer than N independent tasks.
**Avoid:** **One task per pane** via `JoinSet::spawn`. Per-pane biased `select!` as in Phase 3. The router only owns the `mpsc::Sender` halves, never the actor loops themselves.

### Pitfall D: Resize event storms during drag

**What goes wrong:** macOS sends `WindowEvent::Resized` continuously during live drag (60Hz+ at the OS level). Naive code calls `Term::resize` + `transport.resize` + walks the split tree on every event → kernel SIGWINCH storm → shell can't keep up.
**Why it happens:** Phase 3 D-49 already debounces single-pane resize at 50 ms; Phase 4 must extend the debounce *per pane* in the split tree (a single window resize emits N pane resizes).
**Avoid:** Phase 3's `App::pending_resize: Option<(u16, u16)>` + `flush_pending_resize_if_quiescent` becomes per-`TabWindow` state. Inside it, the split tree's `redistribute()` runs once per quiescent flush; only then are per-pane `transport.resize` calls dispatched (via the per-pane resize_tx channel).

### Pitfall E: NSWindow first-tab quirk (winit issue #2238)

**What goes wrong:** First Cmd-T after app launch opens a separate NSWindow not grouped with the first window, even though both share the same tabbing identifier.
**Why it happens:** winit's NSWindow lifecycle vs. AppKit's tab-group lifecycle race condition (open issue, not fixed as of 2026-05).
**Avoid:** Document in manual smoke item #1. If reproducible on the target macOS, fall back to manual `setTabbingMode(NSWindowTabbingModePreferred)` via objc2-app-kit on each `WindowAttributes::default().build()`. Implementation: ~10 lines, drops below winit's helper.

### Pitfall F: `libproc::pidcwd` failure on zombie shells

**What goes wrong:** User runs `:q` in vim mid-split; the shell exits between `Cmd-D` keystroke and `libproc::pidcwd` call → pidcwd returns Err → split fails or new pane starts in `/`.
**Why it happens:** Race: between focus-pane shell PID being valid and the split actually executing.
**Avoid:** D-64 fallback chain — `pidcwd` Err → `$HOME` (NOT `/`). Trace-log at WARN. Tests: `cwd_fallback.rs` mocks the failure path.

### Pitfall G: Holding the Mux singleton during `Drop` of a Pane

**What goes wrong:** `impl Drop for Pane { fn drop(&mut self) { Mux::get().panes.write().remove(&self.id); } }` — if the pane is dropped while Mux is locked, deadlock. Worse: if Mux is being torn down (app exit), `Mux::get()` panics.
**Why it happens:** Reasonable impulse to "auto-clean up."
**Avoid:** Pane drop is a no-op. Closing logic lives in `Mux::close_pane(pane_id)`, called explicitly by the Cmd-W cascade handler. No `Drop` magic.

### Pitfall H: First-paint gate flipping per-pane instead of per-window

**What goes wrong:** Each pane has its own `first_paint_ready`; the overlay drops only after every pane has produced output. If a user opens an empty extra pane (e.g., a shell waiting for `read`) the overlay stays.
**Why it happens:** Naive generalization of D-51.
**Avoid:** Per-window (per-`TabWindow`) gate. ANY pane's first non-empty drain flips the window's gate. New panes opened *after* first paint don't re-engage the gate.

### Pitfall I (Pitfall 21 reaffirm): Scope creep into broadcast-input / layout save / leader-key

**What goes wrong:** Adding "small" features that turn Phase 4 into tmux-clone-lite.
**Avoid:** Pitfall 21 is the explicit scope guard. If a feature is not in CONTEXT.md `<decisions>`, it's deferred. Period.

## Code Examples

### Example 1: Mux::create_tab + split

```rust
// crates/vector-mux/src/mux.rs

impl Mux {
    pub async fn create_tab(
        &self,
        window_id: WindowId,
        cwd: Option<PathBuf>,
    ) -> Result<(TabId, PaneId)> {
        let pane_id = self.allocate_pane_id();
        let SpawnedPane { transport, pid, master_fd } = self.default_domain
            .spawn(SpawnCommand {
                argv: None,
                cwd,
                rows: 24,
                cols: 80,
                env: vec![],
            }).await?;
        let pane = Arc::new(Pane::new(pane_id, transport, pid, master_fd));
        self.panes.write().insert(pane_id, Arc::clone(&pane));
        let tab_id = self.allocate_tab_id();
        let tab = Tab {
            id: tab_id,
            root: PaneNode::Leaf(pane_id),
            active_pane_id: pane_id,
        };
        let mut windows = self.windows.write();
        let win = windows.entry(window_id).or_insert_with(|| Window::new(window_id));
        win.tabs.push(tab);
        win.active_tab_id = Some(tab_id);
        Ok((tab_id, pane_id))
    }

    pub async fn split_pane(
        &self,
        pane_id: PaneId,
        direction: SplitDirection,
        cwd: Option<PathBuf>,
    ) -> Result<PaneId> {
        let new_pane_id = self.allocate_pane_id();
        let SpawnedPane { transport, pid, master_fd } =
            self.default_domain.spawn(SpawnCommand { cwd, .. /* inherit dims */ }).await?;
        let new_pane = Arc::new(Pane::new(new_pane_id, transport, pid, master_fd));
        self.panes.write().insert(new_pane_id, Arc::clone(&new_pane));
        // Walk the tree to find pane_id leaf and replace with HSplit/VSplit.
        let mut windows = self.windows.write();
        let (tab, _win_id) = locate_tab_mut(&mut windows, pane_id)?;
        tab.root = split_at_leaf(std::mem::replace(&mut tab.root, PaneNode::Leaf(pane_id)),
                                  pane_id, new_pane_id, direction);
        tab.active_pane_id = new_pane_id;
        Ok(new_pane_id)
    }
}
```

### Example 2: Directional focus (Cmd-Opt-Right)

```rust
// crates/vector-mux/src/split_tree.rs

pub enum Direction { Left, Right, Up, Down }

pub fn get_pane_direction(tab: &Tab, from: PaneId, dir: Direction) -> Option<PaneId> {
    let viewport = TerminalSize { rows: tab.last_rows, cols: tab.last_cols };
    let layout = compute_layout(&tab.root, viewport);   // HashMap<PaneId, Rect>
    let src = layout.get(&from)?.clone();
    let mut best: Option<(PaneId, u16)> = None;          // (id, overlap_len)
    for (id, rect) in &layout {
        if *id == from { continue; }
        let overlap = edge_overlap(&src, rect, dir);
        if overlap == 0 { continue; }
        // Adjacency check: candidate must be on the far side of the relevant edge of `src`.
        if !is_adjacent_in_direction(&src, rect, dir) { continue; }
        match best {
            None => best = Some((*id, overlap)),
            Some((_, prev)) if overlap > prev => best = Some((*id, overlap)),
            Some((prev_id, prev)) if overlap == prev && id.0 < prev_id.0 =>
                best = Some((*id, overlap)),
            _ => {}
        }
    }
    best.map(|(id, _)| id)
}
```

### Example 3: WIN-04 arch-lint test

```rust
// crates/vector-term/tests/no_transport_discrimination.rs

use std::fs;
use std::path::Path;

const FORBIDDEN: &[&str] = &[
    "enum PaneSource",
    "TransportKind::Local",
    "TransportKind::Codespace",
    "TransportKind::DevTunnel",
    "transport.kind()",
    ".kind() == TransportKind",
    "match transport.kind",
];

#[test]
fn vector_term_does_not_discriminate_on_transport_kind() {
    let crate_root = env!("CARGO_MANIFEST_DIR");
    let src = Path::new(crate_root).join("src");
    let mut violations = vec![];
    walk(&src, &src, &mut violations);
    assert!(
        violations.is_empty(),
        "WIN-04 violation: vector-term must not discriminate on transport kind. Found:\n{}",
        violations.join("\n")
    );
}

fn walk(root: &Path, dir: &Path, violations: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() { walk(root, &p, violations); continue; }
        if p.extension().is_some_and(|e| e == "rs") {
            let body = fs::read_to_string(&p).unwrap();
            for f in FORBIDDEN {
                if body.contains(f) {
                    let rel = p.strip_prefix(root).unwrap().display();
                    violations.push(format!("  {rel}: `{f}`"));
                }
            }
        }
    }
}
```

### Example 4: Per-pane PTY actor spawn

```rust
// crates/vector-app/src/pty_actor.rs (sketch)

pub struct PtyActorRouter {
    proxy: EventLoopProxy<UserEvent>,
    pane_writers: HashMap<PaneId, mpsc::Sender<Vec<u8>>>,
    pane_resizers: HashMap<PaneId, mpsc::Sender<(u16, u16)>>,
    join_set: JoinSet<PaneId>,
}

impl PtyActorRouter {
    pub fn spawn_pane(
        &mut self,
        pane_id: PaneId,
        transport: Box<dyn PtyTransport>,
        coalesce: Arc<CoalesceBuffer>,
    ) {
        let (write_tx, write_rx) = mpsc::channel(64);
        let (resize_tx, resize_rx) = mpsc::channel(8);
        self.pane_writers.insert(pane_id, write_tx);
        self.pane_resizers.insert(pane_id, resize_tx);
        let proxy = self.proxy.clone();
        self.join_set.spawn(async move {
            pane_io_loop(pane_id, transport, proxy, coalesce, write_rx, resize_rx).await;
            pane_id
        });
    }

    pub fn send_write(&self, pane_id: PaneId, bytes: Vec<u8>) {
        if let Some(tx) = self.pane_writers.get(&pane_id) {
            let _ = tx.try_send(bytes);
        }
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `lazy_static!` for the Mux singleton | `std::sync::OnceLock<Arc<Mux>>` | Rust 1.70 (Jun 2023) | Drop a dep; std-native |
| `proc_pidinfo` + `PROC_PIDVNODEPATHINFO` hand-FFI | `libproc::proc_pid::pidcwd / pidpath` | libproc 0.10+ (2023) | One-line vs ~30 lines of FFI |
| Custom NSWindow tabbing via objc2 | winit `WindowExtMacOS::set_tabbing_identifier` | winit 0.30 (2024) | Higher-level API, less ObjC code |
| Custom subscriber/callback dispatch (WezTerm 1.0-era) | `EventLoopProxy::send_event` (winit's blessed pattern) | winit 0.27+ (2022) | Single-mechanism cross-thread signal |
| `glutin`-based windowing (Alacritty's choice) | `wgpu` + `winit` | Already Phase 3 decision | Cross-platform-ready renderer |

**Deprecated/outdated:**
- `cocoa-rs` for NSWindow tabbing: use `objc2-app-kit` (already pinned)
- `tokio_pty_process`: use `portable-pty` (already in tree)
- `bintree` crate for split trees: a plain `enum PaneNode` is sufficient at our scale

## Open Questions

1. **`Cargo.toml` workspace member ordering matters for arch-lint count.**
   - What we know: `no_tokio_main.rs` exists in 15 crates today (vector-mux/term/render/input/fonts/app/pty/codespaces/secrets/tunnels/ssh/theme/headless/config/ui).
   - What's unclear: whether Phase 4 adds `no_transport_discrimination.rs` as a *new* test file in vector-term (count goes from 15 → 16) or extends the existing `no_tokio_main.rs` in vector-term to include the new forbidden patterns (count stays at 15).
   - Recommendation: **new file** (`no_transport_discrimination.rs`) — keeps tokio-lint and transport-lint orthogonal. Plan must update the arch-lint count invariant from 15 to 16 in the appropriate place (likely a CI script or a doc).

2. **Is `child_pid` accessible from `portable_pty::Child` post-spawn, or only at spawn time?**
   - What we know: `portable_pty::Child` exposes `process_id() -> Option<u32>` per the crate's API.
   - What's unclear: whether the value remains valid after the child reparents (e.g., shell `exec`s another command, replacing pid in place — but the pid is preserved across exec, only the binary changes).
   - Recommendation: Plan 04-04 includes a smoke test that runs `exec true` in the shell, then queries `child_pid` — must be the same pid; if not, fall back to `tcgetpgrp` on the master fd as the canonical pid source (which is what we already use for D-57 anyway).

3. **Does winit's `set_tabbing_identifier` work BEFORE `EventLoop::run_app` starts, or only after a Window exists?**
   - What we know: it's a method on `Window`, so it requires a Window first.
   - What's unclear: whether it's a no-op if called on the initial window (which doesn't yet have peers to tab with) or whether it pre-registers the window for future tabbing.
   - Recommendation: call it on EVERY window at creation time, including the first (Phase 3 `resumed()`). Then issue #2238's "first window not in a tab" risk is purely about the second window vs. first, not about whether tabbing is "armed."

4. **Multi-pane Compositor: when the active pane changes, do we redraw both panes (old loses border, new gains border) or only the new one?**
   - What we know: per-pane Compositor architecture lets each pane manage its own border uniform independently.
   - What's unclear: whether changing `border_color` on one pane's Compositor uniform automatically triggers a redraw, or whether the App must `request_redraw()` explicitly.
   - Recommendation: explicit `request_redraw()` after every focus change. The Pane's Compositor uniform is a buffer write, not a draw; the app must repaint the affected panes (both old + new — old to drop its border, new to gain).

## Sources

### Primary (HIGH confidence)

- WezTerm `mux/src/lib.rs` source (Mux singleton, panes/tabs/windows HashMap, subscriber pattern) — `https://raw.githubusercontent.com/wezterm/wezterm/main/mux/src/lib.rs`
- WezTerm `mux/src/tab.rs` source (recursive split tree `Tree = bintree::Tree<Arc<dyn Pane>, SplitDirectionAndSize>`, `get_pane_direction` algorithm, cell-count split sizing, `apply_sizes_from_splits` resize propagation) — `https://raw.githubusercontent.com/wezterm/wezterm/main/mux/src/tab.rs`
- winit 0.30 `WindowExtMacOS` docs — `https://docs.rs/winit/latest/x86_64-apple-darwin/winit/platform/macos/trait.WindowExtMacOS.html` (`set_tabbing_identifier`, `tabbing_identifier`, `select_next_tab`, `select_previous_tab`, `select_tab_at_index`, `num_tabs`)
- WezTerm tab key tables (`wezterm.org/config/key-tables.html`) — directional focus default bindings (Ctrl+Shift+Arrow; Vector overrides to Cmd-Opt-Arrow per D-59)
- WezTerm `get-pane-direction` CLI doc — confirms the direction enum (Up/Down/Left/Right/Next/Prev)
- libproc-rs crate docs (`https://docs.rs/libproc/latest/libproc/`) — `proc_pid::pidpath`, `proc_pid::pidcwd`, MIT, version 0.14.11 (2026-05-11)
- Existing Phase 3 source: `crates/vector-app/src/{app,pty_actor,frame_tick,input_bridge,render_host,menu}.rs`, `crates/vector-render/src/compositor.rs`, `crates/vector-input/src/{keymap,selection,mods}.rs`, `crates/vector-mux/src/{lib,domain,transport,local_domain}.rs`, `crates/vector-term/src/{lib,term}.rs` — all read in full as research input
- `.planning/research/ARCHITECTURE.md` §"Pattern 2: Domain" + "Recommended Project Structure" — Mux ↔ Domain seam
- `.planning/research/PITFALLS.md` §Pitfall 8 + Pitfall 21 + Pitfall 22 — scope guards
- `./CLAUDE.md` §"Stack Patterns by Variant" — NSWindowTabbingMode + hand-rolled splits directives

### Secondary (MEDIUM confidence)

- winit issue #2238 (`https://github.com/rust-windowing/winit/issues/2238`) — first-dynamic-window-not-tabbed quirk, still open
- Apple Terminal / ghostty / iTerm2 reference behaviors for Cmd-W cascade, Cmd-Opt-Arrow focus, foreground-process tab title (verified by user direction in CONTEXT.md, not by source inspection)
- ghostty's use of `libproc` for the same purpose (Cmd-D cwd inheritance) — inferred from dependency graph, not directly inspected

### Tertiary (LOW confidence — needs validation during planning)

- "1Hz polling at <0.1% CPU is what ghostty does for fg-process tracking" — asserted in ghostty community discussions; not measured directly on Vector yet. Plan 04-05 manual smoke item #6 (idle CPU with N panes) is the indirect verification.
- The exact memory cost of "N per-pane 2048×2048×2 RGBA atlases" — back-of-envelope ~10 MiB × N. Real measurement deferred to Plan 04-05 smoke; if it surfaces as a problem (e.g., N=8 panes × 16 MiB = 128 MiB unexpected RAM), the fallback is to share the atlas as a wgpu `BindGroup` reference between Compositors (no architectural change, just a `Arc<Atlas>` shared via the per-window state).

## Metadata

**Confidence breakdown:**
- Standard stack (libproc 0.14): HIGH — docs.rs verified, single new dep
- Mux topology (Mux::get + binary split tree): HIGH — WezTerm source inspected, ownership model matches D-67 verbatim
- Native NSWindowTabbingMode integration: MEDIUM — winit 0.30 helper covers 95%; objc2-app-kit fallback path documented for #2238 quirk
- Per-pane PTY actor extension: HIGH — generalizes Phase 3 pty_actor cleanly via JoinSet
- Compositor strategy (per-pane vs shared): HIGH — per-pane wins on every dimension at our scale
- Directional focus algorithm: MEDIUM — WezTerm pattern is well-documented; the from-scratch port is well-trod but Vector hasn't shipped it yet
- WIN-04 grep invariant: HIGH — direct extension of Phase 1 D-08 pattern
- proc_pidinfo + tcgetpgrp tracking: HIGH — libproc + libc both standard
- Validation architecture (test map + Wave 0 stubs): HIGH — mirrors Phase 3 Plan 03-01's proven pattern

**Research date:** 2026-05-11
**Valid until:** 2026-06-10 (30 days; stack is stable. Re-validate if Phase 4 planning slips past June 2026 — winit and libproc both have monthly release cadence.)

---
*Researched 2026-05-11 by gsd-researcher for Phase 4: Mux — Tabs & Splits.*
