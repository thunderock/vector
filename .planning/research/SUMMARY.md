# Project Research Summary

**Project:** Vector
**Domain:** Native macOS GPU-accelerated terminal in Rust with built-in GitHub Codespaces SSH and Microsoft Dev Tunnels client
**Researched:** 2026-05-10
**Confidence:** HIGH for terminal core / stack / architecture; MEDIUM for Codespaces gRPC plumbing and Dev Tunnels Rust SDK

## Executive Summary

Vector is a native macOS terminal emulator built in Rust — GPU-accelerated via wgpu/Metal — that treats GitHub Codespaces and Microsoft Dev Tunnels as headline UX rather than generic SSH targets. The research converges on a well-understood path for the terminal core: `alacritty_terminal` for VT parsing/grid, `wgpu 29` for Metal-backed rendering, `winit + objc2-app-kit` for the AppKit window shell, `portable-pty` for local PTY, `russh` for SSH, `octocrab + oauth2` for GitHub auth, and TOML config throughout. The build and distribution story is clear: cargo workspace with per-crate separation, `lipo` + `cargo-bundle` + `hdiutil` for an unsigned Universal DMG, CI-produced from day one. There is no disagreement across the four research domains on any of these choices.

The two highest-risk items both involve remote connectivity. For Codespaces SSH, the pitfalls researcher identified a decisive shortcut: subprocess `gh codespace ssh --stdio` as the v1 transport, deferring the native russh + gRPC reimplementation (the `cli/cli` port-16634 path) to v1.x. This eliminates the gnarliest protocol work from the critical path while delivering the full user-facing feature. For Dev Tunnels, a decision tree must be applied at the start of that phase: (a) subprocess `code tunnel client` if available, (b) vendor the `microsoft/dev-tunnels/rs/` crate at a pinned commit, (c) defer Dev Tunnels to v2 entirely. A 1-2 day spike resolves the branch before any integration code is written. The persistence story is equally clear: auto-attach to a Vector-managed tmux session on the remote — no mosh, no custom remote agent.

Scope discipline is the most important non-technical finding. Every researcher flagged the same traps: IME completeness, Sixel/Kitty graphics, plugins, file browser, Blocks UI, cloud account, AI bundled by default, mosh-style remote agent, Lua config. These are not minor deferrals — they represent the category of decisions that have wrecked comparable projects. The recommended phase structure explicitly keeps them out of scope and builds the constraint into each phase boundary.

## Key Findings

### Recommended Stack

The entire stack is conventional for a Rust GPU terminal and carries HIGH confidence on crates.io-verified versions. The only non-standard element is the `microsoft/dev-tunnels` Rust SDK, which exists at `rs/` in the GitHub repo but is not published to crates.io — it must be vendored as a git dep with a pinned rev.

**Core technologies:**
- `alacritty_terminal 0.26` — VT parser + grid + scrollback — battle-tested, library-split, re-exports `vte`; never roll a custom state machine
- `wgpu 29.0.3` (Metal backend) — GPU rendering — industry standard; raw `metal-rs` offers no benefit at v1 scope
- `winit 0.30.13` + `objc2-app-kit 0.3` — window/event loop + native AppKit — winit for event loop and raw-window-handle, drop into `objc2` for `NSWindowTabbingMode`, menus, and Services
- `tokio 1.52` — async runtime — required by russh, octocrab, reqwest, tonic; PTY reads go on a blocking thread via `portable-pty`, not directly on the async runtime
- `portable-pty 0.9` — local PTY — cross-platform abstraction, handles `posix_openpt`/`forkpty` edge cases, future Windows port stays cheap
- `russh 0.60.2` — SSH client — tokio-native, pure Rust, used by Microsoft's own Dev Tunnels Rust SDK; `openssh` (shell-out wrapper) is not acceptable for programmatic port forwarding
- `octocrab 0.50` + `oauth2 5.0` — GitHub REST API + device-code flow — typed, async, RFC 8628 device flow for desktop auth
- `keyring 4.0` — macOS Keychain — tokens never on disk; requires Rust 1.88+ (pin in `rust-toolchain.toml`)
- `tonic 0.14` + `prost 0.13` — gRPC client — needed for the Codespaces internal port-16634 management API (v1.x, after subprocess phase); proto files vendored from `cli/cli/internal/codespaces/rpc/`
- `crossfont 0.9` — font rasterization (CoreText on macOS) — handles ligatures, emoji, CJK fallback; `cosmic-text`/`glyphon` is the alternative if `crossfont` hits limits
- `serde + toml 1.1.2` — config — TOML only; no Lua (`mlua` is 1.5 MB of binary bloat and a scripting attack surface); config files hot-reload via `notify`
- `cargo-bundle 0.10` + `lipo` + `hdiutil` — packaging — unsigned Universal DMG; CI builds both arches separately then `lipo -create`
- `microsoft/dev-tunnels` (`rs/`, git dep, pinned SHA) — Dev Tunnels client — Management API + Tunnel Client green in support matrix; see decision tree in Pitfalls

