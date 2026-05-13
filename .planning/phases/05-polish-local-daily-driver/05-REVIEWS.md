---
phase: 5
reviewers: [claude]
reviewed_at: 2026-05-13T03:38:03Z
plans_reviewed:
  - 05-11-PLAN.md
  - 05-12-PLAN.md
  - 05-13-PLAN.md
  - 05-14-PLAN.md
  - 05-15-PLAN.md
  - 05-16-PLAN.md
review_scope: gap-closure plans (10 completed plans 05-01..05-10 included as summary-only context)
note: |
  gemini and codex CLIs not installed on this system; only `claude` CLI was available.
  Since the orchestrating runtime is also Claude, this is not a fully independent
  cross-AI review — single-reviewer with same model family. User explicitly opted to
  proceed. Re-run with gemini or codex installed for stronger adversarial coverage.
---

# Cross-AI Plan Review — Phase 5

## Claude Review

### Summary

The six gap-closure plans (05-11..05-16) form a coherent, well-structured closure for the "logic-complete, wiring-incomplete" pattern the verifier surfaced. The dependency chain is sound: 05-13 and 05-11 run in parallel (they touch disjoint crates), 05-12 follows 05-11, 05-15 follows 05-12, 05-14 follows 05-13 and 05-15, and 05-16 closes at the tail. Every gap in 05-VERIFICATION.md maps to at least one plan. The scope stays firmly inside Phase 5 — no Phase 6 transport wiring leaks in. The main execution risks are (1) the objc2 0.6 `declare_class!` macro in 05-15 is the hardest technical step and carries real API-churn risk, (2) 05-16 Task 2 is large and the borrow structure for the per-frame chrome pass against a mutably borrowed `RenderHost` needs careful attention, and (3) 05-12's channel-plumbing through the Mux pane spawn path requires verifying the actual `LocalDomain`/`Pane` constructor chain before coding.

### Strengths

- **Correct dep-cycle avoidance in 05-11.** The `TermGridAccess` newtype placed in `vector-app` (not `vector-term`) correctly avoids the `vector-input → vector-mux → vector-term → vector-input` cycle. The plan document explains the rationale clearly and the acceptance criterion greps enforce it (`cargo build -p vector-app` must succeed with no dep-cycle error).
- **AppShortcut as a parallel enum to MuxCommand (05-13).** Using `EncodedKey::App(AppShortcut)` rather than extending `MuxCommand` cleanly separates concerns: Mux keys route through the mux command handler; App/chrome keys route to the App's event loop directly. This avoids polluting the mux crate with chrome-level concerns.
- **Serialization via `depends_on` rather than `merge_order_in_wave`.** The plan set correctly uses `depends_on` for plans that all modify `app.rs`, preventing parallel-execution git-index collisions that plagued Plans 05-01..05-10 (as documented in those SUMMARYs).
- **Pure-logic test extraction for chrome surfaces (05-16).** The `ChromeDrawPlan` struct and `chrome_draw_plan()` helper allow verifying the conditional dispatch logic without a wgpu surface in `cargo test`. The W6 draw-order test (reading `app.rs` as a string and asserting monotonically increasing byte offsets) is an unusual but effective way to enforce UI-SPEC §11 ordering.
- **submenu_rows_for() pure-Rust testability (05-11).** By splitting `submenu_rows_for(cfg) -> Vec<(String, bool)>` from the AppKit-touching `rebuild_switch_profile_submenu()`, the data-shape logic is testable in `cargo test` without AppKit runtime. This follows the established project pattern (pure logic testable; AppKit gated).
- **Toast tick site correctly placed in render_window (05-16).** Calling `toasts.tick(Instant::now())` at the top of `render_window` is the right place — it ties auto-dismiss to the render cadence, which is appropriate for a 5-second info toast in a terminal that redraws on events.
- **W7 write_tx clone-before-move fix is explicit (05-15).** The plan documents the ownership ordering hazard (InputBridge takes `write_tx` by value; `ImeState::new` needs a clone) and provides the exact fix with a verification grep. This is a real Rust ownership footgun that many plans would silently break.

### Concerns

**[HIGH] 05-15 Task 1: objc2 0.6 `declare_class!` syntax is version-sensitive and the plan's pseudocode is not verified.**

