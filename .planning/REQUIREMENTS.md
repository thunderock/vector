# Requirements: Vector

**Defined:** 2026-05-10
**Core Value:** Open the app, pick a Codespace, get a fast remote shell — no VS Code, no browser, no clunky `gh codespace ssh` plumbing. Local-terminal niceties are table-stakes; the differentiator is that a Codespaces / Dev-Tunnels session feels native, not bolted on.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases. Categories are derived from `.planning/research/SUMMARY.md` and the 10-phase ordering converged on by all four research dimensions.

### Build & Distribution

- [x] **BUILD-01**: A cargo workspace skeleton compiles on macOS 13+ with Rust 1.88+ (pinned via `rust-toolchain.toml`)
- [x] **BUILD-02**: GitHub Actions CI builds Universal binaries (arm64 + x86_64 via `lipo`) on every push to main and on every tag _(implemented + locally verified in Plan 01-05 commit 506b6bb; pending first-real-CI-run telemetry capture per `01-05-SUMMARY.md §Outstanding Verification Debt` — user pushes asynchronously per CLAUDE.md)_
- [x] **BUILD-03**: An `xtask dmg` command produces an unsigned `Vector.dmg` locally, identical to what CI ships
- [x] **BUILD-04**: Tagged releases publish the unsigned `.dmg` to GitHub Releases (ghostty-style "tip" + tagged release pattern) _(tip-release half implemented in Plan 01-05 ci.yml commit 506b6bb; tagged-release half implemented in Plan 01-06 release.yml commit 4dd0c4e; pending first-real-CI-run telemetry capture per `01-05-SUMMARY.md §Outstanding Verification Debt` AND first-real-tagged-release run per `01-06-SUMMARY.md §Outstanding Verification Debt`)_
- [x] **BUILD-05**: README documents the `xattr -dr com.apple.quarantine /Applications/Vector.app` Gatekeeper bypass for teammates _(implemented in Plan 01-06 README.md commit 4dd0c4e; D-26 closed at artifact level — xattr literal byte-identical across 4 surfaces: README install block, ci.yml tip-release body, release.yml tagged-release body, DMG background PNG via xtask/scripts/render-dmg-bg.sh)_

### Terminal Core

- [x] **CORE-01**: VT parser passes a basic xterm conformance corpus (CSI / OSC / DCS dispatch, partial-UTF-8 reads, alternate screen DECSET 1049, scroll regions DECSTBM, tab stops, ED/EL erase semantics)
- [x] **CORE-02**: Terminal grid supports 24-bit truecolor and 256-color modes, with grapheme-cluster-aware cell width (East Asian width tables, emoji ZWJ sequences)
- [x] **CORE-03**: Scrollback buffer holds at least 10,000 lines and supports regex search across history
- [x] **CORE-04**: A local PTY can spawn a user's login shell, propagate `SIGWINCH` on resize, and survive child-process exit cleanly
- [x] **CORE-05**: `TERM=xterm-256color` (or equivalent) is advertised; `terminfo` quirks specific to Vector are kept zero in v1
- [x] **CORE-06**: Bracketed paste (mode 2004), mouse modes 1000/1002/1003 with SGR 1006 encoding, and DECSCUSR cursor-shape escapes work end-to-end

### Rendering

- [x] **RENDER-01**: GPU-accelerated rendering targets the Metal backend of `wgpu`, with damage-tracked redraws (only dirty rows shaped/uploaded)
- [x] **RENDER-02**: Sustained `cat large.log` output reaches at least 60 fps on Apple Silicon at 1080p; ProMotion (120 Hz) is detected and honored
- [x] **RENDER-03**: Idle CPU usage stays below 1% on Apple Silicon (no redraw when nothing is dirty)
- [x] **RENDER-04**: Glyph atlas separates monochrome and emoji textures, evicts via bounded LRU, and survives mid-session scale changes (Retina ↔ external monitor)
- [x] **RENDER-05**: Cursor and selection overlays render correctly under the live text grid

### Window & Mux