**Critical version constraints:**
- Rust 1.88+ required (`keyring 4.0`); pin in `rust-toolchain.toml`
- `russh 0.60` vs. `dev-tunnels/rs/` internal `russh 0.37` — dual-version in dep graph; either fork dev-tunnels and bump, or accept ~3 MB binary duplication
- `wgpu 29` + `winit 0.30` compatible via `raw-window-handle 0.6`; use `wgpu::Instance::create_surface_unsafe`
- Do NOT use: `cocoa-rs` (unmaintained), `harfbuzz_rs` (stale 2021), `thrussh` (russh predecessor), `ssh2` (C dep, not async), `gpui` (full UI framework, wrong layer), `tauri` (webview-based), Lua config

### Expected Features

**Must have (table stakes — terminal core):**
- xterm-compatible VT parser, 24-bit truecolor, 256-color, Unicode + emoji + grapheme clustering, East Asian width
- Scrollback >=10k lines (ring buffer), scrollback regex search
- OSC 7 (cwd), OSC 8 (hyperlinks), OSC 52 (clipboard — raw AND DCS-wrapped), OSC 133 (semantic prompt marks), OSC 10/11/12 (fg/bg/cursor color queries)
- Bracketed paste (mode 2004), mouse modes 1000/1002/1003/1006 SGR, alternate screen (DECSET 1049), DECSCUSR cursor shape, SIGWINCH
- GPU-accelerated rendering (60+ fps under `yes` / large `cat`), damage tracking (per-row dirty flags), ProMotion/120Hz support
- Tabs (Cmd-T, native `NSWindowTabbingMode` or custom), horizontal/vertical splits (recursive pane tree)
- Bring-your-own-font, ligatures (opt-in), Nerd Font glyphs, iTerm `.itermcolors` import
- Hot-reload config on save, native macOS chrome, fullscreen, Secure Keyboard Entry toggle
- Profiles (`local`, `codespace`, `dev_tunnel`) with per-profile env, theme, tint, startup command
- Universal binary, unsigned `.dmg` with CI pipeline on every tag

**Must have (remote differentiators):**
- GitHub OAuth Device Flow + macOS Keychain token storage
- Codespaces picker: list, state, repo, branch, last-used, region, latency hint; one-click start of Shutdown codespaces
- Codespaces SSH connect — subprocess `gh codespace ssh --stdio` for v1; native russh + gRPC for v1.x
- Dev Tunnels connect — decision-tree approach (subprocess -> vendor SDK -> defer); picker for tunnel list
- Saved profiles = one-click reconnect; visual "this is remote" tab tint + status badge
- Auto-attach to Vector-managed tmux session (`tmux new -A -s vector`) on remote for session persistence
- Reconnect overlay UI with exponential backoff; keep local grid+scrollback in memory across disconnects
- tmux pass-through correctness: DCS-wrapped OSC 52, DECSCUSR, mouse modes, `TERM=xterm-256color` advertisement

