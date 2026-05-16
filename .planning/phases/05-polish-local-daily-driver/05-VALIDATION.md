---
phase: 5
slug: polish-local-daily-driver
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-05-12
---

# Phase 5 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Derived from `05-RESEARCH.md §"Validation Architecture"`. Planner fills the per-task map; checker enforces.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (workspace-native; no external runner) |
| **Config file** | `Cargo.toml` workspace `[workspace.dependencies]` |
| **Quick run command** | `cargo test --workspace --tests --no-fail-fast` |
| **Full suite command** | `cargo test --workspace --all-targets --no-fail-fast && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check && cargo deny check` |
| **Phase-gate command** | `cargo test --workspace --all-targets -- --include-ignored` (picks up `#[ignore]` tmux + IME integration tests; CI dedicated job pre-installs `tmux 3.4+` via `brew install tmux`) |
| **Project lint entry** | `make lint` (per CLAUDE.md) |
| **Estimated runtime** | ~90 seconds (quick) / ~300 seconds (full) / +60s (phase-gate w/ tmux job) |

---

## Sampling Rate

- **After every task commit:** `cargo test --workspace --tests --no-fail-fast`
- **After every plan wave:** `cargo test --workspace --all-targets --no-fail-fast && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check && cargo deny check`
- **Before `/gsd:verify-work`:** Full suite green AND `-- --include-ignored` green AND manual smoke matrix below executed
- **Max feedback latency:** ~90 seconds (quick suite)

---

## Per-Task Verification Map

