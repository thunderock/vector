---
phase: 04-mux-tabs-splits
gathered: 2026-05-11
status: Ready for planning
discuss_mode: discuss
---

# Phase 4: Mux — Tabs & Splits — Context

**Gathered:** 2026-05-11
**Status:** Ready for planning

<domain>
## Phase Boundary

A user can open a new tab with Cmd-T and split a pane with Cmd-D / Cmd-Shift-D, with each pane running an independent local shell. Focus routes spatially between split panes via Cmd-Opt-Arrow. The `Domain / Pane / PtyTransport` seam is the only contract between terminal model and transport — Phases 7/8/9 will plug remote transports into the same shape with zero changes to `vector-term`.

**Covers requirements:** WIN-02, WIN-03, WIN-04

**Explicitly out of phase (deferred):**
- Cmd-N multi-window → Phase 5
- Layout save/restore, broadcast-input, leader-key chord motion → Pitfall 21 scope guard, **not in v1 at all**
- OSC 7 cwd tracking (used as canonical cwd source for inheritance) → Phase 5; Phase 4 ships `proc_pidinfo` fallback
- Cmd-F search overlay → Phase 5
- Cmd-C copy + selection-to-string → Phase 5
- Mouse-reporting modes (DEC 1006/1015/1016 → PTY) → Phase 5
- Remote tab tint + remote badge (CS-06) → Phase 7; Phase 4 ships Unicode-prefix scaffolding only

</domain>

<decisions>
## Implementation Decisions

### Tab bar style

- **D-56:** **Native `NSWindowTabbingMode.preferred`.** One `NSWindow` per tab; AppKit groups them into the system-drawn tab bar at the top of the title bar. Matches Apple Terminal and ghostty. Far less code than a custom wgpu-drawn bar; CLAUDE.md "Stack Patterns by Variant" explicitly recommends this approach. WezTerm's hand-drawn bar is overkill for v1 macOS-only. The trade — the bar's appearance is whatever macOS chooses, no custom theming — is acceptable.

- **D-57:** **Tab title = foreground process name, tracked dynamically.** Each pane tracks its PTY's foreground process group (`tcgetpgrp` on the master fd) → resolves to a process name via `proc_pidpath` / `comm`. Tab title updates as the user runs commands (e.g., `zsh` → `vim` → `zsh`). Matches Apple Terminal default. The Phase 1 menu bar stays installed once; key/menu events route to whichever NSWindow AppKit reports as `keyWindow`.

- **D-58:** **CS-06 remote-tab differentiation = plan as Unicode-prefix; revisit in Phase 7.** Phase 4 leaves a hook (the Tab title is derived from `Domain.label() + ": " + foreground_process` — currently always `Local: vim`-style), so Phase 7 can produce `☁ codespace-name: vim` titles purely via string composition. No AppKit accessoryView plumbing in Phase 4. If pure-text proves insufficient in Phase 7, that's Phase 7's call.

### Focus + split keymap + close semantics

