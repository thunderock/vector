# Feature Research

**Domain:** Native macOS GPU terminal with first-class GitHub Codespaces and Microsoft Dev Tunnels client
**Researched:** 2026-05-10
**Confidence:** HIGH for terminal-core / table-stakes (well-documented across ghostty, WezTerm, Alacritty, kitty); MEDIUM for Codespaces/Dev-Tunnels protocol specifics (public APIs but the SSH-over-port-forwarding and Dev-Tunnels relay protocols require reading `cli/cli` and `microsoft/dev-tunnels` source); LOW only for Claude-AI autosuggest UX bar (no direct prior art outside Warp).

---

## Feature Landscape

### Table Stakes (Users Expect These)

Missing any of these and the product fails the "is this even a terminal?" smell test for a 2026 daily-driver replacing iTerm/ghostty.

#### 1. Terminal Core (VT / xterm parity)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| xterm-compatible VT parser (CSI/OSC/DCS/SGR) | Anything less and shells, vim, htop break | M | Use `vte` crate or vendor `alacritty_terminal` — both already battle-tested |
| 256-color + 24-bit truecolor | Themes, syntax highlighting, modern TUI apps | S | Free with `alacritty_terminal` |
| Unicode + emoji + grapheme clustering | Non-ASCII users, modern emoji prompts | M | Use `unicode-width` + `unicode-segmentation`; emoji width is the foot-gun |
| East Asian width + CJK ambiguous-width handling | Adobe has APAC users; users typing CJK file paths | M | UAX #11; configurable "ambiguous = wide" option per iTerm/kitty convention |
| Scrollback buffer (configurable, ≥10k lines) | Users expect to scroll up and read prior output | S | Ring buffer; Alacritty default 10k is the floor |
| Scrollback search (regex, incremental) | iTerm/WezTerm/ghostty all have this; missing = "broken" | M | Highlight matches, jump between, copy match |
| Bracketed paste (mode 2004) | Prevents accidental command execution from clipboard newlines | S | Default-on; let shell decide |
| Mouse modes (1000, 1002, 1003, 1006 SGR) | tmux/vim/lazygit need this | S | Standard; SGR (1006) is the modern variant |
| OSC 7 — current working directory reporting | "New tab here" / "duplicate session" UX needs cwd | S | Trivial to read; shell integration script must emit it |
| OSC 8 — clickable hyperlinks | Compiler errors, `gh`, `cargo` all emit them | S | Render underline + Cmd-click; trivial parser change |
| OSC 52 — clipboard set/get from inside terminal | SSH/tmux clipboard sync; vim users live on this | S | But: see tmux DCS-passthrough gotcha below — accept both raw and DCS-wrapped |
| OSC 133 — semantic prompt marking (A/B/C/D) | Foundation for "jump prev prompt", command timing, future blocks | S | Accept; act on it later — at minimum don't choke on it |
| OSC 9;4 — progress reporting | Modern installers, tests; ghostty 1.2 added it | S | Optional taskbar/tab indicator |
| OSC 10/11/12 — fg/bg/cursor color queries | vim/neovim use these to detect dark mode | S | Standard |
| Cursor shape (DECSCUSR) | vim/zsh insert-vs-normal mode indicators | S | Trivial — but watch tmux passthrough |
| Alternate screen buffer (DECSET 1049) | vim, less, htop need it; without it scrollback gets trashed | S | Standard |
| Bell (audible + visual) | Some users still want it; `\a` should at least not hang | S | Mac NSBeep + optional flash |
| Input Method Editor (IME) for CJK/emoji on macOS | Anyone typing Japanese/Chinese commits/file paths | M-L | NSTextInputClient — non-trivial but mandatory; candidate window must follow cursor |
| Bracketed-paste safe paste (multiline detection w/ confirmation) | iTerm-style "do you really want to paste 50 lines with newlines?" | S | Optional but expected since 2018 |