- [x] **WIN-01**: Native macOS AppKit window with title bar, fullscreen, and standard window-control buttons
- [x] **WIN-02**: Tabs — open new tab (Cmd-T), cycle (Cmd-Shift-]/[), close (Cmd-W). Native `NSWindowTabbingMode` or visually equivalent custom bar.
- [x] **WIN-03**: Splits — horizontal (Cmd-D) and vertical (Cmd-Shift-D) splits within a tab, with focus routing and per-pane resize
- [x] **WIN-04**: A `Domain / Pane / PtyTransport` abstraction (WezTerm-style) is the only seam between terminal model and transport — local, SSH, and tunnel transports all implement the same trait
- [x] **WIN-05**: `winit::EventLoop` runs on the main thread; `tokio` runs on background threads; cross-thread signaling goes through `EventLoopProxy::send_event` (no `block_on` on main, no shared mutex held across `await`)

### Polish (Local Daily-Driver)

- [x] **POLISH-01**: TOML configuration with hot-reload via `notify` (FSEvents); profile inheritance (`[default]` + named overrides) without a scripting language
- [ ] **POLISH-02**: Bring-your-own-font from system or `~/Library/Fonts`; opt-in ligatures; Nerd Font glyphs render correctly
- [ ] **POLISH-03**: Built-in light + dark themes plus an importer for `.itermcolors` palettes
- [ ] **POLISH-04**: OSC 7 (cwd), OSC 8 (hyperlinks), OSC 10/11/12 (color queries), and OSC 133 (semantic prompt marks) are implemented
- [ ] **POLISH-05**: OSC 52 clipboard copy works in both raw and DCS-wrapped forms (tmux pass-through compatibility)
- [ ] **POLISH-06**: Scrollback regex search with match highlighting and next/prev navigation
- [x] **POLISH-07**: Profiles — saved targets named `local`, `codespace`, `dev_tunnel` with per-profile env, theme, tint, and startup command
- [ ] **POLISH-08**: Secure Keyboard Entry toggle and basic IME composition display via `NSTextInputClient` (no candidate window UI; full IME is v2)

### GitHub Auth & Codespaces Picker

- [ ] **AUTH-01**: GitHub OAuth Device Flow (RFC 8628) sign-in works from inside the app — no browser plugin, no PAT pasting
- [ ] **AUTH-02**: OAuth tokens are stored in macOS Keychain via `keyring 4.0`; never written to disk in plaintext, never logged
- [ ] **AUTH-03**: Token refresh is handled silently; expired tokens trigger a re-auth prompt rather than silent failure
- [ ] **CS-01**: After sign-in, a Codespaces picker lists every codespace for the user with state (Available / Shutdown / Starting), repository name, branch, and last-used time
- [ ] **CS-02**: Selecting a Shutdown codespace from the picker triggers `POST /start`, polls until Available (with 409 swallowed), then connects
- [ ] **CS-03**: A picked codespace can be saved as a one-click profile that survives app restart

### Codespaces SSH Connect

- [ ] **CS-04**: Connecting to a codespace opens a remote shell in a Vector pane, end-to-end, via subprocess `gh codespace ssh --stdio` as the v1 transport
- [ ] **CS-05**: Vector generates and registers an SSH keypair (ed25519) per machine — no manual ssh-add dance for the user
- [ ] **CS-06**: A connected codespace tab is visually distinct (tinted tab + "remote" badge in the tab title) so the user always knows what they're typing into
- [ ] **CS-07**: Resize events propagate through the SSH transport (`window-change` request) so remote `vim`/`tmux` reflow correctly

### Dev Tunnels Connect

- [ ] **DT-01**: A 1–2 day spike at the start of the Dev Tunnels phase commits a written decision among (a) subprocess `code tunnel client`, (b) vendor `microsoft/dev-tunnels/rs/` at a pinned SHA, (c) defer to v2 — before any integration code is written
- [ ] **DT-02**: A signed-in user can list active Dev Tunnels alongside Codespaces in the picker
- [ ] **DT-03**: Connecting to a Dev Tunnel opens a remote shell in a Vector pane, end-to-end, using whichever transport the spike chose
- [ ] **DT-04**: Dev Tunnel sessions are visually distinct from Codespaces sessions (different tab tint color)

### Persistence & Reconnect

