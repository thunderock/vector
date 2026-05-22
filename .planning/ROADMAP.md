# Roadmap: Vector

**Created:** 2026-05-10
**Pivoted:** 2026-05-19 — see Phase 7 note
**Granularity:** fine (10 phases)
**Total v1 requirements:** 47
**Coverage:** 47 / 47 mapped

## Core Value

Open the app, pick a remote machine via VS Code Remote Tunnels (`code tunnel`), get a fast remote shell — no VS Code, no browser. Local-terminal niceties (tabs, splits, GPU rendering) are table-stakes; the differentiator is that a Dev-Tunnels session feels native, not bolted on.

## Phases

- [ ] **Phase 1: Foundation & CI/DMG Pipeline** — Cargo workspace + winit/tokio threading skeleton + Universal unsigned DMG produced by CI on every push.
- [ ] **Phase 2: Headless Terminal Core** — `alacritty_terminal`-backed VT parser + grid + scrollback + local PTY; conformance tests pass headless.
- [ ] **Phase 3: GPU Renderer & First Paint** — wgpu/Metal renderer with damage-tracked atlas; single-window single-tab terminal you can run `vim` in.
- [ ] **Phase 4: Mux — Tabs & Splits** — Window/Tab/Pane tree with `Domain`/`PtyTransport` abstractions; iTerm-class local terminal.
- [ ] **Phase 5: Polish (Local Daily-Driver)** — TOML config + hot-reload, themes/fonts/ligatures, OSC 7/8/52/133/10/11/12, scrollback search, tmux pass-through.
- [ ] **Phase 6: GitHub Auth + Codespaces Picker** — OAuth device flow, Keychain token storage, codespace picker UI; clicking "Connect" still shows a placeholder.
- [~] **Phase 7: Remote SSH Transport Scaffolding (DESCOPED 2026-05-19)** — pivoted away from Codespaces. Reusable scaffolding shipped: `vector-ssh` crate (russh client, SshChannelTransport, ChildStdioStream, host-key fingerprint handler), `Mux::create_tab_async_with_transport`, `format_tab_title` with `TransportKind`, `[remote]` badge. Codespace-specific code reverted.
- [x] **Phase 8: VS Code Remote Tunnels Connect** — Owns DT-01..04. User runs `code tunnel` on their own machine (EC2, home server); Vector attaches over the Microsoft Dev Tunnels relay. Day-1 spike resolves the subprocess/vendor/defer decision tree. (completed 2026-05-22)
- [ ] **Phase 9: Persistence + Reconnect** — `Domain::reconnect()` hot-swap, inline "Reconnecting…" status bar on remote panes. **Scope revised 2026-05-22:** Vector no longer auto-attaches to tmux; user owns tmux lifecycle on the remote (see 09-CONTEXT.md D-04..D-06).
- [ ] **Phase 10: Hardening & Release** — Renderer snapshot + VT conformance suites in CI, perf gates, tagged unsigned Universal DMG on GitHub Releases.

## Phase Details

### Phase 1: Foundation & CI/DMG Pipeline
**Goal**: A black `Vector.app` opens from a CI-produced unsigned Universal DMG, with the winit/tokio main-thread ownership pattern locked in from day one.
**Depends on**: Nothing (first phase).
**Requirements**: BUILD-01, BUILD-02, BUILD-03, BUILD-04, BUILD-05, WIN-05
**Success Criteria** (what must be TRUE):
  1. Pushing a commit to `main` triggers GitHub Actions and produces a downloadable `Vector.dmg` artifact for the empty app shell.
  2. Tagging a release publishes the unsigned Universal DMG to GitHub Releases with the `xattr -dr com.apple.quarantine` instructions in the README.
  3. Running `cargo xtask dmg` locally produces an identical DMG on an Apple Silicon dev machine.
  4. The skeleton runs `winit::EventLoop` on the main thread and a multi-thread `tokio` runtime on background threads, with `EventLoopProxy::send_event` as the only cross-thread signal — verified by an architecture lint and a smoke test that crashes the build if a `tokio::main` macro reappears.
**Plans**: 6 plans
  - [x] 01-01-PLAN.md — Workspace skeleton + toolchain pin + xtask alias + 14 crate stubs
  - [x] 01-02-PLAN.md — Workspace lints + cargo-deny + cargo-husky + per-crate architecture-lint tests
  - [x] 01-03-PLAN.md — Threading skeleton + AppKit window + native menu + version overlay + build.rs SHA
  - [x] 01-04-PLAN.md — xtask separate workspace + cargo-bundle + create-dmg + universal DMG pipeline (Wave-0 spike)
  - [x] 01-05-PLAN.md — GitHub Actions ci.yml: matrix-then-merge build + tip release (macos-15-intel amendment)
  - [x] 01-06-PLAN.md — release.yml on v* tags + README install block + ADRs 0001..0006 + branch protection setup
