# Vector

## What This Is

Vector is a native macOS terminal — written in Rust, GPU-accelerated — with first-class VS Code Remote Tunnels support baked in. It is meant to replace iTerm/ghostty as a daily-driver local terminal *and* let me (and a few Adobe teammates) sign in with GitHub, attach to one of our own remote machines running `code tunnel`, and drop into a remote dev shell without ever opening VS Code or a browser.

**Pivoted 2026-05-19:** Original spec called for GitHub Codespaces support. Mid-Phase 7, user clarified the actual use case is connecting to *their own* machines via VS Code Remote Tunnels, not GitHub-managed codespace containers. Phase 7 codespace-specific code was reverted; transport scaffolding (russh client, SSH channel transport, tab tint + `[remote]` badge) kept for Phase 8.

## Core Value

**Open the app, pick a remote machine running `code tunnel`, get a fast remote shell — no VS Code, no browser.** Local-terminal niceties (tabs, splits, GPU rendering) are table-stakes; the differentiator is that a Dev-Tunnels session feels native, not bolted on.

## Requirements

### Validated

- [x] CI build pipeline that produces installable `.dmg` artifacts (Phase 1 — operationally validated 2026-05-11; CI tip + tagged v2026.5.10 Universal DMG both confirmed launching on macOS Sequoia)
- [x] xterm-compatible terminal core (parser + grid + scrollback) suitable as a daily-driver local shell (Phase 2 — `vector-headless` proxy ran vim/tmux/htop/less cleanly on 2026-05-11; CORE-01..06 backed by 53 passing tests, conformance suite 0.326s vs 1s D-37 budget)
- [x] GPU-accelerated terminal rendering on Mac (Metal via wgpu) — Phase 3 operationally validated 2026-05-11: wgpu Metal `Surface<'static>` with PresentMode::Fifo, crossfont + dual-atlas (mono RGBA8 + color emoji) with bounded LRU, Compositor reading `Term::damage()` with truecolor/256-color SGR + per-cell selection bit + block cursor, xterm keymap + bracketed paste + click-drag selection + scroll-wheel scrollback, PTY-burst coalescing (8 ms), LPM 30 fps cap, DPR atlas invalidation, debounced resize, first-paint timing gate. RENDER-01..05 + WIN-01 all verified. Workspace: 175 passing / 0 failed / 0 ignored. 9-item manual smoke matrix signed off (vim, large.log fps, idle <1% CPU, Retina swap, selection, Cmd-V paste, ProMotion, LPM, Cmd-Ctrl-F fullscreen).
- [x] Tabs and splits (horizontal/vertical), multiple sessions per window — Phase 4 operationally validated 2026-05-12 after Plan 04-06 gap closure. `vector-mux` Mux singleton + Window/Tab/PaneNode tree + split tree with directional focus + resize-nudge + close-cascade; per-pane PTY actors via `tokio::task::JoinSet` with per-pane `CoalesceBuffer`/`frame_tick`; foreground process name polling (D-57) + cwd inheritance via `proc_pidinfo` (D-63/D-64); native NSWindow tab groups via winit `set_tabbing_identifier` + objc2-app-kit (D-56) routing one `NSWindow` per Tab; per-pane Compositor map in `AppWindow` with chained `LoadOp::Clear`/`LoadOp::Load` per leaf and visible D-66 active-pane border; 14 mux keymap entries (Cmd-Opt-Arrow, Cmd-Shift-Arrow, Cmd-T/D/Shift-D/W/Shift-]/Shift-[) that never reach the PTY. WIN-02/03/04 all validated. Workspace: 231 passing / 0 failed / 3 ignored. D-38 invariant intact (zero diff in `vector-mux/src/{domain,transport}.rs`). 9-item smoke matrix signed off (multi-pane visible render, per-pane `tput cols` after SIGWINCH, visible D-66 border, Cmd-T tab group, Cmd-W cascade, cwd inheritance, idle <1% CPU, zsh↔vim title flip, DPR re-rasterize across panes).

### Active