**Should have (v1.x after validation):**
- macOS dark/light mode auto-follow, command palette (Cmd-Shift-P), quick terminal / hotkey window
- ssh-terminfo auto-install on remote (ghostty pattern)
- Apple Developer signing + notarization (if right-click friction becomes painful for teammates)

**Defer to v2+:**
- Claude API autosuggest (BYO-key, opt-in, ghost-text only)
- Port-forwarding PORTS panel, file transfer GUI, Codespaces lifecycle (create/delete/rebuild)
- Arbitrary SSH targets as first-class profiles, Sixel / Kitty graphics, Linux / Windows builds, plugin/scripting layer

**Anti-features (explicitly out of scope):**
- Cloud account / mandatory login, telemetry, AI bundled by default, Blocks UI, web companion
- IME completeness beyond basics (NSTextInputClient — multiple weeks, out for v1)
- Mosh-style remote agent / custom state-sync protocol
- Lua config, plugin marketplace, file browser / sidebar

### Architecture Approach

The architecture follows the WezTerm `Domain/Pane/PtyTransport` model. The critical boundary is that `vector-term` accepts a generic `PtyTransport` trait (`AsyncRead + AsyncWrite + resize`) and knows nothing about whether it is talking to a local pty, an SSH channel, or an SSH-over-WebSocket-over-Dev-Tunnels stream. `vector-mux` owns the Window -> Tab -> Pane tree and `Domain` instances; `vector-codespaces` and `vector-tunnels` produce `Box<dyn PtyTransport>` that mux hands to a pane. Threading follows the mandatory macOS pattern: `winit::EventLoop` on the main thread (non-negotiable), `tokio` multi-thread runtime on background threads, `EventLoopProxy::send_event` as the only legal cross-thread signal. Never `block_on` on the main thread.

**Major components (workspace crates):**
1. `vector-app` — macOS app shell: `winit` event loop + `objc2`/AppKit menus, URL scheme handler (`vector://`), DMG packaging via `xtask`
2. `vector-term` — VT parser + grid + scrollback: wraps `alacritty_terminal`; accepts `PtyTransport` trait; zero transport awareness
3. `vector-render` — wgpu Metal pipeline: damage-tracked glyph atlas (`crossfont`/`glyphon`), instanced quads, cursor/selection overlays
4. `vector-mux` — Window/Tab/Pane tree: `Domain` trait (`spawn_pane`, `reconnect`, `is_alive`), routes I/O, holds session state across reconnects
5. `vector-pty` — local PTY: `portable-pty` wrapper, `spawn_blocking` bridge to tokio
6. `vector-ssh` — russh SSH client: PTY channel, port-forward, agent-forward; accepts any `AsyncRead+AsyncWrite` as underlying stream
7. `vector-codespaces` — GitHub auth (OAuth device flow), REST (octocrab), gRPC tunnel to port 16634 (tonic + vendored protos), produces `PtyTransport` via SSH-over-tunnel
8. `vector-tunnels` — Microsoft Dev Tunnels client: wraps `microsoft/dev-tunnels/rs/`, WebSocket relay, produces `PtyTransport` via SSH-over-WebSocket
9. `vector-config` — TOML + `notify` hot-reload, profile inheritance (`[default]` + per-profile overrides)
10. `vector-secrets` — macOS Keychain via `keyring 4.0`; tokens never logged (manual `Debug` impls that redact)
11. `vector-fonts` — font discovery + shaping (`crossfont` primary, `cosmic-text`/`glyphon` alternative)
12. `vector-input` — keymap, IME basics, mouse encoding
13. `vector-theme` — palette types, builtin themes, `.itermcolors` import

**Rendering pipeline:** PTY bytes -> `vte::Perform` on `TerminalModel` -> dirty-row flags -> CPU shape via `crossfont`/`cosmic-text` (cached per `(font, run)`) -> glyph atlas miss rasterizes via `swash` -> per-cell instance buffer for dirty rows only -> one wgpu draw call per frame.

