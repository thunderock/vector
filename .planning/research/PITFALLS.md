# Pitfalls Research

**Domain:** Native macOS GPU-accelerated terminal in Rust with GitHub Codespaces SSH and Microsoft Dev Tunnels client integration
**Researched:** 2026-05-10
**Confidence:** HIGH for terminal/PTY/GPU/macOS pitfalls (mature ecosystem, abundant prior art); MEDIUM for Codespaces SSH (source-available reference in `cli/cli` Go); LOW-to-MEDIUM for Dev Tunnels (a `rs/` directory exists in `microsoft/dev-tunnels` but is not a published, supported crate — see Pitfall 14)

> Reading guide: pitfalls are ordered by **damage potential** for this specific project, not by domain. The single biggest risk is over-scoping (Pitfalls 16–22). The single biggest technical wildcard is Dev Tunnels (Pitfalls 13–15). Read those first.

## Critical Pitfalls

### Pitfall 1: Building a VT parser from scratch instead of using `vte` / `alacritty_terminal`

**What goes wrong:**
You decide "the parser is the heart of the terminal, I should own it" and write your own state machine. Six weeks later you are still discovering that `ED 3` (erase scrollback) behaves differently from `ED 2`, that `DECSTR` (soft reset) must NOT clear scrollback, that `DCS … ST` payloads can span PTY reads, and that mode 1049 is not just "switch buffer" — it must save cursor, switch, clear, and restore on exit in a specific order or `vim` corrupts the user's primary screen on quit.

**Why it happens:**
The Paul Williams state diagram looks small (~20 states). It is not. The hard part is the dispatch table — hundreds of CSI/OSC/DCS sequences with version-dependent semantics, and a long tail of de-facto standards (xterm extensions, kitty graphics, mouse modes 1006/1015/1016, OSC 52 base64 paste, OSC 8 hyperlinks). Every mainstream terminal has a multi-thousand-line dispatch table and decades of bug fixes baked in.

**How to avoid:**
- **Use `alacritty_terminal` as the grid + parser.** It is published as a separate crate precisely for reuse, ships with the Williams parser via `vte`, and has the dispatch table battle-tested by every Alacritty user since 2017.
- If `alacritty_terminal` is too opinionated about the grid, drop one level and use the `vte` crate directly for parsing only — then you only own the grid and dispatch handlers.
- **Never** roll your own escape-sequence state machine. Even the existence of bracketed-paste-mode bypass CVEs in shipping terminals (MinTTY, Xshell, ZOC) shows this is harder than it looks.

**Warning signs:**
- You catch yourself writing `match byte { 0x1b => ... }` outside a vendored parser.
- A test fixture of "real-world `vim`/`tmux`/`htop` output" shows visual artifacts.
- Issue triage starts skewing toward "X program looks weird in our terminal."

**Phase to address:**
Phase 1 (terminal core). Pick the parser the day you start. Do not defer.

---

### Pitfall 2: Glyph atlas churn under font fallback (CJK / emoji / ligatures)

**What goes wrong:**
You build the renderer with a single font + monospace assumption. Then someone runs `git log` with a Chinese commit message, or a CI log spits out 🎉 in green, or a Rust log shows `=>` rendered with ligatures. The atlas ping-pongs between font faces, an emoji glyph (color bitmap, not SDF) blows the layout because it claims double width *or* the "wrong" advance width, and the entire row re-rasterizes every frame.

**Why it happens:**
Mac users have CJK and emoji content in real terminal output (commit messages, container logs, AI tool output). Emoji are color bitmap fonts (Apple Color Emoji), not vector — they live in a different texture format than your monospace SDF/MSDF cache. Wide characters use Unicode East Asian Width, which is not what most fonts say. Ligatures need shaping (HarfBuzz / `swash` / `cosmic-text`) which is per-cluster, not per-glyph.

**How to avoid:**
- **Use `cosmic-text` (or `swash`) for shaping + fallback** — it solves font fallback chains, BiDi (free benefit), emoji, and shaping in one library and is designed for Rust GPU terminals (Cosmic Term uses it).
- **Two atlases, not one:** monochrome SDF/grayscale for primary font, RGBA bitmap for color emoji. Different texture formats, different shader paths. Don't try to unify.
- **Cap the atlas and evict on LRU.** A bounded atlas with eviction beats an unbounded one that fragments.
- **Width = `unicode-width` crate value, not font advance.** Fonts lie about CJK width; the Unicode property table doesn't.
- **Ligatures off by default in remote sessions.** A flicker in `nvim` over Codespaces is worse than no ligatures. Make it a per-profile toggle.

**Warning signs:**
- Frame time spikes when scrolling logs that contain emoji.
- Column alignment drifts after a CJK character.
- `tmux` status bar wobbles by one cell after a non-ASCII update.

**Phase to address:**
Phase 1 (renderer) — the choice between `cosmic-text`, `swash`, or rolling your own shaping is permanent. Don't punt.

---

### Pitfall 3: Frame pacing on macOS: presenting unsynchronized to display

**What goes wrong:**
You hook `wgpu` to a `winit` window and call `present()` whenever the PTY reader has bytes. Result: tearing, occasional 30 fps stutter on ProMotion (120 Hz), and a fan that spins under `cat large.log` because you're rendering 8000 frames per second while the display only consumes 120.

**Why it happens:**
On macOS, the canonical frame-pacing primitive is `CADisplayLink` (or `CAMetalDisplayLink` in newer SDKs) which fires at the display's refresh rate and adapts to ProMotion. `winit` does not natively give you display-link callbacks; it pumps based on its event loop. `wgpu` defaults to `PresentMode::Fifo` (vsync), but if you call `present()` faster than vsync, you queue up frames and burn power. iTerm2 explicitly notes that low-power mode disables Metal rendering — this is a real concern users will file bugs about.

**How to avoid:**
- Use `wgpu`'s `PresentMode::Fifo` (vsync) on macOS. Verify by inspecting `surface.get_capabilities()`.
- Render only when there is dirty state. PTY input → mark dirty → request redraw via `Window::request_redraw()`. Do not render on a wall-clock timer.
- Coalesce PTY bursts into the next frame. A `cat large.log` should produce one render per vsync, not one render per `read()`.
- On low-power mode (detect via `NSProcessInfo.lowPowerModeEnabled`), cap to 30 fps explicitly and document the trade-off — don't fight the OS.
- Watch for ProMotion (120 Hz) and don't hardcode 60.

**Warning signs:**
- Activity Monitor shows multi-watt GPU draw on idle terminal.
- Visible tearing at the bottom of the window.
- Battery measurably worse than ghostty/Alacritty under the same load.

**Phase to address:**
Phase 1 (renderer). Get this right at first paint; retrofitting frame-pacing fixes scrollback bugs later.

---

### Pitfall 4: Partial UTF-8 reads from the PTY