**Confidence:** HIGH. These are documented standards (ECMA-48, xterm ctlseqs, Unicode UAX #11) and every reference (ghostty, WezTerm, Alacritty, kitty) implements them.

Sources: [ghostty OSC commands](https://deepwiki.com/ghostty-org/ghostty/3.4-osc-commands-and-protocols), [iTerm2 feature reporting spec](https://iterm2.com/feature-reporting/), [State of the Terminal](https://gpanders.com/blog/state-of-the-terminal/), [Terminal Compatibility Matrix](https://tmuxai.dev/terminal-compatibility/), [Contour IME demo](https://contour-terminal.org/demo/ime/).

#### 2. GPU Rendering & Performance

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| GPU-accelerated glyph atlas (Metal via wgpu) | The whole point of going Rust/wgpu vs Electron | L | Reference: Alacritty / WezTerm renderer arch |
| Sub-cell antialiasing on Mac | macOS users have Retina expectations | M | CoreText subpixel; Mac HiDPI default |
| Sustained 60fps under heavy output (`yes`, large `cat`) | Alacritty set this bar in 2017 | M | Damage-tracking + frame-coalescing |
| ProMotion / 120Hz support on Mac | New MacBook Pros run at 120Hz | S | Honor `CADisplayLink`'s preferred rate |
| Damage tracking / dirty-region rendering | Don't redraw the whole grid every frame | M | Common pattern in alacritty_terminal |

**Confidence:** HIGH. Alacritty/ghostty/WezTerm all set the bar.

Sources: [Modern Terminal Emulators 2026 comparison](https://calmops.com/tools/modern-terminal-emulators-2026-ghostty-wezterm-alacritty/), [Choosing a Terminal on macOS 2025](https://medium.com/@dynamicy/choosing-a-terminal-on-macos-2025-iterm2-vs-ghostty-vs-wezterm-vs-kitty-vs-alacritty-d6a5e42fd8b3).

#### 3. Window UX

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Tabs (Cmd-T new, Cmd-Shift-[/] navigate, Cmd-W close) | Universal Mac terminal expectation | M | AppKit `NSWindowTabbing` or custom; ghostty does custom |
| Splits — horizontal and vertical | iTerm/WezTerm/ghostty/tmux baseline | M | Tree of panes; one focused; resize via drag + keybind |
| Multi-window | Cmd-N for separate workspace | S | Free if tabs/splits done right |
| Native macOS window chrome | Traffic lights, vibrancy, fullscreen, tabbing | M | AppKit via `objc2`; titlebar styling |
| Native fullscreen + Spaces support | Cmd-Ctrl-F | S | Free with AppKit |
| Hot-reload config (signal- or fs-watch-driven) | ghostty does USR2 reload; iTerm reloads on save | S | Use `notify` crate; debounce; SIGHUP/USR2 also |
| Secure Keyboard Entry toggle | iTerm has it; protects passwords from keyloggers | S | macOS `EnableSecureEventInput()` API |
| Command palette (fuzzy command picker) | ghostty 1.2 added it; modern Mac apps have it | M | Cmd-Shift-P; lists actions, profiles |
| Quick-terminal / hotkey window | iTerm's drop-down; ghostty's quick terminal | M | Optional but expected by power users |
| Theme switching at runtime (light/dark following macOS) | macOS auto dark mode; users expect terminal to follow | S | Subscribe to `NSApp.effectiveAppearance` |

**Confidence:** HIGH.

Sources: [WezTerm splits](https://wezterm.org/config/lua/pane/split.html), [Ghostty 1.2 release notes](https://www.omgubuntu.co.uk/2025/10/ghostty-1-2-new-features-for-linux), [iTerm2 menu items](https://iterm2.com/documentation/2.1/documentation-menu-items.html), [Apple Secure Keyboard Entry guide](https://support.apple.com/guide/terminal/use-secure-keyboard-entry-trml109/mac).

#### 4. Themes / Fonts / Ligatures

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Importable iTerm `.itermcolors` themes | 700+ existing themes; users have favorites | S | Plist parser; map XML keys to RGB |
| Bring-your-own-font (TTF/OTF/Variable) | Users have JetBrains Mono/FiraCode/Monaspace | S | CoreText handles loading |
| Nerd Font glyph rendering | starship/powerlevel10k/p10k prompts depend on PUA glyphs | S | Just don't break them — accept fallback fonts |
| Programming ligatures (`==`, `=>`, `!=`) | FiraCode/JetBrains Mono users; configurable on/off | M | HarfBuzz shaping; opt-in (some users hate them) |
| Variable font axes (weight/width sliders) | Monaspace/Recursive users | M | CoreText supports variations natively |
| Multiple font fallback (emoji, CJK, math) | Apple Color Emoji, Hiragino, etc. | M | Font fallback chain; emoji presentation selectors (FE0F) |
| Per-profile font/size/theme | "Codespaces sessions get a tinted background" | S | Profile model includes appearance |

**Confidence:** HIGH.

Sources: [iTerm2-Color-Schemes (mbadolato)](https://github.com/mbadolato/iterm2-color-schemes), [WezTerm appearance config](https://wezterm.org/config/appearance.html), [Nerd Fonts](https://www.nerdfonts.com/).

---

### Differentiators (Competitive Advantage)

Where Vector earns its existence vs iTerm/ghostty/Warp/Wave/Tabby.

#### 5. Codespaces-First Connection Flow

The headline differentiator. Every step below is what makes the product feel like "a Codespaces terminal" instead of "a terminal that you can also use to SSH into a Codespace if you set up `gh` first."

**User journey (target):**

1. **First run — sign in**
   - Welcome screen, single button: "Sign in with GitHub"
   - Use **OAuth Device Flow** (no redirect URL needed, ideal for desktop apps; GitHub recommends it for "constrained environments" which includes desktop apps without a registered redirect host).
   - Show user-code (e.g., `ABCD-1234`) + open `https://github.com/login/device` in default browser.
   - Poll for token; show progress.
   - Required scopes: `codespace`, `read:user`, plus enough to list repos. Cache scope set.
   - Store token in **macOS Keychain** (via `security-framework` crate or `keyring` crate).
   - On token expiry / 401 from GitHub, transparently re-run device flow (don't kick the user out mid-session).

2. **Picker — Codespaces list**
   - Pull from `GET /user/codespaces` (REST API).
   - Render: name, repo (`owner/repo`), branch, machine type, **state** (`Available`, `Shutdown`, `Starting`, `Provisioning`, `Unavailable`), last-used timestamp, region.
   - Sort: most-recently-used first; group by state.
   - Empty state: "No Codespaces yet — open `gh codespace create` or github.com/codespaces" (lifecycle is out of scope for v1, per PROJECT.md).
   - For `Shutdown` Codespaces: clicking starts them via `POST /user/codespaces/{name}/start`, then polls until `Available`. Show progress.
   - Latency hint: optional ping to the codespace's region (cheap, async, don't block UI).

3. **Connect**
   - Mirror what `gh codespace ssh` does internally:
     1. Resolve codespace details from API (`GET /user/codespaces/{name}`).
     2. Acquire/generate an SSH keypair (we should default to a Vector-managed key like `~/.ssh/vector-codespace.{pub}` to avoid touching user's keys).
     3. Upload public key to the codespace via the GitHub API if not already present.
     4. Open a port-forwarding **tunnel** to the codespace's SSH server (Codespaces SSH is reached over a port-forwarded session, not directly — `gh` uses a Live Share-style relay).
     5. Speak SSH over that tunnel using `russh` or similar Rust SSH client.
   - Visual: tab title shows `🟢 codespace-name` with a status pip; tab background tinted (e.g., subtle GitHub-purple) so user always knows "this is remote".
   - Show round-trip latency in the tab/status bar.

4. **In-session UX**
   - Title bar pattern: `[profile] · user@codespace:~/repo` (cwd via OSC 7).
   - Status bar (optional, configurable): codespace name, machine type, billing region, latency, "remote" badge.
   - Disconnect indicator if heartbeat fails.

5. **Reconnect**
   - On disconnect (wifi drop, codespace stopped, token expired), show a clear "Reconnecting…" overlay.
   - If codespace is `Shutdown`, offer "Start codespace" (one-tap).
   - Token refresh: handle 401s by re-running device flow silently (token won't have expired in <30 days for GitHub PATs/OAuth, but handle it).
   - **Session preservation**: see #7 (Persistence + Reconnect).

6. **Profiles**
   - "Save this Codespace as a profile": `my-cs-frontend` becomes a one-click reconnect target on the start screen.
   - Profile = `{name, kind: codespace, codespace_name, key_path?, env, theme, font, working_dir?, startup_command?}`.
   - Profile auto-uses last-known codespace state; if shutdown, auto-starts on connect.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| GitHub OAuth Device Flow sign-in | Eliminates `gh auth login` dependency; in-app onboarding | M | `octocrab` covers REST; device flow is ~50 lines of HTTP polling |
| Codespaces picker UI (list, state, repo, branch) | The headline UX vs `gh codespace list` | M | Native SwiftUI-ish list; live state updates (every 5s when picker is open) |
| One-click start of `Shutdown` codespaces | `gh` requires a separate `codespace edit`/CLI; we make it native | S | API call + poll |
| In-app SSH client (no shell-out to `ssh`) | Removes dependency on system OpenSSH; control over key paths | L | `russh` (Rust SSH-2 client); reuse known-good keys |
| Auto-keypair management | Don't pollute user `~/.ssh`; use Vector-scoped key | S | Generate ed25519, register via API, persist in app data |
| Port-forwarding tunnel to codespace SSH | The connection path `gh` uses internally | L | Live-Share/Dev-Tunnels-style relay; this is the riskiest part of v1 |
| Codespaces saved as one-click "profiles" | Power users open Vector → click profile → already in shell | S | Just persist the codespace ID + display name |
| Visual "this is remote" indicator | Prevents `rm -rf` on wrong machine; user safety | S | Tab tint + status bar |
| Region latency hint | Helps users pick a closer region next time | S | Optional, async |

**Confidence:** HIGH on the API surface (REST endpoints documented; `cli/cli` source is open). MEDIUM on the SSH-tunnel internals — we'll need to read `cli/cli/internal/codespaces/ssh.go` and `internal/codespaces/connection/connection.go` carefully, since the relay handshake isn't separately documented.

Sources: [GitHub Codespaces REST API](https://docs.github.com/en/rest/codespaces/codespaces), [gh codespace ssh manual](https://cli.github.com/manual/gh_codespace_ssh), [cli/cli ssh.go](https://github.com/cli/cli/blob/trunk/pkg/cmd/codespace/ssh.go), [GitHub OAuth device flow changelog](https://github.blog/changelog/2022-03-16-enable-oauth-device-authentication-flow-for-apps/), [GitHub OAuth best practices](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/best-practices-for-creating-an-oauth-app).

#### 6. Dev Tunnels-First Connection Flow

The second-headline differentiator. Use case: user runs `code tunnel` on their corp laptop / home box / EC2 box, signed in with their GitHub account, then connects from Vector — no VS Code on their local machine.

**User journey (target):**

1. **Discovery — list tunnels**
   - From the same start screen, a "Dev Tunnels" tab/section alongside "Codespaces."
   - Authenticated GitHub user → query the Dev Tunnels management API with the user's GitHub identity (the Dev Tunnels service accepts GitHub-issued auth alongside Microsoft/Entra).
   - List: tunnel name, host machine name, status (online/offline), last-seen.
   - Note: `code tunnel` registers a tunnel by name (default = hostname); user picks by name.

2. **Connect**
   - Establish SSH-over-WebSocket-over-Dev-Tunnels-relay session.
   - Dev Tunnels uses an SSH protocol embedded inside a WSS connection to the relay (`wss://*.rel.tunnels.api.visualstudio.com`). The relay does not see plaintext — it's end-to-end encrypted SSH, AES-256-CTR.
   - Auth headers use `X-Tunnel-Authorization` (not standard `Authorization`) per the Dev Tunnels spec.

3. **Profiles**
   - Same profile model as Codespaces: `kind: dev_tunnel, tunnel_id, name, …`.

4. **In-session UX**
   - Same visual pattern as Codespaces (remote tint, machine name in tab, latency).
   - Tab tint **different color** from Codespaces so users can tell "this is my own box" vs "this is GitHub-managed."

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Dev Tunnels client (WSS + SSH-over-relay) | The "I run my own box" use case; Vector replaces VS Code Remote-Tunnels for shell users | XL | Riskiest piece of v1; no public Rust SDK. Reference: `microsoft/dev-tunnels` (TypeScript/.NET/Go SDKs available — port the protocol) |
| Tunnel list / picker | Same UX consistency as Codespaces | M | Different REST surface (`global.rel.tunnels.api.visualstudio.com`) |
| GitHub-auth → Dev Tunnels token exchange | Users don't want a Microsoft account just to connect to their own box | M | Dev Tunnels accepts GitHub-issued auth tokens (verified) |
| Visual differentiation from Codespaces | Tabs colored differently so users instantly know context | S | Just config |

**Confidence:** MEDIUM. The protocol is documented in the security and architecture docs; multiple SDKs (TS, .NET, Go) exist, so porting the protocol to Rust is feasible but non-trivial. Flag this as the highest-risk v1 work.

Sources: [Microsoft Dev Tunnels SDK (microsoft/dev-tunnels)](https://github.com/microsoft/dev-tunnels), [Dev tunnels security](https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/security), [Dev tunnels overview](https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/overview), [Developing with Remote Tunnels](https://code.visualstudio.com/docs/remote/tunnels).

#### 7. Session Persistence + Transparent Reconnect

Per PROJECT.md, "wifi drop should not lose Codespace state." This is a defining requirement.

**Possible implementations (recommend (c)):**

(a) **Mosh-style UDP datagram protocol** — would be ideal but requires a server-side `mosh-server` we'd need to install on each Codespace. Codespace devcontainers don't ship mosh by default. Defer.

(b) **Just SSH `ServerAliveInterval` + reconnect-on-failure** — minimum viable; on disconnect, re-establish SSH, but the remote shell *is killed* (PTY dies). User loses running process and shell history. **Not acceptable** per the requirement.

(c) **Auto-attach to a remote tmux session** ★ **recommend** — On connect, run `tmux new-session -A -s vector-{profile-id}` so first connection creates the session and reconnects attach to it. PTY dies on disconnect, but tmux preserves the shell + running commands. On reconnect, transparently re-attach.

   - Requires tmux on the remote (Codespaces ship with tmux; for Dev Tunnels, the user might not — graceful degradation: warn + offer to install, or fall back to plain SSH).
   - Subtle: the user might *also* be running tmux interactively. Solution: nest carefully; or use a single `vector-` named session and trust the user to know it's there.
   - Handles wifi drops gracefully: 30 seconds → auto-reconnect, attach back to the same session, no data loss. 30 minutes → same, but a "reconnecting…" overlay shows for that whole period; user can cancel and try later.

(d) **VS Code-style "persistent terminal" inside our own server agent** — an agent we install on the remote that owns the PTY and we re-attach to it. This is what VS Code Remote does. Heavyweight; requires shipping & installing an agent. Defer to v2.

**Recommendation: ship (c) as the v1 default.** It uses tmux (already on Codespaces; trivial on Dev Tunnels boxes), gives a true zero-data-loss reconnect, and integrates with #8 (tmux pass-through) so the user can also use tmux explicitly without conflict.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| SSH `ServerAlive*` + auto-reconnect | Floor: don't hang on dead TCP connections | S | russh keepalives |
| Auto-attach to managed tmux session | Zero-data-loss across wifi drops | M | Smart wrapper command on connect |
| Reconnect overlay UI | "Reconnecting… (15s)" with cancel | S | UI state machine |
| Configurable retry/backoff | Don't hammer GitHub during outages | S | Exp backoff, max retries |
| Session bookmark on disconnect | "Vector-frontend-cs lost connection at 2:34pm. Reconnect?" | S | Notification |

**Confidence:** HIGH on tmux strategy (well-trodden VS Code workaround); MEDIUM on UX details (we're inventing the overlay UX to suit our needs).

Sources: [Persistent VS Code Remote Terminals with tmux](https://www.wenbo.io/en-US/Tools/Persistent-VSCode-Remote-Terminals), [Mosh and Tmux comparison](https://hoop.dev/blog/mosh-and-tmux-uninterrupted-remote-terminal-sessions), [VS Code Remote-SSH reconnect issue thread](https://github.com/microsoft/vscode-remote-release/issues/3096).

#### 8. tmux Pass-Through That "Just Works"

Per PROJECT.md, "no double-multiplex visual glitches." This is a quality-of-life table-stakes-for-power-users feature; users running tmux inside our terminal must not see broken cursor shapes, missing clipboard sync, or eaten prefix keys.

**The four hazards:**

1. **DCS passthrough (`\eP...\e\\`)** — tmux 3.3+ defaults `allow-passthrough off`, which silently drops `\eP` envelopes. Tools (Claude Code, neovim with OSC52) wrap their OSC 52 in DCS passthrough specifically because tmux historically swallowed raw OSC 52. Net effect: clipboard "works" outside tmux, breaks inside, user has no idea why. **Fix:** Vector itself accepts both raw OSC 52 *and* DCS-wrapped OSC 52 at the outermost terminal layer. Document `set -g allow-passthrough on` for tmux users in our default-shell-integration script's optional advice.

2. **Cursor shape (DECSCUSR)** — escape `\e[N q` is a CSI, not DCS, so it survives most tmux configs. But the escape needs to make it to Vector unmangled. Verified working in tmux ≥3.4 with `terminal-overrides`. Vector should advertise itself as `xterm-256color` or `xterm-ghostty`-style terminfo and ensure `Ss`/`Se` capabilities are set.

3. **Mouse modes** — modes 1000/1002/1003/1006 must be propagated. tmux usually handles this fine when the outer terminal supports SGR (1006), which is what we'll do.

4. **Alternate screen + scrollback interaction** — when tmux is running on remote, scrolling in our terminal should scroll the tmux pane (mouse mode), not Vector's scrollback. tmux usually handles this; Vector just needs to forward mouse mode correctly and not steal scroll events when alt-screen is active.

5. **Keys not getting eaten** — Vector keybindings should default to **Cmd-modifier** (Mac convention). tmux uses Ctrl-b. iTerm-style "send keypress to terminal even though it's our hotkey" is helpful (users can override). No conflicts in default config because Mac apps use Cmd, terminals use Ctrl.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Accept both raw and DCS-wrapped OSC 52 | Clipboard works in tmux without any user config | S | Parser handles both forms |
| Correct terminfo entry shipped for Vector | `setopt terminfo` complete; `tmux` recognizes capabilities | M | Generate `vector` terminfo entry; offer ssh-terminfo-style auto-install on remote (ghostty's pattern) |
| Default keybinds use Cmd, never Ctrl-b | Don't steal tmux prefix | S | Just config discipline |
| Forward all standard mouse modes (incl. SGR 1006) | tmux mouse, vim mouse, lazygit work | S | Standard |
| Cursor shape DECSCUSR pass-through | vim modes work correctly inside tmux-over-ssh | S | CSI; usually fine |
| Bracketed paste through tmux | Pasted text isn't interpreted as commands | S | tmux usually handles |

**Confidence:** HIGH. These are well-documented gotchas; we have the receipts.

Sources: [tmux allow-passthrough](https://tmuxai.dev/tmux-allow-passthrough/), [DCS-wrapped OSC 52 in xterm.js through tmux](https://max.nardit.com/articles/osc52-clipboard-xterm-tmux), [tmux cursor shape issue 3404](https://github.com/tmux/tmux/issues/3404), [On tmux OSC-52 support](https://kalnytskyi.com/posts/on-tmux-osc52-support/), [opencode allow-passthrough issue](https://github.com/anomalyco/opencode/issues/19982).

#### 9. Profiles

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Profile = launch-target + env + appearance | Standard model (iTerm, WezTerm, Warp) | M | TOML/JSON file in `~/.config/vector/profiles/`; hot-reload |
| Profile kinds: `local`, `codespace`, `dev_tunnel`, (later) `ssh` | Single mental model unifies local + remote | S | Discriminated union in Rust |
| Per-profile env vars, working dir, startup command | Standard | S | Trivially layered into spawn |
| Per-profile theme/font/cursor | "My Codespaces look different from local" | S | Profile overrides global |
| Per-profile tab tint / icon | Visual distinction at-a-glance | S | UI niceness |
| Default profile (opened by Cmd-N / Cmd-T) | Most users want one default | S | Setting |

**Confidence:** HIGH. Standard pattern.

Sources: [WezTerm SSH domains](https://wezterm.org/config/lua/config/ssh_domains.html), [Warp blog on terminal completions](https://docs.warp.dev/terminal/command-completions/autosuggestions/).

#### 10. Optional Claude API Autosuggest (v2 differentiator)

Per PROJECT.md, optional/v2 ambition. Bar to clear (Warp's): autocomplete that sometimes helps, often gets in the way, requires login. Bar we *want* to clear:

- **Strictly opt-in**, behind a feature flag and explicit API key entry. Off by default.
- Users provide **their own Anthropic API key**, stored in Keychain. No Vector account, no Vector-mediated billing, no cloud-routed prompts.
- **Inline ghost-text suggestions only** (single-line, dim color, accept with right-arrow). Not a chat panel, not a "blocks UI", not an agent. Just shell-history-aware completions augmented by an LLM.
- **Local context only**: shell history, current cwd, current command line. No file contents, no terminal output (privacy-positive default; Warp got burned for sending session contents without consent — see [HN thread](https://news.ycombinator.com/item?id=44953470)).
- Debounced (200-500ms idle), short prompts, low cost. Cancel on user keystroke.
- Status bar indicator when AI is "thinking" so users know what's network-traffic-wise happening.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Inline ghost-text shell autosuggest via Claude API | Modern terminal, lighter than Warp, BYO-key | M | Keep it modest; this is v2, not v1 |
| Strictly opt-in + BYO-API-key | Trust + privacy; hard differentiator vs Warp | S | Off by default; never sends without explicit enable |
| No prompt-leak of file contents / scrollback | Privacy-safe; "what gets sent" is observable | S | Send only: cwd, recent commands, current line |
| Cmd-/ to toggle suggestion on demand | Doesn't get in the way when not wanted | S | Keybind |

**Confidence:** MEDIUM (UX bar is judgment, not measurement).

Sources: [Warp autosuggestions docs](https://docs.warp.dev/terminal/command-completions/autosuggestions/), [Warp privacy concerns HN](https://news.ycombinator.com/item?id=44953470), [Warp AI complaints issues](https://github.com/warpdotdev/Warp/issues/3540).

---

### Anti-Features (Deliberately NOT Building)

These are features competitors ship that we will explicitly omit. Each entry justifies the omission and gives a non-feature alternative.

| Feature | Why Competitors Ship It | Why We Skip | What We Do Instead |
|---------|-------------------------|-------------|--------------------|
| **Mandatory Vector account / cloud login** | Warp (until backlash), enables collaboration & billing | This is the #1 user complaint about Warp; users got locked out when servers were down | No account at all. GitHub auth is just for the GitHub APIs we call; tokens are local. |
| **Cloud-synced settings/themes/history** | Warp/Wave nice-to-have | Adds backend, requires account; "Mac-fast" + "no bloat" is the brand | Config is a TOML file; sync via Dropbox/iCloud/git like every other terminal user already does |
| **Telemetry / usage analytics by default** | Product analytics, helps prioritize features | User complaint magnet; trust hit; we have no product team to consume the data anyway | None. If we ever add it, opt-in with full disclosure. |
| **AI bundled by default / always-on** | Warp's go-to-market | Forces memory + network even when off; users hate the "off" toggle that doesn't actually turn things off | Optional, BYO-API-key, off by default, no resident process when disabled |
| **"Blocks" UI (command-output as cards)** | Warp's signature interaction model | Breaks `tmux`, breaks `vim` mouse, breaks `htop`, breaks any TUI; not compatible with "tmux must just work" | Plain VT grid. Trust the shell prompt + OSC 133 marks for prompt navigation if users want it. |
| **Built-in chat / collaboration / shared sessions** | Warp's selling point | We're a terminal, not a collab tool; tmate exists | Nothing. Users who want this know where tmate / Tailscale Tunnel are. |
| **Web companion / browser-based "Vector Web"** | vscode.dev style; broad reach | Explicit anti-goal per PROJECT.md ("Native-only is a feature") | Nothing. |
| **Codespaces lifecycle UI (create/delete/rebuild)** | Power-feature for full IDE replacements | Per PROJECT.md, deferred. Adds API surface and complexity for marginal v1 value | `gh codespace create/delete` still works in our terminal |
| **Port-forwarding "PORTS" panel** | VS Code parity | Per PROJECT.md, deferred to v2. Useful but not on critical path | `ssh -L` and `gh codespace ports forward` still work |
| **File transfer GUI (drag-drop, scp panel)** | Mosh users etc. like it | Per PROJECT.md, deferred. `scp`/`rsync` work in shell | Nothing in v1 |
| **Login wall before first use** | Forced funnel for AI signups | The single biggest reason users abandon Warp | Vector starts up and gives a local terminal immediately. GitHub sign-in is optional and only needed for remote features. |
| **Mandatory onboarding survey** | Marketing data | Hostile UX | None. Empty start screen + "open shell." |
| **"Codespace blocks" / GitHub-account-list-everywhere visual chrome** | Feels integrated | Adds surface area; small icon + tab tint is enough | Visual cues (tint, badge) only |
| **Plugin/extension marketplace** | Tabby has one | Maintenance burden, security surface, scope creep | Native config + (if needed) Lua/scripting via embedded engine — but defer well past v1 |
| **AI agent that runs commands for you** | Warp 2026 push | Risky (deletes files), needs review/sandboxing, totally different product | Nothing. The user types commands. |
| **Cross-platform support (Linux, Windows)** | Bigger TAM | Per PROJECT.md, Mac-only v1 | Mac-only |
| **Built-in SSH-key generation wizard for arbitrary hosts** | Some terminals do this | Out of scope (arbitrary SSH is deferred per PROJECT.md) | We do auto-keypair *only* for our managed Codespaces flow |
| **Auto-update mechanism** | Modern apps have it | Adds backend (release endpoint, signing); deferred until signing is sorted | Manual `.dmg` re-download for v1 |

**Confidence:** HIGH. Each anti-feature is justified directly from PROJECT.md, user-reported pain in competitors, or scope discipline.

Sources: [Warp behind login popup (DEV)](https://dev.to/krunchdata/warp-a-terminal-behind-login-popup-5284), [Warp open source and login](https://www.warp.dev/blog/open-source-and-login-for-warp), [Warp telemetry/AI complaints](https://github.com/warpdotdev/Warp/issues/8629), [Warp HN review thread](https://news.ycombinator.com/item?id=44704043), [Warp sends terminal session to LLM](https://news.ycombinator.com/item?id=44953470), [Wave Terminal alternatives](https://alternativeto.net/software/wave-terminal/).

---

## Feature Dependencies

```
[GPU renderer (wgpu/Metal)]
    └──foundation for──> [Terminal core (VT parser + grid + scrollback)]
                              └──foundation for──> [Tabs/Splits/Panes]
                              └──foundation for──> [Themes/Fonts/Ligatures]
                              └──foundation for──> [Profiles]

[GitHub OAuth Device Flow]
    └──unlocks──> [GitHub REST API client (octocrab)]
                       ├──unlocks──> [Codespaces picker]
                       └──unlocks──> [Dev Tunnels list]

[Codespaces picker] + [SSH client (russh)] + [Port-forwarding tunnel]
    └──compose──> [Codespaces connect flow]
                       └──saved as──> [Codespace profile]

[Dev Tunnels client (WSS+SSH-over-relay)]
    └──compose──> [Dev Tunnels connect flow]
                       └──saved as──> [Dev Tunnel profile]

[Terminal core] + [Codespaces connect] + [tmux on remote]
    └──compose──> [Session persistence + reconnect]

[Terminal core (VT parser)] — must accept DCS passthrough
    └──enables──> [tmux pass-through (clipboard, cursor, mouse)]

[Profiles] ──enhances──> [Codespaces flow] (one-click reconnect)
[Profiles] ──enhances──> [Dev Tunnels flow] (one-click reconnect)
[Hot-reload config] ──enhances──> [Profiles] (edit-and-go)

[Optional Claude Autosuggest]  (v2)
    └──depends-on──> [Shell integration / OSC 133 mark recognition]
    └──depends-on──> [Keychain-stored Anthropic API key]

[Mandatory account / login wall]  ── CONFLICTS WITH ──> [Trust / "no bloat" promise]
[AI bundled by default]           ── CONFLICTS WITH ──> [Lightweight memory footprint]
[Blocks UI]                       ── CONFLICTS WITH ──> [tmux pass-through; vim/htop compat]
```

### Dependency Notes

- **Terminal core is the only thing on the critical path before anything else** — tabs, splits, themes, profiles, and remote connection all sit on top of it. If the core is wrong (e.g., we miss DCS passthrough), every dependent feature is degraded.
- **Codespaces and Dev Tunnels share the GitHub auth + REST client substrate** — build that infrastructure once, then both flows are mostly UI + protocol-specific transport.
- **Session persistence (#7) chooses to depend on tmux on the remote** rather than on a custom server-side agent. This is a *deliberate* design choice that keeps Vector lightweight and v1-shippable.
- **The Blocks UI / Account / AI-by-default antipatterns conflict structurally with the "no-bloat / Mac-fast" core value.** They're not just unselected features; including any of them would break the value prop.
- **Optional AI must wait for OSC 133 prompt marking** — it needs to know where the prompt is to suggest meaningfully. So shell-integration scripts are a v1 prerequisite even though AI itself is v2.

---

## MVP Definition

### Launch With (v1) — aligned with PROJECT.md "Active" requirements

**Terminal core (the daily-driver):**
- [ ] xterm-compatible VT parser, 24-bit color, Unicode w/ CJK width, scrollback ≥10k, scrollback regex search — *because daily-driver*
- [ ] OSC 7, 8, 52 (raw + DCS-wrapped), 133 — *because tmux pass-through and shell integration depend on it*
- [ ] Bracketed paste, mouse modes incl. SGR 1006, alternate screen — *because vim/tmux/htop need them*
- [ ] Cursor shape DECSCUSR, OSC 10/11/12 fg/bg/cursor color queries — *because vim insert/normal modes*
- [ ] IME for CJK — *because we have APAC users; non-negotiable on Mac*
- [ ] GPU renderer via wgpu/Metal at 60+fps under heavy output — *because raison d'être*
- [ ] Tabs + horizontal/vertical splits — *PROJECT.md must-have*
- [ ] Bring-your-own-font, ligatures opt-in, Nerd Font glyphs render correctly, iTerm `.itermcolors` import — *because users have favorites*
- [ ] Hot-reload config on save — *quality-of-life standard*
- [ ] Native macOS chrome, fullscreen, tabbing, Secure Keyboard Entry toggle — *because Mac app*
- [ ] Profiles (local + codespace + dev_tunnel) with per-profile env, theme, tint — *PROJECT.md must-have*
- [ ] Universal binary, unsigned `.dmg` produced by CI on tag — *PROJECT.md must-have*

**Remote (the differentiator):**
- [ ] GitHub OAuth Device Flow + Keychain token storage — *gateway to everything remote*
- [ ] Codespaces picker (list, state, repo, last-used, region, latency hint) — *headline UX*
- [ ] One-click start of Shutdown codespaces — *part of headline UX*
- [ ] In-app SSH client (russh) + port-forwarding tunnel for Codespaces SSH — *removes `gh` dependency*
- [ ] Auto-managed SSH keypair (Vector-scoped) — *clean default*
- [ ] Dev Tunnels client list + connect — *PROJECT.md must-have, riskiest piece*
- [ ] Saved profile = one-click reconnect — *PROJECT.md must-have*
- [ ] Visual "this is remote" tab tint + status — *user safety*

**Resilience (the trust-builder):**
- [ ] Auto-attach to managed tmux session on remote — *PROJECT.md must-have ("wifi drop should not lose state")*
- [ ] tmux pass-through that just works (DCS, cursor, mouse, clipboard) — *PROJECT.md must-have*
- [ ] Reconnect overlay UI with auto-retry — *required to make the above visible*

### Add After Validation (v1.x — minor releases on top of v1)

- [ ] Theme switching follows macOS dark/light mode — when users start asking
- [ ] OSC 9;4 progress bar surfaced in tab/dock — minor delight
- [ ] Command palette (Cmd-Shift-P) — when users discover keybinds aren't enough
- [ ] Quick terminal / hotkey window — popular request
- [ ] Per-Codespace tinted background colors auto-derived from repo — *delight*
- [ ] Ssh-terminfo auto-install on remote (ghostty pattern) — for users on non-Codespaces Dev Tunnel boxes
- [ ] Apple Developer signing & notarization — defer until friction grows ([PROJECT.md decision](#))
- [ ] Auto-update / Sparkle integration — needs signing first

### Future Consideration (v2+)

- [ ] **Claude API autosuggest** (BYO key, opt-in, ghost-text only) — flagship v2 differentiator
- [ ] **Port-forwarding panel** — per PROJECT.md, deferred to v2
- [ ] **File transfer (drag-drop / scp UI)** — per PROJECT.md, deferred
- [ ] **Codespaces lifecycle (create/delete/rebuild)** — per PROJECT.md, deferred
- [ ] **Arbitrary SSH targets as first-class profiles** — per PROJECT.md, deferred
- [ ] **Linux build** — per PROJECT.md, deferred
- [ ] **Custom server-side agent for true persistent terminal (a-la VS Code Remote)** — only if tmux-attach proves insufficient
- [ ] **Tmux control mode integration** (render tmux panes as native splits) — power-user feature; complex; only after core is solid
- [ ] **Sixel / Kitty graphics protocol support** — neat, low priority; mostly for `image-cli`/`viu`/`chafa` users
- [ ] **Plugin/scripting layer (Lua-style)** — only if users demand it; default position is "config TOML is enough"

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| GPU-accelerated VT terminal core | HIGH | HIGH | **P1** |
| Tabs + Splits | HIGH | MEDIUM | **P1** |
| OSC 7/8/52/133, mouse, bracketed-paste, alt-screen | HIGH | LOW-MED | **P1** |
| OSC 52 raw + DCS-wrapped acceptance | HIGH | LOW | **P1** |
| IME (CJK) on macOS | HIGH (for APAC users) | MEDIUM | **P1** |
| Hot-reload config | MEDIUM | LOW | **P1** |
| Themes (.itermcolors import), fonts, ligatures | HIGH | LOW-MED | **P1** |
| Native macOS chrome (tabs, fullscreen, vibrancy) | HIGH | MEDIUM | **P1** |
| Secure Keyboard Entry toggle | MEDIUM | LOW | **P1** |
| GitHub OAuth Device Flow + Keychain | HIGH | LOW-MED | **P1** |
| Codespaces picker + connect (SSH+tunnel) | HIGH | HIGH | **P1** |
| Dev Tunnels picker + connect | HIGH | HIGH (XL — riskiest) | **P1** |
| Profiles | HIGH | MEDIUM | **P1** |
| tmux pass-through correctness | HIGH | LOW (parser tweaks + terminfo) | **P1** |
| Auto-tmux-attach for session persistence | HIGH | MEDIUM | **P1** |
| Reconnect overlay UI | MEDIUM | LOW | **P1** |
| Visual remote-session indicator | MEDIUM | LOW | **P1** |
| `.dmg` CI pipeline (universal, unsigned) | HIGH | MEDIUM | **P1** |
| Command palette | MEDIUM | MEDIUM | P2 |
| Quick terminal / hotkey window | MEDIUM | MEDIUM | P2 |
| Auto-follow macOS dark/light | MEDIUM | LOW | P2 |
| OSC 9;4 progress in tab | LOW | LOW | P2 |
| ssh-terminfo auto-install on remote | MEDIUM | MEDIUM | P2 |
| Apple Developer signing/notarization | MEDIUM (less friction) | MEDIUM (cost+CI) | P2 |
| Auto-update | MEDIUM | MEDIUM | P2 |
| Claude API autosuggest (opt-in, BYO) | MEDIUM | MEDIUM-HIGH | **P3 (v2)** |
| PORTS panel | MEDIUM | MEDIUM | **P3 (v2)** |
| File transfer GUI | LOW-MED | MEDIUM | **P3 (v2)** |
| Codespaces lifecycle | MEDIUM | MEDIUM | **P3 (v2)** |
| Arbitrary SSH profiles | MEDIUM | MEDIUM | **P3 (v2)** |
| Linux/Windows build | LOW (today) | HIGH | **P3 (v2+)** |
| Sixel / Kitty graphics | LOW | MEDIUM | **P3 (v2+)** |
| Custom remote agent (VS-Code-style) | MEDIUM | XL | **P3 (only if tmux insufficient)** |

**Priority key:**
- **P1**: Must have for v1 launch (the daily-driver bar + headline differentiators)
- **P2**: Should have, add in v1.x patch releases
- **P3**: v2+; deliberately deferred per PROJECT.md or explicitly anti-feature

---

## Competitor Feature Analysis

| Feature | iTerm2 | ghostty | WezTerm | Alacritty | Warp | Wave | Vector (planned) |
|---------|--------|---------|---------|-----------|------|------|------------------|
| GPU rendering | Partial | Yes (Metal) | Yes (wgpu) | Yes (OpenGL) | Yes | Yes | **Yes (wgpu/Metal)** |
| Native macOS chrome | Yes | Yes (best-in-class) | Partial | Minimal | Yes | Partial (Electron) | **Yes** |
| Tabs + splits | Yes | Yes | Yes | No (use tmux) | Yes | Yes | **Yes** |
| Themes (.itermcolors) | Native | Importable | Importable | Importable (via TOML) | Custom | Custom | **Importable** |
| Ligatures | Yes | Yes | Yes | Yes | Yes | Yes | **Yes (opt-in)** |
| OSC 52 (clipboard) | Yes | Yes | Yes | Yes | Yes | Yes | **Yes (raw + DCS-wrapped)** |
| OSC 133 (shell integration) | Yes | Yes | Yes | No | Yes | Yes | **Yes** |
| OSC 8 (hyperlinks) | Yes | Yes | Yes | Yes | Yes | Yes | **Yes** |
| Sixel | Yes | No | Yes | No | No | Yes | **No (v2+)** |
| Kitty graphics | No | Yes | Yes | No | No | No | **No (v2+)** |
| IME (CJK) | Yes | Yes | Yes | Partial | Yes | Yes | **Yes (P1)** |
| Codespaces UX | None | None | None | None | Generic SSH | None | **Headline UX** |
| Dev Tunnels UX | None | None | None | None | None | None | **Headline UX (we're first)** |
| Built-in SSH client | Shells out | Shells out | Built-in (russh-ish) | Shells out | Shells out | Shells out | **Built-in (russh)** |
| Session persistence | None | None | Mux domains (own protocol) | None | Cloud (account) | Cloud (account) | **tmux-attach (no account)** |
| Account / login required | No | No | No | No | **Yes** | No | **No** |
| AI bundled | No | No | No | No | **Yes (forced load)** | Optional | **No (v2 opt-in BYO key)** |
| Telemetry default | No | No | No | No | Optional / required-for-AI | Optional | **No** |
| Blocks UI | No | No | No | No | **Yes** | Yes | **No** |
| Hot-reload config | Yes (UI) | Yes (USR2) | Yes | Yes | n/a | n/a | **Yes** |
| Distribution | App Store + signed DMG | Signed DMG | Signed DMG | Signed DMG | Signed DMG | Signed DMG | **Unsigned DMG (v1) / signed (v2)** |
| Open source | No | Yes (MIT) | Yes (MIT) | Yes (Apache) | Partial | Yes | **TBD; PROJECT.md says no public push for v1** |

**Key differentiators (where Vector wins):**
1. **Codespaces-as-headline-UX** — every other terminal treats Codespaces as a generic SSH target.
2. **Dev-Tunnels client built-in** — *no other terminal has this*. We're first to market.
3. **No account, no AI, no telemetry, no Blocks UI** — actively positioned against Warp/Wave bloat.

**Key parities (the price of admission):**
- Terminal core, GPU rendering, tabs/splits, OSC sequences, IME, fonts/themes — must match the field.

**Where we lose by design:**
- Sixel/Kitty graphics (defer; specialty)
- Plain SSH UX (deferred — `ssh` still works in shell)
- File transfer GUI (deferred)
- Cross-platform (Mac-only v1)

---

## Sources

### Reference terminals (for parity & UX patterns)
- [ghostty terminal — features](https://ghostty.org/docs/features) and [shell integration](https://ghostty.org/docs/features/shell-integration)
- [ghostty 1.2 release notes](https://www.omgubuntu.co.uk/2025/10/ghostty-1-2-new-features-for-linux)
- [WezTerm multiplexing & SSH](https://wezterm.org/multiplexing.html), [SSH domains](https://wezterm.org/config/lua/config/ssh_domains.html)
- [WezTerm scrollback](https://wezterm.org/scrollback.html), [appearance/colors](https://wezterm.org/config/appearance.html)
- [iTerm2 feature reporting spec](https://iterm2.com/feature-reporting/)
- [iTerm2 menu items / Secure Keyboard Entry](https://iterm2.com/documentation/2.1/documentation-menu-items.html)
- [Choosing a Terminal on macOS 2025](https://medium.com/@dynamicy/choosing-a-terminal-on-macos-2025-iterm2-vs-ghostty-vs-wezterm-vs-kitty-vs-alacritty-d6a5e42fd8b3)
- [Modern Terminal Emulators 2026 (Calmops)](https://calmops.com/tools/modern-terminal-emulators-2026-ghostty-wezterm-alacritty/)
- [Terminal Compatibility Matrix (tmuxai)](https://tmuxai.dev/terminal-compatibility/)
- [State of the Terminal (gpanders)](https://gpanders.com/blog/state-of-the-terminal/)
- [State of Terminal Emulation 2025 (jeffquast)](https://www.jeffquast.com/post/state-of-terminal-emulation-2025/)

### OSC sequences & escape codes
- [Ghostty OSC commands and protocols](https://deepwiki.com/ghostty-org/ghostty/3.4-osc-commands-and-protocols)
- [Are We Sixel Yet?](https://www.arewesixelyet.com/)
- [Terminal Graphics Protocols (Akmatori)](https://akmatori.com/blog/terminal-graphics-protocols)
- [Ghostty Knowledge Base: Shell Integration](https://rexbrahh.github.io/ghostty-knowledge-base/guide/24-terminal-io/shell-integration-and-startup-hooks/)

### tmux pass-through gotchas
- [How to configure allow-passthrough in tmux](https://tmuxai.dev/tmux-allow-passthrough/)
- [How I Fixed OSC 52 Clipboard in xterm.js Through tmux (Max Nardit)](https://max.nardit.com/articles/osc52-clipboard-xterm-tmux)
- [On tmux OSC-52 support (Kalnytskyi)](https://kalnytskyi.com/posts/on-tmux-osc52-support/)
- [tmux issue #3192 — pass through OSC 52 selection](https://github.com/tmux/tmux/issues/3192)
- [tmux issue #3404 — cursor shape](https://github.com/tmux/tmux/issues/3404)
- [opencode issue #19982 — DCS in tmux](https://github.com/anomalyco/opencode/issues/19982)
- [tmux + system clipboard guide (Samoshkin)](https://medium.com/free-code-camp/tmux-in-practice-integration-with-system-clipboard-bcd72c62ff7b)
- [Sunaku tmux-yank-osc52](https://sunaku.github.io/tmux-yank-osc52.html)

### GitHub Codespaces protocol & SSH internals
- [GitHub Codespaces REST API](https://docs.github.com/en/rest/codespaces/codespaces)
- [Using GitHub Codespaces with GitHub CLI](https://docs.github.com/en/codespaces/developing-in-a-codespace/using-github-codespaces-with-github-cli)
- [`gh codespace ssh` manual](https://cli.github.com/manual/gh_codespace_ssh)
- [cli/cli ssh.go (trunk)](https://github.com/cli/cli/blob/trunk/pkg/cmd/codespace/ssh.go)
- [Forwarding ports in Codespaces](https://docs.github.com/en/codespaces/developing-in-a-codespace/forwarding-ports-in-your-codespace)
- [Codespaces SSH tunnel discussion](https://github.com/orgs/community/discussions/25497)

### GitHub OAuth Device Flow
- [Authorizing OAuth apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps)
- [Enable OAuth Device Flow for Apps changelog](https://github.blog/changelog/2022-03-16-enable-oauth-device-authentication-flow-for-apps/)
- [Best practices for creating an OAuth app](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/best-practices-for-creating-an-oauth-app)
- [PKCE support for OAuth (2025)](https://github.blog/changelog/2025-07-14-pkce-support-for-oauth-and-github-app-authentication/)
- [CLI authentication methods (Logto)](https://blog.logto.io/cli-authentication-methods)

### Microsoft Dev Tunnels protocol
- [microsoft/dev-tunnels SDK](https://github.com/microsoft/dev-tunnels)
- [Dev tunnels security](https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/security)
- [What are dev tunnels?](https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/overview)
- [Dev tunnels CLI commands](https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/cli-commands)
- [Dev tunnels FAQ](https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/faq)
- [VS Code Remote Tunnels docs](https://code.visualstudio.com/docs/remote/tunnels)
- [Diving into Microsoft's dev tunnels (InfoWorld)](https://www.infoworld.com/article/2336324/diving-into-microsofts-dev-tunnels.html)

### Session persistence (mosh / tmux / SSH)
- [Persistent VS Code Remote Terminals with tmux (Wenbo Pan)](https://www.wenbo.io/en-US/Tools/Persistent-VSCode-Remote-Terminals)
- [Mosh and Tmux uninterrupted sessions (hoop.dev)](https://hoop.dev/blog/mosh-and-tmux-uninterrupted-remote-terminal-sessions)
- [Mosh project](https://mosh.org/)
- [VS Code Remote-SSH issue #3096 — persistent SSH](https://github.com/microsoft/vscode-remote-release/issues/3096)
- [VS Code Remote-SSH issue #5755 — sleep/wifi disconnect](https://github.com/microsoft/vscode-remote-release/issues/5755)
- [Persistent terminals in VS Code with tmux (Honeywood)](https://george.honeywood.org.uk/blog/vs-code-and-tmux/)

### Anti-features (what to avoid; user complaints in competitors)
- [Warp issue #3540 — Disable AI](https://github.com/warpdotdev/Warp/issues/3540)
- [Warp issue #8629 — agentic AI release mess](https://github.com/warpdotdev/Warp/issues/8629)
- [Warp.dev terminal HN review](https://news.ycombinator.com/item?id=44704043)
- [Warp sends terminal session to LLM HN](https://news.ycombinator.com/item?id=44953470)
- [Warp behind login popup (DEV)](https://dev.to/krunchdata/warp-a-terminal-behind-login-popup-5284)
- [Warp open source and login (Warp blog)](https://www.warp.dev/blog/open-source-and-login-for-warp)
- [Warp privacy](https://www.warp.dev/privacy)
- [Wave Terminal alternatives (AlternativeTo)](https://alternativeto.net/software/wave-terminal/)
- [Wave Terminal review (4sysops)](https://4sysops.com/archives/wave-a-modern-terminal-with-ai-features/)
- [Tabby vs Warp comparison (Slashdot)](https://slashdot.org/software/comparison/Tabby.sh-vs-Warp/)

### IME / CJK / emoji rendering
- [Contour IME demo](https://contour-terminal.org/demo/ime/)
- [kitty issue #6560 — CJK ambiguous width](https://github.com/kovidgoyal/kitty/issues/6560)
- [Microsoft Terminal issue #2213 — IME](https://github.com/microsoft/terminal/issues/2213)
- [Claude Code issue #19207 — IME for CJK](https://github.com/anthropics/claude-code/issues/19207)

### Themes / fonts / ligatures
- [iTerm2-Color-Schemes (mbadolato)](https://github.com/mbadolato/iterm2-color-schemes)
- [Nerd Fonts](https://www.nerdfonts.com/)
- [Nerd Fonts terminal emulators (DeepWiki)](https://deepwiki.com/ryanoasis/nerd-fonts/4.3-using-with-terminal-emulators)
- [How to set up Alacritty (Josean)](https://www.josean.com/posts/how-to-setup-alacritty-terminal)
- [How I use WezTerm (mwop)](https://mwop.net/blog/2024-07-04-how-i-use-wezterm.html)

### Secure Keyboard Entry / Mac specifics
- [Apple — Use secure keyboard entry in Terminal](https://support.apple.com/guide/terminal/use-secure-keyboard-entry-trml109/mac)
- [Fig docs — Secure Keyboard Input](https://fig.io/docs/support/secure-keyboard-input)
- [Warp issue #2891 — Add Secure Keyboard Entry](https://github.com/warpdotdev/Warp/issues/2891)

### Hot-reload config patterns
- [Runtime Configuration Reloading (Vorner)](https://vorner.github.io/2019/08/11/runtime-configuration-reloading.html)
- [Ghostty discussion #4280 — runtime theme change](https://github.com/ghostty-org/ghostty/discussions/4280)

### Warp AI feature reference (the bar to *lighten* against)
- [Warp Autosuggestions docs](https://docs.warp.dev/terminal/command-completions/autosuggestions/)
- [Warp Tab completions docs](https://docs.warp.dev/terminal/command-completions/completions)

---

*Feature research for: Vector (native macOS terminal w/ Codespaces + Dev Tunnels)*
*Researched: 2026-05-10*