**Persistence architecture:** On disconnect, `Pane` keeps grid+scrollback in memory and enters `Reconnecting` state. `Domain::reconnect()` re-establishes the transport. On success, `PtyTransport` is hot-swapped under the live pane. Session state lives in remote tmux, not in Vector. This is not Mosh; it is sufficient for the stated requirement.

### Critical Pitfalls

1. **Rolling a custom VT parser** — use `alacritty_terminal` or `vte` directly; never write `match byte { 0x1b => ... }`. The dispatch table is hundreds of sequences with decades of edge cases. Decides on day 1 of Phase 2; irrevocable.

2. **Assuming Codespaces SSH is plain TCP SSH** — it is SSH-over-a-tunneled-relay with an OAuth-derived ephemeral cert behind a stateful API (`?internal=true&refresh=true`, state polling, 409-swallow, port-16634 gRPC). v1 strategy: subprocess `gh codespace ssh --stdio` and prove end-to-end shell first; replace with native russh later.

3. **Dev Tunnels integration approach unresolved** — the Rust SDK exists but is unpublished, sparsely documented, and may not stay in sync with the wire protocol. Apply the decision tree on day 1 of the Dev Tunnels phase: subprocess -> vendor -> defer. Do not write clean-room relay protocol code.

4. **`winit` EventLoop + `tokio` runtime on the same thread** — macOS panics. `winit` owns the main thread. `tokio` gets a dedicated background runtime. `EventLoopProxy::send_event` is the only legal crossing. Any `block_on` on the main thread is a deadlock. Decided in Phase 1 skeleton; expensive to fix later.

5. **Scope creep killing velocity** — IME completeness, Sixel, plugins, file browser, Blocks UI, cloud account, mosh-style agent. Each looks small; each has wrecked comparable projects. The anti-feature list is a first-class architectural constraint, not a wish list. Re-read it at every phase boundary.

## Implications for Roadmap

All four research files converge on the same 10-phase ordering. The rationale is dependency-driven: terminal core before GPU, GPU before mux, mux before remote, auth before SSH, Codespaces before Dev Tunnels, reconnect last because it depends on both remotes existing.

### Phase 1: Foundation + CI/DMG Pipeline

**Rationale:** Ghostty's "CI produces a real DMG from day one" is a non-negotiable principle from PROJECT.md. If packaging is deferred it never gets done right.
**Delivers:** Cargo workspace skeleton, unsigned Universal DMG produced by CI on every push, `xtask dmg` command, right-click-Open docs in README.
**Stack:** cargo workspace, `rust-toolchain.toml`, `lipo`, `cargo-bundle 0.10`, `hdiutil`, GitHub Actions on `macos-14` + `macos-13` matrix, `cargo-deny`.
**Avoids:** Gatekeeper friction pitfall (document `xattr -dr` command from day 1). winit/tokio threading pitfall (establish main-thread ownership in the skeleton).

### Phase 2: Headless Terminal Core

**Rationale:** VT conformance is foundational. Every subsequent feature depends on the parser being correct. Finding parser bugs early is cheap; finding them in Phase 7 is expensive.
**Delivers:** `vector-term` (wrapping `alacritty_terminal`), `vector-pty` (`portable-pty`), spawn local shell, byte plumbing, VT conformance test suite. Headless `cargo run` echoes a real shell.
**Stack:** `alacritty_terminal 0.26`, `portable-pty 0.9`, `vte 0.15`, `tokio spawn_blocking` PTY bridge.
**Avoids:** Custom VT parser (Pitfall 1). Partial UTF-8 reads (Pitfall 4 — feed raw bytes to the parser, never `from_utf8_lossy` on PTY chunks). PTY signal/resize handling (Pitfall 7 — `portable-pty` handles `posix_openpt`/`forkpty`).

### Phase 3: GPU Renderer + First Paint