**What goes wrong:**
You `read()` from the master PTY, get a buffer ending mid-multibyte sequence, call `String::from_utf8(buf)`, and either panic (on `unwrap`), produce U+FFFD garbage on screen (on `from_utf8_lossy`), or — worst case — feed the truncated bytes into a UTF-8-naive parser that miscounts cells. CJK/emoji rows show transient corruption every burst.

**Why it happens:**
PTY reads are bounded by buffer size, not by Unicode boundaries. A 3-byte UTF-8 sequence (e.g. CJK ideograph) split across two reads is the rule, not the exception, on busy output. `from_utf8_lossy` is *correct* but *destructive* — it replaces the partial bytes with U+FFFD and the next read's continuation bytes also become U+FFFD orphans.

**How to avoid:**
- **Operate on bytes through the parser.** `vte` and `alacritty_terminal` accept `&[u8]` precisely so you can feed PTY chunks directly without Unicode boundary worries — the parser handles it.
- If you must hold a UTF-8 string buffer somewhere, use `encoding_rs` or `simdutf8` with a "carry trailing partial sequence to next call" pattern. The `servo/futf` crate solves exactly this.
- Never call `from_utf8` / `from_utf8_lossy` on a raw PTY chunk.

**Warning signs:**
- Glitches scrolling Chinese log lines.
- Random `?` (U+FFFD) characters in `git log` output with non-ASCII commit messages.
- Differential output vs. iTerm2 on the same `cat` of a binary or non-UTF-8 file.

**Phase to address:**
Phase 1 (PTY plumbing). Foundational, irreversible if you build the wrong abstraction on top.

---

### Pitfall 5: `winit` event loop ownership vs. `tokio` runtime

**What goes wrong:**
You start with `tokio::main` and try to drive `winit::EventLoop` from inside an async task. macOS panics: `EventLoop must be created on the main thread`. You move `winit` to main, then your `tokio` PTY reader is on a background thread that can't easily wake the UI. Worse: you accidentally call `Window::set_title` from a worker thread → AppKit crash, or you trigger `winit`'s `RefCell` reentrancy panic on macOS by sending events while the loop is dispatching another event.

**Why it happens:**
AppKit *requires* its event loop on the main thread. `winit`'s `EventLoop::run` consumes the main thread. `tokio` wants to own its own runtime. These two facts are in tension, and there is no clean async abstraction over `EventLoop` today.