- [x] Polish local terminal to daily-driver quality — config hot-reload, theme engine, search bar, profile picker, OSC 52 clipboard, IME, Secure Keyboard Entry, hyperlinks, OSC 7 cwd, Cmd-N window spawning — Phase 5 operationally validated 2026-05-14: all 8 POLISH requirements verified; 16/16 plans complete; 332 tests passing; 10-item smoke matrix 10/10 approved.
- [x] GitHub OAuth sign-in flow (device-code) with token caching in macOS Keychain — Phase 6 code-complete 2026-05-14: AUTH-01/02/03 fully wired (device-flow + Keychain via vector-secrets + 401 silent-refresh chain); AppKit `AuthDeviceFlowModal` NSPanel + `Sign in with GitHub` menu item + Cmd-Shift-G; 363 workspace tests pass; Pitfall-14 arch-lint enforces zero-Debug-on-token discipline; token-leak grep 0 hits. Human smoke matrix (11 items) tracked in `06-HUMAN-UAT.md` — drive via `/gsd:verify-work 6`.
- [~] List / pick GitHub Codespaces from the UI (no `gh` CLI required) — Phase 6 code-complete 2026-05-14 but functionally dormant after 2026-05-19 pivot. CS-01/02/03 wiring (`CodespacesPickerModal` NSPanel + `CodespacesClient` REST + start/poll + Save-as-profile) survives in-tree; `Connect` is a placeholder toast. Will be repurposed or removed once the tunnel-picker UX is designed.
- [ ] Native macOS app distributed as an unsigned `.dmg` (right-click → Open), Universal binary
- [ ] Session persistence + transparent reconnect — wifi drop should not lose remote-session state
- [ ] tmux pass-through that "just works" — no double-multiplex visual glitches when remote tmux is running
- [ ] Connect to a remote machine running `code tunnel` (Microsoft Dev Tunnels) using GitHub auth
- [ ] Saved profiles (`my-cs-frontend`, `my-corp-box`, etc.) for one-click reconnect
- [ ] Themes, fonts, ligatures (table-stakes terminal eye-candy)
- [x] CI build pipeline that produces installable `.dmg` artifacts on every tag (ghostty-style "tip" + tagged releases) — Phase 1 operationally validated 2026-05-11: ci.yml DAG produces `tip` DMG on each master push; release.yml dual-trigger (CLI tag push OR GitHub UI Publish) produces tagged DMG with xattr install footer

### Out of Scope

- **Apple Developer signing & notarization** — deferred. v1 ships unsigned with right-click-Open instructions; revisit only if right-click flow is too painful for teammates.
- **Linux and Windows builds** — Mac-only for v1. The user runs Mac and so do the teammates. Cross-platform doubles the surface area for no payoff today.
- **GitHub Codespaces support entirely** — descoped 2026-05-19. The actual use case is VS Code Remote Tunnels (own machines), not GitHub-managed containers. Phase 6 picker code stays in-tree but dormant; Phase 7 codespace-specific code was reverted.
- **Port-forwarding UI ("PORTS" panel)** — deferred to v2. Useful but not on the critical path; remote dev works without it for most flows.
- **File transfer (drag-drop / scp UI)** — deferred. `scp`/`rsync` in the shell suffice while we focus on terminal core.
- **Arbitrary SSH targets as first-class profiles** — deferred. v1 is VS Code Remote Tunnels only. Plain SSH still works because the terminal launches whatever command you give it; there's just no special UI.
- **Browser-based / web companion (vscode.dev style)** — explicitly anti-goal. Native-only is a feature.
- **AI features beyond the optional Claude integration** — no command sharing, no analytics, no account system. Bloat is part of why we are not using Warp/Wave.
- **Fork of ghostty or VS Code** — we read them as reference but build fresh in Rust. Submodule references in this repo will be removed.

## Context

**Background — what triggered this:** I currently sign into GitHub from VS Code (or Claude Code) just to use their remote-tunnel feature, then do all my actual development inside their built-in terminal. I want that connection capability inside a real terminal so I can stop launching a heavyweight IDE just to get a shell.

**Reference implementations to read for design ideas (not vendored):**
- **ghostty** (Zig) — the gold standard for Mac-native terminal UX, AppKit integration, and DMG release pipeline. Reference for app shell, window/tab structure, packaging.
- **Alacritty** (Rust) — minimal GPU terminal. Reference for renderer architecture, escape-sequence parser, the `alacritty_terminal` crate (split out as a library).
- **WezTerm** (Rust) — closest existing Rust terminal to what we want; has SSH, multiplexing, tabs/splits, lua config. Reference for tab/split UX and SSH transport.
- **VS Code Remote Tunnels** — defines the Dev Tunnels client behavior we need to replicate. Microsoft Dev Tunnels has no public Rust SDK, so this is the riskiest piece.