**Rationale:** The renderer must be correct before mux is built on top. Glyph atlas strategy (two atlases for monochrome + emoji, damage tracking) is permanent — retrofitting is painful.
**Delivers:** `vector-render` (wgpu Metal), glyph atlas with damage-tracked redraw, `vector-fonts` (`crossfont` + `glyphon`), `vector-input` keymap. A real terminal you can run `vim` in. Single tab, single pane.
**Stack:** `wgpu 29`, `winit 0.30`, `objc2-app-kit 0.3`, `crossfont 0.9`, `glyphon` (or custom atlas).
**Avoids:** Glyph atlas churn under font fallback (two atlases, `unicode-width` for cell width, bounded LRU atlas). Frame pacing on macOS (`PresentMode::Fifo`, render only on dirty state, ProMotion-aware).
**Research flag:** wgpu + winit version skew on macOS has known integration traps; pin a bundled font for snapshot tests to avoid CoreText non-determinism.

### Phase 4: Mux — Tabs + Splits

**Rationale:** The `Domain/Pane` abstraction must be in place before any remote transport work begins. Adding it later means retrofitting the entire transport layer.
**Delivers:** `vector-mux` (Window/Tab/Pane tree, `Domain` trait, `PtyTransport` trait), `LocalDomain`, tab bar UI, split layout (recursive binary tree), focus routing, resize propagation. iTerm-class local terminal.
**Stack:** WezTerm-style `Mux::get()` singleton pattern, `Box<dyn PtyTransport>`, `EventLoopProxy<UserEvent>` for I/O to UI signaling.
**Avoids:** Embedding transport logic in terminal model (Architecture Anti-Pattern 1). Pane navigation scope creep (splits + tabs only, no broadcast input, no layout save/restore).

### Phase 5: Polish — Themes, Fonts, Profiles, OSC Sequences, Scrollback Search, Hot-Reload

**Rationale:** Daily-driver quality for local use before any remote work begins. If the local terminal is not solid, remote sessions built on top of it will feel broken.
**Delivers:** `vector-config` (TOML + `notify` hot-reload + profile inheritance), `vector-theme` (palette types, builtin themes, `.itermcolors` import), `vector-secrets` (Keychain), scrollback regex search, copy/paste, ligatures, OSC 7/8/52/133/10/11/12 correctness, DCS-wrapped OSC 52, DECSCUSR, tmux pass-through smoke test, `TERM=xterm-256color` advertisement.
**Stack:** `serde + toml 1.1.2`, `notify` (FSEvents on macOS), `keyring 4.0` initialized here even though remote auth comes in Phase 6.
**Avoids:** Config sprawl (single TOML, `deny_unknown_fields`, no DSL). tmux passthrough regression (test OSC 52 DCS round-trip through real tmux 3.4+ early).

### Phase 6: GitHub Auth + Codespaces REST + Picker UI (no SSH yet)

**Rationale:** Deliberate de-risking split. GitHub OAuth + REST + picker UI is straightforward and well-documented. SSH + gRPC + port-forwarding is the gnarly part. Shipping auth and picker first gives a usable UI while the transport is still WIP.
**Delivers:** `vector-codespaces::auth` (OAuth device flow, Keychain storage), `octocrab`-backed codespace list with state/repo/branch/region/latency, one-click start of Shutdown codespaces, picker UI with profile save. Clicking "Connect" shows a "not implemented" toast.
**Stack:** `oauth2 5.0` device flow, `octocrab 0.50`, `reqwest 0.13` (rustls), `keyring 4.0`.
**Avoids:** Fine-grained PAT scopes that do not work with Codespaces (use classic OAuth scopes: `codespace`, `read:user`). Token storage in plaintext file (Keychain from day 1). Token leaks in logs (manual `Debug` redaction on all token-bearing structs).

### Phase 7: SSH Transport + Codespaces Connect