> Planner fills this with concrete `{N}-XX-YY` task IDs from each PLAN.md. Test names come from `05-RESEARCH.md §"Phase Requirements → Test Map"`.

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| {planner fills} | 01 | 0 | D-83 #1/#2 | unit | `cargo test --test workspace_lints_inheritance && cargo test --test path_deps_have_versions` | ❌ W0 | ⬜ pending |
| {planner fills} | 02 | 1 | POLISH-01 | unit | `cargo test -p vector-config schema::parse_rejects_unknown_field` | ❌ W0 | ⬜ pending |
| {planner fills} | 02 | 1 | POLISH-01 | unit | `cargo test -p vector-config schema::profile_overrides_flat` | ❌ W0 | ⬜ pending |
| {planner fills} | 02 | 1 | POLISH-01 | unit | `cargo test -p vector-config loader::error_line_col` | ❌ W0 | ⬜ pending |
| {planner fills} | 02 | 1 | POLISH-07 | unit | `cargo test -p vector-config schema::profile_kinds_parse` | ❌ W0 | ⬜ pending |
| {planner fills} | 03 | 1 | POLISH-03 | unit | `cargo test -p vector-theme builtins_loadable` | ❌ W0 | ⬜ pending |
| {planner fills} | 03 | 1 | POLISH-03 | unit | `cargo test -p vector-theme itermcolors::parses_full_scheme` | ❌ W0 | ⬜ pending |
| {planner fills} | 03 | 1 | POLISH-03 | unit | `cargo test -p vector-theme itermcolors::unknown_key_warns` | ❌ W0 | ⬜ pending |
| {planner fills} | 03 | 1 | POLISH-03 | unit (mock) | `cargo test -p vector-theme appearance::dark_light_flip` | ❌ W0 | ⬜ pending |
| {planner fills} | 04 | 2 | POLISH-01 | integration | `cargo test -p vector-config watcher::debounce_150ms` | ❌ W0 | ⬜ pending |
| {planner fills} | 04 | 2 | POLISH-01 | integration | `cargo test -p vector-config watcher::atomic_rename_single_event` | ❌ W0 | ⬜ pending |
| {planner fills} | 04 | 2 | POLISH-01 | integration | `cargo test -p vector-config apply::parse_error_keeps_last_good` | ❌ W0 | ⬜ pending |
| {planner fills} | 04 | 2 | POLISH-02 | unit | `cargo test -p vector-config apply::font_family_change_requires_restart` | ❌ W0 | ⬜ pending |
| {planner fills} | 05 | 3 | POLISH-04 | unit | `cargo test -p vector-term osc_sniff::osc7_file_url_parses` | ❌ W0 | ⬜ pending |
| {planner fills} | 05 | 3 | POLISH-04 | unit | `cargo test -p vector-term osc_sniff::osc7_percent_encoded` | ❌ W0 | ⬜ pending |
| {planner fills} | 05 | 3 | POLISH-04 | unit | `cargo test -p vector-term osc_sniff::osc133_marks` | ❌ W0 | ⬜ pending |
| {planner fills} | 05 | 3 | POLISH-04 | unit | `cargo test -p vector-term osc_sniff::prompt_ring_1000` | ❌ W0 | ⬜ pending |
| {planner fills} | 05 | 3 | POLISH-04 | unit | `cargo test -p vector-term hyperlink::id_groups_run` | ❌ W0 | ⬜ pending |
| {planner fills} | 05 | 3 | POLISH-04 | unit | `cargo test -p vector-term hyperlink::anonymous_by_uri` | ❌ W0 | ⬜ pending |
| {planner fills} | 05 | 3 | POLISH-04 | unit | `cargo test -p vector-term hyperlink::scheme_allowlist` | ❌ W0 | ⬜ pending |
| {planner fills} | 05 | 3 | POLISH-04 | unit | `cargo test -p vector-term listener::osc10_query_response` | ❌ W0 | ⬜ pending |
| {planner fills} | 06 | 3 | POLISH-05 | unit | `cargo test -p vector-term osc52::raw_clipboard_store` | ❌ W0 | ⬜ pending |
| {planner fills} | 06 | 3 | POLISH-05 | integration | `cargo test -p vector-term osc52::dcs_wrapped_round_trip` | ❌ W0 | ⬜ pending |
| {planner fills} | 06 | 3 | POLISH-05 | unit | `cargo test -p vector-input clipboard::outbound_58_byte_chunks` | ❌ W0 | ⬜ pending |
| {planner fills} | 06 | 3 | POLISH-05 | unit | `cargo test -p vector-term osc52::read_denied` | ❌ W0 | ⬜ pending |
| {planner fills} | 06 | 3 | POLISH-05 | integration (`#[ignore]`) | `cargo test -p vector-term --test osc52_tmux -- --ignored` | ❌ W0 | ⬜ pending |
| {planner fills} | 07 | 4 | POLISH-02 | unit | `cargo test -p vector-fonts ligature_glyph_present` | ❌ W0 | ⬜ pending |
| {planner fills} | 07 | 4 | POLISH-02 | unit | `cargo test -p vector-fonts ligature_toggle_off` | ❌ W0 | ⬜ pending |
| {planner fills} | 07 | 4 | POLISH-02 | unit | `cargo test -p vector-fonts nerd_font_codepoint_renders` | ❌ W0 | ⬜ pending |
| {planner fills} | 08 | 4 | POLISH-06 | unit (exists) | `cargo test -p vector-term search` | ✅ |
| {planner fills} | 08 | 4 | POLISH-06 | unit | `cargo test -p vector-app search_bar::smart_case_lower` | ❌ W0 | ⬜ pending |
| {planner fills} | 08 | 4 | POLISH-06 | unit | `cargo test -p vector-app search_bar::smart_case_upper` | ❌ W0 | ⬜ pending |
| {planner fills} | 08 | 4 | POLISH-06 | unit | `cargo test -p vector-app search_bar::cache_1000_lazy` | ❌ W0 | ⬜ pending |
| {planner fills} | 08 | 4 | POLISH-06 | integration | `cargo test -p vector-app search_bar::esc_restores_selection` | ❌ W0 | ⬜ pending |
| {planner fills} | 09 | 4 | Cmd-C / D-53/54 | unit | `cargo test -p vector-input selection::wide_chars_collapse` | ❌ W0 | ⬜ pending |
| {planner fills} | 09 | 4 | Cmd-C | unit | `cargo test -p vector-input selection::trailing_ws_stripped` | ❌ W0 | ⬜ pending |
| {planner fills} | 09 | 4 | Cmd-C | unit | `cargo test -p vector-input selection::rect_uses_newline` | ❌ W0 | ⬜ pending |
| {planner fills} | 10 | 4 | POLISH-07 | integration | `cargo test -p vector-mux profile_local_spawn` | ❌ W0 | ⬜ pending |
| {planner fills} | 10 | 4 | POLISH-07 | unit | `cargo test -p vector-app profile_picker::codespace_warning_label` | ❌ W0 | ⬜ pending |
| {planner fills} | 10 | 4 | POLISH-07 | unit | `cargo test -p vector-app profile_picker::fuzzy_ranking` | ❌ W0 | ⬜ pending |
| {planner fills} | 10 | 4 | POLISH-07 | unit | `cargo test -p vector-render tint_stripe::geometry` | ❌ W0 | ⬜ pending |
| {planner fills} | 11 | 4 | Cmd-N / D-82 | integration | `cargo test -p vector-app cmd_n::spawns_default_profile_home` | ❌ W0 | ⬜ pending |
| {planner fills} | 12 | 5 | POLISH-08 | unit | `cargo test -p vector-app ske::toggle_calls_carbon` | ❌ W0 | ⬜ pending |
| {planner fills} | 12 | 5 | POLISH-08 | unit | `cargo test -p vector-app ske::raii_disables_on_drop` | ❌ W0 | ⬜ pending |
| {planner fills} | 13 | 5 | POLISH-08 | unit | `cargo test -p vector-app ime::preedit_not_to_pty` | ❌ W0 | ⬜ pending |
| {planner fills} | 13 | 5 | POLISH-08 | unit | `cargo test -p vector-app ime::commit_to_pty` | ❌ W0 | ⬜ pending |
| {planner fills} | 13 | 5 | POLISH-08 | unit | `cargo test -p vector-app ime::unmark_clears` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*Note: plan numbers (01..13) are illustrative groupings derived from the validation map; the planner is free to consolidate or split waves so long as every row above maps to at least one task.*