**Stack additions**: `cargo` workspace, `rust-toolchain.toml` pinned to 1.88.0, `cargo-bundle 0.10`, `lipo`, `iconutil`, `hdiutil`, `create-dmg`, `cargo-deny`, `cargo-husky`, `convco`, `git-cliff`, GitHub Actions on `macos-14` (arm64) + `macos-15-intel` (x86_64) matrix (D-21 amended: macos-13 retired Dec 2025).
**Risks & notes**:
  - Universal binary on CI: macOS 14 runners are arm64-only; macOS 13 runners are x86_64. Validate matrix-build + `lipo` end-to-end here, do not assume.
  - **winit/tokio main-thread ownership must be established in this phase's skeleton.** Getting it wrong here bites in Phase 3 and is expensive to retrofit.
  - Bake the `xattr` Gatekeeper-bypass instructions into the README on first publish, not as a footnote.
  - Re-check the Out-of-Scope list at phase boundary: no signing, no notarization, no Sparkle.

### Phase 2: Headless Terminal Core
**Goal**: Running `cargo run --bin vector-headless` opens a local shell whose output renders correctly into the in-memory grid for a VT conformance corpus, with no GPU code involved.
**Depends on**: Phase 1 (workspace, threading skeleton).
**Requirements**: CORE-01, CORE-02, CORE-03, CORE-04, CORE-05, CORE-06
**Success Criteria** (what must be TRUE):
  1. The headless binary spawns a user's login shell, pipes bytes through `alacritty_terminal`, and `echo hello` lands in cell (0,0) of the grid.
  2. A VT conformance corpus (CSI / OSC / DCS dispatch, partial-UTF-8 reads, alt-screen DECSET 1049, scroll regions DECSTBM, tab stops, ED/EL erase) runs as `cargo test` and passes.
  3. The grid renders 24-bit truecolor and 256-color SGR correctly for indexed and direct-color sequences, with grapheme-cluster cell width verified for emoji ZWJ + East Asian width samples.
  4. Resizing the headless window propagates `SIGWINCH` to the child process group, and closing the binary leaves no zombie shell processes (verified in `ps`).
  5. `TERM=xterm-256color` is advertised and 10,000+ lines of scrollback survive a regex search across history.
**Plans**: 5 plans
  - [x] 02-01-PLAN.md — Wave 0: workspace deps + new vector-headless crate scaffold + alacritty_terminal 0.26 API spike + 13 test-file #[ignore] stubs
  - [x] 02-02-PLAN.md — Wave 1: vector-term wrapper (Term::new/feed/resize/grid/cursor/mode/search) + 10 conformance test files filled (CORE-01/02/03/06)
  - [x] 02-03-PLAN.md — Wave 2: vector-pty LocalPty (portable-pty + spawn_blocking + bounded mpsc + drop(pair.slave) + Drop kill+wait) + 5 lifecycle/term-env tests (CORE-04/05)
  - [x] 02-04-PLAN.md — Wave 3: vector-mux PtyTransport + Domain traits (D-38 final shape) + LocalDomain full impl + Codespace/DevTunnel stubs + object-safety test
  - [x] 02-05-PLAN.md — Wave 4: vector-headless binary — raw-mode bridge + 30Hz ANSI repaint + SIGWINCH watcher + manual smoke checkpoint (vim/tmux/htop/less +F)
**Stack additions**: `alacritty_terminal 0.26`, `vte 0.15` (transitive), `portable-pty 0.9`, `tokio::task::spawn_blocking` PTY bridge.
**Risks & notes**:
  - **Never roll a custom VT parser.** Pitfall 1 — decided day 1 of this phase, irrevocable.
  - Feed raw `&[u8]` to the parser. Never `from_utf8_lossy` on PTY chunks (Pitfall 4).
  - PTY signal/resize handling requires `portable-pty` (handles `posix_openpt`/`forkpty` edge cases per Pitfall 7).
  - Re-check Out-of-Scope: no Sixel, no Kitty graphics, no custom terminfo.

### Phase 3: GPU Renderer & First Paint
**Goal**: Launching `Vector.app` opens a single window-single tab-single pane GPU-rendered terminal where you can run `vim` at sustained 60+fps on Apple Silicon.
**Depends on**: Phase 2 (headless terminal core).
**Requirements**: RENDER-01, RENDER-02, RENDER-03, RENDER-04, RENDER-05, WIN-01
**Success Criteria** (what must be TRUE):
  1. `Vector.app` opens a native AppKit window with title bar, fullscreen, and standard window-control buttons; running `vim` inside renders correctly with a visible cursor.
  2. `cat large.log` sustains 60+ fps on Apple Silicon at 1080p; on a ProMotion display the frame rate honors the 120 Hz refresh.
  3. Idle CPU stays below 1% on Apple Silicon when the terminal has no dirty rows (verified via Activity Monitor over a 60-second idle).
  4. Switching from a Retina internal display to a non-Retina external monitor (and back) keeps the glyph atlas correct — no broken glyphs, no visible re-rasterization stutter beyond the first frame.
  5. Selecting text and moving the cursor with arrow keys composites the selection rectangle and cursor over the live grid without flicker.