**Rationale:** The Codespaces connection flow is the primary headline feature. It must work before Dev Tunnels to validate the `Domain/PtyTransport` seam under real network conditions.
**Delivers:** `vector-ssh` (russh SSH client, PTY channel, port-forward), `CodespaceDomain`, subprocess `gh codespace ssh --stdio` as v1 transport, full end-to-end Codespaces shell in picker. Vector-managed SSH keypair (ed25519, registered via API). Tab tint + "remote" badge.
**Strategy:** Day 1 of phase: subprocess `gh codespace ssh --stdio` and verify the end-to-end shell. This eliminates the port-16634 gRPC work from the v1 critical path. The native russh + tonic reimplementation becomes a v1.x task.
**Stack:** `russh 0.60`, `portable-pty` (remote PTY channel as `PtyTransport`).
**Avoids:** Codespaces SSH non-trivial protocol assumption. SSH host-key TOFU bypass (pin to API-provided fingerprint via `ServerCheckMethod::PublicKey`). SSH `pty-req` resize missing (send initial cols/rows and `window-change` on resize).
**Research flag:** If/when replacing subprocess with native: gRPC tunneling to port 16634 is undocumented except via `cli/cli` Go source. Allocate a research spike before that sub-phase. Read `cli/cli/internal/codespaces/grpc/client.go` and `rpc/ssh/ssh_server_host_service.v1.proto` end-to-end.

### Phase 8: Dev Tunnels Integration

**Rationale:** Codespaces is the more validated flow. Dev Tunnels Rust SDK is less battle-tested. Building the second remote flow after the first validates the `Domain/PtyTransport` abstraction is general enough.
**Delivers:** `vector-tunnels`, `DevTunnelDomain`, Dev Tunnels picker alongside Codespaces picker, connect to `code tunnel` machines. Visual differentiation from Codespaces (different tab tint color).
**Strategy (day 1 of phase — mandatory spike):** Apply the decision tree: (a) subprocess `code tunnel client` if available on PATH; (b) vendor `microsoft/dev-tunnels/rs/` at a pinned SHA, resolving russh 0.37 vs. 0.60 version conflict (fork and bump, or accept binary duplication); (c) defer to v2 if both fail. Commit the decision document before writing integration code.
**Avoids:** Dev Tunnels SDK risk (go/no-go decision committed before code; nightly smoke test against live service; subprocess fallback behind feature flag).
**Research flag:** This phase has the highest known risk. The 1-2 day spike at phase start is mandatory. Do not estimate the rest of the phase until the spike resolves the branch.

### Phase 9: Persistence + tmux Pass-Through + Reconnect Overlay + Profiles

**Rationale:** Reconnect depends on both `Domain` implementations existing and `PtyTransport` hot-swap being proven. tmux pass-through correctness needs the full escape sequence pipeline. This is polish on the seams between all previous phases.
**Delivers:** `Domain::reconnect()`, hot-swap `PtyTransport` under live `Pane` without dropping bytes (reconnect state machine: Active -> Reconnecting -> Swapping -> Active), "Reconnecting..." overlay UI, auto-tmux-attach on connect (`tmux new -A -s vector-{profile-id}`), saved profiles with one-click reconnect, URL scheme handler (`vector://codespace/<name>`), DCS-wrapped OSC 52 verified through real tmux on remote, tmux 3.4+ smoke test suite.
**Avoids:** Custom state-sync / mosh-style protocol (tmux on remote is the answer). Double-multiplex visual glitches. PtyTransport lock held across `await` (lock, mutate, drop, then await — never await while holding the terminal lock).

### Phase 10: Hardening + Release

**Rationale:** The "looks done but isn't" checklist from PITFALLS.md is long. A dedicated hardening phase catches regressions before the tagged release.
**Delivers:** Snapshot test suite for renderer (wgpu headless, pinned font, perceptual tolerance), VT conformance corpus (bracketed paste, DECSET 1049 alt-screen, DECSTBM scroll regions, tab stops, ED/EL, mouse modes 1006, OSC 52 round-trip), perf pass (idle CPU <1%, input latency <16ms, `cat large.log` at vsync cap), tagged unsigned DMG release on GitHub Releases, README with `xattr -dr` install instructions front-and-center.

### Phase Ordering Rationale