The plan body says "NOTE: confirm exact objc2 0.6 macro syntax... before committing." The `Ivars` type using `Mutex<ImeState>`, the `DeclaredClass` trait, and especially accessing `self.ivars().lock()` from within an `unsafe impl` block may not compile against objc2 0.6.4 as written. In objc2 0.6, `DeclaredClass::Ivars` must be `Send + Sync`; `Mutex<ImeState>` is only `Send + Sync` if `ImeState: Send + Sync`. `ImeState` contains `mpsc::Sender<Vec<u8>>` which is `Send` but the struct itself isn't declared `Send` explicitly. Additionally, the `msg_send_id![text, string]` pattern for extracting a `NSString` from an `NSObject` or `NSAttributedString` may need the `objc2-foundation` `NSAttributedString` import and a protocol conformance check. The plan must be validated against the actual objc2 0.6.4 API surface before the executor begins — the compiler errors here could require structural changes to the approach.

**[HIGH] 05-16 Task 2: Borrow structure for chrome passes against RenderHost is unresolved.**

The plan acknowledges "Adjust borrow rules so `host` is not double-borrowed" but does not provide a concrete solution. In the existing `render_window`, `host` is borrowed via `aw.render_host.as_mut()` for the compositor loop. The chrome pass block then needs to call `host.tint_mut()`, `host.search_bar_pass()`, `host.queue()`, and `host.device()` — potentially simultaneously. The `tint_mut()` accessor returning `(&mut TintStripePipeline, &Queue, (u32,u32))` requires `&mut self` on RenderHost, while the other accessors take `&self`; since `tint.draw()` takes `&'a self` on `TintStripePipeline` while the render pass is also `'a`-parameterized, the borrow checker will reject naive approaches. The plan should either: (a) move chrome pipeline ownership into a dedicated `ChromeState` struct that can be borrowed independently of `RenderHost`'s surface/compositor, or (b) accept that chrome pipelines are constructed inside `AppWindow` (parallel to `render_host: Option<RenderHost>`) so they can be borrowed separately. Neither approach is pre-decided, and Task 2's pseudocode glosses over this.

**[HIGH] 05-12 Task 1: Pane/LocalDomain clipboard_tx plumbing is under-specified.**

The plan says "Inspect `crates/vector-mux/src/pane.rs` — if `Pane::new` / `LocalDomain::spawn_local` accepts a `clipboard_tx`, thread it through. If it does NOT, introduce a `set_clipboard_tx` or similar." This is a significant design gap. `Term::new` (the no-arg constructor) uses internal dummy channels; `Term::with_channels` accepts `clipboard_tx`. But `LocalDomain::spawn_local` (called by `Mux::create_tab_async`) currently builds `Term::new`, not `Term::with_channels`. Changing this means threading `clip_tx` through `LocalDomain`, `Mux::create_tab_async`, and every pane spawn site — a non-trivial refactor that touches multiple crates. The plan should either pre-commit to a specific wire-up strategy or acknowledge this as a blocking prerequisite that may require its own plan segment.

**[MEDIUM] 05-14 Task 1: `create_ungrouped` using a UUID or counter is fragile.**

The plan proposes `format!("vector-ungrouped-{}", uuid::Uuid::new_v4())` (adding a new dep) OR an `AtomicUsize` counter to generate a unique tabbing identifier. However, the comment notes "verify `WindowAttributesExtMacOS` for an explicit 'disallow tabbing' setter." winit 0.30's `WindowAttributesExtMacOS` does expose `with_tabbing_identifier` but NOT a `with_tabbing_mode_disallowed` setter. The correct macOS approach is to set `NSWindowTabbingModeDisallowed` via direct objc2-app-kit on the NSWindow after creation (same pattern as `set_tabbing_mode_preferred` in `apply_tabbing_identifier`). Using a unique string is a valid workaround but risks AppKit still trying to group windows in edge cases. The plan should commit to the `setTabbingMode:NSWindowTabbingModeDisallowed` approach via the existing `apply_tabbing_identifier` helper pattern, rather than leaving it as an unresolved choice for the executor.

**[MEDIUM] 05-16 Task 2: `active_pane_bottom_y_px` returns `None` as "v1 fallback".**