- **D-59:** **Cmd-Opt-Arrow for directional pane focus.** Cmd-Opt-Left / Right / Up / Down moves focus to the spatial neighbor across split boundaries. Matches ghostty + iTerm2. No Cmd-[/] cycle alternative (avoid keymap doubling, avoid Cmd-[ conflicts with vim-in-pane). Recursive binary split tree traversal: from a Leaf, find the nearest ancestor split in the right direction, descend the opposite child.

- **D-60:** **Pane resize = mouse drag on divider + Cmd-Shift-Arrow keyboard nudge.** Drag the divider line for visual resize; Cmd-Shift-Left/Right shrinks/grows the active pane's horizontal axis in 1-cell increments; Cmd-Shift-Up/Down does vertical. Stored as cell-ratio (not pixels) in the split node so window resize preserves proportions.

- **D-61:** **Cmd-W = close pane → fallback to close tab → fallback to close window → fallback to quit app.** Ghostty-style cascade. Implementation: `Cmd-W` always targets the focused pane; if that pane is the only Leaf in its Tab, close the Tab; if the Tab is the only Tab in the Window, close the Window; if the Window is the only one, terminate the app (matches existing Cmd-Q semantics from D-15). No separate Cmd-Shift-W.

- **D-62:** **Tab cycling = Cmd-Shift-]/[ as specified in ROADMAP.** Standard macOS browser-style cycling. Goes through the AppKit-managed tab group order. No Cmd-1..9 jump-to-tab in v1.

### Split cwd inheritance

- **D-63:** **Inherit cwd via `proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, ...)` for both Cmd-D split and Cmd-T new tab.** macOS libproc API (already on every Mac via the system libSystem; no new dep needed beyond a small FFI binding or the `libproc` crate). Resolve the active pane's shell PID's working directory and set it as the new pane's `SpawnCommand.cwd`. When OSC 7 lands in Phase 5, swap the inheritance source from proc_pidinfo to the OSC-7-reported cwd (more accurate — it tracks user-visible `cd` even when foreground proc is e.g. vim).

- **D-64:** **Cwd inheritance fallback chain.** If `proc_pidinfo` fails (zombie shell, permissions edge case, child died mid-split): fall back to `$HOME` and trace-log the failure. Symlinks: take whatever proc_pidinfo returns (resolved, not the symlink path) — matches tmux behavior. No special handling for SIP-protected directories — if the shell ran from one, the new pane will too via inheritance.

### Multi-window scope guard

- **D-65:** **Cmd-N (new window) is DEFERRED to Phase 5.** The File menu keeps "New Window" disabled. The Mux singleton must support multiple `Window`s internally regardless (native NSWindowTabbingMode is implemented by AppKit as N grouped NSWindows), so the architecture is in place — only the user-facing shortcut is gated. Phase 5 wires up Cmd-N as part of broader polish.

### Active-pane indicator

- **D-66:** **Thin colored border on the focused pane.** 1–2 px border in an accent color around the active pane. Reuse Phase 3's per-cell tint uniform with a border-only mask (cheap — no new pipeline). Matches ghostty / iTerm2 default. No dimming of inactive panes (Phase 3's selection_tint is already used for selection; reuse it for the border but with a different uniform binding).

### Mux architecture (Claude's discretion, locked by research)

- **D-67:** **`Mux::get()` singleton + recursive binary split tree** per `.planning/research/ARCHITECTURE.md`. WezTerm pattern. `Mux` owns a `Vec<Window>`; each `Window` owns a `Vec<Tab>`; each `Tab` owns a `Pane = Leaf(PaneId) | HSplit(Box<Pane>, Box<Pane>, ratio) | VSplit(Box<Pane>, Box<Pane>, ratio)`. `PaneId → (Arc<Mutex<Term>>, Box<dyn PtyTransport>, FocusState)` lookup in a `HashMap` owned by `Mux`. Cross-thread signaling continues to use `EventLoopProxy<UserEvent>` per D-09/D-10/D-11.

### Claude's Discretion

These are downstream-agent calls — researcher/planner pick the best approach without re-asking the user:

- **`vector-ui` crate decision** — `.planning/research/ARCHITECTURE.md` proposes a separate `vector-ui` crate for chrome. For Phase 4, planner may either land `vector-ui` now (hosting the split-border-uniform code, pane-divider hit-testing, etc.) or fold split chrome into `vector-render` and create `vector-ui` later when Phase 6's Codespaces picker actually needs non-grid UI. Either decision is acceptable as long as the crate boundary stays clean.
- **Tab close animation / drag-to-reorder** — whatever NSWindowTabbingMode gives us natively is fine. No custom animation work in Phase 4.
- **Maximum splits per tab** — no hard limit needed; rely on minimum-pane-size enforcement during resize to prevent absurd nesting.
- **Pane minimum size** — pick a sensible floor (e.g., 20×4 cells) below which a split is rejected with a no-op + trace log. Planner's call on exact number.
- **Per-pane process-exit policy** — when a pane's shell exits, mark the pane "exited", show a sentinel line (e.g., `[Process completed]` like Apple Terminal), and require Cmd-W or Cmd-R-to-restart to close/reuse it. No auto-close-on-exit.
- **Cursor visibility in inactive panes** — show hollow/outlined cursor in inactive panes vs filled in active (the cursor pipeline from Plan 03-03 already takes a uniform; flip a `focused` bit).
- **PaneId allocator** — monotonic `u64` from a `Mux`-owned `AtomicU64`. Standard.

### Folded Todos