- **CI/DMG before terminal code (Phase 1 before 2):** A green CI pipeline from day 1 means packaging never becomes a debt item. ghostty ships this way; it is a feature.
- **Headless terminal before GPU (Phase 2 before 3):** VT conformance bugs are caught cheaply with unit tests; they are expensive to debug inside a running GPU renderer.
- **Local terminal fully solid (Phases 1-5) before any remote work (Phases 6+):** The product is a daily-driver local terminal AND a remote client. If remote is prioritized, the local experience is neglected.
- **Auth + picker before SSH (Phase 6 before 7):** De-risks by separating the known-good (OAuth/REST) from the unknown (SSH+gRPC+tunnel). Picker UX ships and can be tested before the transport is done.
- **Codespaces before Dev Tunnels (Phase 7 before 8):** Codespaces has a public open-source reference (`cli/cli`). Dev Tunnels Rust SDK is undocumented and potentially unstable. Build on validated ground first.
- **Persistence last (Phase 9):** Hot-swapping `PtyTransport` requires `Domain` to be fully working for both remote flows. Threading it through earlier would add speculative complexity.

### Research Flags

Phases needing deeper research during planning:

- **Phase 7** (SSH + Codespaces connect): The subprocess approach (`gh codespace ssh --stdio`) is clear for v1. The native russh + gRPC replacement (v1.x) requires careful reading of `cli/cli/internal/codespaces/grpc/client.go`. The `?internal=true&refresh=true` query parameters and 409-swallow behavior are undocumented in the public REST API. Research the native path before committing to v1.x estimates.
- **Phase 8** (Dev Tunnels): The 1-2 day spike IS the research. No phase planning before the spike resolves the decision tree. The russh version conflict (0.37 internal vs. 0.60 in Vector) needs a concrete resolution before any work estimate is valid.

Phases with standard patterns (skip research-phase):

- **Phase 1** (Foundation + CI): Universal binary with `lipo`, `cargo-bundle`, `hdiutil` — standard macOS Rust packaging with published tutorials.
- **Phase 2** (Headless terminal core): `alacritty_terminal` + `portable-pty` combination is exactly how WezTerm and Cosmic Term are structured. Zero unknowns.
- **Phase 3** (GPU renderer): `wgpu 29` + `winit 0.30` + `glyphon` is documented; WezTerm's renderer architecture is a public reference.
- **Phase 4** (Mux): WezTerm's `Domain/Pane` pattern is open-source and thoroughly documented.
- **Phase 6** (GitHub auth + picker): OAuth device flow is RFC 8628; `octocrab` Codespaces endpoints are documented; no surprises expected.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All versions verified on crates.io (2026-05-10). Only gap: `microsoft/dev-tunnels/rs/` API surface is sparsely documented vs. C#/TS SDKs. |
| Features | HIGH | Terminal table-stakes are ECMA-48/xterm standards. Codespaces API surface verified in public docs and `cli/cli` source. Dev Tunnels protocol is documented in security docs and open-source SDKs. |
| Architecture | HIGH | `Domain/Pane/PtyTransport` pattern lifted directly from WezTerm (public source). Threading model is winit + tokio standard pattern confirmed by multiple published Rust desktop apps. |
| Pitfalls | HIGH (terminal), MEDIUM (Dev Tunnels) | Terminal/PTY/GPU pitfalls have abundant prior art and documented mitigations. Dev Tunnels SDK churn risk is real but quantified with a contingency plan. |

**Overall confidence:** HIGH for all terminal-core and local-app work. MEDIUM for the Dev Tunnels integration. The subprocess fallback path means Dev Tunnels risk does not block v1.

### Gaps to Address