The function stub returns `None` with a comment "v1 fallback to `content_h - SEARCH_BAR_HEIGHT_PX` is acceptable." But for the search bar to appear at the correct position *inside* the active pane (UI-SPEC §5.2: "inside active pane's content rect, anchored to its bottom edge"), returning `None` and falling back to the surface bottom is functionally wrong when there are multiple panes. In a 50/50 horizontal split, the search bar would appear halfway down the window when Cmd-F is pressed in the top pane. The plan should either (a) snapshot `active_pane_bottom_y_px` during the per-pane compositor loop (when the layout rect is in scope) and store it in `App.active_pane_bottom_y_px: f32`, or (b) accept the wrong geometry explicitly and document it as a known limitation.

**[MEDIUM] 05-15 Task 2: `set_ime_allowed(true)` call site in `handle_app_shortcut` (Plan 05-14) creates a cross-plan edit dependency.**

Plan 05-15 says "In Plan 05-14's `SpawnNewWindow` handler (the ungrouped-NSWindow branch), call `window.set_ime_allowed(true)` on the freshly created window. Edit `handle_app_shortcut` accordingly." But Plan 05-14 `depends_on: ["05-13", "05-15"]` — meaning 05-15 runs BEFORE 05-14. If 05-15 adds the `set_ime_allowed(true)` call to the `SpawnNewWindow` branch — which doesn't yet exist in app.rs at the time 05-15 runs — 05-15's edit will fail or produce a no-op. The plan needs to clarify: 05-15 should add `set_ime_allowed(true)` only to `resumed()` and `handle_new_tab()`, and Plan 05-14 should add it to the new `SpawnNewWindow` handler body.

**[MEDIUM] 05-11 Task 2: `rebuild_switch_profile_submenu` NSApplication walk is fragile.**

The plan instructs locating the 'Switch Profile' NSMenuItem by walking `NSApplication::sharedApplication(mtm).mainMenu()` → `itemAtIndex(0)` ('Vector') → `submenu()` → iterating items looking for `title() == "Switch Profile"`. This assumes 'Vector' is always at index 0, which is an implicit ordering assumption. A safer approach is to store a reference to the Switch Profile submenu's `NSMenu` (or a `Retained<NSMenu>`) in a thread-local or module-level `OnceLock` at `add_switch_profile_submenu()` install time, so `rebuild_switch_profile_submenu` doesn't need to walk the menu tree at runtime.

**[LOW] 05-13: Compile warning window between 05-13 landing and 05-14 landing.**

The plan explicitly says adding `EncodedKey::App` will produce a compile WARNING on `vector-app` (unmatched `App` variant in `encode_key` match). This is acceptable, but CI should not have `-D warnings` globally enabled between these two plan executions. The plan should confirm that workspace lint enforcement will pass because `depends_on` prevents intermediate states from being a PR merge target.

**[LOW] 05-16: Search bar `content_w = surface_w` spans all panes in multi-pane layout.**

The `SearchBarPass::update_for_pane` call uses `content_w = surface_w` (full window width). For a multi-pane layout, the search bar should only span the active pane's width (UI-SPEC §5.2: "active pane width − 2 × spacing.2"). The same active-pane-rect data needed for `active_pane_bottom_y_px` (W5) is also needed for `content_width_px`. Both should come from the same layout snapshot stored during the compositor loop.

**[LOW] 05-14: `ReloadConfig` handler calling `rebuild_switch_profile_submenu` idempotency.**

The handler calls `rebuild_switch_profile_submenu` via `unsafe { crate::menu::rebuild_switch_profile_submenu(mtm, c); }`. This is also called from `UserEvent::ConfigReloaded`. Confirm that Plan 05-11's `rebuild_switch_profile_submenu` is idempotent (safe to call twice in rapid succession if FSEvents and Cmd-Shift-R both fire close together), which it should be as long as it atomically drains-and-rebuilds the submenu items.

### Suggestions

