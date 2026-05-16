---
phase: 05-polish-local-daily-driver
plan: 04
subsystem: config
tags: [notify, debouncer, hot-reload, apply-pipeline, POLISH-01, POLISH-02]

requires:
  - phase: 05-polish-local-daily-driver
    provides: ConfigFile / ProfileBlock / FontCfg / Appearance / KeyBind / Action schema + parse() from Plan 05-02
  - phase: 05-polish-local-daily-driver
    provides: workspace deps notify 8 + notify-debouncer-full 0.5 declared at workspace root
provides:
  - "vector-config::spawn_watcher(config_path, themes_dir, tx) -> impl Drop (notify-debouncer-full, 150ms debounce, parent-dir + themes-dir watch)"
  - "vector-config::ConfigEvent { Dirty { paths }, Error(String) } — emitted post-debounce-flush"
  - "vector-config::diff_config(old, new) -> ApplyPlan { live: Vec<LiveChange>, restart: Vec<RestartReason> } classifying per D-69 table"
  - "vector-config::try_load_or_keep(source, &mut Option<ConfigFile>) — parse-error keeps last-good (D-69)"
  - "LiveChange { Theme, Appearance, Tint, FontSize, Ligatures, Keybinds, PerProfile } + RestartReason { FontFamily }"
  - "4 ignored test stubs flipped green: debounce_150ms, atomic_rename_single_event, parse_error_keeps_last_good, font_family_change_requires_restart"
affects: [05-08-app-wiring]

tech-stack:
  added: [notify 8, notify-debouncer-full 0.5, tempfile 3 (dev-dep)]
  patterns:
    - "notify-debouncer-full new_debouncer with Duration::from_millis(150) — D-69 quiescent debounce"
    - "Parent-dir RecursiveMode::NonRecursive watch — Pitfall 1 atomic-rename inode-swap survival"
    - "FontSize carried as u32 milli-pt (size * 1000) to satisfy Eq on LiveChange variant"
    - "try_load_or_keep — last_good held by caller (vector-app owns the storage), parse error bubbles without mutation"
    - "Per-profile add/remove/change all emit LiveChange::PerProfile (callers decide cascade)"

key-files:
  created:
    - crates/vector-config/src/watcher.rs
    - crates/vector-config/src/apply.rs
  modified:
    - crates/vector-config/Cargo.toml
    - crates/vector-config/src/lib.rs
    - crates/vector-config/tests/watcher_debounce.rs
    - crates/vector-config/tests/apply_pipeline.rs
    - Cargo.lock

key-decisions:
  - "ConfigEvent lives in lib.rs (not watcher.rs) so apply-layer consumers (Plan 05-08) need only one import"
  - "Debouncer collapses Vec<DebouncedEvent> → single ConfigEvent::Dirty { paths } with sort+dedup; callers never see notify internals"
  - "Profile removal also emits LiveChange::PerProfile (callers may need to drop active pane's profile ref)"
  - "tempfile = \"3\" added as direct dev-dep (not workspace) — only vector-config test surface needs it in Phase 5"

patterns-established:
  - "spawn_watcher returns impl Drop — caller holds the Debouncer alive by binding the value; dropping stops the watcher cleanly"
  - "try_load_or_keep is the load-bearing entrypoint for hot-reload: caller owns last_good, never mutated on Err"

requirements-completed: [POLISH-01, POLISH-02]

duration: 3min
completed: 2026-05-12
---

# Phase 05 Plan 04: vector-config Watcher + Apply Pipeline Summary

**notify-debouncer-full file watcher with 150 ms debounce + atomic-rename re-arm (Pitfall 1) and a `diff_config()` pipeline that classifies every config delta as `LiveApply` (theme/keybinds/font-size/ligatures/tint/per-profile) or `RestartRequired::FontFamily` (Pitfall 7 — CoreText cache); `try_load_or_keep` keeps the last-good `ConfigFile` in memory on parse error per D-69.**

## Performance