**Plans**: 5 plans
  - [x] 03-01-PLAN.md — Wave 1: wgpu surface lifecycle + clear-color frame + Wave-0 test stubs + workspace deps + Term::damage wrapper
  - [x] 03-02-PLAN.md — Wave 2: crossfont rasterizer + bundled JetBrains Mono + two-atlas wgpu textures + bounded LRU eviction
  - [x] 03-03-PLAN.md — Wave 3: cell pipeline + cursor pipeline + Grid→quads compositor + truecolor/256-color + offscreen render harness
  - [x] 03-04-PLAN.md — Wave 4: vector-input xterm keymap (≥80 cases) + Cmd-V bracketed paste + click-drag selection + write/resize mpsc into I/O actor
  - [x] 03-05-PLAN.md — Wave 5: PTY coalesce + render-on-dirty + LPM throttle + DPR atlas clear + resize debounce + first-paint gate + manual smoke matrix (autonomous=false)
**Stack additions**: `wgpu 29`, `winit 0.30`, `objc2-app-kit 0.3`, `crossfont 0.9`, `unicode-width 0.2`, `bytemuck 1`, `etagere 0.2`, `parking_lot 0.12`, `pollster 0.4`, `bytes 1`.
**Risks & notes**:
  - Two atlases (monochrome + color emoji), bounded LRU eviction (Pitfall 2).
  - `wgpu::PresentMode::Fifo` only; render only on dirty (Pitfall 3).
  - Pin a bundled font (e.g. JetBrains Mono) for snapshot-test determinism — CoreText shaping is not version-stable.
  - The winit/tokio threading model from Phase 1 is now exercised under real load. Any cross-thread regression surfaces here.
  - Re-check Out-of-Scope: no IME composition window UI, no Sixel.

