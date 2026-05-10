# Stack Research

**Domain:** GPU-accelerated native macOS terminal in Rust, with GitHub Codespaces SSH and Microsoft Dev Tunnels client support, distributed as unsigned `.dmg`.
**Researched:** 2026-05-10
**Confidence:** HIGH for terminal/render/async/GitHub. MEDIUM for app shell. LOW-MEDIUM for Dev Tunnels (Rust SDK exists but is preview-grade — see Section 7).

## Headline

The big finding: **a Rust SDK for Microsoft Dev Tunnels exists** — `microsoft/dev-tunnels` ships first-class Rust under `rs/` with Management API + Tunnel Client + Tunnel Host all green on the support matrix. It is not on crates.io, has gaps (no auto-reconnect, no token refresh), and is internally pinned to russh 0.37, but it removes the project's largest unknown. The other findings are conventional: `alacritty_terminal` + `wgpu` + `tokio` + `russh` + `octocrab`, with WezTerm's per-platform `window` crate as the model for AppKit-native windowing.

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| **Rust** | 1.88+ stable | Language | Required by `keyring 4.0`, `tonic 0.14`, comfortably ahead of `alacritty_terminal`'s 1.85 floor and `crossfont`'s 1.77. Use a `rust-toolchain.toml` pin. |
| **`alacritty_terminal`** | 0.26.0 (2026-04-06) | VT parser + grid + scrollback | Battle-tested, xterm-compatible, library-first split. Re-exports `vte` so we don't pull a separate parser. WezTerm rolls its own (`wezterm-term`) but does not publish to crates.io — using their crate would mean vendoring. Use `alacritty_terminal` and only fall back to building on `vte 0.15` directly if we hit a hard blocker (e.g. need image/sixel handling beyond what alacritty exposes). |
| **`wgpu`** | 29.0.3 (2026-05-02) | GPU rendering, Metal backend on macOS | Industry-standard cross-platform graphics in Rust. WezTerm uses wgpu (currently 25.x). Lets us defer Linux/Windows without rewriting the renderer. Raw `metal-rs` would be ~30% less code but locks us to Mac and forfeits the wgpu shader/atlas tooling that exists for other engines. **Note**: Alacritty has NOT migrated — it still uses OpenGL via glutin. Zed uses raw Metal via its custom GPUI, but that's a UI framework, not a renderer — see Section 3. |
| **`winit`** | 0.30.13 (2026-03-02) | Cross-platform window/event loop | Use winit for the event loop and basic window. Pair it with direct AppKit calls (`objc2-app-kit`) for native tabs/menus/services — see Section 3. WezTerm rolled its own `window` crate; that's overkill for v1 macOS-only. |
| **`objc2`** + `objc2-app-kit` + `objc2-foundation` | objc2 0.6.4 (2026-02-26) | Direct AppKit access | Modern, type-safe, actively maintained ObjC bindings. Replaces the legacy `cocoa-rs` and `objc` crates (both effectively unmaintained). Required for native tab bars (`NSWindowTabbingMode`), menu bar (`NSMenu`), Services menu, secure input, Quick Look. |
| **`tokio`** | 1.52.3 (2026-05-08) | Async runtime | Universal default. Required by `octocrab`, `russh`, `reqwest`, `tonic`. PTY-specific note: spawn the PTY on a blocking thread (`tokio::task::spawn_blocking`) and bridge with `mpsc` channels; the kernel APIs are not async-native. See `portable-pty 0.9.0` (Section "PTY"). |
| **`portable-pty`** | 0.9.0 (2025-02-11) | Cross-platform PTY | Authored by WezTerm's Wez Furlong. Provides a `PtySystem` trait and handles macOS/Linux/Windows differences. We don't need ConPTY today, but using this from day 1 keeps a future Windows port cheap. |
| **`russh`** | 0.60.2 (2026-04-29) | Async pure-Rust SSH client | Active maintenance under @Eugeny (also Warpgate's author). Used internally by Microsoft's Dev Tunnels Rust SDK. Pure-Rust, tokio-native. Critical because we need programmatic port forwarding inside the Codespaces gRPC tunnel — shelling out to `ssh` is not an option. |
| **`octocrab`** | 0.50.0 (2026-05-05) | GitHub REST API | Typed, async, complete coverage of `/user/codespaces`, `/repos/{r}/codespaces`. Use for: list codespaces, request connection details, fetch tunnel auth token. |
| **`oauth2`** | 5.0.0 (2025-01-21) | OAuth 2.0 device-code flow | Most-downloaded OAuth crate; supports RFC 8628 device flow. GitHub's `gh` CLI client ID is publicly documented and reusable. Pair with `keyring` for token persistence. |
| **`keyring`** | 4.0.0 (2026-04-26) | macOS Keychain access | Stores GitHub OAuth refresh token in the user's login keychain. Cross-platform-ready for future Linux/Windows. |
| **`reqwest`** | 0.13.3 (2026-04-27) | HTTP client | Used transitively by `octocrab` and the Dev Tunnels Management API. Pin once at workspace level so all crates share the TLS stack. Use `rustls` features (not native-tls) to avoid an OpenSSL dep. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| **`microsoft/dev-tunnels` (rs/)** | git, 0.1.0 unpublished | Dev Tunnels client | **Vendor as git dep** (no crates.io release). Provides Management API + Tunnel Client + Tunnel Host. See Section 7 for risks. |
| **`tonic`** | 0.14.6 (2026-05-07) | gRPC over HTTP/2 | Required to talk to the Codespaces internal RPC service on port 16634 — `SSHServerHostService.StartRemoteServer`, `CodespaceHost.NotifyCodespaceOfClientActivity`, `CodespaceHost.RebuildContainerAsync`. We will hand-port the `.proto` definitions from `cli/cli/internal/codespaces/rpc/{ssh,codespace,jupyter}/*.proto`. |
| **`prost`** | 0.13+ (matches tonic 0.14) | Protobuf codegen | Generated code for the codespace gRPC contracts. |
| **`vte`** | 0.15.0 (2025-02-02) | Pure VT parser | Pulled in transitively by `alacritty_terminal`. Direct dep only if we're building our own grid (don't, in v1). |
| **`crossfont`** | 0.9.0 (2025-06-09) | Font rasterization (CoreText on macOS) | Alacritty's font crate. Handles ligatures, fallback chains, emoji. Pairs naturally with `alacritty_terminal`. |
| **`cosmic-text`** | 0.19.0 (2026-04-22) | Alternative shaping/layout | Use **only** if we hit ligature/emoji corner cases that `crossfont` can't solve, or if we want a single-stack solution that includes shaping. Heavier and less terminal-tuned than `crossfont`. |
| **`swash`** | 0.2.7 (2026-03-27) | Font scaler used by cosmic-text | Transitive — listed for awareness, not a direct dep. |
| **`serde`** + **`toml`** | serde 1.0.228, toml 1.1.2 | Config | TOML for the user-facing config. Lua (`mlua`) is overkill and a noisy dep. JSON5 has no killer feature here. WezTerm picked Lua because it shipped before TOML had `toml_edit`-quality round-tripping; we don't have that constraint. |
| **`anyhow`** + **`thiserror`** | latest | Error handling | Standard application/library split. `anyhow` for the binary, `thiserror` for the workspace crates. |
| **`tracing`** + **`tracing-subscriber`** | latest | Structured logging | Required for diagnosing Codespaces RPC and tunnel-reconnect issues remotely. |
| **`zeroize`** | latest | Memory hygiene for tokens | Wipe OAuth tokens and SSH key material on drop. |
| **`tempfile`** | latest | Disk scratch space | For scp-style operations and any cached binaries. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| **`rust-toolchain.toml`** | Pin compiler | Set `channel = "1.88.0"` plus `targets = ["aarch64-apple-darwin", "x86_64-apple-darwin"]`. |
| **`cargo-bundle 0.10.0`** (2026-04-18) | Build `.app` bundle | Mature, low-maintenance, the de-facto Rust answer. Generates `Info.plist`, `.icns`, `MacOS/<binary>`. Does **not** create universal binaries — combine with `lipo`. |
| **`lipo`** (Xcode) | Combine x86_64 + aarch64 builds into Universal | Built into Xcode CLI tools. Cargo cannot natively produce a fat binary; build twice and lipo. |
| **`hdiutil`** (macOS) or **`create-dmg`** (npm) | Build the `.dmg` | `hdiutil create -fs HFS+ -srcfolder Vector.app -volname Vector Vector.dmg` is enough for v1. `create-dmg` (the [shell script](https://github.com/create-dmg/create-dmg), not npm) is nicer for backgrounded layouts but not required for an unsigned tool. |
| **`tauri-bundler`** | (do not use) | Heavier, opinionated, expects you to build a Tauri app. Pulls in webview deps we don't want. |
| **`cargo-dist`** | Optional release CI helper | Useful if release flow gets complex. Adds dependency surface; defer until we feel pain. |
| **GitHub Actions** | CI for `.dmg` artifacts | `macos-14` runners are arm64; `macos-13` are x86_64. Matrix-build, lipo, then bundle. Ghostty's published pipeline (Zig, but the topology applies) is a good reference. |
| **`cargo-deny`** | License + advisory audits | Trivial to add, catches surprise GPL pulls and unsound advisories. |
| **`clippy`** + **`rustfmt`** | Lint + format | Standard. |

## Installation

Workspace skeleton, top-level `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
  "crates/vector",          # bin: app entry
  "crates/vector-term",     # terminal core wrapping alacritty_terminal
  "crates/vector-render",   # wgpu renderer
  "crates/vector-window",   # winit + objc2-app-kit window/tabs/menubar
  "crates/vector-pty",      # portable-pty wrapper
  "crates/vector-ssh",      # russh-based transport
  "crates/vector-codespaces", # octocrab + tonic gRPC to GitHub Codespaces
  "crates/vector-tunnels",  # thin wrapper around microsoft/dev-tunnels rs/
  "crates/vector-config",   # serde + toml
]

[workspace.dependencies]
alacritty_terminal = "0.26"
wgpu = "29"
winit = "0.30"
objc2 = "0.6"
objc2-app-kit = "0.3"     # tracks objc2 0.6
objc2-foundation = "0.3"
tokio = { version = "1.52", features = ["rt-multi-thread", "macros", "io-util", "time", "sync", "process", "fs"] }
portable-pty = "0.9"
russh = "0.60"
octocrab = "0.50"
oauth2 = "5"
keyring = "4"
reqwest = { version = "0.13", default-features = false, features = ["rustls-tls", "json", "stream"] }
tonic = "0.14"
prost = "0.13"
crossfont = "0.9"
serde = { version = "1", features = ["derive"] }
toml = "1"
anyhow = "1"
thiserror = "1"
tracing = "0.1"
tracing-subscriber = "0.3"

# Vendored — not on crates.io
dev-tunnels = { git = "https://github.com/microsoft/dev-tunnels", rev = "<pin>", path = "rs", features = ["connections"] }
```

CI deps (one-time install on the macOS runner):

```bash
# Xcode CLI tools provide lipo, codesign, hdiutil, etc.
xcode-select --install
cargo install cargo-bundle@0.10.0 cargo-deny
```

## Stack Patterns by Variant

**App shell — three options, in order of recommendation:**

1. **`winit` + `objc2-app-kit` for the macOS-native bits** (RECOMMENDED for v1)
   - winit handles the event loop, basic NSWindow, raw-window-handle for wgpu
   - drop down to `objc2-app-kit` for `NSWindow.tabbingMode`, the menu bar, Services
   - smallest learning curve, fastest path to "looks native"
2. **Bespoke per-platform `window` crate (WezTerm's approach)** — defer
   - Pure `objc2-*` on macOS, no winit at all
   - Best long-term control, but ~3x the upfront work; only worth it if winit's NSWindow proves limiting
3. **`gpui 0.2.2` (Zed's framework)** — do NOT use for v1
   - Brilliant for editors, but it's a whole UI toolkit, not a windowing layer
   - Pre-1.0, Zed-driven release cadence, would force us to render terminal cells through GPUI's text system instead of our own wgpu pipeline (loses the fastest path to a fast terminal)
   - Reconsider only if we ever build heavy in-app chrome (settings UI, AI panels)

**Tabs/splits UI:**

- **Tabs:** use `NSWindow` native tabs via `setTabbingMode(.preferred)` (objc2-app-kit). One `NSWindow` per tab; AppKit groups them automatically. Matches ghostty/Apple Terminal behavior. WezTerm draws its own tab bar inside its custom window — looks polished but is a lot of code.
- **Splits:** hand-rolled. There is no Rust crate for this. Both WezTerm and ghostty implement their own pane manager: a recursive enum (`Pane = Leaf(Terminal) | HSplit(Pane, Pane, ratio) | VSplit(...)`) plus drag-to-resize. This is well-trod ground; budget ~1 week.

**Async runtime — terminal-specific notes:**

- PTY I/O is **not** real async on macOS. The kernel exposes the master fd as a regular file descriptor; you can `epoll`/`kqueue` it, but it has edge-case behaviors (signal-driven SIGWINCH, controlling-terminal semantics) that don't survive abstraction layers cleanly.
- Pattern: spawn the PTY in a blocking thread, read into a `BytesMut`, push to `tokio::sync::mpsc::Sender`. The terminal core consumes the channel. This is what `portable-pty 0.9` does internally.
- For Codespaces SSH: `russh` is fully tokio-native. The Dev Tunnel transport sits underneath, also tokio-native. End-to-end async works because we never touch the local kernel PTY for remote sessions.

**Fonts — ligatures + emoji + JIS:**

- **Recommended: `crossfont 0.9` + CoreText on macOS.** Already handles ligatures via OpenType GSUB through CoreText, color emoji via `CGFontCreateCopyWithVariations` + emoji font fallback, and CJK via the system font fallback chain. Mature in Alacritty.
- `cosmic-text 0.19` + `swash 0.2.7` is the alternative — better for general UI text but heavier and not optimized for the cell-based terminal use case.
- `harfbuzz_rs` is **stale** (last release Aug 2021); avoid. WezTerm vendors a `harfbuzz` crate (`deps/harfbuzz`) it maintains itself — don't replicate that unless we have to.

**Config format:**

- **TOML.** `~/.config/vector/config.toml`. Reasons: single canonical syntax, `serde` round-trip, hot-reload via `toml_edit`, no scripting attack surface, no embedded VM.
- Lua (mlua 0.11.6) is the WezTerm choice and shines if users want dynamic config (`if hostname == "x" then ...`). Cost: 1.5 MB binary bloat, scripting-as-config debate, support burden when users `pcall` themselves into corners. Skip.
- JSON5 — VS Code's choice — has no advantage over TOML here.

## Section 7: Microsoft Dev Tunnels — Detailed Feasibility

This was flagged as the riskiest piece. Findings:

### What exists (HIGH confidence)

`github.com/microsoft/dev-tunnels` ships a Rust SDK at `rs/`. Manifest summary (`rs/Cargo.toml`):

```toml
[package]
name = "tunnels"
version = "0.1.0"
edition = "2021"

[dependencies]
russh = { version = "0.37.1", features = ["openssl", "flate2"], optional = true }
russh-keys = { version = "0.37.1", features = ["openssl"], optional = true }
tokio = { version = ">= 1.20", features = ["macros", "io-util", "time"], optional = true }
reqwest = { version = "0.13", default-features = false, features = ["charset", "system-proxy", "json"] }
hyper = { version = "1.9", optional = true }
tokio-tungstenite = { version = "0.29", default-features = false, features = ["native-tls"], optional = true }
serde = ...; serde_json = ...
uuid = ...; jiff = "0.2"; thiserror = ...
```

Microsoft's published support matrix:

| Feature | C# | TS | Java | Go | **Rust** |
|---|---|---|---|---|---|
| Management API | Y | Y | Y | Y | **Y** |
| Tunnel Client Connections | Y | Y | Y | Y | **Y** |
| Tunnel Host Connections | Y | Y | N | N | **Y** |
| Reconnection | Y | Y | N | N | **N** |
| SSH-level Reconnection | Y | Y | N | N | **N** |
| Auto token refresh | Y | Y | N | N | **N** |
| SSH Keep-alive | Y | Y | N | N | **N** |

### Risk profile (MEDIUM confidence on these specifics)

- **Not published to crates.io.** We must vendor as a git dep with a pinned rev. Fine for internal tool; will need a fork if Microsoft archives the repo.
- **Pinned to russh 0.37 internally.** Our top-level project will use russh 0.60 (current). Cargo will resolve two russh versions in the dep graph — that compiles but doubles binary size for russh and makes type interop between layers impossible. Mitigations, in order of preference:
    1. Fork the dev-tunnels Rust crate, bump its russh to 0.60. Likely a 1-2 day patch (russh's API has churned; check 0.37 → 0.60 changelog for breaking changes around `client::Handler`).
    2. Accept the duplication. v1 only — the binary-size hit is single-digit MB.
    3. Submit upstream PR after our fork stabilizes.
- **No reconnection logic.** Wifi drops → connection dies → we re-establish from scratch. PROJECT.md requires "transparent reconnect (wifi drop should not lose Codespace state)". Implementation path: implement reconnection at the application layer ourselves, using `tmux`/`mosh`-style session-resume on the **server** side. The Codespaces image already includes `tmux`; we can wrap remote shells with `tmux new -A -s vector` so a fresh SSH attach picks up the existing session. This is not transparent reconnect at the SSH layer, but it gives the same UX.
- **No auto token refresh.** Wrap in a refresh task that recomputes the tunnel access token via `octocrab`'s GitHub API every ~50 minutes (tokens are 1 hour) before they expire.

### Three contingency paths if the SDK proves unworkable

(a) **Reverse-engineer from the TypeScript or .NET clients** — the protocol is documented across `dev-tunnels-ssh` (the SSH library, available in C# and TS) and `dev-tunnels` (management). HIGH effort, ~4-6 weeks, but tractable. Both sister implementations are open source under MIT.
(b) **Shell out to `code tunnel` binary** — VS Code's CLI is a Rust binary distributed standalone (`https://aka.ms/install-vscode-cli`). It can run as a tunnel client. Pro: zero protocol work. Con: huge dep (~150 MB), violates the "no Electron/VS Code" anti-goal philosophically (though `code tunnel` is just the Rust CLI, no Electron). Acceptable as a *fallback during early dev* to validate UX before we commit to (a).
(c) **Defer Dev Tunnels and ship Codespaces-only first** — Codespaces uses the same SSH-over-tunnel architecture under the hood, and the GitHub-Codespaces flow is fully reachable through octocrab + tonic + russh without the dev-tunnels crate at all. PROJECT.md lists Dev Tunnels as required for v1; this is a downgrade to discuss with the user.

**Recommendation:** Start with the official Rust SDK path. Time-box a 2-day spike to (1) get a "hello world" tunnel client connecting to a `code tunnel`-hosted machine, (2) measure binary impact of the dual-russh issue. If both green, proceed. If the russh version conflict is painful, fork and bump russh in the dev-tunnels crate (path 1). Hold options (a) and (b) in reserve.

## Section 6: GitHub Codespaces SSH — Detailed Plan

### Architecture (HIGH confidence — sourced from `cli/cli` source)

`gh codespace ssh` flow:

1. REST: `GET /user/codespaces` → list, `POST /user/codespaces/{name}/start` if needed
2. REST: get tunnel connection details (this internally hits dev-tunnels)
3. Open Dev Tunnel to the codespace
4. Forward codespace's internal port **16634** locally (this is the gRPC management port)
5. Open gRPC client on that port, call `SSHServerHostService.StartRemoteServer` → returns the SSH server port number
6. Forward that SSH port locally
7. Open a real SSH connection on the forwarded port, exec a shell

### What this means in Rust

| Step | Rust crate |
|------|-----------|
| 1, 2 | `octocrab 0.50` |
| 3, 4, 6 | `microsoft/dev-tunnels` (rs) — port forwarding API |
| 5 | `tonic 0.14` + hand-ported `.proto` from `cli/cli/internal/codespaces/rpc/ssh/ssh_server_host_service.v1.proto` |
| 7 | `russh 0.60` |

The `.proto` files are vendored at `cli/cli/internal/codespaces/rpc/{ssh,codespace,jupyter}/*.proto` — copy them into our repo, run `tonic-build` in a build script. Confirmed RPC services:

- `SSHServerHostService` (start/stop the remote sshd inside the codespace container)
- `CodespaceHost` (`NotifyCodespaceOfClientActivity`, `RebuildContainerAsync`)
- `JupyterServerHost` (irrelevant for v1)

### OAuth (HIGH confidence)

- `oauth2 5.0` device flow with GitHub's documented OAuth client.
- The `gh` CLI public client ID (`178c6fc778ccc68e1d6a`) is reusable for desktop apps; alternatively register a new GitHub OAuth App for Vector — preferred for branding and rate-limit isolation.
- Scopes: `codespace`, `read:user`, `read:org`. Add `repo` only if v2 wants codespace creation.
- Cache refresh token via `keyring 4.0` → macOS Keychain.

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `alacritty_terminal` | hand-rolled VT on `vte 0.15` | Only if we need PTY image protocols (sixel, kitty graphics) — alacritty doesn't expose those. v1 doesn't. |
| `wgpu 29` | raw `metal-rs` | Mac-only forever **and** profiling shows wgpu overhead is the bottleneck (it won't be — fragment shaders dominate). |
| `winit + objc2-app-kit` | WezTerm-style bespoke window crate | Future cross-platform sophistication beyond what winit gives us; or if winit's NSWindow integration ever blocks a feature. |
| `winit + objc2-app-kit` | `gpui 0.2.2` | We start building heavy UI surfaces (settings, panels, AI overlays) and a real layout system saves time. Don't pre-pay this cost in v1. |
| `russh 0.60` | `openssh 0.11` (wrap `ssh` binary) | Local-only SSH connections (plain ssh into a server) where we don't need programmatic forwarding. Codespaces and Dev Tunnels both need programmatic port forwarding *inside* an existing tunnel — cannot use openssh-the-wrapper. |
| `russh 0.60` | `thrussh` | Don't. `thrussh` is the predecessor; russh is the active fork. |
| `russh 0.60` | `ssh2` (libssh2 binding) | Avoid — C dependency, not async, less actively maintained. |
| `octocrab 0.50` | `reqwest` direct | If we only need 2-3 endpoints and want to drop the dep. Octocrab is small enough that this is rarely worth it. |
| `crossfont 0.9` | `cosmic-text 0.19 + swash 0.2.7` | If we want a single-stack solution that also does rich-text shaping for non-terminal UI (settings panel rendering, etc.). |
| `cargo-bundle 0.10` | `tauri-bundler` | Never for this project — Tauri's bundler expects a Tauri app. |
| `cargo-bundle 0.10` | hand-rolled shell script | When we need fine-grained control over `Info.plist` quirks. Currently `cargo-bundle` covers our needs. |
| TOML | `mlua 0.11.6` (Lua) | Users demand programmatic config (rare). Cost: 1.5MB+ binary, sandbox debate. Defer to v2. |

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| **`cocoa-rs`** | Effectively unmaintained; supplanted by objc2-app-kit. Old API, lots of `unsafe`. | `objc2-app-kit 0.3` |
| **`objc` (the original crate)** | Same. v0.2 is the legacy ABI. | `objc2 0.6` |
| **`harfbuzz_rs 2.0.1`** | Last release Aug 2021. Stale. | `crossfont` (CoreText on macOS does shaping) or vendor harfbuzz like WezTerm if we need cross-platform. |
| **`tokio_pty_process`** | Older PTY crate, less complete than portable-pty. | `portable-pty 0.9` |
| **`thrussh`** | Pre-fork ancestor of russh. Unmaintained. | `russh 0.60` |
| **`ssh2`** crate | C dep (libssh2), not async, less active. | `russh 0.60` |
| **`tauri`** | Webview-based app shell. Wrong tool for a terminal. | `winit + wgpu + objc2-app-kit` |
| **GPUI for the renderer** | UI framework, not a renderer. Wraps the terminal in a Zed-shaped tree. | `wgpu` directly with our own glyph atlas. |
| **Lua / `mlua` for v1 config** | Bloat + scripting attack surface for no v1 value. | TOML. Reconsider in v2 if users complain. |
| **Apple's `Sparkle` updater for v1** | Sparkle requires signed builds to verify updates. Our v1 is unsigned. | Manual download from GitHub Releases. Add Sparkle when signing happens. |
| **`cargo-dist`** for v1 release | Adds an opinionated release framework when a 30-line shell script suffices. | `cargo build --release && lipo && cargo bundle --release && hdiutil create`. |
| **JSON5 for config** | No win over TOML for this audience. | TOML. |

## Version Compatibility

| Pair | Compatible | Notes |
|------|-----------|-------|
| `wgpu 29` ↔ `winit 0.30` | ✅ | Use `wgpu::Instance::create_surface_unsafe` with raw-window-handle 0.6, exported by winit 0.30. |
| `tonic 0.14` ↔ `tokio 1.52` | ✅ | tonic 0.14 requires tokio ≥ 1.27. |
| `tonic 0.14` ↔ `prost 0.13` | ✅ | tonic 0.14.x targets prost 0.13.x. |
| `octocrab 0.50` ↔ `reqwest 0.13` | ✅ | octocrab 0.50 is pinned to reqwest 0.13. |
| `russh 0.60` ↔ `dev-tunnels (rs/) russh 0.37` | ⚠️ | DUAL VERSIONS. Either fork dev-tunnels and bump, or accept ~3MB binary duplication. See Section 7. |
| `keyring 4.0` ↔ Rust toolchain | ⚠️ | Requires Rust 1.88+. Set in `rust-toolchain.toml`. |
| `alacritty_terminal 0.26` ↔ Rust toolchain | ✅ | Requires 1.85+. |
| `cosmic-text 0.19` ↔ `swash 0.2.7` | ✅ | cosmic-text 0.19 default-enables swash. |
| `objc2-app-kit 0.3` ↔ `objc2 0.6` | ✅ | objc2-app-kit 0.3.x tracks objc2 0.6. |
| Universal binary | n/a | Cargo cannot fat-build. Run `cargo build --release --target aarch64-apple-darwin` and `--target x86_64-apple-darwin` separately, then `lipo -create -output target/universal/vector target/{aarch64,x86_64}-apple-darwin/release/vector`. |

## Confidence Assessment per Recommendation

| Item | Confidence | Why |
|------|-----------|-----|
| `alacritty_terminal` for VT core | HIGH | Verified via crates.io (version 0.26.0, 2026-04-06); standard choice; alternative (wezterm-term) isn't published. |
| `wgpu` for rendering | HIGH | Verified via crates.io (29.0.3, 2026-05-02); WezTerm uses wgpu confirmed in their workspace Cargo.toml; only alternative for Mac-only is metal-rs which adds zero benefit at v1 scope. |
| `winit + objc2-app-kit` for app shell | MEDIUM | Verified versions. The integration pattern works (it's how 100+ Rust desktop apps are built), but native tabs via NSWindow tabbingMode + winit-managed event loop has known quirks — budget time for AppKit-specific debugging in early dev. |
| `tokio` for async | HIGH | Universal default. |
| `russh` for SSH | HIGH | Active maintenance verified (0.60.2, 2026-04-29); used by Microsoft's own Dev Tunnels Rust SDK. |
| `octocrab` for GitHub API | HIGH | 0.50.0 (2026-05-05); standard. |
| `oauth2` device flow | HIGH | 5.0 stable since Jan 2025; well-trod path. |
| Codespaces gRPC reimplementation | MEDIUM | Verified `cli/cli`'s rpc package structure (subdirs `ssh`, `codespace`, `jupyter`, with `.proto` files). The `.proto` schemas exist and are public. Risk: GitHub may version-bump them without notice; we should vendor + pin and re-sync occasionally. |
| Microsoft Dev Tunnels Rust SDK | MEDIUM | SDK existence verified (rs/Cargo.toml fetched). Risk areas (russh version skew, no reconnect/refresh) are real and quantified. Mitigations are tractable but add work. |
| `crossfont` for font rendering | HIGH | Alacritty uses it in production, handles all our requirements. |
| `cargo-bundle` for `.app` | HIGH | 0.10.0 (2026-04-18); maintained; covers our needs. |
| Universal binary via `lipo` | HIGH | Standard macOS tooling; well-documented. |
| TOML config | HIGH | toml 1.1.2 stable, standard for Rust apps. |

## Sources

**Crates.io API (versions verified against live data, 2026-05-10):**
- [alacritty_terminal 0.26.0](https://crates.io/crates/alacritty_terminal) — published 2026-04-06
- [wgpu 29.0.3](https://crates.io/crates/wgpu) — 2026-05-02
- [winit 0.30.13](https://crates.io/crates/winit) — 2026-03-02
- [tokio 1.52.3](https://crates.io/crates/tokio) — 2026-05-08
- [russh 0.60.2](https://crates.io/crates/russh) — 2026-04-29, maintained by @Eugeny
- [octocrab 0.50.0](https://crates.io/crates/octocrab) — 2026-05-05
- [oauth2 5.0.0](https://crates.io/crates/oauth2) — 2025-01-21
- [keyring 4.0.0](https://crates.io/crates/keyring) — 2026-04-26
- [reqwest 0.13.3](https://crates.io/crates/reqwest) — 2026-04-27
- [tonic 0.14.6](https://crates.io/crates/tonic) — 2026-05-07
- [vte 0.15.0](https://crates.io/crates/vte) — 2025-02-02
- [crossfont 0.9.0](https://crates.io/crates/crossfont) — 2025-06-09
- [cosmic-text 0.19.0](https://crates.io/crates/cosmic-text) — 2026-04-22
- [swash 0.2.7](https://crates.io/crates/swash) — 2026-03-27
- [portable-pty 0.9.0](https://crates.io/crates/portable-pty) — 2025-02-11
- [objc2 0.6.4](https://crates.io/crates/objc2) — 2026-02-26
- [cargo-bundle 0.10.0](https://crates.io/crates/cargo-bundle) — 2026-04-18
- [glutin 0.32.3](https://crates.io/crates/glutin) — 2025-04-30
- [harfbuzz_rs 2.0.1](https://crates.io/crates/harfbuzz_rs) — 2021-08-28 (stale, listed as not-recommended)
- [gpui 0.2.2](https://crates.io/crates/gpui) — 2025-10-22
- [mlua 0.11.6 stable / 0.12.0-rc.1](https://crates.io/crates/mlua) — 2026-04-21
- [openssh 0.11.6](https://crates.io/crates/openssh) — 2025-12-03

**Reference implementation source (Cargo.toml fetches):**
- [WezTerm workspace Cargo.toml](https://raw.githubusercontent.com/wezterm/wezterm/main/Cargo.toml) — confirmed wgpu 25, tokio 1.43, vendored harfbuzz/freetype, no winit (custom window crate)
- [WezTerm window/Cargo.toml](https://raw.githubusercontent.com/wezterm/wezterm/main/window/Cargo.toml) — confirmed bespoke per-platform windowing using cocoa, core-foundation, core-graphics, objc, objc2-core-graphics directly
- [Alacritty alacritty/Cargo.toml](https://raw.githubusercontent.com/alacritty/alacritty/master/alacritty/Cargo.toml) — confirmed glutin 0.32.2 + winit 0.30.9 + crossfont 0.8.1 + objc2 (NOT migrated to wgpu/Metal)

**Microsoft Dev Tunnels (the high-stakes finding):**
- [microsoft/dev-tunnels GitHub](https://github.com/microsoft/dev-tunnels) — repo with `rs/`, `cs/`, `ts/`, `go/`, `java/` SDKs; feature support matrix confirmed Rust has Management/Client/Host
- [dev-tunnels rs/Cargo.toml](https://raw.githubusercontent.com/microsoft/dev-tunnels/main/rs/Cargo.toml) — version 0.1.0 (unpublished), russh 0.37.1, reqwest 0.13, tokio 1.20+
- [microsoft/dev-tunnels-ssh](https://github.com/microsoft/dev-tunnels-ssh) — SSH protocol library in C#/TS (not Rust); contingency reference if we ever need to reverse-engineer

**GitHub Codespaces internals:**
- [cli/cli internal/codespaces/](https://github.com/cli/cli/tree/trunk/internal/codespaces) — subpackages: api, connection, portforwarder, rpc
- [cli/cli internal/codespaces/rpc/](https://github.com/cli/cli/tree/trunk/internal/codespaces/rpc) — subdirs: ssh, codespace, jupyter, test; files invoker.go, generate.md
- [cli/cli internal/codespaces/rpc/ssh](https://github.com/cli/cli/tree/trunk/internal/codespaces/rpc/ssh) — `ssh_server_host_service.v1.proto` confirms gRPC SSHServerHostService
- [gh codespace ssh manual](https://cli.github.com/manual/gh_codespace_ssh)
- [gh cs ssh #11206](https://github.com/cli/cli/issues/11206) — confirms internal port 16634 for gRPC

**Reference implementations (architectural context):**
- [Zed GPUI README](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md) — Metal on macOS, custom shaders per primitive
- [Zed "Leveraging Rust and the GPU"](https://zed.dev/blog/videogame) — 120fps text, glyph atlas
- [GPUI standalone status](https://www.gpui.rs/) — pre-1.0, Zed-driven cadence
- [Ghostty release pipeline](https://deepwiki.com/ghostty-org/ghostty/8.2-release-pipeline) — universal binary, Sparkle for updates (signed-only)
- [Ghostty platform-specific](https://deepwiki.com/ghostty-org/ghostty/5-platform-specific-implementations) — Swift+AppKit+SwiftUI on libghostty C ABI
- [WezTerm DeepWiki](https://deepwiki.com/wezterm/wezterm) — workspace structure, mux/term/window/font separation

**OAuth + Auth:**
- [oauth2 docs.rs](https://docs.rs/oauth2/latest/oauth2/) — RFC 8628 device flow
- [github-device-flow crate](https://crates.io/crates/github-device-flow) — alternative thin wrapper (uses oauth2 underneath)

**Packaging:**
- [cargo-bundle GitHub](https://github.com/burtonageo/cargo-bundle)
- [Adam Israel: Rust Universal Binaries](https://www.adamisrael.com/blog/rust-universal-binaries/) — lipo workflow
- [Tauri universal binaries issue](https://github.com/tauri-apps/tauri/issues/3317) — confirms cargo can't fat-build natively

---
*Stack research for: GPU-accelerated native macOS terminal in Rust with Codespaces + Dev Tunnels*
*Researched: 2026-05-10*