- **Duration:** ~3 min (4 task commits + 1 chore commit)
- **Tasks:** 2 (both TDD: RED then GREEN)
- **Files created:** 2 (watcher.rs, apply.rs)
- **Files modified:** 4 (Cargo.toml, lib.rs, both test files)
- **Tests:** 4 new green (10 total in vector-config: 5 schema/loader + 2 watcher + 2 apply + 1 lib-internal); 0 ignored remaining

## Accomplishments

- **POLISH-01 hot-reload watcher infrastructure delivered.** `spawn_watcher(config_path, themes_dir, tx)` returns a `Debouncer` handle that watches the config file's parent directory (Pitfall 1 — atomic-rename swaps the file's inode, so the file-itself watch dies; the parent-dir watch survives) plus the themes directory (D-73, non-recursive). 150 ms `Duration::from_millis(150)` quiescent debounce per D-69. Every flush collapses the underlying `Vec<DebouncedEvent>` into one `ConfigEvent::Dirty { paths }` after sort+dedup.
- **POLISH-02 font-family restart classification delivered.** `diff_config(&old, &new) -> ApplyPlan` walks `[default]`, `[default.font]`, `[[keybind]]`, and `[profile.X]` deltas and pushes them into `live: Vec<LiveChange>` or `restart: Vec<RestartReason>` per the D-69 table. Font family is the only current `RestartReason` (Pitfall 7 — CoreText caches glyph atlases per-font; family swap forces process restart for a sharp first paint).
- **D-69 parse-error-keep-last-good delivered.** `try_load_or_keep(source, &mut Option<ConfigFile>)` calls `parse(source)` and only mutates `last_good` on `Ok`. On `Err(ConfigError)`, `last_good` is byte-identical to its prior state, and the caller surfaces the error to the Plan 05-08 toast layer.
- All 4 Wave-0 stub tests un-ignored and green: `debounce_150ms` (3 rapid writes collapse to 1 event), `atomic_rename_single_event` (vim `:w` pattern via parent-dir re-arm), `parse_error_keeps_last_good` (bad TOML returns Err + last_good unchanged), `font_family_change_requires_restart` (JetBrains Mono → Fira Code lands `RestartReason::FontFamily` in the plan).

## Task Commits

1. **Task 1 RED — watcher tests** — `c5d37fe` (test)
   - 2 files: Cargo.toml (notify + notify-debouncer-full deps + tempfile dev-dep), tests/watcher_debounce.rs (both bodies + use vector_config::{spawn_watcher, ConfigEvent})
2. **Task 1 GREEN — watcher impl** — `dc55d6e` (feat)
   - 2 files: src/watcher.rs (new — spawn_watcher + DebounceEventResult handler), src/lib.rs (pub mod watcher + pub use + ConfigEvent enum)
3. **Task 2 RED — apply tests** — `2294fb1` (test)
   - 1 file: tests/apply_pipeline.rs (both bodies + use vector_config::{diff_config, parse, try_load_or_keep, ConfigFile, RestartReason})
4. **Task 2 GREEN — apply impl** — `21189de` (feat)
   - 2 files: src/apply.rs (new — LiveChange enum + RestartReason enum + ApplyPlan + diff_config + try_load_or_keep + profile_per_pane_differs + profile_tint_change), src/lib.rs (pub mod apply + pub use)