**How to avoid:**
- **`winit` on the main thread, period.** No exceptions, no `tokio::main`.
- Spawn `tokio::runtime::Runtime` on a *separate* thread. Use `EventLoopProxy::send_event` (or `winit`'s newer `EventLoopProxy::send_event`) to wake the UI from async tasks.
- All AppKit-touching work (window title, decorations, dock, menu bar) goes through `EventLoopProxy` → main-thread handler. Never call `objc2`/AppKit from a worker thread.
- Audit every `block_on` — they are landmines on the main thread (deadlocks the UI).
- Use a single MPSC channel from "PTY/network workers" → "UI" with explicit message types. No `Arc<Mutex<UiState>>` shared across threads if you can avoid it.

**Warning signs:**
- Any `tokio::main` macro on the binary entry.
- A `set_title` call in code that isn't obviously on the main loop.
- CI tests that pass on Linux fail on macOS with a thread-affinity panic.

**Phase to address:**
Phase 0 (project skeleton). The decision of "who owns main" is the first architectural decision and the most expensive to reverse.

---

### Pitfall 6: macOS Gatekeeper friction worse than expected

**What goes wrong:**
You ship the unsigned `.dmg`, your teammate downloads it, double-clicks the app, and gets "Vector.app is damaged and can't be opened. You should move it to the Trash." (not even "unidentified developer"). They bounce. You troubleshoot and discover that on macOS Sequoia (14.x+), Apple removed the Settings → "Open Anyway" panel button for unsigned apps; the only escape hatch is `xattr -d com.apple.quarantine /Applications/Vector.app`, which a non-CLI-fluent teammate will not do.

**Why it happens:**
- Browsers (Chrome, Safari) automatically tag downloads with `com.apple.quarantine`.
- The DMG inherits the attribute; mounted contents inherit it; the `.app` inside inherits it.
- Recent macOS versions have made the bypass UI progressively harder. "Right-click → Open" still works in Sonoma/Sequoia for now, but the popup wording has changed multiple times and is non-obvious.
- Universal binaries built without `lipo` correctly producing both architectures fail silently on the "wrong" CPU (e.g., builds on a Mac Studio that only ship arm64 will refuse to launch on an Intel MacBook with no helpful error).

**How to avoid:**
- **Document the dequarantine command in the release notes prominently.** Not a footnote. Step 1.
- Provide a one-liner installer script users can pipe to `bash` (the script does `xattr -dr com.apple.quarantine /Applications/Vector.app` after copy).
- Build true Universal binaries: `cargo build --release --target aarch64-apple-darwin && cargo build --release --target x86_64-apple-darwin && lipo -create -output ...`. Verify with `file Vector.app/Contents/MacOS/vector` showing both architectures.
- Set the macOS deployment target via `MACOSX_DEPLOYMENT_TARGET=13.0` in CI to match the stated baseline. Without this you'll accidentally use Sonoma-only APIs and break Ventura users.
- Have an "if this gets too painful, sign with a free Apple ID dev cert" escape valve documented. ($99/yr cost is a v2 trade-off, not a tech blocker.)

**Warning signs:**
- Teammate Slack DM: "the app says it's damaged."
- Crash reports without symbols (un-notarized + crashed = no Apple Crash Reporter symbolication).
- Binary size differs by 2x between developer machine and CI artifact (one architecture missing).

**Phase to address:**
Phase 5 (packaging/CI). Bake the `xattr` instructions into the README the day you publish the first DMG.

---

### Pitfall 7: PTY signal & resize handling — hangs and dead processes

**What goes wrong:**
User resizes the window. Your code calls `pty.resize(new_size)` which sends `SIGWINCH` to the foreground process group. But:
- You forgot to track which process group is foreground (it's the one in `tcgetpgrp(slave_fd)`, not the pid you spawned).
- On a `gh codespace ssh`-like remote shell, SIGWINCH must be re-emitted on the *remote* side via the SSH `window-change` channel request, which is a separate code path.
- The user kills the shell, the master fd doesn't immediately EOF (read blocks indefinitely), the UI hangs on close.
- You call `read()` on the master in a `tokio` blocking task, the task gets stuck, runtime worker count drops, every other PTY hangs.

**Why it happens:**
Unix PTY semantics are subtle. The controlling-terminal/process-group dance is decades old, every detail matters, and the failure modes are silent (a stuck shell is the *normal* failure for a missing detail).

**How to avoid:**
- **Use `portable-pty`** for the local PTY (cross-platform abstraction, handles `posix_openpt`/`forkpty`/ConPTY differences) rather than rolling raw `nix` calls.
- For SSH (Codespaces / Dev Tunnels), the SSH transport must propagate window changes via the SSH `pty-req` extension. `russh` / `thrussh` handle this; verify it actually fires by `printf '\e[18t'` from inside the remote shell after a resize and watching the response width.
- **Never** `read()` on the master from a `tokio` worker with the default runtime. Use `tokio::task::spawn_blocking`, or use `tokio-fd` / `mio`-based registration so the read is properly poll-driven and cancellable on close.
- On window close: explicitly send SIGHUP to the process group, *then* close the master. Don't rely on FD close to clean up.
- Honor the user's `ISIG`, `IUTF8`, `OPOST` termios flags — the wrong defaults make `Ctrl-C` not work or break Unicode in `bash`.

**Warning signs:**
- After a window resize, `tput cols` reports the old width.
- Closing a tab leaves a defunct shell process visible in `ps`.
- Remote `htop` doesn't redraw on resize until you press a key.
- `tokio` reports starved workers under load.

**Phase to address:**
Phase 1 (local PTY) and Phase 3 (remote SSH transport) — both must implement resize, separately, and both can break independently.

---

### Pitfall 8: Tmux + remote terminal escape-sequence layering ("double multiplex")

**What goes wrong:**
User runs `tmux` in their Codespace (very common pattern). Their local Vector terminal is the outer multiplexer in spirit (tabs/splits/scrollback). When the user copies text in remote `tmux`, OSC 52 clipboard escape goes: remote app → remote tmux → SSH → Vector. tmux's `allow-passthrough` setting and the DCS `tmux;` wrapping make this work *or* fail in subtle ways. Worse, if Vector advertises itself as something tmux doesn't recognize (`TERM=vector`), tmux's terminfo lookup falls back to safe defaults and the user loses true-color, mouse, and bracketed paste.

**Why it happens:**
- Tmux passes through escapes only if `allow-passthrough on` is set, and only for sequences shorter than ~60 chars (a known bug). Longer sequences (e.g. base64-encoded clipboard via OSC 52) get truncated.
- `TERM` advertising must be a value remote `terminfo` actually has compiled (`xterm-256color`, `tmux-256color`, or a value compiled-in via `tic`). Custom values silently degrade.
- Bracketed-paste detection differs between local and remote layers; double-wrapping is a known UX bug.

**How to avoid:**
- **Advertise `TERM=xterm-256color` by default.** Don't invent a custom terminfo. Add `xterm-kitty`-style features only behind an opt-in (and ship the `terminfo` file separately, like kitty does).
- Test with `tmux` running locally *and* on the remote. Run `infocmp` to confirm capabilities are present. Run the OSC 52 paste test through both layers. Run a bracketed-paste test (`bash -c 'read -p "> " x; echo got: $x'` and paste a multi-line block) through both layers.
- Document the user-side `set -g allow-passthrough on` requirement for clipboard passthrough; don't pretend it's transparent.
- Don't try to "out-multiplex" tmux (Pitfall 21). Coexist with it.

**Warning signs:**
- Clipboard via OSC 52 works locally but not over SSH.
- True colors look "off" only in remote tmux.
- Mouse selection works in plain remote shell but breaks inside remote tmux.

**Phase to address:**
Phase 1 (escape dispatch + clipboard) and Phase 3 (remote SSH testing). Add a "tmux integration smoke test" to CI early.

---

### Pitfall 9: Codespaces SSH — assuming it's plain SSH

**What goes wrong:**
You assume `gh codespace ssh` is a thin wrapper around `ssh user@host`. You implement TCP+SSH and discover Codespaces SSH does not expose a public TCP endpoint. You then try the `--stdio` flag's behavior and discover it's actually a websocket-tunneled SSH stream behind an OAuth-derived ephemeral cert, gated by a `state == "Available"` check that requires polling after a `POST /user/codespaces/{name}/start` and the connection details are returned in `connection.tunnelProperties` only when you call the API with `?internal=true&refresh=true`.

**Why it happens:**
- Codespaces SSH uses GitHub's tunneling relay (the same primitive Dev Tunnels uses).
- The connection flow is: list → start (idempotent, swallow 409) → poll until Available → fetch tunnel properties → open tunneled SSH → `pty-req` → shell.
- The `cli/cli` Go reference is the source of truth. Anything inferred from blog posts is stale.
- The `internal=true&refresh=true` query parameters on the GET endpoint are undocumented in the public REST docs but mandatory for getting tunnel credentials.

**How to avoid:**
- **Read `cli/cli/internal/codespaces/api/api.go` and `cli/cli/internal/codespaces/ssh.go` end-to-end.** That is the spec. Re-read on every protocol bump.
- Implement the state machine explicitly: List → Start (handle 409 + 202) → Poll (1s interval, ≤2min ceiling, GitHub RPC has a 10s timeout per call) → Connect.
- Use `gh codespace ssh -c <name> --stdio` as the *first* implementation: spawn `gh` as a subprocess, pipe stdio into your SSH layer. This is a working day-1 fallback that lets you defer the websocket-relay reimplementation. You can replace it with a native Rust client later without users noticing.
- Confirm the OAuth scopes you request actually allow Codespaces access. Fine-grained PATs explicitly do *not* work today (`gh cs ssh` fails with FGPATs — issue cli/cli#7819). Use device-code flow with classic-PAT scopes (`codespace`, `repo`, `read:user`).

**Warning signs:**
- Your code makes raw TCP connections to a hostname like `*.github.dev`.
- You hit 401s after 30 minutes (token refresh missing).
- Codespaces in "Stopped" state get a connection error instead of automatically starting.

**Phase to address:**
Phase 3 (Codespaces integration). Day 1 of that phase: subprocess `gh` and prove the end-to-end shell works. Day N: replace with native Rust.

---

### Pitfall 10: Dev Tunnels — assuming the protocol is documented enough to reimplement cleanly

**What goes wrong:**
You commit to building a native Rust Dev Tunnels client for v1 because the `microsoft/dev-tunnels` repo *has* an `rs/` directory. You discover the `rs/` SDK is **not published to crates.io**, has limited public docs vs. the C#/TS SDKs, and may lag protocol changes. You either: (a) vendor it as a Git dependency and break on the next protocol bump, (b) reverse-engineer the wire protocol from the Go/TS SDKs and re-implement it (estimated multi-week effort), or (c) give up and ship Codespaces-only.

**Why it happens:**
- Microsoft's "first-party" SDKs are C#, TypeScript, Java, Go. Rust is in the repo but is not in the README badge row, not on NuGet/npm/Maven equivalents, and not advertised.
- The Dev Tunnels protocol is "SSH-over-WebSocket via a Microsoft-hosted relay" with auth via Microsoft service tokens exchanged from GitHub OAuth. The protocol *itself* is SSH (so the wire crypto is standard), but the relay handshake, tunnel allocation, and auth-token exchange are Microsoft-specific.
- A community Rust crate `tunnels` (by `btwiuse` on crates.io) wraps/vendors the in-repo SDK, but is third-party and not officially supported.

**How to avoid:**
- **Adopt the same v1 fallback as Codespaces: subprocess the official `devtunnel` CLI** (or `code tunnel`) for the connection bootstrap, then attach SSH on top. This sidesteps the SDK question for v1.
- If subprocessing is unacceptable, **vendor `microsoft/dev-tunnels/rs/`** as a `path = "vendor/dev-tunnels-rs"` Cargo dep, pin the commit hash, and have a script to refresh it. Accept that breakage is possible on protocol bumps.
- **Ship Codespaces-only v1.** This is the contingency. See "Dev Tunnels Contingency Plan" below.
- Do *not* attempt clean-room reverse engineering of the relay handshake in v1. That is a multi-month rabbit hole and the protocol can change.

**Warning signs:**
- You start writing custom websocket framing or token-exchange code.
- You're reading the C# SDK to understand a wire format.
- Your "Dev Tunnels" subtask grows past 1 week of estimated effort.

**Phase to address:**
Phase 3 or Phase 4 (Dev Tunnels integration). **Make the Go/No-Go decision the first day of that phase**, not midway through.

---

### Pitfall 11: Configuration sprawl — Lua/TOML/JSON/DSL

**What goes wrong:**
You add config "while we're here." First it's TOML for keybinds. Then someone wants conditional bindings, so you add a mini-DSL. Then themes need theming, so you add hot-reload. Then someone wants per-host PTY env, so you add a `[hosts.*]` section. Six months later you have 1500 lines of config-schema code, two file formats, and a bug where reloading on save crashes if the user has a syntax error.

**Why it happens:**
WezTerm uses Lua. Alacritty uses TOML (and migrated *from* YAML, breaking everyone — see issue #7474). Kitty uses its own format. ghostty uses a flat key=value format. Each terminal eventually faces "user wants logic in config" pressure. Configurable terminals end up shipping config as a programming environment.

**How to avoid:**
- **Single TOML file. No DSL. No hot-reload in v1.** Restart on config change.
- Strict schema (use `serde` with `#[serde(deny_unknown_fields)]`). Refuse to start with a clear error on unknown keys — better than silent ignoring.
- Cap the surface: keybinds, theme, font, default profile, list of profiles. Nothing else.
- Resist the "advanced.experimental" namespace. Once you ship it, you support it.

**Warning signs:**
- You're considering Lua / Rhai / Dyon as a dependency.
- A config feature requires a feature flag.
- You catch yourself writing config-validation tests with more cases than terminal-grid tests.

**Phase to address:**
Phase 2 (settings/profiles). The constraint is mostly cultural — re-read this when tempted to add config keys.

---

### Pitfall 12: Compile time and dependency bloat

**What goes wrong:**
A Rust workspace with `wgpu`, `cosmic-text`, `tokio`, `octocrab`, `russh`, plus a UI layer (`winit` + custom drawing) hits a clean-build time of 5–10 minutes on a developer's laptop. Incremental builds help, but the moment you change something deep (e.g. a feature flag in `wgpu`), you wait. Workspace recompile time becomes the iteration-speed bottleneck.

**Why it happens:**
- `wgpu` itself is heavy.
- `tokio` with `full` features is heavy.
- `octocrab` pulls in a lot of HTTP/serde machinery.
- `cosmic-text` pulls in shaping/font libraries.
- These are all *necessary*; the question is how much you compound them.

**How to avoid:**
- Use `tokio` with explicit feature flags (`rt-multi-thread`, `io-util`, `net`, `process`, `signal`, `macros`). Not `full`.
- Workspace structure: keep the renderer, terminal core, and network code in *separate crates* so unrelated changes don't trigger a renderer rebuild.
- Use `sccache` or `cargo`'s `-Z` build-std=false (default) caches. Add `[profile.dev] opt-level = 1` for usable runtime in dev builds.
- Set CI release builds with LTO off in PR builds, on only for tagged releases.
- Audit `cargo tree` quarterly. Reject crates that pull in `openssl-sys` if `rustls` works (consistency reduces build time).
- Don't add a logging framework crate (`tracing-subscriber` with all features) unless you're using the features.

**Warning signs:**
- `cargo build` takes >2 minutes on incremental.
- `cargo tree | wc -l` over ~600.
- CI duration creeping past 10 minutes for `cargo test`.

**Phase to address:**
Phase 0 (workspace skeleton) sets the default features. Re-audit at Phase 5 (CI hardening).

---

## High-Risk Domain Pitfalls

### Pitfall 13: Dev Tunnels SDK churn breaking the build

**What goes wrong:**
You vendor `microsoft/dev-tunnels/rs/` at a commit. Three months later, Microsoft refactors the relay handshake (or the auth token format, or adds a new required field). Your vendored copy stops working against the production service. There is no semver guarantee on an unpublished SDK.

**Why it happens:**
- Unpublished SDKs are not part of Microsoft's public API surface. Breaking changes are normal.
- The first-party SDKs (C#/TS) are versioned and released; Rust is not.

**How to avoid:**
- Treat Dev Tunnels as a **separate crate inside your workspace** with a tightly bounded API surface (`connect`, `disconnect`, `port_forward`, `status`).
- Have a smoke test that runs against the live service in CI nightly. Failure ≠ block release; it = "investigate immediately."
- Keep a "fall back to subprocessed `devtunnel` CLI" implementation behind a feature flag. If the native Rust path breaks, flip the flag.

**Warning signs:**
- A `cargo update` changes the vendored SHA and you don't know why.
- A nightly smoke test fails with a deserialize error.

**Phase to address:**
Phase 4 (Dev Tunnels integration), with the contingency plan documented before the first commit on that phase.

---

### Pitfall 14: OAuth token storage — Keychain vs. file

**What goes wrong:**
You cache GitHub OAuth tokens in `~/.config/vector/auth.json` for simplicity. Tokens persist with mode 0644 by accident. Or: malware on the Mac reads the file. Or: you get `tokens` mixed with rendering state and accidentally log them via `tracing`.

**Why it happens:**
Persisted plaintext credentials on a multi-user/shared dev box are a known credential-theft vector. macOS Keychain is the right primitive, but it requires an Apple Developer entitlement to share a Keychain item between unsigned and signed builds, which you don't have in v1.

**How to avoid:**
- Store OAuth tokens in the macOS Keychain via `security-framework` or `keyring` crate. It works on unsigned apps for *application-scoped* items (the unsigned-app caveat applies to *Keychain access groups*, not basic Keychain items).
- File-mode 0600 on any fallback file. Verify in tests.
- Never `Debug`-derive on token-bearing structs. Implement `Debug` manually to redact.
- Refresh-token logic: scope it tight. A device-code flow that refreshes silently is one token-leak away from a credential-theft post-mortem.

**Warning signs:**
- Token strings appear in panic backtraces or `tracing` output.
- Auth file is world-readable.
- Two users on the same Mac share a token by accident.

**Phase to address:**
Phase 2 (auth), with a security review checkbox before Phase 3 (Codespaces integration).

---

### Pitfall 15: SSH key trust on first connect ("TOFU bypass")

**What goes wrong:**
For Codespaces and Dev Tunnels, the SSH host key is provided dynamically by the API. Naïve implementation: disable host-key checking ("we trust GitHub's OAuth flow"). This is *correct in principle* (the OAuth-authenticated tunnel is the security boundary, not SSH host keys) but invites subtle bugs (you pin the wrong fingerprint, or you log "host key changed" warnings in user-visible UI for every reconnect).

**Why it happens:**
- The Codespaces tunnel proxy returns SSH host keys via API alongside connection metadata.
- Users (and reviewers) see "host-key checking disabled" and assume a security flaw.

**How to avoid:**
- Use the host-key fingerprint provided by the API as a per-connection accept-list. Implement `ServerCheckMethod::PublicKey(...)` in `russh` with that exact fingerprint, not "disabled."
- Document explicitly in the README why this isn't a TOFU violation (the trust root is GitHub OAuth + tunnel encryption, not SSH host keys).
- Don't show "WARNING: host key has changed!" UI during normal reconnects.

**Phase to address:**
Phase 3 (Codespaces) and Phase 4 (Dev Tunnels).

---

## Scope Creep Traps (read this twice)

Each of these has wrecked terminal-emulator side projects. Each one looks small.

### Pitfall 16: "We need IME for completeness"

**What goes wrong:**
True IME (Input Method Editor) support — for Japanese/Chinese/Korean composition with marked text, candidate windows, and inline previews — requires deep AppKit integration (`NSTextInputClient`, `setMarkedText:selectedRange:replacementRange:`, candidate-window placement coordinated with cursor cell). It is multiple weeks of work, full of edge cases (active composition during scroll, undo, pane focus changes), and used by ~1% of the target audience for an internal Adobe tool.

**How to avoid:**
- **Defer to v2 unconditionally.** Document that IME is not supported in v1 in the README.
- If you need basic CJK *display* (without IME composition), that comes free from Pitfall 2 (font fallback). Don't conflate them.

**Phase to address:**
Out of scope for v1.

---

### Pitfall 17: Sixel / Kitty graphics protocol

**What goes wrong:**
Adding image-display protocols seems like "just decode base64 and blit a texture." Then you discover: positioning relative to text cells (and what happens on scroll), animation frames, transparency compositing, OSC 1337 (iTerm protocol) vs. Sixel vs. Kitty's modern protocol — three different specs with overlapping but incompatible feature sets — and the fact that *every* graphics image needs its own GPU texture lifecycle separate from your glyph atlas.

**How to avoid:**
- **No image protocols in v1.** None. Not even Sixel.
- If `imgcat` / `chafa` users complain, point them at a v2 issue.

**Phase to address:**
Out of scope for v1. Track in a v2 wishlist.

---

### Pitfall 18: Built-in extension/plugin system

**What goes wrong:**
A plugin system implies a stable API, isolation (process / WASM / Lua sandbox), permissions, distribution (registry?), versioning, and security review. None of which are on this project's critical path.

**How to avoid:**
- **No plugins. Period.** Configurable colors and keybinds are not plugins.
- Resist any framing that includes "and users could write…"

**Phase to address:**
Out of scope, forever-or-until-v2.

---

### Pitfall 19: Web-based settings UI

**What goes wrong:**
"A nice settings panel" → embed a webview → suddenly you're shipping WebKit, JavaScript, and a built-in HTTP localhost server. iTerm2 has a real settings UI built in AppKit; that's the standard, and it took years.

**How to avoid:**
- **TOML file in `$XDG_CONFIG_HOME/vector/config.toml`. Open in `$EDITOR` on a menu item.** Done.
- No webview. No `wry`. No embedded browser.
- The user explicitly rejected this in PROJECT.md.

**Phase to address:**
Out of scope. Keep saying no.

---

### Pitfall 20: File browser / sidebar / IDE features

**What goes wrong:**
"Just a tree-view sidebar for files in the remote workspace" → sftp client → file watching → context menus → and now you're VS Code without the editor.

**How to avoid:**
- **The terminal is the UI.** `ls`, `tree`, `nvim` are the file browser.
- Out of scope is out of scope.

**Phase to address:**
Out of scope.

---

### Pitfall 21: "Vim-style modal pane navigation" or built-in multiplexing exceeding tmux

**What goes wrong:**
You add splits and tabs (in scope). Then someone wants "leader-key motion between panes." Then "save/restore window layouts." Then "send keystrokes to all panes." Each one looks small. Each one has tmux semantics to fight with. You end up reimplementing tmux poorly.

**How to avoid:**
- Splits + tabs only. No layout save/restore (rely on profile = "open this remote, run this command").
- No "broadcast input to all panes." Use `tmux setw synchronize-panes on` if needed.
- Honor the user's tmux. Coexist; don't compete.

**Phase to address:**
Phase 2 (tabs/splits) — set boundaries early.

---

### Pitfall 22: Persistent-session reimplementation creep ("our own mosh")

**What goes wrong:**
"WiFi drops shouldn't lose state" sounds simple. Then you discover that doing it *correctly* means tracking terminal grid state on the client, replaying it on reconnect, and possibly running a server-side agent to buffer output during disconnect — i.e., you're building Mosh.

**How to avoid:**
- **Cheap-but-good-enough strategy: encourage `tmux` on the remote.** A profile auto-runs `tmux new -A -s vector` on connect; reconnect just reattaches. Document this as the resilience story.
- For non-tmux remotes, use `autossh`-style reconnect with exponential backoff. On reconnect, re-issue `clear; tput reset` and let the remote shell redraw via `$PROMPT_COMMAND` / fish `fish_prompt`.
- Display a clear "Reconnecting…" UI overlay during disconnects (this is genuine value).
- **Do not implement a state-sync protocol.** That is Mosh territory. zmx/libghostty-vt show this is feasible but each is a project unto itself.

**Phase to address:**
Phase 3 (remote reconnect) — set the "tmux is the answer" position before writing reconnect code.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Subprocess `gh codespace ssh --stdio` instead of native client | Working day-1 SSH; defers protocol work | `gh` must be installed; UX feels less integrated | **Acceptable for v1.** Replace in v2 if friction warrants. |
| Subprocess `devtunnel` CLI for Dev Tunnels | Same as above; bypasses the unpublished Rust SDK | Same as above | **Acceptable for v1, possibly forever.** |
| Vendor `microsoft/dev-tunnels/rs/` at a pinned SHA | Native Rust path without subprocess | Breakage on protocol bumps; manual refresh | Acceptable if subprocess approach blocked by UX requirements. |
| Use `alacritty_terminal` rather than rolling parser | Skip 6+ weeks of escape-sequence work | Inherits Alacritty's opinions on grid/scrollback | **Always acceptable.** This is the right call. |
| Single TOML config, no hot-reload | Half the surface area | Power-user feature requests | **Always acceptable for v1.** |
| Unsigned DMG with right-click-Open | $99/yr saved; CI simpler | Teammate friction; more support; future Sequoia versions may break | Acceptable for internal tool. Revisit if external users grow. |
| Skip notarization | No Apple Developer enrollment | Future macOS may require it; users distrust unsigned | Acceptable for v1; plan v2 budget. |
| `String::from_utf8_lossy` on PTY chunks | One-line implementation | CJK corruption (Pitfall 4) | **Never.** Use parser-level bytes. |
| Roll own SSH client | "We need finer control" | russh/thrussh maintained by experts; rolling your own is multi-month | **Never.** |
| Copy-paste WezTerm code | Fast prototyping | License (Apache 2 — OK), but you inherit unmaintained code | Acceptable as reference, never as vendored code, unless you commit to maintaining it. |
| Skip tests for terminal grid | "It's visually obvious" | Regressions are not visually obvious; bracketed-paste, scroll regions, tab stops break silently | **Never.** Ship a vt-test fixture suite from Phase 1. |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| GitHub OAuth | Use `gh`'s cached token directly | Implement own device-code flow; `gh`'s token has different scopes and may expire silently |
| Codespaces API | `GET /user/codespaces/{name}` to fetch connection details | Add `?internal=true&refresh=true` query params or `tunnelProperties` is missing |
| Codespaces start | Treat 409 Conflict as error | 409 means "already running" — swallow it |
| Codespaces start | One-shot start request | Poll `state` for up to 2 min at 1s intervals; transition through Provisioning/Starting → Available |
| SSH transport | Disable host-key check | Use the API-provided fingerprint as an explicit accept-list |
| SSH `pty-req` | Forget to send window size on connect | Send initial cols/rows; re-send on resize via `window-change` |
| OSC 52 clipboard | Send through tmux without passthrough | Document `set -g allow-passthrough on`; expect ~60-char limit; chunk if larger |
| `winit` on macOS | Run async runtime on main thread | Main thread is `winit`-only; tokio gets its own thread |
| `wgpu` present | `Mailbox` present mode | `Fifo` (vsync) on macOS; `Mailbox` is the wrong default for a battery device |
| `objc2` AppKit calls | Call from worker thread | Always main thread; use `EventLoopProxy` to dispatch |
| MTKView ↔ wgpu | Try to use both | Use `wgpu` directly with a `CAMetalLayer`-backed surface from `winit`; do not mix MTKView |
| Universal binary | `cargo build --release` on Apple Silicon | Build both `x86_64-apple-darwin` and `aarch64-apple-darwin`, then `lipo -create` |
| MSRV | Bump willy-nilly | Pin in `Cargo.toml` `rust-version`; check CI matrix tests it |
| Quarantine attr | Assume DMG-mounted apps inherit user trust | They inherit `com.apple.quarantine`; document `xattr -dr` |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Re-rasterize glyphs every frame | Fans spin under `cat`; >50% GPU on idle | Cache rasterized glyphs in a bounded atlas | At first non-trivial use |
| Render on every PTY byte instead of vsync-synced | Same as above | `request_redraw` on dirty; render once per vsync | Immediately |
| Allocating in the render hot loop | Frame time variance, GC pauses (yes, even in Rust — `Vec` reallocation pauses) | Reuse vertex buffers; profile with `cargo flamegraph` | Under heavy scrollback |
| Rebuilding the entire grid on resize | Resize judders | Re-layout only the wrapping; preserve unwrapped logical lines | At 100k+ scrollback lines |
| Synchronous PTY reads on UI thread | UI hangs during burst output | All PTY I/O on a worker thread, batched into UI via channel | At any sustained output |
| Unbounded scrollback Vec | Memory grows without bound | Ring buffer with `--scrollback-lines` cap (default 10k) | Long-running shells, log streaming |
| Per-cell `String` allocations | Each `vte::Perform::print` allocates | Store `[u8; 4]` (UTF-8 max) per cell, not `String` | Always — measure |
| Bilinear-filtered text | Blurry text on non-integer scale | `Nearest` filter for monochrome SDF text; integer pixel snapping | Retina at fractional scale |
| Re-creating wgpu pipelines on theme change | 100ms freeze on theme reload | Theme = uniform buffer update, not pipeline rebuild | Live-reload theme features |
| Tokio `block_on` on main thread for any reason | Deadlocks UI | Never; use `EventLoopProxy::send_event` | First time you forget |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Logging OAuth tokens | Credential exfiltration via crash report or `tracing` | Manual `Debug` impls that redact; review every `log::*` / `tracing::*` call near auth code |
| Plaintext token file with default permissions | Local-attacker token theft | macOS Keychain via `keyring` crate; fallback file `chmod 0600` |
| Using user-content as escape sequences without sanitization | Malicious file/log content abusing OSC 8 hyperlinks, OSC 52 clipboard, window-title (CVE-class issues found in shipping terminals in 2023) | Implement OSC 52 paste only with explicit user opt-in; sanitize OSC 8 destinations to `https://`/`mailto:` only; cap window-title length |
| Wide-open OSC 52 paste | Remote process can write to local clipboard silently | Off by default, or prompt-on-first-use, or rate-limit |
| Disabled SSH host-key check (no fingerprint pinning) | MitM if tunnel is compromised | Pin to fingerprint returned by Codespaces/Tunnels API |
| GitHub PAT scopes too broad | Token leak gives more access than needed | Request minimum scopes (`codespace`, `read:user`, `user:email`); never `repo` write unless user opts in |
| Token in process arg list | Visible via `ps`/`/proc` | Pass via env var or stdin to subprocesses |
| Trust ANSI color escapes from user files | "evil cat" attacks (rare but real) | Strip C1 (0x80–0x9F) controls from `cat`-style display unless user opts in |
| Loading themes from arbitrary URLs | Code execution if theme format ever supports logic | Theme = pure data only (no code); load from local files only |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| First-launch shows "configure GitHub" wall | User can't poke around to evaluate the terminal | Open a local shell by default; auth is a menu action, not a gate |
| "Damaged" gatekeeper popup with no recovery hint | Teammate gives up | DMG includes a `INSTALL.md` (or `.command` script) with explicit `xattr` instructions, and the launch failure path opens a help URL |
| Codespace in "Stopped" state errors out | User has to switch to browser to start it | Auto-detect, prompt "Start codespace?", call `POST /start`, show progress |
| Resize judders when remote is `tmux` | Visual flicker on every drag | Coalesce resizes (only re-emit on drag-end + on size delta > N cells), debounce 100ms |
| Scrollback persists across `clear` | User confused why `clear` didn't | Match xterm: `clear` calls `ED 2` which leaves scrollback; `reset` clears scrollback. Document. |
| No "copy on select" or no clear copy mechanism | "I just want to copy text" rage | Cmd-C copies selection; selection auto-clears on focus loss; document |
| Bell behavior surprises (audio, dock-bounce, none) | Either annoying or silent-broken | Per-profile bell setting, default = visual flash + dock badge, no audio |
| Wrong cursor shape vs. shell expectation | `vim` insert mode shows block instead of bar | Honor DECSCUSR (shape) and DECSET 12 (blink); test in `vim`/`nvim` |
| Slow drag-to-select on large output | Frustrating | Selection is logical (line+col), not pixel-bound; recompute on viewport scroll only |

## "Looks Done But Isn't" Checklist

These are the things every new terminal claims to support but ships broken on first commit. Verify each one with a *specific test*:

- [ ] **Bracketed paste:** paste a multi-line block into `bash -c 'IFS= read -r x; echo got: $x'`; verify newlines arrive escaped and `read` does not execute lines.
- [ ] **Cursor shape (DECSCUSR):** in `nvim`, switch insert mode; cursor must change to bar (or underscore) and back.
- [ ] **DECSET 1049 alternate screen:** run `vim`, quit; primary screen contents must be exactly what you saw before launching `vim`.
- [ ] **Soft reset (DECSTR):** after `tput reset`, `printf '\e[!p'` must restore default modes without clearing scrollback.
- [ ] **Scroll regions (DECSTBM):** `tmux` status bar must stay anchored at the bottom while pane content scrolls.
- [ ] **Tab stops:** `tput hts` after `printf '\t'` to set, `printf '\t\t\t'` lands on each.
- [ ] **ED/EL erase:** `clear` (`ED 2`) must NOT erase scrollback; `reset` (`ED 3` + RIS) must.
- [ ] **Mouse modes 1006/1015:** `htop` mouse selection works; coordinates beyond column 223 work (1006 SGR mode).
- [ ] **OSC 52 paste:** `printf '\e]52;c;%s\a' "$(echo hello | base64)"` puts "hello" in the macOS clipboard.
- [ ] **OSC 8 hyperlinks:** `printf '\e]8;;https://example.com\e\\example\e]8;;\e\\\n'` renders a clickable link.
- [ ] **Bracketed paste mode (2004):** confirm `\e[200~` … `\e[201~` wrapping appears in shell input.
- [ ] **UTF-8 across read boundaries:** a 1MB file of CJK + emoji `cat`'d shows zero U+FFFD.
- [ ] **Resize propagates over SSH:** resize while remote `vim` open, the buffer reflows.
- [ ] **`tmux` true-color (Tc capability):** `:set termguicolors` in remote nvim shows real colors.
- [ ] **Codespace in Stopped state:** UI offers to start it, polls until Available, then connects.
- [ ] **Reconnect under WiFi drop:** disconnect WiFi for 30s, reconnect — terminal session resumes (via tmux on remote).
- [ ] **Universal binary:** `lipo -info Vector.app/Contents/MacOS/vector` shows both architectures.
- [ ] **Quarantine remediation:** README has the `xattr -dr com.apple.quarantine` command on the install page.
- [ ] **Low-power mode:** plug machine in, run `cat large.log`; unplug, low-power kicks in, frame rate drops gracefully (not visibly broken).
- [ ] **No token leaks:** `tracing`/log dump from a session with auth must not contain any token-shaped string (grep for `gho_`, `ghp_`, `eyJ`).

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Built own VT parser, found out it's broken | HIGH (weeks) | Swap in `alacritty_terminal`; accept some test churn; existing rendering can stay |
| Glyph atlas churns | MEDIUM | Switch to `cosmic-text`; bound the atlas; profile before/after |
| `winit` + `tokio` deadlock | MEDIUM | Refactor entry to `winit::EventLoop` on main, dedicated tokio thread with `EventLoopProxy` |
| Dev Tunnels native client breaks | LOW with prep | Flip a feature flag to subprocess `devtunnel` CLI fallback (Pitfall 13) |
| Codespaces SSH protocol changes | LOW with prep | Same: subprocess `gh codespace ssh --stdio` fallback |
| Config schema design wrong | MEDIUM | Single TOML, semver the config keys, ship a migrator (small Rust function); never ship YAML→TOML migration nightmare like Alacritty did |
| Universal binary missing one arch | LOW | Re-run CI with explicit `--target` for both, `lipo` |
| Token leaked in logs | HIGH (security incident) | Rotate all PATs, document in security advisory, review every log site |
| User-reported "damaged" DMG | LOW | Ship `INSTALL.md`; consider a $99/yr Apple cert |
| Scope creep already happened | MEDIUM | Audit Active requirements vs. Out of Scope monthly; revert or branch features that crept in |

## Pitfall-to-Phase Mapping

Assumes a phase structure roughly: P0 skeleton → P1 local terminal core → P2 tabs/splits/profiles/auth → P3 Codespaces → P4 Dev Tunnels → P5 packaging/release.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 1. Roll-own parser | P0 (decision) / P1 | Cargo deps include `alacritty_terminal` or `vte`; no hand-written escape state machine |
| 2. Glyph atlas / fallback | P1 | CJK + emoji + ligature smoke test passes; bounded atlas verified by metrics |
| 3. Frame pacing | P1 | `wgpu::PresentMode::Fifo`; idle terminal <1% GPU; ProMotion 120Hz works |
| 4. Partial UTF-8 reads | P1 | 1MB CJK+emoji file `cat` test shows no U+FFFD |
| 5. winit/tokio threading | P0 | No `tokio::main`; main-thread audit doc in repo; CI runs on macOS |
| 6. Gatekeeper | P5 | INSTALL doc with `xattr` step; CI artifact installed cleanly on a clean Mac VM |
| 7. PTY signals/resize | P1 + P3 | Resize test under local & SSH; close-tab leaves no zombies |
| 8. tmux passthrough | P1 + P3 | Nested tmux smoke test; OSC 52 round-trip through tmux works |
| 9. Codespaces SSH non-trivial | P3 | First commit subprocesses `gh ... --stdio`; later commit is native |
| 10. Dev Tunnels SDK risk | P4 — go/no-go on day 1 of phase | Decision document committed before any DT code |
| 11. Config sprawl | P2 | TOML schema with `deny_unknown_fields`; no DSL dependency in `Cargo.toml` |
| 12. Compile-time bloat | P0 + P5 audit | `tokio` features explicit; clean build under target threshold; `cargo tree` reviewed |
| 13. Dev Tunnels SDK churn | P4 | Vendored crate has pinned SHA; nightly smoke test exists |
| 14. OAuth token storage | P2 | Token in macOS Keychain; `Debug` redaction tested; file fallback chmod 0600 |
| 15. SSH host-key trust | P3 + P4 | Fingerprint pinning verified by integration test |
| 16. IME | Out of scope (v1) | README states "no IME in v1" |
| 17. Sixel/Kitty graphics | Out of scope (v1) | Same |
| 18. Plugins | Out of scope (forever-ish) | Same |
| 19. Web settings UI | Out of scope (forever) | No `wry`/webview deps in `Cargo.toml` |
| 20. File browser/IDE features | Out of scope (v1) | Same |
| 21. Pane navigation creep | P2 | Splits + tabs only; no layout save; no broadcast input |
| 22. Persistent session reimplementation | P3 | Reconnect docs say "use tmux on remote"; no state-sync protocol code |

---

## Dev Tunnels Contingency Plan (explicit, per request)

**Risk model:** The Microsoft Dev Tunnels Rust SDK exists in `microsoft/dev-tunnels/rs/` but is not published to crates.io, has lower visibility than C#/TS/Go, and may not be maintained at the same cadence as the protocol. There is no public, stable wire-format spec. Reverse-engineering is multi-week effort.

**Decision tree (apply day 1 of Phase 4):**

```
Q: Can we subprocess `devtunnel` CLI (or `code tunnel client`) and pipe SSH on top?
├── YES: Ship that. v1 native UX is "Vector launches devtunnel for you, then connects."
│        Cost: extra binary dependency. Benefit: zero protocol risk.
│
└── NO (subprocess is somehow blocked):
    Q: Does the in-repo `rs/` SDK build cleanly and connect to the live service?
    ├── YES: Vendor it as `path = "vendor/dev-tunnels-rs"`. Pin the SHA.
    │        Add nightly smoke test against the live service.
    │        Hide native path behind a feature flag; default = subprocess fallback.
    │
    └── NO: Defer Dev Tunnels to v2. Ship Codespaces-only v1.
             Document the deferral in PROJECT.md. This is acceptable.
```

**Soft red lines (do NOT cross):**
- Do not commit to writing a clean-room Dev Tunnels relay/auth implementation in v1.
- Do not block v1 release on Dev Tunnels. Codespaces-only is a complete product.
- Do not ship a Dev Tunnels integration without a smoke test against the live service.

**The v1 trade-off, plainly:** Codespaces is the user's primary use case (managed dev VMs at Adobe). Dev Tunnels is the "connect to my own remote box" extension. Cutting Dev Tunnels saves 2–4 weeks of risky integration work and removes the single largest unknown. If Dev Tunnels matters by ship-date, the subprocess approach is the right answer; otherwise defer.

---

## Sources

- [microsoft/dev-tunnels (Dev Tunnels SDK repo)](https://github.com/microsoft/dev-tunnels) — confirms `rs/` directory exists; multi-language SDK
- [microsoft/dev-tunnels-ssh](https://github.com/microsoft/dev-tunnels-ssh) — SSH library; C# and TS only, no Rust
- [Dev Tunnels security docs](https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/security) — auth model, GitHub OAuth flow
- [VS Code Remote Tunnels](https://code.visualstudio.com/docs/remote/tunnels) — describes the user-facing flow and SSH-over-tunnel architecture
- [Diving into Microsoft's dev tunnels (InfoWorld)](https://www.infoworld.com/article/2336324/diving-into-microsofts-dev-tunnels.html) — overview
- [cli/cli pkg/cmd/codespace/ssh.go](https://github.com/cli/cli/blob/trunk/pkg/cmd/codespace/ssh.go) — Codespaces SSH command implementation
- [cli/cli internal/codespaces/ssh.go (Fossies mirror)](https://fossies.org/linux/gh-cli/internal/codespaces/ssh.go) — the Shell/Copy/NewRemoteCommand reference
- [`gh cs ssh --stdio` documentation issue (cli/cli#8368)](https://github.com/cli/cli/issues/8368) — confirms hidden `--stdio` flag for ProxyCommand use
- [`gh cs ssh` and fine-grained PAT (cli/cli#7819)](https://github.com/cli/cli/issues/7819) — FGPATs do not work; use classic PAT scopes
- [SSH-over-WebSocket relay protocol (Chromium nassh docs)](https://chromium.googlesource.com/apps/libapps/+/master/nassh/doc/relay-protocol.md) — describes the underlying relay model GitHub uses
- [GitHub Codespaces with GitHub CLI docs](https://docs.github.com/en/codespaces/developing-in-a-codespace/using-github-codespaces-with-github-cli) — public flow
- [VT100.net DEC ANSI parser](https://vt100.net/emu/dec_ansi_parser) — Paul Williams state machine reference
- [alacritty/vte](https://github.com/alacritty/vte) — Rust VT parser
- ["ANSI Terminal security in 2023"](https://dgl.cx/2023/09/ansi-terminal-security) — 10 CVEs found in shipping terminals (escape-sequence sanitization)
- [Don't Trust This Title (CyberArk)](https://www.cyberark.com/resources/threat-research-blog/dont-trust-this-title-abusing-terminal-emulators-with-ansi-escape-characters) — escape-injection attacks
- [tmux passthrough docs (`allow-passthrough`)](https://tmuxai.dev/tmux-allow-passthrough/) — tmux passthrough behavior
- [tmux passthrough cut-off bug (#4377)](https://github.com/tmux/tmux/issues/4377) — ~60-char passthrough truncation
- [portable-pty docs](https://docs.rs/portable-pty/latest/portable_pty/) — Rust PTY library
- [winit + tokio integration discussion (tokio#2953)](https://github.com/tokio-rs/tokio/discussions/2953) — main-thread / runtime split
- [winit macOS main-thread requirement (winit#1199, emacs-ng#366)](https://github.com/rust-windowing/winit/issues/1199) — EventLoop must be on main thread on macOS
- [winit macOS reentrancy panic (winit#3992)](https://github.com/rust-windowing/winit/issues/3992) — borrowed-while-borrowed event handling
- [Apple CADisplayLink docs](https://developer.apple.com/documentation/quartzcore/cadisplaylink) — frame pacing primitive
- [Apple Metal smooth frame rate sample](https://developer.apple.com/documentation/metal/metal_sample_code_library/achieving_smooth_frame_rates_with_metal_s_display_link) — CAMetalDisplayLink usage
- [iTerm2 issue: low-power mode disables Metal (#10671)](https://gitlab.com/gnachman/iterm2/-/issues/10671) — low-power mode behavior
- [Are We Sixel Yet?](https://www.arewesixelyet.com/) — terminal graphics protocol landscape
- [Kitty graphics protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/) — modern alternative to Sixel
- [Mosh + tmux comparison (hoop.dev)](https://hoop.dev/blog/mosh-and-tmux-uninterrupted-remote-terminal-sessions) — persistence approaches
- [zmx (libghostty-vt session persistence)](https://lobste.rs/s/fvdh2d/zmx_session_persistence_for_terminal) — state-sync alternative
- [strip-quarantine macOS quick action](https://github.com/systemsoftware/strip-quarantine) — `xattr -d com.apple.quarantine` flow
- [Homebrew quarantine issue (#17979)](https://github.com/Homebrew/brew/issues/17979) — illustrates user-facing friction with unsigned macOS apps

---
*Pitfalls research for: Vector — native macOS Rust GPU terminal with Codespaces + Dev Tunnels client*
*Researched: 2026-05-10*