**Differentiators vs Warp / Wave / Tabby (which I tried):**
- They treat remote tunnels as a second-class SSH target; we treat them as headline UX (sign-in, picker, profile).
- They bundle cloud accounts, AI products, command sharing, analytics — we ship a terminal and a tunnel client, full stop.

**Why Rust:** I asked about Rust explicitly. We're not forking ghostty (Zig) or VS Code (TypeScript/Electron) — we're building fresh. Rust gives the right balance of performance, ecosystem (alacritty_terminal, vte, wgpu, tokio, octocrab/reqwest), and cross-platform potential when we eventually go beyond Mac.

**No special approval needed:** Microsoft Dev Tunnels is a public, documented, OAuth-authenticated API (GitHub OAuth works as the identity provider). No partner approval is required to ship a third-party client.

## Constraints

- **Tech stack:** Rust (workspace). GPU rendering via `wgpu` (Metal backend on macOS). Terminal core via `alacritty_terminal` crate or in-house VT parser using `vte`. Async runtime: `tokio`. GitHub API: `octocrab` or `reqwest`-based client. App shell: native AppKit via `objc2` / `cocoa-rs`, or a minimal cross-platform layer like `winit` + a Mac-native window scaffold.
- **Platform:** macOS only for v1. Apple Silicon + Intel via Universal binary. macOS 13 (Ventura) baseline.
- **Distribution:** Unsigned `.dmg` for v1. CI must produce a downloadable artifact per release. No Apple Developer subscription required initially.
- **Audience:** Personal use first; a handful of Adobe teammates as a soft second wave. No public open-source push for v1.
- **Workflow:** Commit each logical stage separately; **do not push** — the user reviews diffs and pushes asynchronously.
- **Scope discipline:** Resist scope creep. If a feature is not on the v1 list, default to deferring it.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Build in Rust from scratch (not fork ghostty/VS Code) | User explicitly asked about Rust; ghostty is Zig and VS Code is Electron, neither matches the desired stack. Rust ecosystem (alacritty_terminal, wgpu, tokio) is mature enough. | — Pending |
| VS Code Remote Tunnels only (Codespaces dropped) | 2026-05-19 mid-Phase-7 pivot. User clarified they want "sign into my own remote box and connect" UX — Codespaces lifecycle ceremony (create/start/pick) was the wrong fit. CS-04..07 dropped from REQUIREMENTS.md; Phase 8 (DT-01..04) now owns the remote flow. | Confirmed 2026-05-19 |
| Replace iTerm/ghostty as default local terminal (not remote-only launcher) | Halving the surface area to remote-only would shrink scope, but the user explicitly wants a daily-driver. Local terminal is "free" once we have the rendering core. | — Pending |
| Defer signing/notarization to v2 | Apple Developer cert costs $99/yr and adds CI complexity. Right-click-Open is acceptable for an internal tool. Revisit if Gatekeeper friction becomes painful for teammates. | — Pending |
| Defer port-forwarding UI and file-transfer to v2 | They're VS-Code-terminal niceties but not on the critical path; remote dev works without them. Keeps v1 scope finite. | — Pending |
| Remove the empty `ghostty` and `vscode` submodule references | The references are stale (160000 entries with no `.gitmodules`). We'll read those repos out-of-tree if we need them, not vendor them. | — Pending |
| Capture optional Claude-API integration (autocomplete/autosuggest) as a v2 ambition | User raised it as "if possible". It's a real differentiator vs Warp, but it must not gate the terminal-core work. | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-22 after Phase 8 code-complete — VS Code Remote Tunnels integration shipped. Three new crates: `vector-tunnel-protocol` (JSON-framed agent ↔ client codec), `vector-tunnel-agent` (Linux binary: Dev Tunnel Host registration via `microsoft/dev-tunnels` SDK, OAuth device flow with token persistence to disk mode 0600, `portable-pty` session spawn per accepted relay channel), `vector-tunnels` (Mac-side: Microsoft OAuth Device Flow driver against `common` endpoints with Keychain-backed `MicrosoftTokenStore` mirroring Phase-6 `GitHubAuth` one-to-one; Management REST list/get/connect-scope tokens; `DevTunnelTransport` opening a relay channel via the SDK and speaking the JSON agent protocol; `DevTunnelDomain` glue for `Mux::create_tab_async_with_transport`). `vector-app` adds `MicrosoftAuthDeviceFlowModal` (480x280), `DevTunnelsPickerModal` (640x480), `devtunnels_actor`, 3 menu items (Sign in/out of Microsoft, Dev Tunnels…), Cmd-Shift-T keymap, Microsoft-blue (#0078d4) tint stripe distinct from Phase-6 codespace purple. Workspace `[patch.crates-io] russh = vscode-russh` activated — dual-russh dep graph (0.37 Microsoft fork for SDK + 0.60 vanilla for vector-ssh) coexists cleanly. Pitfall-14 arch-lint extended to all three new crates + new identifier guards (`agent_token`, `tunnel_access_token`). DT-01..04 code-complete; 9-item manual smoke matrix tracked in `08-SMOKE.md` and `.deb` install round-trip in `08-06-HUMAN-UAT.md` (deferred — no Linux env on dev box). Cross-phase regression in Phase-6 `auth_refresh` tests (rustls CryptoProvider auto-select panic from dual-feature activation) fixed via shared `tests/support::ensure_crypto_provider` helper installing `aws-lc-rs` once at test entry. Workspace tests 142 suites passing / 0 failed; token-leak scan (`gho_/ghu_/ghp_/eyJ/Bearer`) returns 0 hits. Phase 9 (persistence + reconnect + tmux auto-attach) is next.*