- [ ] **PERSIST-01**: On TCP/SSH disconnect, the affected pane enters a `Reconnecting` state, the local grid + scrollback are kept in memory, and a reconnect overlay is shown
- [ ] **PERSIST-02**: `Domain::reconnect()` re-establishes the transport with exponential backoff and hot-swaps the `PtyTransport` under the live `Pane` without dropping bytes already in flight
- [ ] **PERSIST-03**: Remote sessions auto-attach to a Vector-managed tmux session (`tmux new -A -s vector-{profile-id}`) so the remote shell state survives full disconnects
- [ ] **PERSIST-04**: tmux pass-through correctness is verified by an end-to-end smoke test (real tmux 3.4+ on a Codespace) — DCS-wrapped OSC 52, DECSCUSR, mouse modes, and `TERM` advertisement all round-trip cleanly

### Hardening & Release

- [ ] **HARDEN-01**: Renderer snapshot test suite runs headless against a pinned font and a perceptual-tolerance comparator; CI gate on regression
- [ ] **HARDEN-02**: VT conformance corpus (alt-screen, scroll regions, tab stops, ED/EL, mouse 1006, OSC 52 round-trip) runs in CI
- [ ] **HARDEN-03**: Tokens are redacted in logs by manual `Debug` impls on every token-bearing struct; `cargo deny` blocks a tree of crates that allow unaudited unsafe in the release profile
- [ ] **HARDEN-04**: Tagged release ships an unsigned Universal `.dmg` to GitHub Releases with install instructions front-and-center

## v2 Requirements

Deferred to a future release. Tracked but not in the current roadmap.

### Native Codespaces Transport

- **CS-V2-01**: Replace subprocess `gh codespace ssh --stdio` with native `russh` + `tonic` over the `cli/cli` port-16634 gRPC management API; vendor the `.proto` files from `cli/cli/internal/codespaces/rpc/`

### Distribution & Signing

- **DIST-V2-01**: Apple Developer ID signing + notarization workflow in CI (only if right-click-Open friction proves painful for teammates)
- **DIST-V2-02**: Sparkle-based auto-update on signed builds

### AI Assist (BYO-Key, Opt-In)

- **AI-V2-01**: Optional Claude API ghost-text autosuggest at the prompt; the Anthropic API key lives in Keychain; off by default
- **AI-V2-02**: Context for the autosuggest is bounded to cwd + last N commands + current line — never scrollback contents, never file contents

### Remote-Dev Surface Area

- **RDEV-V2-01**: Port-forwarding "PORTS" panel UX (auto-detect listening ports on the remote, forward locally)
- **RDEV-V2-02**: File transfer (drag-drop upload, scp-like UI)
- **RDEV-V2-03**: Codespaces lifecycle from inside the app (create / delete / rebuild)
- **RDEV-V2-04**: Arbitrary SSH targets as first-class profiles (alongside Codespaces and Dev Tunnels)

### Terminal Surface Area

- **TERM-V2-01**: Full IME (CJK candidate window UI) via complete `NSTextInputClient` integration
- **TERM-V2-02**: macOS dark/light mode auto-follow
- **TERM-V2-03**: Command palette (Cmd-Shift-P)
- **TERM-V2-04**: Quick terminal / hotkey window (system-wide drop-down)
- **TERM-V2-05**: ssh-terminfo auto-install on remote (ghostty-style)

### Cross-Platform

- **PLAT-V2-01**: Linux build (Wayland + X11)
- **PLAT-V2-02**: Windows build

## Out of Scope

Explicitly excluded. Documented to prevent scope creep — the four research dimensions all flagged scope creep as the project's biggest non-technical risk.

| Feature | Reason |
|---------|--------|
| Cloud account / login wall / telemetry | Anti-feature. Part of why we're not using Warp/Wave. |
| AI bundled by default | Optional and BYO-key only (v2 ambition); never bundled with default UI |
| Blocks UI / structured-output panes | Breaks tmux pass-through and TUI apps; explicit non-goal |
| Web companion (vscode.dev style) | The user explicitly hates browser dependencies — this is core to "why Vector" |
| Lua / Python / JS scripting config | TOML only; scripting attack surface + binary bloat |
| Plugin marketplace / extension system | Maintenance trap; we're a terminal, not a platform |
| File browser / sidebar / IDE features | Vector is a terminal, not an IDE; that's why VS Code exists |
| Sixel / Kitty graphics protocols | Rabbit hole; rare use cases; deferred indefinitely |
| Mosh-style custom remote agent | tmux on the remote covers the persistence requirement at 1% of the cost |
| Built-in package manager | Hard scope: we ship binaries, users install fonts/themes themselves |
| Forking ghostty or VS Code | Decided in PROJECT.md — building fresh in Rust |
| Adobe Developer ID signing | Deferred to v2 (DIST-V2-01); we ship unsigned for v1 |