---

## Wave 0 Requirements

> All test files below must be stubbed (with `#[ignore]` or `panic!("WAVE 0 STUB")` markers) in Wave 0, before any feature task runs. Wave 0 is also the home for D-83's lint hardening so subsequent waves run under the final lint regime.

- [ ] `crates/vector-config/tests/schema_and_loader.rs` — POLISH-01 parse + line/col + profile kinds
- [ ] `crates/vector-config/tests/watcher_debounce.rs` — POLISH-01 debounce + atomic-rename
- [ ] `crates/vector-config/tests/apply_pipeline.rs` — POLISH-01 last-good + font-restart classification (POLISH-02 path)
- [ ] `crates/vector-theme/tests/itermcolors.rs` + fixture `crates/vector-theme/tests/fixtures/Solarized-Dark.itermcolors` — POLISH-03 importer
- [ ] `crates/vector-theme/tests/builtins.rs` — POLISH-03 builtins
- [ ] `crates/vector-theme/tests/appearance.rs` — POLISH-03 appearance flip
- [ ] `crates/vector-term/tests/osc_sniff.rs` — POLISH-04 OSC 7 + 133 sniffer
- [ ] `crates/vector-term/tests/hyperlinks.rs` — POLISH-04 OSC 8 id + anonymous grouping + allowlist
- [ ] `crates/vector-term/tests/dynamic_color_response.rs` — POLISH-04 OSC 10/11/12 PtyWrite reply
- [ ] `crates/vector-term/tests/osc52.rs` — POLISH-05 raw + DCS-wrapped + read-denied
- [ ] `crates/vector-term/tests/osc52_tmux.rs` (`#[ignore]` by default) — POLISH-05 real tmux 3.4+ round-trip
- [ ] `crates/vector-input/tests/clipboard.rs` — POLISH-05 58-byte chunking
- [ ] `crates/vector-input/tests/selection_string.rs` — Cmd-C wide chars + trailing ws + rect newlines
- [ ] `crates/vector-fonts/tests/ligatures.rs` — POLISH-02 ligature + Nerd Font (or place under `vector-render` if that's where shaping lives)
- [ ] `crates/vector-app/tests/search_bar.rs` — POLISH-06 smart-case + cache cap + esc restore
- [ ] `crates/vector-app/tests/profile_picker.rs` — POLISH-07 fuzzy + label
- [ ] `crates/vector-app/tests/cmd_n.rs` — Cmd-N spawn-defaults path
- [ ] `crates/vector-app/tests/ske.rs` — POLISH-08 toggle + RAII disable
- [ ] `crates/vector-app/tests/ime.rs` — POLISH-08 preedit + commit + unmark
- [ ] `crates/vector-mux/tests/profile_local_spawn.rs` — POLISH-07 LocalDomain end-to-end
- [ ] `crates/vector-render/tests/tint_stripe.rs` — POLISH-07 tint quad geometry
- [ ] `tests/workspace_lints_inheritance.rs` — D-83 #1 (top-level integration test)
- [ ] `tests/path_deps_have_versions.rs` — D-83 #2 (top-level integration test; extend ADR-0003)
- [ ] `.pre-commit-config.yaml` — D-83 #3 cargo-deny hook (`pass_filenames: false`, stages: `[pre-commit]`)
- [ ] `.github/workflows/ci.yml::unused-deps` job — D-83 #4 cargo-machete
- [ ] `.github/workflows/ci.yml::tmux-smoke` job — runs `cargo test -p vector-term --test osc52_tmux -- --ignored` with `brew install tmux` prereq
- [ ] Framework install: **none** — `cargo test` is workspace-native and already wired since Phase 1

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Font hot-swap shows `restart required` toast | POLISH-02 / D-69 | NSWindow toast renders against AppKit compositor; headless asserting is brittle | Edit `~/.config/vector/config.toml`, change `[default.font].family`, save; observe banner |
| `.itermcolors` drop-and-go | POLISH-03 / D-73 | Live FSEvents on real macOS user filesystem | Drop `Solarized-Dark.itermcolors` in `~/.config/vector/themes/`; set `theme = "Solarized-Dark"`; save; observe palette flip without restart |
| IME preedit (Japanese / Pinyin) underlines under cursor | POLISH-08 / D-81 | Driven by real macOS Input Source, no headless harness | System Prefs → Keyboard → Input Sources → add Hiragana; in Vector, switch to Hiragana; type `ka`; verify underlined preedit at cursor; press Enter to commit |
| Secure Keyboard Entry blocks event interception | POLISH-08 / D-80 | Cross-app side effect; requires another app capturing keystrokes | Toggle `Vector → Secure Keyboard Entry` on; type in 1Password browser autofill; verify Vector's keystrokes don't leak |
| Tmux DCS round-trip on a real Codespace | POLISH-05 / D-71 | Network + tmux 3.4 + remote PTY; CI integration job covers automated portion | `gh cs ssh -c <codespace>`; inside: `tmux new -A -s vector`; `printf "\eP\e]52;c;%s\a\e\\" "$(printf hi | base64)"`; verify macOS clipboard via `pbpaste` returns `hi` |
| Cmd-Shift-P picker UX with 50+ profiles | POLISH-07 / D-75 | Subjective UX feel | Generate `config.toml` with 50 named profiles; open picker; type a few chars; verify fuzzy ranking feels right and rendering is < 16 ms |
| Cmd-N spawns ungrouped `NSWindow` | D-82 | `NSWindowTabbingMode` behavior under user-set system preference | Cmd-N twice from a focused window; verify two separate top-level windows (no tab merge) regardless of system "Prefer Tabs" setting |
| Cmd-Shift-R reload-config menu fallback works when FSEvents misses | POLISH-01 / D-69 | FSEvents miss is rare; manual trigger is a known-good fallback | Edit config; instead of saving via editor, modify via `echo` redirect that the watcher might miss; press Cmd-Shift-R; verify reload toast |
| Title-bar tint stripe renders for active profile | POLISH-07 / D-75 | Visual check at the NSWindow chrome / wgpu boundary | Set `tint = "#7a3aaf"` in a profile; Cmd-Shift-P → switch; verify 24–32 px stripe under titlebar |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references in `crates/vector-*/tests/`
- [ ] No watch-mode flags (`cargo watch` etc.) used in sampling commands
- [ ] Feedback latency < 90s for quick suite
- [ ] tmux smoke job and IME preedit smoke matrix executed before `/gsd:verify-work`
- [ ] `nyquist_compliant: true` set in frontmatter once planner finalizes the per-task map and checker passes

**Approval:** pending