*Previously updated: 2026-05-14 after Phase 6 code-complete — GitHub auth + Codespaces picker shipped. `vector-codespaces` crate: OAuth Device Flow (RFC 8628) + Keychain-backed `TokenStore` + `CodespacesClient` with raw octocrab `_get`/`_post` + 401 silent-refresh chain. `vector-config::writer::append_codespace_profile` + `derive_profile_name` (toml_edit round-trip + atomic rename matching Plan 05-04 watcher). `vector-app`: `AuthDeviceFlowModal` NSPanel (440x280, NSFloatingWindowLevel, clipboard save/restore per Pitfall 7), `CodespacesPickerModal` NSPanel (640x480, LoadState, per-row poll tasks), `auth_actor` + `codespaces_actor` tokio drivers, 10 new UserEvent variants, 3 menu items (Sign in / Sign out / Codespaces…), Cmd-Shift-G keymap, D-84 sign-in chokepoint, Pitfall-14 arch-lint (manual Debug on token-bearing structs, no tracing macros near tokens). Workspace tests 363 passing / 0 failed / 5 ignored (manual UAT placeholders). Token-leak audit (`gho_/ghu_/ghp_`) returns 0 hits. AUTH-01/02/03 + CS-01/02/03 all code-complete; 11-item manual smoke matrix tracked in `06-HUMAN-UAT.md` for `/gsd:verify-work 6`. Phase 7 (Dev Tunnels + gRPC SSH transport via russh) is next.*

*Previously updated: 2026-05-12 after Phase 4 complete — tabs + splits shipped. `vector-mux` adds a Mux singleton + Window/Tab/PaneNode tree + split tree with directional focus, resize-nudge, and close-cascade; per-pane PTY actors via `tokio::task::JoinSet` with per-pane `CoalesceBuffer`/`frame_tick`; foreground process name polling (D-57) + cwd inheritance via `proc_pidinfo` (D-63/D-64); native NSWindow tab groups via winit `set_tabbing_identifier` + objc2-app-kit (D-56) with one `NSWindow` per Tab; per-pane Compositor map in `AppWindow` with chained `LoadOp::Clear`/`LoadOp::Load` and visible D-66 active-pane border; 14 mux keymap entries (Cmd-Opt-Arrow, Cmd-Shift-Arrow, Cmd-T/D/Shift-D/W/Shift-]/Shift-[) that never reach the PTY. Plan 04-06 closed three gaps (smoke #3 multi-pane visible render, #4 per-pane `tput cols` after SIGWINCH, #8 visible D-66 border) by extending `AppWindow` with `compositors: HashMap<PaneId, Compositor>` + `active_pane_id` and routing per-pane SIGWINCH through `Mux::resize_window` → `PtyActorRouter::send_resize`. Workspace tests 231 passing / 0 failed / 3 ignored; D-38 byte-identical invariant intact (zero diff in `vector-mux/src/{domain,transport}.rs`); arch-lint count 16; 9-item manual smoke matrix signed off. WIN-02 + WIN-03 + WIN-04 all validated.*