## Traceability

Every v1 requirement maps to exactly one phase. No orphans, no duplicates.

| Requirement | Phase | Status |
|-------------|-------|--------|
| BUILD-01 | Phase 1 | Complete |
| BUILD-02 | Phase 1 | Complete |
| BUILD-03 | Phase 1 | Complete |
| BUILD-04 | Phase 1 | Complete |
| BUILD-05 | Phase 1 | Complete |
| WIN-05 | Phase 1 | Complete |
| CORE-01 | Phase 2 | Complete |
| CORE-02 | Phase 2 | Complete |
| CORE-03 | Phase 2 | Complete |
| CORE-04 | Phase 2 | Complete |
| CORE-05 | Phase 2 | Complete |
| CORE-06 | Phase 2 | Complete |
| RENDER-01 | Phase 3 | Complete |
| RENDER-02 | Phase 3 | Complete |
| RENDER-03 | Phase 3 | Complete |
| RENDER-04 | Phase 3 | Complete |
| RENDER-05 | Phase 3 | Complete |
| WIN-01 | Phase 3 | Complete |
| WIN-02 | Phase 4 | Complete |
| WIN-03 | Phase 4 | Complete |
| WIN-04 | Phase 4 | Complete |
| POLISH-01 | Phase 5 | Complete |
| POLISH-02 | Phase 5 | Pending |
| POLISH-03 | Phase 5 | Pending |
| POLISH-04 | Phase 5 | Pending |
| POLISH-05 | Phase 5 | Pending |
| POLISH-06 | Phase 5 | Pending |
| POLISH-07 | Phase 5 | Complete |
| POLISH-08 | Phase 5 | Pending |
| AUTH-01 | Phase 6 | Pending |
| AUTH-02 | Phase 6 | Pending |
| AUTH-03 | Phase 6 | Pending |
| CS-01 | Phase 6 | Pending |
| CS-02 | Phase 6 | Pending |
| CS-03 | Phase 6 | Pending |
| CS-04 | Phase 7 | Pending |
| CS-05 | Phase 7 | Pending |
| CS-06 | Phase 7 | Pending |
| CS-07 | Phase 7 | Pending |
| DT-01 | Phase 8 | Pending |
| DT-02 | Phase 8 | Pending |
| DT-03 | Phase 8 | Pending |
| DT-04 | Phase 8 | Pending |
| PERSIST-01 | Phase 9 | Pending |
| PERSIST-02 | Phase 9 | Pending |
| PERSIST-03 | Phase 9 | Pending |
| PERSIST-04 | Phase 9 | Pending |
| HARDEN-01 | Phase 10 | Pending |
| HARDEN-02 | Phase 10 | Pending |
| HARDEN-03 | Phase 10 | Pending |
| HARDEN-04 | Phase 10 | Pending |

**Coverage:**
- v1 requirements: 51 total (5 BUILD + 6 CORE + 5 RENDER + 5 WIN + 8 POLISH + 3 AUTH + 7 CS + 4 DT + 4 PERSIST + 4 HARDEN)
- Mapped to phases: 51 (100%)
- Unmapped: 0

---
*Requirements defined: 2026-05-10*
*Last updated: 2026-05-10 — Plan 01-06 closed: BUILD-04 (tagged-release half) and BUILD-05 (xattr in README) complete in commits 4dd0c4e + 75b77b1; BUILD-02 / BUILD-04 retain pending-real-CI-run / pending-real-tagged-release caveat per 01-05 + 01-06 Outstanding Verification Debt blocks*
*Last updated: 2026-05-12 — Plan 04-06 closed: WIN-02 + WIN-03 complete after smoke matrix re-run (items #3, #4, #8 PASS).*