### Phase 4: Mux — Tabs & Splits
**Goal**: A user can open a new tab with Cmd-T and split a pane with Cmd-D / Cmd-Shift-D, with each pane running an independent local shell.
**Depends on**: Phase 3 (GPU renderer + window).
**Requirements**: WIN-02, WIN-03, WIN-04
**Success Criteria** (what must be TRUE):
  1. Cmd-T opens a new tab; Cmd-Shift-] / Cmd-Shift-[ cycles tabs; Cmd-W closes a tab. Behavior matches native `NSWindowTabbingMode` or a visually equivalent custom tab bar.
  2. Cmd-D splits the active pane horizontally; Cmd-Shift-D splits vertically. Each pane independently runs a shell and accepts focus, with arrow-key or hjkl-style focus routing.
  3. Resizing the window propagates new sizes to all panes and child shells; `tput cols` in any pane reports the correct width.
  4. The `Domain / Pane / PtyTransport` abstraction is the only seam between the terminal model and the transport — verified by a grep that finds zero `enum PaneSource` discriminations inside `vector-term`.
**Plans**: 6 plans
  - [x] 04-01-PLAN.md — Wave 0: workspace deps + 13 Wave-0 test stubs + SpawnedPane struct + LocalPty child_pid/master_fd accessors (preserves D-38)
  - [x] 04-02-PLAN.md — Wave 1: Mux singleton + Window/Tab/PaneNode tree + split mutation + close cascade + directional focus + resize-nudge + WIN-04 grep arch-lint live
  - [x] 04-03-PLAN.md — Wave 2: per-pane PTY actor router (JoinSet<PaneId>) + UserEvent migration + Mux async helpers + cwd inheritance (libproc::pidcwd) + foreground-process tracking (D-57) + real-PTY integration tests
  - [x] 04-04-PLAN.md — Wave 3: vector-input EncodedKey enum + 14 Mux shortcuts + multi-window NSWindowTabbingMode + per-pane Compositor + active-pane border (D-66) + inactive cursor outline
  - [x] 04-05-PLAN.md — Wave 4: per-TabWindow first-paint gate + focus-change redraw discipline + per-window resize debounce + manual smoke matrix (autonomous=false) — partial: Task 1 fully landed (22a8272); Task 2 smoke matrix returned 6/9 PASS, 3 FAILs (#3 visible side-by-side render / #4 tput cols per-pane viewport math / #8 visible D-66 border) routed to Plan 04-06 gap-closure
  - [x] 04-06-PLAN.md — Wave 6 (gap-closure, autonomous=false): AppWindow → per-pane Compositor map migration; per-pane RedrawRequested LoadOp chain; per-pane viewport SIGWINCH via Mux::resize_window; visible D-66 active-pane border at focus change; closes Gap 1/2/3 from 04-VERIFICATION.md (smoke items #3, #4, #8); flips WIN-02 + WIN-03 to Complete
**Stack additions**: `vector-mux` crate (WezTerm-style `Mux::get()` singleton, recursive split tree, `EventLoopProxy<UserEvent>` for I/O→UI signaling), `Box<dyn PtyTransport>` (WezTerm-style `Mux::get()` singleton, recursive split tree, `EventLoopProxy<UserEvent>` for I/O→UI signaling), `Box<dyn PtyTransport>`.
**Risks & notes**:
  - The `Domain/Pane/PtyTransport` seam established here is a load-bearing decision — Phases 7, 8, and 9 all depend on it. Embedding transport logic in the terminal model is Architecture Anti-Pattern 1.
  - No layout save/restore, no broadcast-input — Pitfall 21 scope creep guard.
  - Re-check Out-of-Scope: no tmux control mode, no command palette yet.

### Phase 5: Polish (Local Daily-Driver)
**Goal**: Vector becomes the user's daily driver — config hot-reloads, ligatures work, OSC 52 copies through tmux, scrollback regex search finds the last error.
**Depends on**: Phase 4 (mux + panes).
**Requirements**: POLISH-01, POLISH-02, POLISH-03, POLISH-04, POLISH-05, POLISH-06, POLISH-07, POLISH-08
**Success Criteria** (what must be TRUE):
  1. Editing `~/.config/vector/config.toml` and saving hot-reloads theme, font, and keybinds without restart; profile inheritance (`[default]` + named overrides) is verified by a test fixture.
  2. Ligatures, Nerd Font glyphs, and `.itermcolors` import all render correctly with a user-supplied font from `~/Library/Fonts`.
  3. `printf '\e]52;c;%s\a' "$(echo hello | base64)"` puts "hello" in the macOS clipboard. Inside real tmux 3.4+ on a Codespace (smoke-tested manually before phase boundary), the DCS-wrapped form `\eP\e]52;c;…\a\e\\` round-trips correctly.
  4. Scrollback regex search highlights matches with next/prev navigation; OSC 7 (cwd), OSC 8 (hyperlinks), OSC 10/11/12 (color queries), and OSC 133 (semantic prompt marks) are observable in a shell-integration smoke test.
  5. Saved profiles named `local`, `codespace`, `dev_tunnel` exist in the config with per-profile env, theme, tint, and startup command. Secure Keyboard Entry can be toggled from a menu item; basic IME composition displays under the cursor (no candidate window UI).
**Plans**: 16 plans (10 original + 6 gap-closure 2026-05-12 after verifier surfaced wiring gaps post-smoke)
  - [x] 05-01-PLAN.md — Wave 0: D-83 hardening (workspace lints + path-dep arch-lint + cargo-deny pre-commit + cargo-machete CI) + 22 Wave-0 test stubs + 10 workspace deps
  - [x] 05-02-PLAN.md — Wave 1: vector-config schema + loader (POLISH-01, POLISH-07)
  - [x] 05-03-PLAN.md — Wave 1: vector-theme palette + chrome tokens + Vector Light/Dark builtins + .itermcolors importer (POLISH-03)
  - [x] 05-04-PLAN.md — Wave 2: notify-debouncer-full watcher + apply pipeline diff_config + parse-error keep-last-good (POLISH-01, POLISH-02)
  - [x] 05-05-PLAN.md — Wave 1: OSC sniffer + ForwardingListener + OSC 8 hyperlink grouping (POLISH-04)
  - [x] 05-06-PLAN.md — Wave 1: OSC 52 raw + DCS-wrapped + 58-byte outbound chunking + tmux smoke (POLISH-05)
  - [x] 05-07-PLAN.md — Wave 2: Cmd-C selection-string + ligatures + Nerd Font + SearchBar smart-case + 1000-cap cache (POLISH-02, POLISH-06)
  - [x] 05-08-PLAN.md — Wave 3: Logic — Tint stripe pipeline + Profile picker + Toast state machine + Clipboard router + OSC 7 consumers (POLISH-07)
  - [x] 05-10-PLAN.md — Wave 4: Wiring & rendering scaffolding (POLISH-04, POLISH-06, POLISH-07)
  - [x] 05-09-PLAN.md — Wave 5: Secure Keyboard Entry + IME data machine + vector-secrets API + manual 10-item smoke matrix checkpoint (POLISH-08)
  - [x] 05-11-PLAN.md — Wave 1 (gap-closure): impl GridAccess for Term + Cmd-C real selection + Switch Profile submenu dynamic rebuild (gap #5 + #6) (POLISH-06, POLISH-07)
  - [x] 05-12-PLAN.md — Wave 1 (gap-closure): App.clipboard_router field + UserEvent::ClipboardStore + ForwardingListener clip_tx drain task (gap #7) (POLISH-05)
  - [x] 05-13-PLAN.md — Wave 1 (gap-closure): vector-input keymap EncodedKey::App(AppShortcut) for Cmd-N/F/Shift-P/Shift-R (gap #2 pure data) (POLISH-06, POLISH-07, POLISH-08)
  - [x] 05-14-PLAN.md — Wave 2 (gap-closure): App.search_bar + App.profile_picker fields + EncodedKey::App handler bodies + ungrouped Cmd-N + config reload (gap #2 App-side + gap #3) (POLISH-01, POLISH-06, POLISH-07)
  - [x] 05-15-PLAN.md — Wave 2 (gap-closure): declare_class! NSTextInputClient subclass + App.ime field + WindowEvent::Ime dispatch + set_ime_allowed (gap #4) (POLISH-08)
  - [x] 05-16-PLAN.md — Wave 3 (gap-closure): RenderHost owns TintStripe+SearchBar+Toast+Picker pipelines + per-frame chrome orchestration UI-SPEC §11 order + smoke matrix re-run (gap #1) (POLISH-04, POLISH-06, POLISH-07)
**Stack additions**: `serde + toml 1.1.2`, `notify` (FSEvents on macOS), `keyring 4.0` initialized here for later phases, `vector-config`, `vector-theme`, `vector-secrets`.
**Risks & notes**:
  - **DCS-wrapped OSC 52 through tmux is a known pitfall (Pitfall 8).** Smoke-test on real tmux 3.4+ with `set -g allow-passthrough on` before declaring the phase done. Truncation at ~60 chars is a real bug to design around.
  - Single TOML file, `deny_unknown_fields`, no DSL (Pitfall 11).
  - Full IME (CJK candidate window) is explicitly v2 — basic NSTextInputClient composition only here.
  - Re-check Out-of-Scope: no Lua, no plugins, no command palette.

### Phase 6: GitHub Auth + Codespaces Picker
**Goal**: A user can sign into GitHub from inside Vector and see a list of their Codespaces with state, repo, branch, and last-used time — no SSH transport yet.
**Depends on**: Phase 5 (config, profiles, Keychain wired in).
**Requirements**: AUTH-01, AUTH-02, AUTH-03, CS-01, CS-02, CS-03
**Success Criteria** (what must be TRUE):
  1. Clicking "Sign in with GitHub" runs the OAuth Device Flow (RFC 8628), shows the user-code, and stores the resulting token in macOS Keychain via `keyring 4.0` — verified by `security find-generic-password` and a `grep -r 'gho_'` of disk and logs returning zero hits.
  2. After sign-in, the picker lists every codespace for the user with state (Available / Shutdown / Starting), repository name, branch, and last-used timestamp; the list refreshes when state changes.
  3. Selecting a Shutdown codespace triggers `POST /user/codespaces/{name}/start`, swallows 409 Conflict, polls `state` at 1s for up to 2 minutes, and shows progress until Available.
  4. A picked codespace can be saved as a one-click profile (kind = `codespace`, codespace_name + tint persisted) that survives app restart. Clicking "Connect" on a profile shows a placeholder toast (Phase 7 wires it).
  5. Token refresh on 401 silently re-runs device flow; expired tokens never silently fail — the user sees a re-auth prompt.
**Plans**: 7 plans
  - [x] 06-01-PLAN.md — Wave 0: vector-codespaces scaffold + workspace deps + Wave-0 test stubs + Pitfall-14 arch-lint
  - [x] 06-02-PLAN.md — Wave 1: OAuth Device Flow driver (oauth2 5.0) + Keychain TokenStore + manual Debug discipline (AUTH-01, AUTH-02)
  - [x] 06-03-PLAN.md — Wave 1: CodespacesClient REST (list/get/start/poll) + 401 silent-refresh chain (CS-01, CS-02, AUTH-03)
  - [x] 06-04-PLAN.md — Wave 1: vector-config writer append_codespace_profile + derive_profile_name with atomic rename (CS-03)
  - [x] 06-05-PLAN.md — Wave 2: UserEvent extensions + AuthDeviceFlowModal NSPanel + Sign in/out menu items + Cmd-Shift-G keymap
  - [x] 06-06-PLAN.md — Wave 2: CodespacesPickerModal NSPanel + codespaces_actor + Connect/Start/Save flows + relative-time formatter
  - [ ] 06-07-PLAN.md — Wave 3: manual UAT smoke matrix (autonomous=false) — 11 items spanning AUTH-01..03 + CS-01..03 + token-leak audit
**Stack additions**: `oauth2 5.0` device flow, `octocrab 0.50`, `reqwest 0.13` (rustls-tls), `keyring-core 1.0` + `apple-native-keyring-store 1.0` (already wired in vector-secrets), `serde_json 1`, `chrono 0.4`, `urlencoding 2`, `tokio-util 0.7 sync`, `http 1`, `wiremock 0.6` (dev), `zeroize 1`.
**Risks & notes**:
  - Use classic OAuth scopes (`codespace`, `read:user`); fine-grained PATs are explicitly broken with Codespaces (`cli/cli#7819`).
  - Manual `Debug` impls on every token-bearing struct — never derive (Pitfall 14).
  - Picker UI ships before SSH transport — deliberate de-risking split. CS-04..07 belong to Phase 7.
  - Re-check Out-of-Scope: no codespace lifecycle (create/delete/rebuild), no PORTS panel.

### Phase 7: Remote SSH Transport Scaffolding (DESCOPED 2026-05-19)
**Status**: Pivoted. Original Codespaces-Connect scope was the wrong product. The transport-layer scaffolding from plans 01..03 was kept (russh client, SSH channel transport, host-key fingerprint pinning, mux transport helper, `[remote]` tab badge). The codespace-specific glue from plan 04 (codespace_actor, CodespaceDomain, `register_ssh_key`, `get_codespace_with_connection`, `gh codespace ssh --stdio` subprocess build) and plan 05 (smoke matrix) was reverted.
**What survived** (reusable groundwork for Phase 8):
  - `crates/vector-ssh/` — russh 0.60 client, `SshChannelTransport` with biased select for resize/write/read, `ChildStdioStream` (AsyncRead+AsyncWrite over subprocess), `VectorHandler` with SHA-256 host-key fingerprint check (Pitfall 3)
  - `Mux::create_tab_async_with_transport` — install `Box<dyn PtyTransport>` directly; vector-mux stays russh-free per WIN-04
  - `format_tab_title` extended for `TransportKind`; `[remote]` badge for any non-local pane
  - Workspace deps: `russh 0.60`, `ssh-key 0.6`
**What was reverted**:
  - `CodespaceDomain::spawn` and `codespace_actor::spawn_connect`
  - `register_ssh_key` (POST /user/keys with 422 dedup), `get_codespace_with_connection`, `CodespaceWithConnection` model
  - `KeyManager` (ed25519 keygen at `~/.ssh/vector_codespace_ed25519`)
  - `build_gh_stdio_command` (gh subprocess wiring)
  - `apply_codespace_tint_if_active`, `UserEvent::CodespacePaneReady`
  - `write:public_key` OAuth scope addition (back to `codespace + read:user`)
  - SMOKE.md (codespace smoke matrix)
**Why pivoted**: The user clarified they want VS Code Remote Tunnels (own machine + `code tunnel`), not GitHub Codespaces. Codespaces lifecycle ceremony (create / start / pick) was the wrong UX for the actual use case. CS-04..07 dropped from REQUIREMENTS.md; DT-01..04 moved into Phase 8 (was already there).
**Plans**: 5 plans
  - [x] 07-01-PLAN.md — vector-ssh scaffold + workspace deps (kept)
  - [x] 07-02-PLAN.md — KeyManager + register_ssh_key + fingerprint fetch (REVERTED — codespace-specific)
  - [x] 07-03-PLAN.md — SshClient + SshChannelTransport real impl (kept)
  - [x] 07-04-PLAN.md — CodespaceDomain + codespace_actor + app.rs wire-up + tint (REVERTED — codespace-specific)
  - [ ] 07-05-PLAN.md — smoke matrix (DELETED — required live codespace)

### Phase 8: VS Code Remote Tunnels Connect
**Goal**: A signed-in user can attach Vector to one of their own machines running `code tunnel`, getting a remote shell in a Vector pane that's visually distinct from local panes.
**Depends on**: Phase 6 (auth), Phase 7 (russh + transport scaffolding), Phase 4 (Domain/PtyTransport seam).
**Requirements**: DT-01, DT-02, DT-03, DT-04
**Success Criteria** (what must be TRUE):
  1. The phase begins with a 1–2 day spike that commits a written decision to `.planning/research/spikes/dev-tunnels-decision.md` choosing among (a) subprocess `code tunnel client`, (b) vendor `microsoft/dev-tunnels/rs/` at a pinned SHA, (c) defer to v2. No integration code is written before the decision lands.
  2. If the spike chose (a) or (b): a signed-in user sees their active VS Code Remote Tunnels listed in a picker, with tunnel name, host machine, and last-seen.
  3. If the spike chose (a) or (b): clicking a tunnel opens a remote shell in a new pane via the chosen transport.
  4. The connected pane is visually distinct from local (tinted tab + `[remote]` badge) so the user always knows what they're typing into.
  5. If the spike chose (c): the decision document is committed, REQUIREMENTS.md moves DT-02..04 to v2 with reason, and Phase 8 closes as "spike + decision document".
**Plans**: 7 plans
  - [x] 08-01-foundations-scaffold-PLAN.md — Wave 1: vendor SDK + russh patch + 3 new crates + arch-lint extension + Wave-0 test stubs
  - [x] 08-02-microsoft-oauth-PLAN.md — Wave 2: Microsoft OAuth Device Flow driver + Keychain TokenStore (D-03..06)
  - [x] 08-03-tunnel-agent-binary-PLAN.md — Wave 2: vector-tunnel-agent Linux binary (RelayTunnelHost + PTY spawn + JSON protocol loop, D-A1/D-07..15)
  - [x] 08-04-mac-client-transport-PLAN.md — Wave 2: vector-tunnels REST + DevTunnelTransport (PtyTransport impl) + connect-tunnel helper
  - [x] 08-05-picker-ui-and-actor-PLAN.md — Wave 3: DevTunnelsPickerModal + MicrosoftAuthDeviceFlowModal + devtunnels_actor + Cmd-Shift-T + Microsoft-blue tint (D-11/D-17, UI-SPEC §S1+S2)
  - [x] 08-06-agent-distribution-PLAN.md — Wave 3: cargo-deb metadata + agent-release.yml CI cross-compile x86_64+aarch64 .deb (D-01)
  - [x] 08-07-uat-smoke-matrix-PLAN.md — Wave 4: DT-01 spike doc + 9-item manual smoke matrix sign-off (Task 1 template authored 2026-05-21 commit b5d006e; Task 2 checkpoint:human-verify — user must walk 08-SMOKE.md end-to-end on real hardware before Phase 8 closes)
**Research-spike-required flag**: **YES.** Day 1 is a mandatory 1–2 day spike. Do not estimate the rest of the phase until the spike resolves the decision tree.
**Stack additions** (conditional on spike outcome): `microsoft/dev-tunnels` at pinned SHA OR subprocess `code tunnel client` OR none (deferred). Existing `russh 0.60` + `vector-ssh` from Phase 7 carry over.
**Risks & notes**:
  - **Highest known risk in v1.** The Rust SDK exists in `microsoft/dev-tunnels/rs/` but is not on crates.io, has gaps (no auto-reconnect, no token refresh, internally pinned to russh 0.37 vs. our 0.60), and may lag protocol changes.
  - russh 0.37 vs. 0.60 conflict: fork + bump or accept ~3MB binary duplication.
  - Defer-to-v2 is an acceptable spike outcome; the phase reduces to "spike + decision document" without blocking v1 release.
  - SSH host-key trust uses the tunnel's API-provided fingerprint, not TOFU bypass.
  - `pty-req` must send initial cols/rows and `window-change` on resize.
  - Re-check Out-of-Scope: no port-forwarding panel, no clean-room reverse-engineering of the relay protocol.

### Phase 9: Persistence + Reconnect
**Goal**: The user closes their laptop lid for a meeting, reopens it, and a Dev Tunnels pane reconnects automatically — the local grid + scrollback never go blank, an inline status bar shows reconnect progress, and the transport hot-swaps under the live `Pane` without losing bytes already in flight. Shell-state-across-disconnect persistence is the user's responsibility (they run tmux themselves on the remote if they want it).
**Depends on**: Phase 8 (Dev Tunnels transport).
**Requirements**: PERSIST-01, PERSIST-02, PERSIST-03 (revised), PERSIST-04 (revised)
**Success Criteria** (what must be TRUE):
  1. On TCP/SSH disconnect, the affected pane enters a `Reconnecting` state, the local grid + scrollback stay in memory (no blank screen), input is locked (not queued), and an inline status bar at the top of the pane shows `Reconnecting to {profile}… (attempt N)`.
  2. `Domain::reconnect()` re-establishes the transport with exponential backoff (1s / 2s / 4s / 8s / 16s / 30s cap, retries forever at the cap) and hot-swaps the `PtyTransport` under the live `Pane` without dropping bytes already in flight — verified by a test that disconnects and reconnects with `cat /dev/urandom` running and asserts no byte loss.
  3. **(REVISED 2026-05-22)** Vector does NOT auto-attach to tmux. Remote panes connect to the user's default shell; the user runs tmux themselves on the remote if they want shell-state persistence across full disconnects. PERSIST-03 captures the same constraint at the requirement level.
  4. **(REVISED 2026-05-22)** An end-to-end smoke test against a live Dev Tunnels agent on a remote box running tmux 3.4+ verifies that Vector's terminal correctly passes through DCS-wrapped OSC 52, DECSCUSR cursor shapes, mouse modes 1000/1002/1003 with SGR 1006, and `TERM=xterm-256color` advertisement when the user is running tmux themselves. The smoke test still verifies Pitfall 8 (DCS passthrough, ~60-char chunking) on the live path — it just doesn't depend on Vector having started tmux.
**Plans**: TBD
**Stack additions**: `Domain::reconnect()` state machine (Active → Reconnecting → Swapping → Active), inline status-bar overlay UI on the renderer, transport hot-swap in the per-pane actor.
**Risks & notes**:
  - **DCS-wrapped OSC 52 through tmux is the known pitfall (Pitfall 8) revisited at the seam.** The Phase 5 smoke test verified the local-only path; this phase verifies the full Vector → Dev Tunnels relay → agent → user-started tmux → Vector round-trip.
  - Never hold the terminal lock across `await` (Architecture Anti-Pattern 5). Lock, mutate, drop, await.
  - **No mosh-style state-sync protocol.** tmux on the remote is the answer for shell-state persistence — but Vector doesn't manage tmux itself (Pitfall 22 still locks the "no state-sync in Vector" stance; tmux ownership moved to the user).
  - Re-check Out-of-Scope: no custom remote agent, no predictive echo, no Vector-managed tmux sessions, no app-restart pane restore.
  - Canonical ref: `.planning/phases/09-persistence-reconnect-tmux-auto-attach/09-CONTEXT.md` — all decisions and the tmux scope change rationale.

### Phase 10: Hardening & Release
**Goal**: Vector v1.0.0 is tagged on GitHub Releases with an unsigned Universal DMG; teammates can install with the documented `xattr` command.
**Depends on**: Phase 9 (all v1 features in place).
**Requirements**: HARDEN-01, HARDEN-02, HARDEN-03, HARDEN-04
**Success Criteria** (what must be TRUE):
  1. A renderer snapshot test suite runs headless against a pinned bundled font with a perceptual-tolerance comparator; the suite is a CI gate that blocks merges on regression.
  2. The VT conformance corpus (alt-screen, scroll regions, tab stops, ED/EL, mouse 1006, OSC 52 round-trip, bracketed paste, DECSCUSR) runs in CI on every push and a perf gate enforces idle CPU <1% and `cat large.log` at the vsync cap.
  3. A `cargo deny` policy blocks unaudited unsafe in release-profile dependencies; a `grep` of `tracing` output from a session with auth shows zero token-shaped strings (`gho_`, `ghp_`, `eyJ`).
  4. Tagging `v1.0.0` publishes the unsigned Universal `Vector.dmg` to GitHub Releases with the README's install instructions (including `xattr -dr com.apple.quarantine /Applications/Vector.app`) front-and-center.
**Plans**: TBD
**Stack additions**: `insta`-style snapshot testing, `cargo deny` policy file, perf benchmark harness.
**Risks & notes**:
  - The "looks done but isn't" checklist from PITFALLS.md is the gate here. Every item gets a specific test before tag.
  - Re-check Out-of-Scope one final time: no signing, no notarization, no Sparkle, no auto-update — these are v2 (DIST-V2-01, DIST-V2-02).

## Phase Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation & CI/DMG Pipeline | 6/6 | Implementation complete; verifier next | 2026-05-10 |
| 2. Headless Terminal Core | 0/5 | Plans created | - |
| 3. GPU Renderer & First Paint | 0/0 | Not started | - |
| 4. Mux — Tabs & Splits | 5/5 | Plans complete; 04-05 partial sign-off (6/9 smoke PASS, #3/#4/#8 FAIL routed to Plan 04-06 gap-closure); verifier next | - |
| 5. Polish (Local Daily-Driver) | 15/16 | In Progress|  |
| 6. GitHub Auth + Codespaces Picker | 0/7 | Plans created | - |
| 7. SSH Transport + Codespaces Connect | 0/0 | Not started | - |
| 8. Dev Tunnels Integration | 7/7 | Complete   | 2026-05-22 |
| 9. Persistence + Reconnect + tmux Auto-Attach | 0/0 | Not started | - |
| 10. Hardening & Release | 0/0 | Not started | - |

## Coverage

**v1 requirements: 51 / 51 mapped (no orphans)**

| Category | IDs | Phase |
|----------|-----|-------|
| Build & Distribution | BUILD-01..05 | Phase 1 |
| Window Threading | WIN-05 | Phase 1 |
| Terminal Core | CORE-01..06 | Phase 2 |
| Rendering | RENDER-01..05 | Phase 3 |
| Window (chrome) | WIN-01 | Phase 3 |
| Window (tabs/splits/mux) | WIN-02..04 | Phase 4 |
| Polish | POLISH-01..08 | Phase 5 |
| GitHub Auth | AUTH-01..03 | Phase 6 |
| Codespaces Picker | CS-01..03 | Phase 6 |
| Remote SSH Transport Scaffolding | — (descoped 2026-05-19) | Phase 7 |
| VS Code Remote Tunnels Connect | DT-01..04 | Phase 8 |
| Persistence & Reconnect | PERSIST-01..04 | Phase 9 |
| Hardening & Release | HARDEN-01..04 | Phase 10 |

## Dependency Graph

```
Phase 1 (Foundation/CI/DMG, threading)
   └── Phase 2 (Headless terminal core)
         └── Phase 3 (GPU renderer + first paint)
               └── Phase 4 (Mux: tabs/splits, Domain/PtyTransport seam)
                     └── Phase 5 (Polish: config, themes, OSC, scrollback)
                           └── Phase 6 (GitHub auth + Codespaces picker)
                                 └── Phase 7 (SSH transport scaffolding — descoped)
                                       └── Phase 8 (VS Code Remote Tunnels — spike-gated)
                                             └── Phase 9 (Persistence + reconnect + tmux)
                                                   └── Phase 10 (Hardening + release)
```

## Phase Boundary Discipline

At every phase transition, re-check the Out-of-Scope list in REQUIREMENTS.md. Every researcher flagged scope creep as the project's biggest non-technical risk; the discipline is to keep saying no.

## Backlog

### Phase 999.1: AI autocomplete + command-history-aware suggestions (BACKLOG)

**Goal:** Claude API-powered inline command suggestions, drawing on shell history (`~/.zsh_history` / `~/.bash_history`) plus Vector's own per-session history. Real differentiator vs Warp/Wave/Tabby.

**Requirements:** TBD (likely a new `AI-*` family in REQUIREMENTS.md when promoted)

**Plans:** 7/7 plans complete

**Trigger:** After milestone v1.0.0 ships (Phase 10 release). Per PROJECT.md key decision: "must not gate terminal-core work."

**Depends on (for context, not phase ordering):**
- Phase 3 (GPU renderer) — needed to render dim/ghost-text suggestion overlays inline
- Phase 4 (mux) — per-tab/per-pane history isolation
- Phase 6 (GitHub auth) — auth-flow pattern reference for Claude API key handling (or reuse Keychain pattern from CORE-secrets)

**Open questions (for /gsd:discuss-phase when promoted):**
- Local Claude (`claude` CLI on PATH) vs Anthropic API direct? Latter needs API-key storage; former piggybacks on user's existing auth.
- History scope: last N commands, or semantic search over full history with embeddings?
- Streaming vs blocking suggestions — what's the keystroke-latency budget?
- Privacy: opt-in only, never send history without explicit toggle.

Plans:
- [ ] TBD (promote with `/gsd:review-backlog` when ready)

---
*Roadmap created: 2026-05-10*