None for Phase 4. The pending `code-quality-hardening` todo (workspace lints, arch-lint upgrade, pre-commit cargo-deny) is correctly scoped to Phase 5 per its frontmatter (`target_phase: 5`) — it surfaces from `/gsd-tools todo match-phase 4` with score 0.6 only via generic keyword overlap (`phase`, `crate`).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents (researcher, planner, executor) MUST read these before research or implementation.**

### Phase 1 carryover (still binding)
- `.planning/phases/01-foundation-ci-dmg-pipeline/01-CONTEXT.md` — D-01..D-33; especially:
  - D-09 / D-10 / D-11 (winit main thread, tokio I/O thread, no `.await` across `parking_lot::Mutex`)
  - D-14 (single 1024×640 window — Phase 4 extends to N windows via NSWindowTabbingMode)
  - D-15 (standard menu bar; File → New Tab / Close already wired but disabled — Phase 4 enables them)

### Phase 2 carryover (still binding)
- `.planning/phases/02-headless-terminal-core/02-CONTEXT.md` — D-36..D-39; especially:
  - D-38 (`Domain`/`PtyTransport` trait shape FINAL — Phase 4 wires Pane/Tab/Window on top, never touches the traits)
- `.planning/phases/02-headless-terminal-core/02-04-SUMMARY.md` — `LocalDomain::spawn` + `LocalTransport` reference impl

### Phase 3 carryover (still binding)
- `.planning/phases/03-gpu-renderer-first-paint/03-CONTEXT.md` — D-40..D-55; especially:
  - D-44 / D-45 / D-47 (frame pacing + dirty-row damage + PTY-burst coalescing — must extend per-pane)
  - D-51 (first-paint gate — must generalize for N panes; gate flips once *any* pane has a first PTY drain)
  - D-52 (xterm key table — D-59/D-60/D-61 extend this with Cmd-Opt-Arrow, Cmd-Shift-Arrow, Cmd-W; new entries must follow the same encoding pattern)
  - D-55 (Phase 3/4 boundary — Cmd-T/Cmd-W menu items in place, ready for Phase 4 handlers)