- **Dev Tunnels decision tree outcome:** Unresolved until the phase-8 spike. If subprocess is not viable and the SDK is broken, Dev Tunnels slips to v2. This is the only genuine v1 scope risk.
- **Codespaces gRPC native path (v1.x):** The `?internal=true&refresh=true` parameters and the full port-16634 state machine need a careful read of `cli/cli` Go source before implementation. This is a v1.x task, not a v1 blocker.
- **russh 0.37 vs. 0.60 version conflict:** Needs a concrete resolution (fork `microsoft/dev-tunnels/rs/` and bump russh, or accept binary duplication) before Phase 8 begins. Low effort to resolve; just needs to happen.
- **IME (CJK input):** PROJECT.md lists it as a requirement; PITFALLS.md estimates multiple weeks and recommends deferring to v2. Recommended resolution: basic `NSTextInputClient` integration (composition display, no candidate window) for v1; full IME for v2.
- **Universal binary on CI:** macOS 14 GitHub Actions runners are arm64-only; macOS 13 runners are x86_64. Matrix build + `lipo` is the correct approach but needs to be validated in Phase 1 CI, not assumed.

## Sources

### Primary (HIGH confidence — versions verified on crates.io 2026-05-10)

- crates.io API — `alacritty_terminal 0.26.0`, `wgpu 29.0.3`, `winit 0.30.13`, `tokio 1.52.3`, `russh 0.60.2`, `octocrab 0.50.0`, `oauth2 5.0.0`, `keyring 4.0.0`, `reqwest 0.13.3`, `tonic 0.14.6`, `crossfont 0.9.0`, `portable-pty 0.9.0`, `objc2 0.6.4`, `cargo-bundle 0.10.0`
- [microsoft/dev-tunnels rs/Cargo.toml](https://raw.githubusercontent.com/microsoft/dev-tunnels/main/rs/Cargo.toml) — SDK existence, version 0.1.0, russh 0.37.1 dep, support matrix
- [cli/cli internal/codespaces/rpc/ssh/](https://github.com/cli/cli/tree/trunk/internal/codespaces/rpc/ssh) — `ssh_server_host_service.v1.proto` confirming gRPC SSHServerHostService and port 16634
- [GitHub Codespaces REST API docs](https://docs.github.com/en/rest/codespaces/codespaces) — endpoints, state transitions, scopes
- [WezTerm workspace Cargo.toml](https://raw.githubusercontent.com/wezterm/wezterm/main/Cargo.toml) — confirmed wgpu 25, tokio 1.43, `Domain/Pane` architecture
- [Alacritty alacritty/Cargo.toml](https://raw.githubusercontent.com/alacritty/alacritty/master/alacritty/Cargo.toml) — confirmed glutin + winit + crossfont (not wgpu)

### Secondary (MEDIUM confidence — community sources, multiple corroborating)

- [WezTerm DeepWiki multiplexer architecture](https://deepwiki.com/wezterm/wezterm/2.2-multiplexer-architecture) — Domain/Pane abstraction details
- [Ghostty release pipeline (DeepWiki)](https://deepwiki.com/ghostty-org/ghostty/8.2-release-pipeline) — universal binary + DMG pipeline reference
- [Dev tunnels security docs](https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/security) — X-Tunnel-Authorization header, auth model
- [tmux allow-passthrough](https://tmuxai.dev/tmux-allow-passthrough/) — DCS passthrough behavior, ~60-char truncation bug
- [winit macOS main-thread requirement (winit#1199)](https://github.com/rust-windowing/winit/issues/1199) — EventLoop threading constraint
- [Alacritty PR #2724](https://github.com/alacritty/alacritty/pull/2724) — damage tracking model
- [gh codespace ssh --stdio (cli/cli#8368)](https://github.com/cli/cli/issues/8368) — subprocess approach confirmed viable
- [Fine-grained PATs broken with Codespaces (cli/cli#7819)](https://github.com/cli/cli/issues/7819) — must use classic PAT scopes

### Tertiary (MEDIUM-LOW confidence — needs validation during implementation)

- microsoft/dev-tunnels support matrix in repo README — Rust listed as having Management API + Client + Host; reconnection and token refresh absent; may not reflect current state of `rs/` implementation
- [glyphon](https://github.com/grovesNL/glyphon) — cosmic-text + etagere + wgpu integration; used by Cosmic Term; needs profiling vs. custom atlas for terminal use case

---
*Research completed: 2026-05-10*
*Ready for roadmap: yes*
