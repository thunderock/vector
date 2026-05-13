# Roadmap: Vector

**Created:** 2026-05-10
**Granularity:** fine (10 phases)
**Total v1 requirements:** 51
**Coverage:** 51 / 51 mapped

## Core Value

Open the app, pick a Codespace, get a fast remote shell — no VS Code, no browser, no clunky `gh codespace ssh` plumbing. Local-terminal niceties (tabs, splits, GPU rendering) are table-stakes; the differentiator is that a Codespaces / Dev-Tunnels session feels native, not bolted on.

## Phases

- [ ] **Phase 1: Foundation & CI/DMG Pipeline** — Cargo workspace + winit/tokio threading skeleton + Universal unsigned DMG produced by CI on every push.
- [ ] **Phase 2: Headless Terminal Core** — `alacritty_terminal`-backed VT parser + grid + scrollback + local PTY; conformance tests pass headless.
- [ ] **Phase 3: GPU Renderer & First Paint** — wgpu/Metal renderer with damage-tracked atlas; single-window single-tab terminal you can run `vim` in.
- [ ] **Phase 4: Mux — Tabs & Splits** — Window/Tab/Pane tree with `Domain`/`PtyTransport` abstractions; iTerm-class local terminal.
- [ ] **Phase 5: Polish (Local Daily-Driver)** — TOML config + hot-reload, themes/fonts/ligatures, OSC 7/8/52/133/10/11/12, scrollback search, tmux pass-through.
- [ ] **Phase 6: GitHub Auth + Codespaces Picker** — OAuth device flow, Keychain token storage, codespace picker UI; clicking "Connect" still shows a placeholder.
- [ ] **Phase 7: SSH Transport + Codespaces Connect** — `gh codespace ssh --stdio` subprocess transport, `CodespaceDomain`, end-to-end remote shell with tab tint and resize.
- [ ] **Phase 8: Dev Tunnels Integration** — Day-1 spike resolves the subprocess/vendor/defer decision tree; `DevTunnelDomain` if green.
- [ ] **Phase 9: Persistence + Reconnect + tmux Auto-Attach** — `Domain::reconnect()` hot-swap, "Reconnecting…" overlay, `tmux new -A -s vector-{profile-id}` on connect.
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
  - [ ] 05-11-PLAN.md — Wave 1 (gap-closure): impl GridAccess for Term + Cmd-C real selection + Switch Profile submenu dynamic rebuild (gap #5 + #6) (POLISH-06, POLISH-07)
  - [ ] 05-12-PLAN.md — Wave 1 (gap-closure): App.clipboard_router field + UserEvent::ClipboardStore + ForwardingListener clip_tx drain task (gap #7) (POLISH-05)
  - [ ] 05-13-PLAN.md — Wave 1 (gap-closure): vector-input keymap EncodedKey::App(AppShortcut) for Cmd-N/F/Shift-P/Shift-R (gap #2 pure data) (POLISH-06, POLISH-07, POLISH-08)
  - [ ] 05-14-PLAN.md — Wave 2 (gap-closure): App.search_bar + App.profile_picker fields + EncodedKey::App handler bodies + ungrouped Cmd-N + config reload (gap #2 App-side + gap #3) (POLISH-01, POLISH-06, POLISH-07)
  - [ ] 05-15-PLAN.md — Wave 2 (gap-closure): declare_class! NSTextInputClient subclass + App.ime field + WindowEvent::Ime dispatch + set_ime_allowed (gap #4) (POLISH-08)
  - [ ] 05-16-PLAN.md — Wave 3 (gap-closure): RenderHost owns TintStripe+SearchBar+Toast+Picker pipelines + per-frame chrome orchestration UI-SPEC §11 order + smoke matrix re-run (gap #1) (POLISH-04, POLISH-06, POLISH-07)
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
**Plans**: TBD
**Stack additions**: `oauth2 5.0` device flow, `octocrab 0.50`, `reqwest 0.13` (rustls), `keyring 4.0`.
**Risks & notes**:
  - Use classic OAuth scopes (`codespace`, `read:user`); fine-grained PATs are explicitly broken with Codespaces (`cli/cli#7819`).
  - Manual `Debug` impls on every token-bearing struct — never derive (Pitfall 14).
  - Picker UI ships before SSH transport — deliberate de-risking split. CS-04..07 belong to Phase 7.
  - Re-check Out-of-Scope: no codespace lifecycle (create/delete/rebuild), no PORTS panel.

### Phase 7: SSH Transport + Codespaces Connect
**Goal**: Clicking a Codespace from the picker drops the user into a remote shell in a Vector pane, with resize, tab tint, and a "remote" badge.
**Depends on**: Phase 6 (auth + picker), Phase 4 (Domain/PtyTransport seam).
**Requirements**: CS-04, CS-05, CS-06, CS-07
**Success Criteria** (what must be TRUE):
  1. Clicking "Connect" on an Available codespace from the picker opens a working remote shell in a new Vector pane via subprocess `gh codespace ssh --stdio` — end-to-end, with `pwd` returning the codespace's working directory.
  2. Vector generates and registers an ed25519 SSH keypair per machine via the GitHub API on first connect; subsequent connects reuse it without prompting the user for `ssh-add`.
  3. A connected codespace pane is visually distinct: tab is tinted (e.g. GitHub-purple) and a "remote" badge appears in the tab title; the user always knows the pane is remote.
  4. Resizing the window or pane sends an SSH `window-change` request through the transport; remote `vim` and `tmux` reflow correctly within one second.
**Plans**: TBD
**Strategy / phasing note**: **v1 transport is subprocess `gh codespace ssh --stdio`.** The native russh + tonic + port-16634 gRPC reimplementation is **v1.x**, not part of v1. The phase plan must reflect the subprocess path explicitly. (See requirement CS-V2-01 in the v2 backlog.)
**Stack additions**: `russh 0.60` (loaded but used only for the SSH-channel layer riding atop the subprocess pipe), `vector-ssh`, `CodespaceDomain`.
**Risks & notes**:
  - **Codespaces SSH is not plain TCP SSH.** It rides a tunneled relay with an OAuth-derived ephemeral cert behind a stateful API. The subprocess path eliminates the gnarliest protocol work from the v1 critical path (Pitfall 9).
  - SSH host-key trust uses the API-provided fingerprint, not TOFU bypass (Pitfall 15).
  - `pty-req` must send initial cols/rows and `window-change` on resize (Pitfall 7).
  - Re-check Out-of-Scope: no native russh + gRPC path (v1.x), no port-forwarding panel.

### Phase 8: Dev Tunnels Integration
**Goal**: A user can pick a Dev Tunnel from the same picker as Codespaces and get a remote shell — using whichever transport the day-1 spike picked.
**Depends on**: Phase 7 (Domain/PtyTransport seam exercised under remote load).
**Requirements**: DT-01, DT-02, DT-03, DT-04
**Success Criteria** (what must be TRUE):
  1. The phase begins with a 1–2 day spike that commits a written decision document to `.planning/research/spikes/dev-tunnels-decision.md` choosing among (a) subprocess `code tunnel client`, (b) vendor `microsoft/dev-tunnels/rs/` at a pinned SHA, or (c) defer to v2. No integration code is written before the decision lands.
  2. If the spike chose (a) or (b): a signed-in user sees active Dev Tunnels listed alongside Codespaces in the picker, with tunnel name, host machine, and last-seen.
  3. If the spike chose (a) or (b): clicking a Dev Tunnel opens a remote shell in a new pane via the chosen transport; the pane is visually distinct from Codespaces (different tab tint color so the user knows "this is my own box" vs "this is GitHub-managed").
  4. If the spike chose (c): the decision document is committed, REQUIREMENTS.md is updated to move DT-02..04 to v2 with reason, and Phase 8 closes as "spike + decision document" — the implementation moves to v2.
**Plans**: TBD
**Research-spike-required flag**: **YES.** Day 1 of this phase is a mandatory 1–2 day spike. Do not estimate the rest of the phase until the spike resolves the decision tree.
**Stack additions** (conditional on spike outcome): `microsoft/dev-tunnels` at pinned SHA OR subprocess `code tunnel client` OR none (deferred).
**Risks & notes**:
  - **Highest known risk in v1.** The Rust SDK exists in `microsoft/dev-tunnels/rs/` but is not on crates.io, has gaps (no auto-reconnect, no token refresh, internally pinned to russh 0.37 vs. our 0.60), and may lag protocol changes.
  - russh 0.37 vs. 0.60 conflict: fork + bump or accept ~3MB binary duplication.
  - Defer-to-v2 is an acceptable spike outcome; the phase reduces to "spike + decision document" without blocking v1 release.
  - Nightly smoke test against the live service on subsequent days (Pitfall 13).
  - Re-check Out-of-Scope: no clean-room reverse-engineering of the relay protocol.

### Phase 9: Persistence + Reconnect + tmux Auto-Attach
**Goal**: The user closes their laptop lid for a meeting, reopens it, and a Codespaces pane reconnects automatically with full session state preserved via tmux.
**Depends on**: Phase 7 (Codespaces transport) and Phase 8 (or its deferral decision).
**Requirements**: PERSIST-01, PERSIST-02, PERSIST-03, PERSIST-04
**Success Criteria** (what must be TRUE):
  1. On TCP/SSH disconnect, the affected pane enters a `Reconnecting` state, the local grid + scrollback stay in memory (no blank screen), and a "Reconnecting…" overlay appears.
  2. `Domain::reconnect()` re-establishes the transport with exponential backoff and hot-swaps the `PtyTransport` under the live `Pane` without dropping bytes already in flight — verified by a test that disconnects and reconnects with `cat /dev/urandom` running and asserts no byte loss.
  3. Codespace and Dev Tunnel sessions auto-attach to a Vector-managed tmux session on connect (`tmux new -A -s vector-{profile-id}`) so the remote shell state survives full disconnects.
  4. An end-to-end smoke test against real tmux 3.4+ on a live Codespace verifies DCS-wrapped OSC 52, DECSCUSR cursor shapes, mouse modes 1000/1002/1003 with SGR 1006, and `TERM=xterm-256color` advertisement all round-trip cleanly.
**Plans**: TBD
**Stack additions**: `Domain::reconnect()` state machine (Active → Reconnecting → Swapping → Active), reconnect overlay UI, profile-driven tmux wrapper command.
**Risks & notes**:
  - **DCS-wrapped OSC 52 through tmux is the known pitfall (Pitfall 8) revisited at the seam.** The Phase 5 smoke test verified the local-only path; this phase verifies the full Codespace → tmux → Vector round-trip on the live service.
  - Never hold the terminal lock across `await` (Architecture Anti-Pattern 5). Lock, mutate, drop, await.
  - **No mosh-style state-sync protocol.** tmux on the remote is the answer (Pitfall 22).
  - Re-check Out-of-Scope: no custom remote agent, no predictive echo.

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
| 5. Polish (Local Daily-Driver) | 0/9 | Plans created | - |
| 6. GitHub Auth + Codespaces Picker | 0/0 | Not started | - |
| 7. SSH Transport + Codespaces Connect | 0/0 | Not started | - |
| 8. Dev Tunnels Integration | 0/0 | Not started | - |
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
| Codespaces SSH Connect | CS-04..07 | Phase 7 |
| Dev Tunnels | DT-01..04 | Phase 8 |
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
                                 └── Phase 7 (SSH transport + Codespaces connect)
                                       └── Phase 8 (Dev Tunnels — spike-gated)
                                             └── Phase 9 (Persistence + reconnect + tmux)
                                                   └── Phase 10 (Hardening + release)
```

## Phase Boundary Discipline

At every phase transition, re-check the Out-of-Scope list in REQUIREMENTS.md. Every researcher flagged scope creep as the project's biggest non-technical risk; the discipline is to keep saying no.

## Backlog

### Phase 999.1: AI autocomplete + command-history-aware suggestions (BACKLOG)

**Goal:** Claude API-powered inline command suggestions, drawing on shell history (`~/.zsh_history` / `~/.bash_history`) plus Vector's own per-session history. Real differentiator vs Warp/Wave/Tabby.

**Requirements:** TBD (likely a new `AI-*` family in REQUIREMENTS.md when promoted)

**Plans:** 0 plans

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