- `.planning/phases/03-gpu-renderer-first-paint/03-03-SUMMARY.md` — `Compositor::render(&mut Term, selection)` API; Phase 4 must extend to render N panes (one Compositor per pane vs single Compositor multiplexed by Mux is planner's call)
- `.planning/phases/03-gpu-renderer-first-paint/03-04-SUMMARY.md` — `vector-input` keymap / selection / paste already extensible; D-59/D-60/D-61 keymap additions land in `keymap.rs`

### Project-level
- `.planning/PROJECT.md` — Core value, v1 scope discipline (Pitfall 21 boundaries)
- `.planning/REQUIREMENTS.md` §WIN-02..WIN-04 — acceptance criteria for this phase
- `.planning/ROADMAP.md` §"Phase 4: Mux — Tabs & Splits" — goal + 4 success criteria + risks & notes (Pitfall 21 callout)
- `./CLAUDE.md` §"Stack Patterns by Variant" — "Tabs: use NSWindow native tabs via setTabbingMode(.preferred)"; "Splits: hand-rolled. There is no Rust crate for this. Both WezTerm and ghostty implement their own pane manager."

### Architecture & Patterns
- `.planning/research/ARCHITECTURE.md` §"Pattern 2: Domain" + §"Pattern 3: Triple-loop threading" — Mux singleton, Domain trait, threading discipline
- `.planning/research/ARCHITECTURE.md` §"Recommended Project Structure" — vector-mux crate boundary; "`vector-mux` lives between term and UI, exactly like WezTerm's `mux` crate sits between `term` and `wezterm-gui`"
- `.planning/research/PITFALLS.md` §"Pitfall 8" — tmux + remote terminal layering (`TERM=xterm-256color` only; don't try to out-multiplex remote tmux)
- `.planning/research/PITFALLS.md` §"Pitfall 21" — "Vim-style modal pane navigation or built-in multiplexing exceeding tmux" — explicit scope guard for Phase 4: splits + tabs ONLY, no layout save/restore, no broadcast-input

### External references (not stored locally, planner/researcher may fetch)
- Apple `NSWindowTabbingMode` docs — `setTabbingMode(.preferred)`, `tabGroup`, `addTabbedWindow(_:ordered:)`
- Apple `proc_pidinfo` / `proc_pidpath` man pages (libproc) — for D-57 process-name tracking and D-63 cwd inheritance
- WezTerm `mux` crate source — reference for Mux::get() singleton + split tree; in particular `wezterm/mux/src/lib.rs` for the Window/Tab/Pane ownership model
- ghostty source — reference for native tab + per-pane cwd inheritance behavior

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`crates/vector-mux/src/{domain,transport,local_domain,codespace_domain,devtunnel_domain}.rs`** — Phase 2 ships the trait surface. Phase 4 adds `mux.rs`, `window.rs`, `tab.rs`, `pane.rs` (or planner's preferred layout) without touching the existing trait files.
- **`crates/vector-app/src/menu.rs`** — File → New Tab (Cmd-T) and File → Close (Cmd-W) already installed but disabled (D-15). Phase 4 enables them and adds Cmd-D / Cmd-Shift-D / Cmd-Opt-Arrow / Cmd-Shift-Arrow / Cmd-Shift-]/[ entries.
- **`crates/vector-app/src/app.rs`** — single-window `App` struct from Phase 3 with `term: Arc<Mutex<Term>>` and `render_host: Option<RenderHost>`. Phase 4 refactors this into per-Window state owned by `Mux`. The first-paint gate (D-51) generalizes to per-pane.
- **`crates/vector-app/src/pty_actor.rs`** — Phase 3 single-PTY actor with biased select over resize/write/read. Phase 4 spawns one actor per pane, each owning its `Box<dyn PtyTransport>`.
- **`crates/vector-render/src/compositor.rs`** — `Compositor::render(&mut Term, selection)` returns rendered cells. Phase 4 either creates one Compositor per pane and composites their outputs into the final surface, or extends Compositor to take a `&[Viewport]` and render N grids in one pass. Planner's call.
- **`crates/vector-input/src/{keymap,selection}.rs`** — already structured for D-52 xterm key table; D-59/D-60/D-61 additions extend `keymap.rs::encode_key` with Cmd-Opt-* and Cmd-Shift-* mux shortcuts before falling through to PTY-bound keys. Selection state stays per-pane.
- **`crates/vector-mux/src/local_domain.rs`** — `LocalDomain::spawn(SpawnCommand)` accepts `cwd: Option<PathBuf>` (D-38). Phase 4 fills `cwd` from `proc_pidinfo` lookup (D-63).
- **`crates/vector-ui/src/lib.rs`** — empty stub from Phase 1. Phase 4 decides whether to populate it (split chrome) or leave for Phase 6.

### Established Patterns
- **Threading split (D-09/D-10/D-11):** winit + AppKit + wgpu on main; tokio I/O on background; `parking_lot::Mutex` held only synchronously; cross-thread signaling via `EventLoopProxy::send_event`. Phase 4 extends `UserEvent` enum with mux-related variants (`PaneOutput(PaneId, Vec<u8>)`, `PaneExited(PaneId)`, etc.).
- **Per-crate arch-lint (D-08):** `tests/no_tokio_main.rs` invariant = 15. Any new file in any crate's `src/` must keep the grep test green. New mux types in `vector-mux/src/` get the same scrutiny.
- **Workspace lints / clippy::pedantic / await_holding_lock = "deny":** Phase 4 code must pass these without per-crate allows (matches Phase 3's clean clippy posture).
- **Bundled assets via `cargo-bundle`:** Phase 1 bundles `icon.icns`; Phase 3 bundles `JetBrainsMono-Regular.ttf`. No new assets in Phase 4.
- **Render-on-dirty (D-44):** Phase 4 extends damage from "Term row dirty" to "Pane dirty (one of its Term rows changed, OR focus state changed, OR resize)." Idle CPU < 1% (RENDER-03) must still hold with N panes that are all idle.

### Integration Points
- **Mux ↔ App:** App owns `Arc<Mutex<Mux>>` (singleton via `OnceLock`). winit `WindowEvent`s route via PaneId (lookup the active pane in the active tab in the window that received the event).
- **Mux ↔ vector-mux Domains:** Pane construction goes through `Domain::spawn(SpawnCommand { cwd: Some(inherited_cwd), .. })` → `Box<dyn PtyTransport>`. Phase 4 only uses `LocalDomain`; Phase 7 will inject `CodespaceDomain` at the same call site.
- **Mux ↔ vector-render:** Each pane has a `Compositor` (or shares one with viewport state — planner's call) bound to a sub-region of the parent NSWindow's wgpu surface. Border drawing reuses the cell-pipeline's tint uniform with a border-only mask.
- **Mux ↔ vector-input:** Keymap branches on the modifier set BEFORE falling through to the xterm key table — Cmd-Opt-Arrow / Cmd-Shift-Arrow / Cmd-D / Cmd-T / Cmd-W / Cmd-Shift-]/[ never reach the PTY.
- **Process-name tracking (D-57):** A periodic poll (e.g., every 1s on the I/O thread) calls `tcgetpgrp` + `proc_pidpath` per pane and emits `UserEvent::PaneTitleChanged(PaneId, String)` only on transition. Title diffing lives in `vector-mux`, not `vector-app`.

</code_context>

<specifics>
## Specific Ideas

- **The `Domain/Pane/PtyTransport` seam is load-bearing.** Phase 7 (Codespaces SSH), Phase 8 (Dev Tunnels), and Phase 9 (Persistence/reconnect) all plug into the trait shape locked in Phase 2 D-38. Phase 4 must NOT add convenience methods on `Term` that branch on transport (Architecture Anti-Pattern 1). The success-criterion-#4 grep (`enum PaneSource` inside `vector-term`) must remain at zero hits.
- **Daily-driver feel matters more than feature count.** The four locked keybinding decisions (Cmd-Opt-Arrow / Cmd-Shift-Arrow / Cmd-W cascade / Cmd-Shift-]/[) are deliberate ghostty-style choices — replicate the muscle memory of a polished native terminal, not the leader-key chord pattern of tmux. Don't add tmux-style chord modes "just in case" — Pitfall 21.
- **Tab title transitions should be smooth, not flickery.** D-57 polling at 1Hz (or event-driven via `kqueue` `EVFILT_PROC` on the shell PID if planner finds it cleaner) is fine; the title should update when the user runs `vim` → on screen within 1s.
- **Splitting a pane in cwd `/Users/ashutosh/personal/vector` should produce a new shell already in that directory.** Canonical smoke test: open Vector, `cd ~/personal/vector`, Cmd-D, observe new pane prompts in `~/personal/vector`. Mirrors tmux + iTerm2 default.

</specifics>

<deferred>
## Deferred Ideas

### Phase 5 (Polish)
- **Cmd-N (new window) shortcut** (D-65) — Mux architecture supports it; File menu has it disabled; Phase 5 wires the handler
- **OSC 7 cwd tracking** + replace `proc_pidinfo` fallback with OSC-7-derived inheritance (D-63 fallback)
- **Cmd-F search overlay** — per D-39, Phase 5 owns user-facing search UI
- **Cmd-C copy + selection-to-string** — per D-53
- **Mouse-reporting modes (DEC 1006/1015/1016 → PTY)** — per D-54
- **Per-pane ligature toggle, per-domain font config** — per D-42

### Phase 7
- **Remote-tab tint / "remote" badge** (CS-06) — per D-58 hook; Phase 7 either composes Unicode-prefix titles or escalates to AppKit accessoryView if pure-text proves weak

### Out of scope (Pitfall 21 / scope guard, NOT a future-phase deferral)
- **Layout save/restore** — never. v1 ships transient mux state only.
- **Broadcast-input across panes** — never. `tmux setw synchronize-panes on` is the answer.
- **Leader-key chord modes** ("prefix-c for new tab" tmux style) — never. Direct shortcuts only.
- **"Maximize current pane" zoom toggle** — explicitly scope-creep per Pitfall 21; if the user wants this in v2, plant a seed then.
- **Custom in-window tab bar drawn in wgpu** — chose native (D-56); revisiting is not a Phase-5 task, only a Phase-7 escalation if CS-06 demands it.

### Backlog
- **999.1 AI autocomplete + history-aware Claude suggestions** — orthogonal; needs Mux in place before per-pane suggestions can be composed.

### Reviewed Todos (not folded)
- **2026-05-11 Code-quality hardening — workspace lints, arch-lint upgrade, pre-commit cargo-deny** (`target_phase: 5`) — out of scope for Phase 4; its frontmatter targets Phase 5 (Polish). Match was generic keyword overlap (`phase`, `crate`), not topical relevance.

</deferred>

---

*Phase: 04-mux-tabs-splits*
*Context gathered: 2026-05-11*