- **05-15 (objc2 safety):** Before writing the full `declare_class!` body, the executor should locate any existing `declare_class!` usage in the codebase (e.g., in `overlay.rs` or `tab_window.rs`) and use it as an in-repo template. If none exists, write a minimal compile-only test stub that declares the class with a single no-op selector to confirm the macro syntax against objc2 0.6.4 before adding the full five-selector NSTextInputClient implementation.
- **05-16 (borrow structure):** Move the four chrome pipeline fields from `RenderHost` into a new `ChromePipelines` struct owned by `AppWindow` (parallel to `render_host: Option<RenderHost>`). This gives `app.rs` the ability to borrow `chrome_pipelines` and `render_host` independently in `render_window`, resolving the double-mutable-borrow problem cleanly. `ChromePipelines::new(device, format)` takes device/format from the just-created `RenderHost` so construction remains colocated.
- **05-12 (channel plumbing):** Pre-check whether `LocalDomain::spawn_local` currently constructs `Term::new` (dummy channels) or `Term::with_channels` (real channels). If it uses `Term::new`, the scope of Task 1 must expand to include threading `clipboard_tx` through `LocalDomain::new_with_clipboard_tx` → `Pane::term` construction path. This is at minimum a 2-crate change (vector-mux + vector-app) and should be added to the plan's `files_modified` list before execution begins.
- **05-14 (`create_ungrouped`):** Implement ungrouping by calling `setTabbingMode:NSWindowTabbingModeDisallowed` on the newly created NSWindow via the existing objc2-app-kit pattern in `mux_commands.rs`, rather than using a unique tabbing identifier string. This avoids adding `uuid` as a new dependency and produces deterministic behavior that AppKit guarantees will not tab-group the window.
- **05-16 (chrome text rendering expectations):** The plan correctly defers chrome text glyph rendering (search bar query display, profile names in picker, toast text) as "Phase-5.x polish." The SUMMARY.md should explicitly note that the visible UI after these plans will show colored backgrounds with no text, so the smoke matrix items #6 (search bar) and #7 (picker) are approved as "colored background visible, text rendering deferred" — not as fully functional UI elements.
- **05-11 (submenu reference storage):** Store `Retained<NSMenu>` for the Switch Profile submenu in a module-level `OnceLock<Retained<NSMenu>>` (or thread-local with main-thread marker) at `add_switch_profile_submenu()` install time, so `rebuild_switch_profile_submenu` can directly access it without walking the menu tree. This is safer and faster than title-based lookup at rebuild time.

### Risk Assessment

**Overall risk: MEDIUM.** The six plans address real, correctly-identified gaps and the dependency ordering is sound. Execution risk concentrates in two plans: **05-15** (the `declare_class!` NSTextInputClient shim is the project's first full AppKit subclass via the macro, and the API details are version-sensitive with real correctness stakes — a wrong implementation could silently corrupt preedit state or crash on text input), and **05-16** (the per-frame chrome orchestration requires resolving a non-trivial wgpu borrow structure that the plan leaves to executor judgment). Plans 05-11, 05-12, 05-13, and 05-14 are lower risk — primarily plumbing with clear acceptance criteria. The `depends_on` serialization is the right architectural choice and will prevent the parallel-execution git conflicts that caused issues in Plans 05-01..05-10. Phase boundary discipline is excellent — no Phase 6 transport work leaks in, and every plan cites the specific gap from 05-VERIFICATION.md that it closes. The three HIGH concerns (objc2 macro syntax unvalidated, wgpu borrow structure unresolved, clipboard channel plumbing under-specified) should each be resolved in writing before executor handoff, not left as judgment calls during execution.

---

## Consensus Summary

Single-reviewer (claude). No multi-AI consensus available — treat findings as one informed perspective, not cross-AI corroboration.

### Top Action Items (HIGH severity — resolve before execution)

1. **05-15 objc2 `declare_class!` validation** — locate an existing in-repo `declare_class!` usage as template, or write a no-op stub against objc2 0.6.4, before authoring the five-selector NSTextInputClient body.
2. **05-16 chrome borrow structure** — pre-decide where chrome pipelines live. Recommended: extract a `ChromePipelines` struct owned by `AppWindow` (parallel to `render_host`), not nested inside `RenderHost`.
3. **05-12 clipboard channel plumbing** — pre-inspect `LocalDomain::spawn_local` to confirm whether it constructs `Term::new` (dummy channels) or `Term::with_channels`. If the former, expand 05-12's `files_modified` to cover the full plumbing path.

### Cross-Plan Issues

- **05-15 ↔ 05-14 set_ime_allowed call site** — 05-15 runs before 05-14, so 05-15 cannot edit a `SpawnNewWindow` branch that doesn't exist yet. Move that call into 05-14.
- **05-14 create_ungrouped strategy** — pick `setTabbingMode:NSWindowTabbingModeDisallowed` over UUID/counter approach. Avoids new dep and is deterministic.