5. **Cargo.lock update** — `fc2245d` (chore)
   - 1 file: Cargo.lock (notify 8.2.0 + notify-debouncer-full 0.5.0 + tempfile 3.27.0 + fsevent-sys 4.1.0 + file-id 0.2.3 + notify-types 2.1.0 + walkdir + same-file + rustix + getrandom + fastrand + errno transitive pulls — all required by Task 1's dep additions)

## Files Created/Modified

- `crates/vector-config/src/watcher.rs` — new; `spawn_watcher(config_path: &Path, themes_dir: &Path, tx: mpsc::Sender<ConfigEvent>) -> anyhow::Result<impl Drop>`
- `crates/vector-config/src/apply.rs` — new; `LiveChange { Theme | Appearance | Tint | FontSize | Ligatures | Keybinds | PerProfile }` + `RestartReason::FontFamily` + `ApplyPlan { live, restart }` + `diff_config(old, new)` + `try_load_or_keep(source, &mut last_good)`
- `crates/vector-config/src/lib.rs` — added `pub mod apply` + `pub mod watcher` + `pub use` for `spawn_watcher`, `diff_config`, `try_load_or_keep`, `ApplyPlan`, `LiveChange`, `RestartReason`; declared `ConfigEvent { Dirty { paths }, Error(String) }` at crate root
- `crates/vector-config/Cargo.toml` — `notify.workspace = true` + `notify-debouncer-full.workspace = true` + `[dev-dependencies] tempfile = "3"`
- `crates/vector-config/tests/watcher_debounce.rs` — both tests un-ignored + implemented per plan
- `crates/vector-config/tests/apply_pipeline.rs` — both tests un-ignored + implemented per plan
- `Cargo.lock` — transitive deps for notify + notify-debouncer-full + tempfile

## Decisions Made

- **`ConfigEvent` declared at `crate::ConfigEvent` (lib.rs root), not `crate::watcher::ConfigEvent`.** Plan 05-08 will import once: `use vector_config::{spawn_watcher, ConfigEvent, ApplyPlan, try_load_or_keep};`.
- **`try_load_or_keep` takes `&mut Option<ConfigFile>` (caller-owned).** vector-app holds the storage cell. Alternatives (`Arc<RwLock<ConfigFile>>` etc.) introduce shared-state ownership questions outside Plan 05-04's mandate.
- **`LiveChange::FontSize(u32)` carries milli-pt.** `f32` doesn't impl `Eq`; rather than pollute the enum with `PartialEq` only or compare with epsilons, `LiveChange` is `PartialEq + Eq` via `(size * 1000).max(0.0) as u32`. Plan 05-08 divides by 1000 to recover the float for the font renderer.
- **`tempfile = "3"` added as direct dev-dep, not at workspace level.** Only vector-config touches tempdirs in Phase 5; no need to advertise it across the workspace.

## Deviations from Plan

**3 Rule-1 auto-fixes — all mechanical clippy pedantic lints in `apply.rs`** (commits rolled into Task 2 GREEN `21189de`):

**1. [Rule 1 - Bug/Lint] `cast_possible_truncation` + `cast_sign_loss` on `(s * 1000.0) as u32`**
- **Found during:** Task 2 GREEN clippy gate
- **Fix:** wrapped the cast with `s.max(0.0) * 1000.0` and an `#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]` annotation on the immediate `let size_mhz`. Font sizes ≤ 0.0 clamp to 0 (sentinel; vector-app validates upstream).
- **Commit:** `21189de`

**2. [Rule 1 - Bug/Lint] `if_not_else` on `profile_tint_change`**
- **Found during:** Task 2 GREEN clippy gate
- **Fix:** flipped the comparison from `if old_tint != new_tint { Some(...) } else { None }` to `if old_tint == new_tint { None } else { Some(...) }`. Same behavior, clippy-happy.
- **Commit:** `21189de`

**Style adjustment to plan snippet — keybind diff:**
The plan snippet's `if old.keybind != new.keybind` requires `KeyBind: PartialEq`, which the schema does not provide (only `Action: PartialEq + Eq` was derived in Plan 05-02). Rather than add a `PartialEq` derive on `KeyBind` (cross-plan schema mutation), I implemented the keybind diff inline as length-mismatch OR pairwise (`o.key != n.key || o.action != n.action`) — semantically identical, no schema change. Documented as a deviation for transparency though it's not a Rule-anything trigger.

## Auth Gates Encountered

None — no external services touched.

## Issues Encountered

- **Cargo.lock incidentally captured pre-existing vector-fonts/loader.rs in-tree edit** by a parallel agent (Plan 05-07 territory — POLISH-02 ligature toggle landed `ligatures_enabled: bool` on `FontStack` + `set_ligatures(on: bool)` method). I noticed it in `git status` and excluded it from my commits; the file change remains in the working tree for Plan 05-07's executor to commit. No interaction with my work — vector-config and vector-fonts are disjoint crates.
- **No workspace-wide build/test run performed.** Per CLAUDE.md project instructions and parallel-executor isolation, scope was limited to `cargo test -p vector-config` + `cargo clippy -p vector-config --all-targets`. Both pass. Workspace-wide regression is the orchestrator's job after parallel agents finish.

## User Setup Required

None — POLISH-01 + POLISH-02 are purely internal data-layer mechanics. No external services, no Keychain, no GitHub API.

## Next Phase Readiness

- **Plan 05-08 (app wiring)** inherits the full hot-reload pipeline:
  ```rust
  use vector_config::{spawn_watcher, ConfigEvent, ApplyPlan, try_load_or_keep};

  let (cfg_tx, cfg_rx) = std::sync::mpsc::channel::<ConfigEvent>();
  let _watcher = spawn_watcher(&config_path, &themes_dir, cfg_tx)?;
  // bridge thread:
  while let Ok(ev) = cfg_rx.recv() {
      if let ConfigEvent::Dirty { .. } = ev {
          let source = std::fs::read_to_string(&config_path)?;
          match try_load_or_keep(&source, &mut last_good) {
              Ok(plan)  => proxy.send_event(UserEvent::ConfigReloaded(plan)),
              Err(err)  => proxy.send_event(UserEvent::ConfigError(err)),
          }
      }
  }
  ```
- Plan 05-08 will dispatch each `LiveChange` to the right subsystem (theme cache, keybind table, font renderer's `set_font_size`, ligature toggle hook landed by Plan 05-07, etc.) and surface `RestartReason::FontFamily` as a toast: *"Restart Vector to apply the new font."*
- `ApplyPlan` is `Debug + Clone`; the `UserEvent::ConfigReloaded(plan)` round-trip across the winit EventLoopProxy is unbounded but realistically O(few-bytes) per reload.

## Self-Check: PASSED

Verified:
- `crates/vector-config/src/watcher.rs` — FOUND (47 lines)
- `crates/vector-config/src/apply.rs` — FOUND (143 lines)
- `crates/vector-config/src/lib.rs` — UPDATED (24 lines; ConfigEvent + 8 pub uses)
- `crates/vector-config/Cargo.toml` — UPDATED (notify + notify-debouncer-full deps + tempfile dev-dep)
- `crates/vector-config/tests/watcher_debounce.rs` — UPDATED (2 tests un-ignored)
- `crates/vector-config/tests/apply_pipeline.rs` — UPDATED (2 tests un-ignored)
- Task 1 RED commit `c5d37fe` — FOUND in `git log --oneline`
- Task 1 GREEN commit `dc55d6e` — FOUND in `git log --oneline`
- Task 2 RED commit `2294fb1` — FOUND in `git log --oneline`
- Task 2 GREEN commit `21189de` — FOUND in `git log --oneline`
- Cargo.lock chore commit `fc2245d` — FOUND in `git log --oneline`
- `cargo test -p vector-config --test watcher_debounce` — 2 passed / 0 failed / 0 ignored
- `cargo test -p vector-config --test apply_pipeline` — 2 passed / 0 failed / 0 ignored
- `cargo test -p vector-config` — 10 passed / 0 failed / 0 ignored across all targets
- `cargo clippy -p vector-config --all-targets -- -D warnings` — exit 0
- `grep -q "Duration::from_millis(150)" crates/vector-config/src/watcher.rs` — D-69 debounce locked
- `grep -q "config_path.parent" crates/vector-config/src/watcher.rs` — Pitfall 1 parent-dir watch
- `grep -q "themes_dir" crates/vector-config/src/watcher.rs` — D-73 themes dir watch
- `grep -q "RestartReason::FontFamily" crates/vector-config/src/apply.rs` — POLISH-02 path
- `grep -q "Pitfall 7" crates/vector-config/src/apply.rs` — pitfall comment present
- `grep -q "pub fn diff_config" crates/vector-config/src/apply.rs` — public API
- `grep -q "pub fn try_load_or_keep" crates/vector-config/src/apply.rs` — public API
- `grep -c "LiveChange::" crates/vector-config/src/apply.rs` — 8 (≥ 6 required)

---
*Phase: 05-polish-local-daily-driver*
*Completed: 2026-05-12*
