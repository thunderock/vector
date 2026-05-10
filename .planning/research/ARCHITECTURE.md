# Architecture Research

**Domain:** Native macOS GPU-accelerated terminal emulator with built-in remote-tunnel client (GitHub Codespaces SSH + Microsoft Dev Tunnels).
**Researched:** 2026-05-10
**Confidence:** HIGH for terminal/render/PTY layers (WezTerm + Alacritty are public references). MEDIUM for Codespaces gRPC plumbing (gh CLI is open-source Go, has to be re-read carefully). MEDIUM for Dev Tunnels Rust crate (`microsoft/dev-tunnels` ships an `rs/` directory with Management + Client + Host support per the project README, but API surface is undocumented relative to the C# SDK). HIGH for the recommendation that the terminal core must not know whether its PTY is local or remote.

---

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│                         App Shell (vector-app)                           │
│   AppKit window/menu/services • URL handler • CLI entrypoint • DMG       │
├──────────────────────────────────────────────────────────────────────────┤
│                     UI / Compositor (vector-ui)                          │
│   Tab bar • Split tree • Codespace picker • Profile sheet • Settings     │
├──────────────────────────────────────────────────────────────────────────┤
│                       Renderer (vector-render)                           │
│   wgpu device • glyph atlas • damage-tracked draw • cursor/selection FX  │
├──────────────────────────────────────────────────────────────────────────┤
│                      Mux / Sessions (vector-mux)                         │
│   Window → Tab → Pane tree • Pane = (TerminalModel, Domain) • routing    │
├──────────────────────────────────────────────────────────────────────────┤
│  Terminal Core (vector-term)            │   Domain abstraction           │
│   VT parser (vte) • Grid + scrollback   │   trait Domain { spawn_pane }  │
│   Cell attrs • Selection • Hyperlinks   │   trait PtyTransport (R/W/sz)  │
├──────────────────────────────────────────────────────────────────────────┤
│  vector-pty       │ vector-ssh        │ vector-codespaces │ vector-tunnels│
│  portable-pty     │ russh client      │ GH OAuth/REST +   │ MS dev-tunnels│
│  spawn local sh   │ + agent-forward   │ gRPC port-fwd to  │ Rust crate +  │
│                   │ (russh-keys)      │ 16634 + russh     │ russh         │
├──────────────────────────────────────────────────────────────────────────┤
│  Cross-cutting:  vector-config (toml + hot reload) • vector-secrets      │
│                  (Keychain via keyring) • vector-fonts (cosmic-text)     │
│                  • vector-input (key encoder, IME) • vector-theme        │
└──────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Typical Implementation |
|-----------|----------------|------------------------|
| `vector-term` | VT/ANSI state machine, grid, scrollback, alt-screen, selection. Knows nothing about PTYs. | Wraps `alacritty_terminal` (preferred — fastest path) or builds on `vte` directly (more control, more risk). |
| `vector-pty` | Local PTY pair, child spawn, `SIGWINCH`. | `portable-pty` crate (cross-platform PTY API, used by WezTerm). |
| `vector-ssh` | Generic async SSH client with PTY channel + port-forward + agent-forward. | `russh` (low-level Tokio SSH2). |
| `vector-codespaces` | GitHub auth, codespace list/get/start, gRPC tunnel to internal port 16634, then SSH-over-tunnel. | `octocrab` for REST; raw `reqwest` for device-flow OAuth; `tonic` only if we need first-class gRPC; otherwise tunnel TCP and let `russh` ride it. |
| `vector-tunnels` | Dev Tunnels client: tunnel relay over WebSocket, auth via `X-Tunnel-Authorization`, then SSH-over-WebSocket. | `microsoft/dev-tunnels` Rust crates (`rs/` directory in the repo) — official, fork if needed. |
| `vector-secrets` | Token/keypair persistence. | `keyring` crate (`apple-native` feature) → Keychain. |
| `vector-mux` | The window/tab/pane tree; owns `Domain` instances; routes input/output between panes and transports. | Modeled on WezTerm's `mux` crate: `Mux::get()` singleton, `Domain` trait, `Pane` trait. |
| `vector-render` | GPU pipeline: glyph atlas, damage-tracked draw calls, cursor and selection overlays. | `wgpu` (Metal backend) + `cosmic-text` for shaping + `etagere` for atlas packing (or wrap `glyphon`). |
| `vector-ui` | Non-grid UI surfaces: tab bar, split chrome, command palette, codespace picker. | Same `wgpu` surface as renderer with a small immediate-mode UI layer (consider `egui` for the picker only, or build minimal in-house chrome). |
| `vector-app` | macOS app shell, menus, Services, URL scheme handler (`vector://`), DMG packaging. | `winit` for the main loop + `objc2` for AppKit-specific bits (menubar, dock menu, Services). |
| `vector-config` | TOML config, hot reload, profile inheritance. | `serde` + `notify` (fsevents on macOS). |
| `vector-input` | Keymap → bytes encoding, IME, mouse encoding. | Steal logic from WezTerm's `termwiz::input` or `alacritty/input.rs`. |

**Crucial boundary:** `vector-term` accepts a generic `PtyTransport` trait (read/write/resize). Local PTY, SSH PTY-over-Codespaces, SSH-over-Dev-Tunnels all implement it. The terminal model never branches on transport.

---

## Recommended Project Structure

```
vector/                         # cargo workspace root
├── Cargo.toml                  # [workspace] members = [...]
├── crates/
│   ├── vector-app/             # bin: macOS app entrypoint, AppKit, DMG
│   │   ├── src/main.rs
│   │   ├── src/menu.rs         # native menu bar
│   │   ├── src/url_scheme.rs   # vector://codespace/<id> handler
│   │   └── resources/          # Info.plist, .icns, entitlements (later)
│   ├── vector-ui/              # tab bar, splits, picker, command palette
│   ├── vector-render/          # wgpu pipeline, atlas, shaders
│   │   └── shaders/*.wgsl
│   ├── vector-mux/             # Window/Tab/Pane tree, Domain trait
│   ├── vector-term/            # VT parser, grid, scrollback (or re-export alacritty_terminal)
│   ├── vector-pty/             # local PTY via portable-pty
│   ├── vector-ssh/             # russh wrapper: pty channel, forwards, agent
│   ├── vector-codespaces/      # GH REST + OAuth + gRPC tunnel + SSH compose
│   ├── vector-tunnels/         # MS Dev Tunnels client (depends on dev-tunnels rs)
│   ├── vector-config/          # TOML, hot reload, profile inheritance
│   ├── vector-secrets/         # Keychain via keyring
│   ├── vector-fonts/           # font discovery + shaping (cosmic-text wrapper)
│   ├── vector-input/           # keymap, IME, mouse encoding
│   └── vector-theme/           # palette types, builtin themes
├── xtask/                      # build automation: dmg, universal binary, ci
│   └── src/main.rs             # `cargo xtask dmg`, `cargo xtask universal`
├── ci/
│   └── github-actions.yml      # tagged release → DMG artifact
├── tests/
│   ├── vt-conformance/         # VT escape conformance corpus
│   └── snapshot/               # render snapshot tests
└── docs/
```

### Structure Rationale

- **One bin (`vector-app`), many libs.** This mirrors WezTerm: a single `wezterm-gui` binary atop ~19 library crates. Library separation makes incremental compile bearable and lets us write headless tests against `vector-term` and `vector-mux` without dragging in `wgpu`.
- **`vector-mux` lives between term and UI**, exactly like WezTerm's `mux` crate sits between `term` and `wezterm-gui`. That single boundary is what lets persistence/reconnect, splits, and remote panes all share code.
- **`vector-codespaces` and `vector-tunnels` depend on `vector-ssh`, not on `vector-term`.** The terminal core must not know about transports. Both produce a `Box<dyn PtyTransport>` that `vector-mux` hands to a `Pane`.
- **`xtask` instead of shell scripts** for DMG/universal-binary work — keeps the Mac packaging logic in Rust, reproducible on CI and locally.

---

## Architectural Patterns

### Pattern 1: `PtyTransport` trait — uniform local/remote interface

**What:** All things that can act as a PTY (local pty, ssh channel, ssh-over-tunnel, ssh-over-websocket) implement one trait. The terminal model only sees the trait.

**When to use:** Any time a pane is opened, regardless of where the shell is running.

**Trade-offs:** Adds a layer of dyn-dispatch on the I/O hot path (negligible — bounded by syscalls / network anyway). Big win: every transport works with every pane feature (splits, scrollback, search, copy mode) for free.

**Example:**
```rust
pub trait PtyTransport: Send + 'static {
    fn reader(&mut self) -> Pin<Box<dyn AsyncRead + Send>>;
    fn writer(&mut self) -> Pin<Box<dyn AsyncWrite + Send>>;
    fn resize(&mut self, rows: u16, cols: u16, px_w: u16, px_h: u16) -> Result<()>;
    fn kind(&self) -> TransportKind; // Local | Ssh | Codespace | DevTunnel
}
```

### Pattern 2: `Domain` — connection context that spawns panes

**What:** Lifted directly from WezTerm. A `Domain` knows how to produce a new `PtyTransport` for a given command/cwd. `LocalDomain`, `CodespaceDomain { codespace_id }`, `DevTunnelDomain { tunnel_id }`.

**When to use:** Every pane carries a reference to the domain that spawned it, so "split this pane" can reuse the same connection.

**Trade-offs:** Domains hold connection state (auth tokens, ssh session). Splitting a Codespaces pane should reuse the existing SSH multiplexed session, not re-auth. This is exactly why WezTerm has the abstraction.

**Example:**
```rust
pub trait Domain: Send + Sync {
    async fn spawn_pane(&self, cmd: SpawnCommand) -> Result<Box<dyn PtyTransport>>;
    fn label(&self) -> String;     // shown in tab bar
    fn is_alive(&self) -> bool;    // for reconnect UX
    async fn reconnect(&self) -> Result<()>;
}
```

### Pattern 3: Triple-loop threading (UI / render / I/O)

**What:**
- **Main thread** runs `winit` event loop (mandatory on macOS — `EventLoop` must be on the main thread; `with_any_thread()` is unavailable on macOS). It owns the window, dispatches input, schedules redraws.
- **Render** is driven from the main thread on `RedrawRequested` (winit's recommendation) — `wgpu` calls themselves are not thread-pinned, but the swapchain + AppKit window are. We do NOT spawn a separate render thread on macOS; we redraw on demand, throttled to vsync via `Present::Mailbox`.
- **Tokio multi-thread runtime on a background thread** handles all I/O: PTY reads, SSH session, OAuth callbacks, REST calls. Output bytes from each PTY are funneled into the terminal grid via a per-pane `mpsc` channel; the main thread drains it before each redraw.

**When to use:** Always. This is the WezTerm/Alacritty pattern adapted for the macOS-mandatory-main-thread constraint.

**Trade-offs:** Crossing the thread boundary on every PTY chunk costs a channel send. Mitigation: batch — read up to N KB or until a quiescence timeout, then send one batch.

**Example:**
```rust
// Main thread (winit + wgpu)
fn main() -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let _guard = rt.handle().clone(); // hand to mux for spawning I/O tasks
    let event_loop = EventLoop::new()?;
    event_loop.run_app(&mut App::new(rt.handle().clone()))?;
    Ok(())
}
```

### Pattern 4: Damage-tracked redraw

**What:** Track per-row dirty flags in the terminal grid (Alacritty PR #2724 model). Renderer composites only changed rows + cursor + selection rect. Idle CPU drops from ~100% (full repaints) to ~20%.

**When to use:** Always. Without it, a blinking cursor on a 4K monitor melts a CPU core.

**Trade-offs:** Adds bookkeeping in `vector-term` cell writes. Worth it.

### Pattern 5: VT parser is a `Perform` trait sink (not a callback maze)

**What:** Standard `vte` API: parser feeds bytes into a `Perform` impl that mutates the grid. Same shape Alacritty uses.

**When to use:** Always.

**Example:**
```rust
impl vte::Perform for TerminalModel {
    fn print(&mut self, c: char) { self.grid.write(c); }
    fn execute(&mut self, byte: u8) { /* C0 */ }
    fn csi_dispatch(&mut self, params: &Params, ..., c: char) { /* CSI */ }
    fn osc_dispatch(&mut self, params: &[&[u8]], _: bool) { /* OSC, hyperlinks */ }
    // ...
}
```

### Pattern 6: Profile inheritance for config

**What:** Config has a `[default]` profile and named profiles that inherit and override. Hot reload watches the file via `notify`.

**When to use:** Once profiles exist (Phase 7+).

**Trade-offs:** Inheritance is slightly more work than flat profiles, but it lets the user define `font_size = 14` once and have every codespace inherit it.

---

## Data Flow

### Flow 1: Local PTY (output path)

```
zsh process
    └─ writes bytes to PTY slave
         │
         ▼
[vector-pty] PTY master read (tokio task)
    └─ AsyncRead → batched into Vec<u8>
         │  (mpsc::Sender<Vec<u8>> — per pane)
         ▼
[vector-mux] dispatcher (main thread, RedrawRequested or PtyData event)
    └─ drains channel, hands bytes to TerminalModel
         │
         ▼
[vector-term] VT parser → Perform impl → Grid mutation
    └─ marks dirty rows
         │
         ▼
[vector-render] (main thread)
    └─ shape new glyphs (cosmic-text, cached)
    └─ pack into atlas (etagere)
    └─ build instance buffer for dirty rows
    └─ wgpu render pass → Metal swapchain
```

### Flow 2: Local PTY (input path)

```
NSEvent (key)
    │
    ▼ winit translates
[vector-input] keymap lookup → encode (modifyOtherKeys / kitty / xterm)
    │
    ▼
[vector-mux] active pane lookup
    │
    ▼
[vector-pty] writer.write_all(bytes) (spawned tokio task)
    │
    ▼ PTY master
zsh
```

Resize is the same shape: `WindowEvent::Resized` → grid resize → `PtyTransport::resize` → kernel sends `SIGWINCH`.

### Flow 3: Codespaces SSH (the gh-codespace-ssh flow, in Rust)

```
User clicks "Connect to my-cs-frontend" in picker
    │
    ▼
[vector-codespaces] auth check → octocrab GET /user/codespaces/:name
    │  state == "Available"? if not, POST /user/codespaces/:name/start
    ▼
[vector-codespaces] open gRPC tunnel to internal port 16634
    │  (this is the magic: codespace exposes an internal grpc server;
    │   we have to forward it to a local TCP port, just like gh cli does)
    │  underlying transport: SSH session to the codespace VM
    ▼
[vector-ssh] establish russh session with codespace SSH credentials
    │  port-forward channel: local 127.0.0.1:RAND <-> remote 127.0.0.1:16634
    ▼
[vector-codespaces] gRPC client over the forwarded port
    │  call into the internal codespace API (start ssh server, get port, etc.)
    │  obtain an SSH endpoint to the dev container
    ▼
[vector-ssh] open a NEW SSH channel for the user shell (PTY request)
    │  this channel implements PtyTransport
    ▼
[vector-mux] new Pane(transport, CodespaceDomain)
    ▼
... data flows are now identical to Flow 1, with russh channel as the PTY.
```

**Reference impl to read closely:** `cli/cli` Go source at `internal/codespaces/grpc/client.go` (port 16634 hardcoded) and `pkg/cmd/codespace/ssh.go`. We are translating ~600 lines of Go.

### Flow 4: Dev Tunnels

```
User clicks "Connect via Dev Tunnel: corp-box"
    │
    ▼
[vector-secrets] load GitHub token (or MS Entra token, depending on tunnel auth)
    │
    ▼
[vector-tunnels] dev-tunnels Rust crate: GET tunnel by id from management API
    │  obtain relay URL + access token
    ▼
[vector-tunnels] open WebSocket to tunnel-relay
    │  X-Tunnel-Authorization: <token>   <-- not "Authorization", per docs
    ▼
[vector-ssh] russh on top of the WebSocket stream
    │  (russh accepts any AsyncRead+AsyncWrite, not just TCP)
    ▼
[vector-ssh] PTY channel → PtyTransport
    ▼
[vector-mux] new Pane(transport, DevTunnelDomain)
```

The win here vs. starting from scratch: `microsoft/dev-tunnels` already has a Rust crate covering Management API + Tunnel Client Connections + Tunnel Host Connections. We consume it; we don't reverse-engineer it. (Verified against the repo README's language matrix.)

### Flow 5: OAuth (GitHub device flow)

```
User clicks "Sign in with GitHub"
    │
    ▼
[vector-codespaces::auth] POST https://github.com/login/device/code
    │  { client_id, scope: "codespace,read:user" }
    ▼ response: { device_code, user_code, verification_uri, interval }
[vector-ui] show "Open https://github.com/login/device, enter ABCD-1234"
    │  also: NSWorkspace.openURL(verification_uri)
    ▼  (poll loop)
[vector-codespaces::auth] POST /login/oauth/access_token until token returned
    │
    ▼
[vector-secrets] keyring::Entry::new("vector", "github").set_password(token)
    ▼
done — vector-codespaces holds an octocrab client for this session.
```

### Flow 6: Persistence / reconnect (v1 realistic plan)

```
wifi drops mid-session
    │
    ▼
[vector-ssh] russh session reports IO error → Domain marks itself "disconnected"
    ▼
[vector-mux] pane status → "Reconnecting…" overlay (rendered on top of last-known grid)
    │  IMPORTANT: we keep the grid + scrollback in memory. We do NOT clear it.
    ▼
[vector-codespaces / vector-tunnels] re-run the connect flow (auth still cached)
    ▼
[vector-mux] swap the Pane's PtyTransport for a new one
    │  send `tput reset`? no — let user reattach to tmux (recommended UX)
    ▼
done. State on the remote is whatever tmux/screen preserved; locally we kept the buffer.
```

**v1 stance on persistence:** the realistic answer is **lean on remote tmux**, not Mosh. We:
- Keep the local grid/scrollback in memory across reconnects (cheap, big UX win — terminal doesn't go blank).
- Re-establish the SSH transport transparently.
- Document "use `tmux` on the remote for full state preservation" as the v1 answer.
- Mosh-style predictive echo + UDP SSP is **explicitly v2** — it's a real protocol we'd have to ship a remote agent for, which is out of scope.

---

## Threading Model — In Detail

```
┌──────────────────────── Main thread ────────────────────────────┐
│  winit::EventLoop                                                │
│    │                                                              │
│    ├─ WindowEvent::KeyboardInput → vector-input encode →          │
│    │     mux.active_pane().write(bytes)  (this hops to tokio)     │
│    │                                                              │
│    ├─ UserEvent(PtyOutput { pane_id, bytes }) →                   │
│    │     mux.pane(id).feed(bytes) → grid.dirty                    │
│    │                                                              │
│    └─ RedrawRequested →                                           │
│          vector-render: composite dirty rows → wgpu submit        │
│                                                                   │
│  AppKit menu callbacks dispatch into the event loop via           │
│  EventLoopProxy::send_event(...)                                  │
└───────────────────────────────────────────────────────────────────┘
              ▲                                  │
              │ EventLoopProxy<UserEvent>        │ Handle (tokio runtime)
              │                                  ▼
┌─────────────────── Tokio multi-thread runtime ───────────────────┐
│  Per-pane I/O task:                                                │
│    loop {                                                          │
│       let n = transport.read(&mut buf).await?;                     │
│       proxy.send_event(UserEvent::PtyOutput {                      │
│           pane_id, bytes: buf[..n].to_vec()                        │
│       })?;                                                         │
│    }                                                               │
│                                                                    │
│  Per-pane writer task: drains an mpsc<Vec<u8>>, writes to transport│
│                                                                    │
│  SSH session task (russh): owns the underlying TCP/WebSocket       │
│                                                                    │
│  REST task pool: octocrab calls, OAuth polling                     │
│                                                                    │
│  fsevents / config-reload watcher: notify crate                    │
└────────────────────────────────────────────────────────────────────┘
```

### Key threading rules

1. **`winit::EventLoop` lives on the macOS main thread.** Non-negotiable — confirmed both by winit docs and macOS itself; `with_any_thread()` is unavailable on macOS.
2. **`tokio::runtime` lives on background threads.** Build `Builder::new_multi_thread().enable_all().build()` once at startup; pass `Handle` everywhere.
3. **Cross-thread signal = `EventLoopProxy::send_event(UserEvent)`.** This is winit's blessed mechanism for waking the main loop from an async task.
4. **Backpressure on PTY reads:** if the renderer is slow, the channel fills. Use `mpsc::channel(64)` with bounded depth and let the I/O task `await` on send — this naturally throttles a runaway `cat /dev/urandom`.
5. **Render is on the main thread, not a render thread.** `wgpu` is not thread-pinned in principle, but the AppKit `CAMetalLayer` is, and there's no win in moving it. Throttle to vsync via `Present::Mailbox` or `Fifo`.
6. **Don't hold the terminal lock across an `await`.** Alacritty uses `FairMutex<Term>` and explicitly drops it before any I/O. We do the same.

### Threading risk callouts

- **Risk:** Nested `block_on` from main thread → deadlock if I/O task tries to wake main. **Mitigation:** main thread never blocks on async; always uses `EventLoopProxy` round-trips.
- **Risk:** `russh` session ownership. A session is `!Send` if used naively; we wrap it behind a per-session task that owns the session and exposes channels via `mpsc`. This is the russh-recommended pattern.
- **Risk:** AppKit menu/services callbacks fire on the main thread but outside the winit dispatch. **Mitigation:** every objc2 callback ends in `EventLoopProxy::send_event(...)` — never mutates state directly.
- **Risk:** `wgpu` surface recreation on resize is not free — debounce. WezTerm reconfigures the swap chain on `Resized`; copy that.

---

## Rendering Pipeline — CPU vs GPU split

```
                     CPU (per frame, only on dirty rows)
                     ──────────────────────────────────
text in grid cells ─► shape segments via cosmic-text ─► glyph IDs + offsets
                                                      │
                                                      ▼
                                       lookup in glyph atlas;
                                       miss? rasterize via swash → upload to atlas tex
                                                      │
                                                      ▼
                                       build per-cell instance buffer:
                                         [pos, atlas_uv, fg, bg, attrs]
                                                      │
                                                      ▼
                     ════════ submit to wgpu ════════════
                                                      │
                            GPU (every frame)         ▼
                            ─────────────────  vertex shader: place quad
                                               fragment shader: sample atlas, blend bg/fg
                                               cursor pass: filled quad with blend
                                               selection pass: rect with alpha
```

### What's CPU-side (per Alacritty/WezTerm)
- Shaping (cosmic-text/HarfRust). Done once per unique `(font, run)` and cached.
- Glyph rasterization (swash). Done once per `(font, glyph_id, size, weight)` and cached in atlas.
- Damage tracking — diffing dirty rows.
- Building the per-frame instance buffer (one quad per visible cell, but only re-built for dirty cells).

### What's GPU-side
- Quad emission via instancing (one draw call for the whole grid).
- Atlas sampling, color blending, cursor and selection compositing.
- Optional: ligature substitution is CPU (it's a shaping concern), but the resulting glyphs render through the same instanced quad pipeline.

### Practical recommendation
Use **glyphon** as a starting point. It's `cosmic-text` + `etagere` atlas + `wgpu` rendering, glued together. Customize where we need terminal-specific quirks (grid alignment, selection FX). If glyphon proves too web-y or too heavy, fall back to building the pipeline directly: `cosmic-text` → `etagere` → custom `wgpu` shader. WezTerm builds it themselves; cosmic-term uses `glyphon`. Either path is fine; glyphon is faster to v1.

---

## Configuration / State Architecture

```
~/Library/Application Support/Vector/
├── config.toml              # main config, hot-reloaded
├── profiles/
│   ├── my-cs-frontend.toml  # profile inheriting [default]
│   └── corp-box.toml
├── themes/
│   └── tokyonight.toml
├── sessions/                # crash-recovery scratch (open tabs/panes layout)
└── logs/

~/Library/Caches/Vector/
├── glyph-cache/             # rasterized glyphs across runs
└── codespaces.json          # cached list (stale-while-revalidate)
```

- **Tokens / SSH keys / refresh tokens** → macOS Keychain via `keyring` crate. Never on disk.
- **TOML over YAML/JSON** — matches Alacritty, ghostty, Helix; user-friendly for hand-editing.
- **Hot reload** via `notify` (FSEvents on macOS). Profile inheritance: each profile is a `[default] + overrides` merge. Reload = re-merge + push deltas to mux.
- **State persistence** for window/tab/pane layout: serialize on quit, restore on launch. Don't try to preserve PTY contents — too fragile, just relaunch the command.

---

## Persistence / Reconnect Architecture — Honest Assessment

| Approach | What it gets us | v1 verdict |
|----------|-----------------|------------|
| **Keep local grid+scrollback in memory across reconnects** | "Looks alive" UX during wifi drop | ✅ **YES — must-have, cheap.** This is what makes "wifi drops, session resumes" feel right even without any remote magic. |
| **Auto-reconnect SSH** | Re-establish transport without user intervention | ✅ **YES — must-have.** Backoff, retry, swap `PtyTransport` under the pane. |
| **Remote tmux pass-through (recommended workflow)** | True remote state preservation across long disconnects | ✅ **YES — document, don't ship as feature.** v1 just needs tmux to render correctly when present. |
| **Mosh-style SSP with predictive echo + UDP** | True liveness over flaky links | ❌ **NO — v2.** Requires a remote agent on every codespace. Codespaces don't have mosh installed. Massive scope add. |
| **Custom replay-buffer protocol (proprietary mosh)** | Bespoke version of the above | ❌ **NO — v2 or never.** Complexity not justified for the user base. |

**v1 architecture:** `Domain::reconnect()` is a method. The `Pane` keeps its grid alive while `Domain::reconnect()` runs. When it returns OK, the new `PtyTransport` is hot-swapped. The user sees the old buffer with a brief "Reconnecting…" overlay, then their prompt comes back. That's the whole feature. It's not Mosh, but it covers the user's stated requirement ("wifi drop should not lose Codespace state") for the realistic case where state lives in tmux/screen on the remote.

---

## macOS Native Integration

| Surface | Approach | Phase |
|---------|----------|-------|
| Window/menu/dock | `winit` window + `objc2`/`cocoa` for menubar (winit's menu support is thin on macOS) | Phase 1 |
| Services menu | `NSApplication.servicesProvider` via objc2; expose "Open in Vector" | Phase 8+ (polish) |
| URL scheme `vector://` | Register via `Info.plist` → handle `application:openURL:` via objc2 → `EventLoopProxy::send_event(OpenUrl)` | Phase 6 (after Codespaces lands; deeplink format `vector://codespace/<name>`) |
| Notifications | `mac-notification-sys` or raw `UNUserNotificationCenter` | Phase 8+ |
| Touch Bar | **Skip.** Apple deprecated; user explicitly asked. | — |
| Universal binary | `lipo` aarch64 + x86_64 builds in `cargo xtask universal` | Phase 1 |
| DMG | `cargo xtask dmg` using `create-dmg` shell tool or pure-Rust DMG writer | Phase 1 (CI from day one — ghostty model) |
| Right-click-Open / Gatekeeper | Documented in README; no entitlements until v2 | Phase 1 |
| Native input methods (IME) | Standard `winit` IME events → `vector-input` composing buffer | Phase 2 |

---

## Testing Architecture

This is the part that's genuinely hard. Three test categories, each with a different pattern.

### 1. VT conformance tests (in `vector-term`, no GPU)

```rust
#[test]
fn csi_sgr_38_5_renders_indexed_color() {
    let mut term = TerminalModel::new(80, 24);
    term.feed(b"\x1b[38;5;42mhello\x1b[0m");
    let cell = term.grid().cell(0, 0);
    assert_eq!(cell.fg, Color::Indexed(42));
}
```

- Build a corpus of input bytes → expected grid state.
- Reuse Alacritty's test corpus where licensing permits; vt100/xterm test suites are public.
- Run on every commit. Fast (< 1 sec).

### 2. Snapshot tests for the renderer

- Use `wgpu` headless (no surface, render to texture).
- Render a known input → readback texture → compare to a checked-in PNG (or a perceptual hash).
- WezTerm has `wezterm-gui/tests/`; Alacritty does ad-hoc; we use `insta`-style snapshots.
- Gotcha: font rendering is non-deterministic across CoreText versions. Pin to a bundled font (e.g., JetBrainsMono) and compare with a tolerance.

### 3. Integration tests for transports

- **Local PTY:** spawn `echo hello`, assert grid contains "hello". Easy.
- **SSH:** spin up `russh` server in-test, point `vector-ssh` at it, assert PTY round-trip.
- **Codespaces:** mock the GitHub REST surface with `wiremock`; mock the gRPC server with `tonic-mock`; assert `vector-codespaces` produces a `PtyTransport` that round-trips.
- **Dev Tunnels:** likewise — mock at the WebSocket layer.

### 4. Manual / smoke test matrix

Document a reproducible smoke test for each release: Codespaces connect, Dev Tunnels connect, wifi-drop reconnect, tmux pass-through, ligature rendering, IME, multi-monitor DPI change. CI-blockable items go in #1 and #2; #4 is the last gate before tagging.

---

## Build Order — 10-Phase Roadmap Map

The user wants fine granularity (8–12 phases). Here's a 10-phase decomposition that maps cleanly onto the architecture.

| # | Phase | Architectural layer added | Done = ship-able |
|---|-------|---------------------------|------------------|
| 1 | **Foundation & app shell** | `vector-app` skeleton, winit window, AppKit menubar, CI → DMG, universal binary, right-click-Open docs | A black window opens from a DMG. |
| 2 | **Local terminal core (headless)** | `vector-term` (alacritty_terminal under the hood or vte-based), `vector-pty` (portable-pty), spawn local zsh, byte plumbing — no GPU yet | Headless `cargo run --bin vt-repl` echoes a real shell. VT conformance tests pass. |
| 3 | **GPU renderer + first paint** | `vector-render` (wgpu Metal), glyph atlas, damage tracking, cursor; `vector-fonts` with cosmic-text + glyphon; `vector-input` keymap | A real terminal you can run `vim` in. Single tab, single pane. |
| 4 | **Mux: tabs and splits** | `vector-mux` (Window/Tab/Pane tree, LocalDomain), tab bar UI, split layout (binary tree à la WezTerm), focus routing | iTerm-class local terminal: tabs, splits, navigate, resize. |
| 5 | **Config, profiles, themes, scrollback polish** | `vector-config` (TOML, hot reload, profile inheritance), `vector-theme`, scrollback search, copy/paste, ligatures verified | Daily-driver-grade for local use. |
| 6 | **GitHub auth + Codespaces picker** | `vector-secrets` (Keychain), `vector-codespaces::auth` (device flow), REST listing, picker UI; **no SSH yet** | Sign in, see codespaces, click → "not implemented" toast. Auth + UI done. |
| 7 | **SSH transport + Codespaces connect** | `vector-ssh` (russh), gRPC tunnel to port 16634, SSH-over-tunnel, `CodespaceDomain`, `Pane` accepts remote `PtyTransport` | Open a working remote shell into a Codespace from the picker. v0.5 of the headline feature. |
| 8 | **Dev Tunnels** | `vector-tunnels` (microsoft/dev-tunnels Rust crate), `DevTunnelDomain`, picker entry for tunnels | Connect to `code tunnel` machines. Both remote flows work. |
| 9 | **Persistence, reconnect, tmux pass-through, profiles** | `Domain::reconnect`, hot-swap `PtyTransport` under live `Pane`, "Reconnecting…" overlay, saved profiles, tmux DCS pass-through verified, URL scheme handler | Wifi drops and you don't notice. One-click reconnect to saved profiles. |
| 10 | **Polish + release** | Snapshot test suite, perf pass (idle CPU, input latency), README + DMG distribution, right-click-Open docs, signed `vector://` deeplinks tested, tagged release pipeline green | v1.0 DMG on GitHub Releases. |

### Why this order

- **Foundation first (1) is intentional.** Ghostty's release pipeline being good from day 1 is a feature, not an afterthought; the user called it out explicitly. CI → DMG before any terminal code means we never slip into "we'll set up CI later."
- **Headless terminal before GPU (2).** This catches VT conformance bugs cheaply. Alacritty's `alacritty_terminal` is its own crate for exactly this reason.
- **Local terminal fully done (3–5) before any remote work (6+).** The user wants a daily-driver replacement for iTerm/ghostty. If we sequence remote first, we'd have a half-baked local terminal that nobody wants to use. Local must be solid; remote is built on top.
- **Auth UI before SSH (6 before 7).** This is a deliberate de-risking split: GitHub OAuth + REST + picker UI is straightforward; SSH+gRPC+port-forwarding is the gnarly bit. By landing auth first, we have a usable picker and only the transport is the WIP.
- **Codespaces before Dev Tunnels (7 before 8).** Codespaces is the more validated flow (gh CLI is open-source Go; clear reference impl). Dev Tunnels Rust crate is less battle-tested in the wild — better to have one working remote flow before tackling the second.
- **Persistence/reconnect last (9), not threaded through earlier phases.** It needs `Domain` to exist and both remotes to work; it's a polish on top of the seams between them.

### Phase-level risk callouts for the roadmap

| Phase | Risk | Mitigation flag |
|-------|------|-----------------|
| 3 | wgpu + winit glue on macOS has version-skew traps; glyph rendering perceptual diffs across CoreText versions | Pin wgpu version; bundle one default font for snapshot tests |
| 7 | gRPC tunneling to port 16634 is the trickiest part of the entire project — undocumented protocol, only ref is gh CLI Go source | **Allocate a research spike at start of Phase 7.** Read `cli/cli` `internal/codespaces/grpc/` carefully. |
| 8 | Dev Tunnels Rust SDK exists per repo README but API surface is sparsely documented | **Allocate research spike.** Build a one-file proof-of-concept that opens a tunnel + gets a stream BEFORE wiring it into mux. |
| 9 | Hot-swapping `PtyTransport` under a live `Pane` without dropping bytes is subtle | Define the swap as a state-machine: `Pane` enters `Reconnecting` state, drops/buffers writer mpsc, swaps transport, replays queued writes |
| 9 | tmux pass-through can break if our DCS handling diverges from xterm | Test against real tmux 3.4+ early; copy WezTerm's known-good handling |

---

## Anti-Patterns

### Anti-Pattern 1: Embedding transport logic into the terminal model

**What people do:** Add `enum PaneSource { Local(Pty), Ssh(Channel), ... }` to `vector-term` and branch on it.
**Why it's wrong:** Every new feature (selection, search, splits) now has to handle each transport variant. Adding Dev Tunnels means touching the terminal core.
**Do this instead:** `PtyTransport` trait. Terminal has zero awareness of where its bytes come from.

### Anti-Pattern 2: One mega-process with everything on the main thread

**What people do:** Tokio current-thread runtime spliced into `winit`, "to avoid threading complexity."
**Why it's wrong:** Long PTY reads, network calls, OAuth polls all stall the UI. Frame drops and jank.
**Do this instead:** Multi-thread Tokio runtime on background threads. Cross to main via `EventLoopProxy`. Standard pattern.

### Anti-Pattern 3: Re-implementing OAuth + SSH from scratch

**What people do:** Hand-roll OAuth flows; manually parse SSH packets.
**Why it's wrong:** Cryptography landmines. `octocrab` for GH, `russh` for SSH, `microsoft/dev-tunnels` for tunnels. All maintained, all do the right thing.
**Do this instead:** Lean on the crate ecosystem hard. Vector's value is the integration, not the protocol re-implementation.

### Anti-Pattern 4: Persistence via custom protocol

**What people do:** Build a Mosh-clone with a remote agent for v1.
**Why it's wrong:** Requires installing software on every Codespace and Dev Tunnel target. Massive scope creep.
**Do this instead:** Keep grid in memory across reconnect; document tmux on the remote. v1 done.

### Anti-Pattern 5: Holding the terminal lock across `await`

**What people do:** `let mut term = self.term.lock(); term.feed(bytes); some_io.await; ...`
**Why it's wrong:** Deadlocks waiting for the renderer or input to acquire the lock.
**Do this instead:** Lock, mutate, drop, await. Never await while holding it. (Alacritty enforces this; we should too.)

### Anti-Pattern 6: Repainting the whole grid every frame

**What people do:** Build instance buffer for all visible cells every frame.
**Why it's wrong:** Idle CPU jumps to ~100%; on a 4K window with a blinking cursor it's noticeable.
**Do this instead:** Damage tracking with per-row dirty flags (Alacritty PR #2724 model). Only rebuild dirty rows.

---

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| GitHub REST | `octocrab` crate; codespace scope; bearer token | Cache codespace list with stale-while-revalidate; respect X-RateLimit |
| GitHub OAuth (device flow) | `reqwest` direct (octocrab device-flow support is thin) | Need `client_id` registered as a public OAuth app — user must register vector at https://github.com/settings/applications/new |
| Codespaces internal gRPC (port 16634) | russh port-forward + `tonic` (only if we end up needing typed gRPC; otherwise raw forward) | **Closed protocol.** Reference impl: `cli/cli` Go source. Spike before committing to phase 7 estimate. |
| Microsoft Dev Tunnels | `microsoft/dev-tunnels` Rust crate (`rs/` in repo) | API surface less documented than C#/TS; expect to read source. WebSocket transport for the tunnel relay. |
| macOS Keychain | `keyring` crate, `apple-native` feature | Unsigned app limitation: some Keychain APIs require entitlements; basic generic-password access works unsigned. |
| Filesystem / fsevents | `notify` crate | Used for config hot-reload only |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `vector-app` ↔ `vector-mux` | direct method calls; UserEvent dispatch via EventLoopProxy | mux is a singleton, accessed via `Mux::get()` à la WezTerm |
| `vector-mux` ↔ `vector-term` | direct (mux owns Pane → owns Term) | feed bytes; query grid; resize |
| `vector-mux` ↔ transports | `Box<dyn PtyTransport>` | the key abstraction; enables persistence/reconnect, splits, every pane feature |
| `vector-mux` ↔ `vector-render` | mux exposes "what to render" (grid + selection + cursor); render reads, doesn't mutate | Read-only access to grid; renderer never mutates terminal state |
| `vector-render` ↔ `vector-fonts` | render asks for glyph; fonts shapes/rasterizes/caches | glyph cache survives runs (on-disk in `~/Library/Caches/Vector/`) |
| `vector-codespaces` ↔ `vector-ssh` | codespaces builds the gRPC tunnel, hands `vector-ssh` an `AsyncRead+AsyncWrite` stream | russh accepts arbitrary stream — that's the design |
| `vector-tunnels` ↔ `vector-ssh` | same: tunnels gives ssh a WebSocket-as-stream | identical pattern to codespaces |
| `vector-app` ↔ `vector-secrets` | direct calls; never log secrets | tokens in Keychain, never on disk |

---

## Sources

- [WezTerm repository (workspace structure reference)](https://github.com/wezterm/wezterm)
- [WezTerm DeepWiki — multiplexer architecture](https://deepwiki.com/wezterm/wezterm/2.2-multiplexer-architecture)
- [WezTerm DeepWiki — domains and panes](https://deepwiki.com/wezterm/wezterm/2.2.1-domains-and-panes)
- [WezTerm DeepWiki — GUI application structure](https://deepwiki.com/wezterm/wezterm/3.1-gui-frontend)
- [Alacritty `event_loop.rs` (PTY I/O + FairMutex pattern)](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/event_loop.rs)
- [Alacritty DeepWiki](https://deepwiki.com/alacritty/alacritty)
- [Alacritty PR #2724 — damage tracking](https://github.com/alacritty/alacritty/pull/2724)
- [`vte` crate — VT escape parser](https://github.com/alacritty/vte)
- [`portable-pty` crate (used by WezTerm)](https://lib.rs/crates/portable-pty)
- [`russh` — Rust SSH client/server (Tokio)](https://github.com/Eugeny/russh)
- [`microsoft/dev-tunnels` — official SDK with Rust support](https://github.com/microsoft/dev-tunnels)
- [`microsoft/dev-tunnels-ssh` — SSH library used by dev tunnels](https://github.com/microsoft/dev-tunnels-ssh)
- [Microsoft Dev Tunnels security (X-Tunnel-Authorization header)](https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/security)
- [`gh codespace ssh` source (port 16634, gRPC tunnel)](https://github.com/cli/cli/issues/11206)
- [GitHub Codespaces REST API](https://docs.github.com/en/rest/codespaces/codespaces)
- [winit + tokio integration discussion](https://github.com/tokio-rs/tokio/discussions/2953)
- [winit issue #1199 — async ecosystem integration](https://github.com/rust-windowing/winit/issues/1199)
- [`keyring` crate — macOS Keychain via `apple-native`](https://docs.rs/keyring)
- [`cosmic-text` — text shaping/layout](https://github.com/pop-os/cosmic-text)
- [`glyphon` — wgpu text renderer = cosmic-text + etagere + wgpu](https://github.com/grovesNL/glyphon)
- [Mosh: Interactive Remote Shell for Mobile Clients (paper)](https://mosh.org/mosh-paper.pdf) — for the persistence/reconnect anti-pattern callout
- [tmux DCS passthrough (`allow-passthrough` requirement)](https://github.com/tmux/tmux/issues/846)
- [VS Code Remote Tunnels (the UX we're modeling)](https://code.visualstudio.com/docs/remote/tunnels)
- [Contour terminal text stack](https://contour-terminal.org/internals/text-stack/) — terminal-emulator-specific text rendering considerations

---
*Architecture research for: Vector — Rust, GPU-accelerated, native macOS terminal with built-in Codespaces + Dev Tunnels client.*
*Researched: 2026-05-10*
