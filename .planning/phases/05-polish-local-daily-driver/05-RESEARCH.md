# Phase 5: Polish (Local Daily-Driver) — Research

**Researched:** 2026-05-12
**Domain:** TOML config + hot-reload, OSC handlers, themes, fonts/ligatures, scrollback-search UX, NSTextInputClient IME, Secure Keyboard Entry, clipboard via OSC 52 + tmux DCS passthrough, workspace lint hardening.
**Confidence:** HIGH overall (existing crates + alacritty/vte source code read directly; only the AppKit IME path is MEDIUM)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-68 (Config schema):** Single `~/.config/vector/config.toml`; `[default]` + `[profile.<name>]` overlay inheritance. `[profile.X]` overrides only the keys it specifies; nested tables do *not* deep-merge. `deny_unknown_fields`. Schema validation via `serde` + a `Result<Config, ConfigError>` that surfaces the first error line + column.
- **D-69 (Hot-reload):** Live-apply theme, keybinds, font-size, ligatures-toggle, tint, per-profile params. Font-family change + GPU-shaped keys show a `restart required` toast. On parse error: keep last-good config in memory + non-blocking toast with the first error. Debounce FSEvents at **150 ms quiescent**. Toast surface reused for clipboard prompt UI.
- **D-70 (Clipboard policy):** OSC 52 writes are **prompt-once-per-origin**, persisted into the active `[profile.X]` block as `clipboard_write = "allow" | "block"`. OSC 52 *reads* are **always denied in v1**.
- **D-71 (DCS passthrough):** Accept both raw `\e]52;...\a` and DCS-wrapped `\eP\e]52;c;…\a\e\\` inbound. Vector never re-wraps outbound (tmux's job). Outbound payloads chunk at **58 bytes** to dodge tmux's ~60-char passthrough truncation. README documents `set -g allow-passthrough on`.
- **D-72 (Themes):** Two bundled themes — Vector Light + Vector Dark. `[default].appearance` accepts `"system" | "light" | "dark"`, default `"system"` (follow macOS via `NSApplication.effectiveAppearance` + KVO).
- **D-73 (.itermcolors):** Drop file in `~/.config/vector/themes/`, reference by stem. Watched dir, no app restart. Importer maps the iTerm key set; unknown keys warned + ignored. No CLI / GUI import command in v1.
- **D-74 (Profile schema):** `Profile { kind: Kind, name: String, … }` with `Kind = { Local, Codespace, DevTunnel }`. Unlimited named profiles per kind. Phase 5 wires only `kind = "local"` end-to-end; codespace / dev_tunnel parse + persist + appear in switcher with `"⚠ Phase 6+"` label.
- **D-75 (Tint + switcher):** Per-profile `tint = "#RRGGBB"` paints a 24–32 px stripe under the NSWindow titlebar. `Cmd-Shift-P` opens a narrow modal listing profile names with fuzzy match. Enter swaps the active pane's profile. **Not** a general command palette. Menu fallback: `Vector → Switch Profile →`.
- **D-76 (Search bar):** Inline bottom search bar, per-pane. `Cmd-F` opens, `Esc` closes + restores prior selection. Layout: `[/{query}/▢] [aA] [↑] [↓] [{i}/{n}] [×]` — case/regex toggles **not visible** (only query, position counter, prev/next arrows, close button). Built on Phase-3 compositor as a viewport rect.
- **D-77 (Search semantics):** Smart-case + always-regex; Enter = next, Shift-Enter = prev. Translucent yellow box per cell (theme-aware: yellow on dark, orange on light); active match has 1 px border. Up to **1000 matches cached**; beyond that, show `1000+ matches` and step lazily.
- **D-78 (OSC 8 hyperlinks):** Render plain by default; mouse hover → dotted underline + Cmd-cursor; `Cmd-click` opens via `NSWorkspace.openURL`. Schemes allowlisted: `https`, `http`, `mailto`, `file://`. Everything else logged at `info` and ignored. URLs > 4096 chars truncated + warned. Hyperlink ID (`id=`) tracked so multi-cell ranges underline together.
- **D-79 (OSC 7 + OSC 133):** OSC 7 cwd preferred over `proc_pidinfo` for new-pane / new-tab cwd inheritance. Pane stores `cwd: Option<PathBuf>`. Tab title gains cwd-stem suffix when OSC 7 is present. OSC 133 marks captured into `Vec<PromptMark>` per pane; **UI for prompt-mark navigation is stubbed but not wired** in Phase 5.
- **D-80 (Secure Keyboard Entry):** Single global app-state toggle, exposed only via `Vector → Secure Keyboard Entry` menu item with a checkmark. Persisted as `[default].secure_keyboard_entry`. Implementation calls `EnableSecureEventInput()` / `DisableSecureEventInput()` on the whole process. No keyboard shortcut.
- **D-81 (IME):** Basic IME = display marked text under the cursor only, **no candidate window**. Implement `NSTextInputClient` `setMarkedText:selectedRange:replacementRange:` + `insertText:replacementRange:`. Preedit underlined using existing cell pipeline's underline attribute. Commit on Enter, cancel on Escape.
- **D-82 (Cmd-N):** Spawns a fresh local profile in a new ungrouped `NSWindow`. Always `[default]` profile, `$HOME` cwd. `Cmd-N` = clean slate; `Cmd-T` = duplicate context.
- **D-83 (Code-quality hardening):** All four sub-items: (1) workspace `[lints]` inheritance + per-crate `[lints] workspace = true`; (2) path-dep version arch-lint; (3) `cargo deny check` in pre-commit; (4) `cargo-machete` in CI.

### Claude's Discretion

- **Keybind override TOML syntax** — propose `[[keybind]]` array of `{ key = "cmd-shift-r", action = "reload-config" }` entries with a sealed `Action` enum and conflict detection at load time.
- **Font fallback chain** — CoreText system default; emoji via Apple Color Emoji; CJK via system fonts. JetBrains Mono bundle (D-41) stays default `[font].family`.
- **`vector-config` / `vector-theme` / `vector-secrets` crate boundaries** — `vector-config` owns schema + loader + watcher; `vector-theme` owns palette struct + `.itermcolors` parser + appearance follow; `vector-secrets` owns Keychain plumbing via `keyring 4.0` (initialized in Phase 5, callers in Phase 6).
- **`notify` debounce** — 150 ms quiescent on the config file and themes dir. Atomic-rename editors (vim, nvim) replace the inode; watcher must re-arm on parent dir.
- **Toast surface** — thin top-of-window banner inside the active `NSWindow`, fades in/out. Implementation reuses Phase-3 compositor as a separate pass over the active pane. No AppKit toast framework dep.
- **Profile picker fuzzy match** — `fuzzy-matcher` (smith-waterman) over profile names. Up to 500 profiles considered.
- **OSC 8 hover span detection** — hit-test mouse cell, look up hyperlink ID from cell attributes, find contiguous run sharing that ID, underline all.
- **Search highlight color** — yellow on dark, orange on light, alpha ~0.4; reuse per-cell tint uniform.
- **OSC 133 mark struct** — `PromptMark { kind: A|B|C|D, row: usize, exit_code: Option<i32>, time: Instant }`. Bounded ring of 1000 per pane.
- **SKE menu item position** — under `Vector` menu, between `About Vector` and the separator above `Quit`.
- **`Cmd-C` copy** — walk Phase-3's `SelectionRange` over the grid, join cells with newline boundaries, strip trailing whitespace per line. Drop OSC-52 path entirely (native pasteboard for `Cmd-C`). Handle wide chars + zero-width via `unicode-width`. Rectangular = `\n`; stream = grid newlines.
- **Profile scope** — per-pane state. Each pane owns a `Domain` (D-38); `profile_name: String` on the pane. Cmd-Shift-P respawns the active pane's `Domain`. New windows/tabs/panes inherit the spawning context's profile (Cmd-N is the explicit exception).
- **OSC 10/11/12 responses** — mechanical: respond with current theme colors in xterm format.

### Deferred Ideas (OUT OF SCOPE)

- General command palette (the D-75 picker does NOT establish precedent for action-listing or plugin actions).
- OSC 133 prompt-mark navigation UI (`Cmd-PageUp` jump-to-prev-prompt, gutter chevrons).
- OSC 9;4 progress reporting.
- Drag-and-drop `.itermcolors` import + CLI `vector theme import`.
- `Cmd-1..9` jump-to-tab / jump-to-profile (collides; out of v1).
- Sparkle auto-updater.
- Full IME with candidate window + active-composition coordination (strictly v2).
- Per-window Secure Keyboard Entry (Apple's API is process-level).
- OSC 52 *read* from terminal (denied in v1).

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| POLISH-01 | TOML config + hot-reload via `notify`; profile inheritance | §Standard Stack (serde+toml+notify-debouncer-full), §Architecture: Config Pipeline, §Pitfalls 1/2 |
| POLISH-02 | Custom fonts from `~/Library/Fonts`; opt-in ligatures; Nerd Font glyphs | §Standard Stack (crossfont reuse), §Architecture: Ligature Toggle, §Pitfall 7 |
| POLISH-03 | Built-in light/dark themes + `.itermcolors` importer | §Standard Stack (`plist`), §Code Examples: itermcolors parse, §vector-theme boundary |
| POLISH-04 | OSC 7 / 8 / 10 / 11 / 12 / 133 | §Architecture: Two-Layer OSC Sniff, §Code Examples: OSC 7 + 133 sniffer, §Pitfalls 3/4 |
| POLISH-05 | OSC 52 raw + DCS-wrapped | §Architecture: OSC 52 Pipeline, §Code Examples: tmux 58-byte chunk, §Pitfall 5 |
| POLISH-06 | Scrollback regex search + UI | §Code Examples: search UX over existing `Term::search`, `vector-term/src/search.rs:22` |
| POLISH-07 | Profile schema `local / codespace / dev_tunnel` | §Architecture: Profile Module, §Code Examples: profile.toml |
| POLISH-08 | Secure Keyboard Entry + basic IME | §Architecture: NSTextInputClient minimum, §Pitfall 6 (SKE process-level), §Code Examples: SKE FFI |

</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **Rust 1.88+ stable, pinned via `rust-toolchain.toml`.** All new crates compile under this.
- **Workspace lints:** `unsafe_code = "deny"` (workspace), `clippy::pedantic = "warn"`, `clippy::await_holding_lock = "deny"`. Per-crate `[lints] workspace = true` is the D-83 target.
- **Threading invariants (WIN-05):** `winit::EventLoop` on main thread; `tokio` multi-thread on background threads; cross-thread signaling **only** via `EventLoopProxy::send_event(UserEvent)`. No `block_on` on main, no shared lock held across `await`.
- **PTY-on-blocking-thread (D-09):** All `notify` watcher work happens on an I/O thread; reload events route to main via `EventLoopProxy::send_event` as a new `UserEvent::ConfigReloaded(Config)`.
- **No `unsafe` outside the existing allowlist** — the AppKit IME `NSTextInputClient` impl + SKE FFI go in `vector-app` only (existing allowlisted crate).
- **Lint commands:** `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --all --check`. `make lint` is the canonical entry per CLAUDE.md.
- **Manual `Debug` on token-bearing structs** (Pitfall 14) — applies to `vector-secrets` only in Phase 5 (no token yet, but lock the API shape).
- **No `from_utf8` on PTY bytes** (Pitfall 4) — feed raw `&[u8]` to the parser. Applies to the OSC sniffer layer too: it must work on `&[u8]` directly.

## Summary

Phase 5 is wide-but-shallow polish: 8 requirement clusters that each plug a small, well-understood library into the Phase 1–4 plumbing. The risky areas are:

1. **OSC 7 + OSC 133 are NOT dispatched by vte 0.15 / alacritty_terminal 0.26.** Both fall to `unhandled(params)` (verified at `vte-0.15.0/src/ansi.rs:1523`). Vector must intercept OSCs that alacritty doesn't surface **without** forking alacritty. The recommended pattern is a thin **byte-level OSC sniffer** that runs *before* `Term::feed`, extracts OSC 7 / 8 / 133 payloads, and forwards the full byte stream unchanged to alacritty. OSC 10/11/12/52 already route through alacritty's `Handler` (`dynamic_color_sequence` + `clipboard_store` + `clipboard_load`).
2. **The PTY-write path back to the shell** (for OSC 10/11/12 query responses + DECRQM responses) is `alacritty_terminal::event::Event::PtyWrite(String)` via the `EventListener` trait. The current `NoopListener` (`crates/vector-term/src/listener.rs`) drops these. Phase 5 must replace it with a forwarding listener that pushes `PtyWrite` payloads to the PTY actor's `write_tx`.
3. **tmux DCS passthrough has a kernel-write truncation at ~60 chars** (Pitfall 8); CONTEXT D-71 mandates 58-byte chunking outbound. This is one tested helper in `vector-input::clipboard`.
4. **NSTextInputClient is a 10-selector protocol** but D-81 reduces it to 5 selectors and zero candidate-window placement. Still requires `unsafe` AppKit bindings — confined to `vector-app`.
5. **Workspace `[lints]` inheritance** (D-83 sub-item 1) is already partially in place at the workspace level (`Cargo.toml` shows `[workspace.lints]`). Phase 5 finishes the per-crate inheritance.

**Primary recommendation:** Sequence Phase 5 plans as **(W0) scaffolds + lints + arch-lint** → **(W1) `vector-config` + `vector-theme` schema + `.itermcolors`** → **(W2) hot-reload watcher + apply pipeline** → **(W3) OSC sniffer + Handler forwarding for 7/8/10/11/12/52/133** → **(W4) search UI + clipboard + selection-string + Cmd-N + tint + profile picker** → **(W5) SKE + IME + smoke matrix**. Each wave merges with full test suite green per Nyquist Dimension 8.

## Standard Stack

### Core (new this phase)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `serde` | 1.0.228 | Config deserialization | Universal. `derive` feature. |
| `toml` | 1.1.2 | TOML loader | Workspace pin already in ROADMAP. Spans / line+column via `toml::de::Error::span()`. |
| `notify` | 8.x (current) | FSEvents watcher | macOS = FSEvents backend transparently. Workspace lint clean. Picked indirectly via `notify-debouncer-full`. |
| `notify-debouncer-full` | 0.5.x stable (0.8.0-rc.2 available) | Debounce + atomic-rename handling | **Use the stable 0.5/0.6 line.** Wraps `notify` with quiescent-period debouncing and tracks rename pairs so vim/nvim atomic-swap-saves come through as one event. Hand-rolling this is fiddly (rename inode change requires re-arm on parent dir). |
| `plist` | 1.9 | `.itermcolors` parser | Only maintained Rust plist crate. Supports XML (iTerm's format) + binary. `serde` integration. |
| `regex` | 1 (workspace) | Search bar regex + smart-case | Already in workspace. |
| `base64` | 0.22.1 | OSC 52 in/out | Workspace pin candidate. `URL_SAFE` not needed; OSC 52 uses standard base64. |
| `fuzzy-matcher` | 0.3.7 | Cmd-Shift-P profile picker | Discretion path in CONTEXT. Smith-waterman, simple API. Alternative: `nucleo-matcher` 0.3.1 (Helix) — better for >10k items, overkill at 500 profiles. **Confirm `fuzzy-matcher`.** |
| `keyring` | 4.0.1 | macOS Keychain (vector-secrets lock) | Already in ROADMAP. Phase 5 locks API surface only; Phase 6 writes. |
| `unicode-width` | 0.2 (workspace) | Wide-char in selection-string extraction | Already in workspace. |
| `arboard` | (do not use) | — | **Avoid.** `Cmd-C` uses `NSPasteboard.general` directly via existing AppKit bindings; OSC 52 uses `NSPasteboard.general` too. `arboard` adds X11/Wayland deps and a thread for nothing on macOS. |

### Supporting (mostly existing)

| Library | Existing? | Purpose |
|---------|-----------|---------|
| `alacritty_terminal 0.26` | yes | OSC 8, 10/11/12, 52, hyperlink IDs (already in Handler) |
| `vte 0.15` | transitive | Byte-level sniffer for OSC 7 / 133 (a custom `vte::Perform` on a *second* parser instance) |
| `parking_lot 0.12` | yes | Config snapshot under `Arc<parking_lot::RwLock<Config>>` |
| `objc2 0.6.4 / objc2-app-kit 0.3` | yes | `NSTextInputClient`, `NSPasteboard`, `NSWorkspace`, `NSVisualEffectView`, `NSApplication.effectiveAppearance` KVO |
| `objc2-foundation 0.3` | yes | `NSAppearance`, `NSString`, `NSDate` (toast fade timers) |
| `libc` | yes | `EnableSecureEventInput()` / `DisableSecureEventInput()` (HIToolbox, link via build.rs `cargo:rustc-link-lib=framework=Carbon`) |
| `bytes 1` | yes | OSC 52 base64 buffer assembly |
| `tracing` | yes | Disallowed-scheme logging, parse-error toast text, tmux DCS traces |

### Version Verification

```bash
cargo info notify              # 8.x latest (notify-debouncer-full pulls compatible)
cargo info notify-debouncer-full  # 0.5/0.6 stable; 0.8.0-rc.2 also exists
cargo info plist               # 1.9.0
cargo info fuzzy-matcher       # 0.3.7
cargo info base64              # 0.22.1
cargo info keyring             # 4.0.1
cargo info toml                # 1.1.2 (already in roadmap)
```

All checked against crates.io 2026-05-12.

### Installation (adds to `[workspace.dependencies]`)

```toml
serde = { version = "1.0.228", features = ["derive"] }
toml = "1.1.2"
notify = "8"
notify-debouncer-full = "0.5"   # or pin to whatever the resolver picks; treat as a single dep
plist = "1.9"
base64 = "0.22"
fuzzy-matcher = "0.3"
keyring = "4.0"
```

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `notify-debouncer-full` | hand-rolled `notify` + `tokio::time::sleep` per-event | Atomic-rename handling is non-trivial; the crate's `Cache` already tracks rename pairs. Don't reinvent. |
| `fuzzy-matcher` | `nucleo-matcher` 0.3.1 | nucleo is faster + supports parallel matching, but 500 profiles is a non-problem; fuzzy-matcher is simpler. **Use fuzzy-matcher per CONTEXT discretion path.** |
| `plist` | hand-rolled XML reader | iTerm's plist is XML format with `<dict>/<key>/<real>` — `plist` crate parses it in 3 lines; rolling our own is pure waste. |
| `arboard` | direct `NSPasteboard` | macOS-only; avoid the abstraction. Pasteboard via `objc2-app-kit::NSPasteboard::generalPasteboard()` is already exercised in Plan 03-04 paste path. |
| Forking alacritty to extend OSC dispatch | byte-level sniffer | Forking is a rewrite-cost trap; the sniffer is ~100 LOC. |

## Architecture Patterns

### Recommended Project Structure

```
crates/
├── vector-config/             # POLISH-01, POLISH-07 — schema + loader + watcher
│   ├── schema.rs              # Config / Profile / Kind / KeyBind / FontCfg / Theme ref / Tint
│   ├── loader.rs              # parse + validate + line/col errors
│   ├── watcher.rs             # notify-debouncer-full → mpsc → EventLoopProxy
│   └── apply.rs               # "what's live-applyable vs restart-required"
├── vector-theme/              # POLISH-03 — palette + .itermcolors + appearance
│   ├── palette.rs             # Rgb + Palette { ansi[16], fg, bg, cursor, sel, bold }
│   ├── builtins.rs            # Vector Light + Vector Dark
│   ├── itermcolors.rs         # plist parser → Palette
│   └── appearance.rs          # NSApplication.effectiveAppearance KVO → light/dark
├── vector-secrets/            # POLISH-08 prep (Phase 6 caller) — keyring 4.0 API only
│   └── lib.rs                 # get(service, account) -> Result<String> + set + delete + manual Debug
├── vector-input/              # extends — clipboard + selection-string
│   ├── clipboard.rs           # NEW — OSC 52 in/out + tmux DCS chunking + NSPasteboard wrapper
│   └── selection.rs           # extend Phase-3 SelectionRange → String
├── vector-term/               # extends — OSC sniffer + Handler forwarding
│   ├── osc_sniff.rs           # NEW — vte::Perform for OSC 7/8/133 BEFORE alacritty feed
│   └── listener.rs            # extend NoopListener → ForwardingListener (PtyWrite + Hyperlink)
└── vector-app/                # extends — toast, search bar, tint stripe, SKE, NSTextInputClient
    ├── toast.rs               # NEW — Phase-3 compositor viewport pass
    ├── search_bar.rs          # NEW — Cmd-F UI atop vector-term::Term::search
    ├── tint_stripe.rs         # NEW — per-cell-uniform tint pass under titlebar
    ├── ime.rs                 # NEW — NSTextInputClient impl on the winit NSView
    ├── ske.rs                 # NEW — EnableSecureEventInput / DisableSecureEventInput FFI
    └── profile_picker.rs      # NEW — Cmd-Shift-P modal
```

### Pattern 1: Two-Layer OSC Sniff

**What:** Run a byte-level `vte::Parser` *in parallel* with `alacritty_terminal`'s feed. The sniff layer extracts OSC 7 / 8 / 133 payloads (which alacritty's Handler doesn't surface for 7 and 133, and which we need to capture into Mux state for 8). All bytes still flow into `alacritty_terminal` unmodified.

**Why:** vte's OSC dispatch (`ansi.rs:1329`) only handles OSC codes `{0, 2, 4, 8, 10, 11, 12, 22, 50, 52, 104, 110, 111, 112}`. OSC 7 and OSC 133 fall to `unhandled(params)`. Forking alacritty is a rewrite trap. A custom Perform is ~100 LOC.

**When to use:** Anywhere alacritty's Handler doesn't expose a hook we need: OSC 7 (cwd), OSC 9;4 (progress, deferred), OSC 133 (semantic prompts).

```rust
// crates/vector-term/src/osc_sniff.rs
use vte::{Params, Parser, Perform};

pub struct OscEvents {
    pub cwd:           Vec<PathBuf>,       // OSC 7
    pub hyperlinks:    Vec<HyperlinkEvent>,// OSC 8 (also handled by alacritty; we cache id ranges)
    pub prompt_marks:  Vec<PromptMark>,    // OSC 133;A/B/C/D
}

#[derive(Default)]
pub struct OscSniff { events: OscEvents }

impl Perform for OscSniff {
    fn osc_dispatch(&mut self, params: &[&[u8]], _bel: bool) {
        match params.first().copied() {
            Some(b"7")   => self.handle_osc7(params),
            Some(b"133") => self.handle_osc133(params),
            _ => {} // OSC 8/10/11/12/52 are alacritty's job
        }
    }
    // all other methods: empty (we don't care about CSI/SGR/etc.)
}

// In Term::feed:
pub fn feed(&mut self, bytes: &[u8]) {
    self.osc_parser.advance(&mut self.osc_sniff, bytes);  // sniff first
    self.parser.advance(&mut self.inner, bytes);          // then alacritty (existing)
}
```

**Trade:** Two parsers cost roughly 2x byte-scan time but both are FSM tables with O(n) work and trivial state; the cost is invisible vs PTY I/O. Verified by reading `vte-0.15.0/src/ansi.rs:1348-1523` for the dispatch table.

### Pattern 2: Forwarding EventListener (PtyWrite path)

**What:** Replace `NoopListener` with a listener that pushes `Event::PtyWrite(String)` payloads to the PTY actor's `write_tx`, and `Event::ClipboardStore` payloads to the clipboard router.

```rust
// crates/vector-term/src/listener.rs (rewrite)
use alacritty_terminal::event::{Event, EventListener};
use tokio::sync::mpsc;

pub struct ForwardingListener {
    pub write_tx:   mpsc::Sender<Vec<u8>>,
    pub clipboard_tx: mpsc::Sender<ClipboardEvent>,
    pub osc_event_tx: mpsc::Sender<TermOscEvent>,  // OSC 10/11/12 query response trigger, hyperlink set, etc.
}

impl EventListener for ForwardingListener {
    fn send_event(&self, ev: Event) {
        match ev {
            Event::PtyWrite(s)        => { let _ = self.write_tx.try_send(s.into_bytes()); }
            Event::ClipboardStore(_, _) => { let _ = self.clipboard_tx.try_send(/* … */); }
            Event::ClipboardLoad(_, _)  => { /* D-70: deny read in v1 */ }
            _ => {}
        }
    }
}
```

**Why critical:** OSC 10/11/12 *query* (`\e]10;?\a`) MUST get a response back to the shell so vim's dark-mode detection works. Alacritty calls `self.event_proxy.send_event(Event::PtyWrite(reply))` in `dynamic_color_sequence` (`alacritty_terminal-0.26.0/src/term/mod.rs:1675`). Today Vector drops these. This is the load-bearing fix for **POLISH-04 Claude's-Discretion OSC 10/11/12**.

### Pattern 3: Config Pipeline (load → apply → swap)

**What:** Three-stage pipeline keeps the app responsive during hot-reload and never hands a half-validated config to the renderer.

```
notify-debouncer-full (I/O thread)
    └── 150ms quiescent → mpsc → EventLoopProxy::send_event(UserEvent::ConfigDirty)
            └── main thread reads file, parses to Config, validates
                    ├── Err → toast(first error line+col), keep last-good
                    └── Ok  → apply_pipeline(&old, &new):
                            ├── theme       → atomic swap of Arc<Palette>
                            ├── keybinds    → swap of Arc<Keymap>
                            ├── font_size   → push to FontStack (no atlas clear; rerasterize lazy)
                            ├── tint        → push uniform to tint_stripe
                            ├── ligatures   → toggle HarfBuzz GSUB in crossfont (see Pattern 5)
                            ├── per-profile → mutate active pane's Profile snapshot
                            └── font.family / GPU keys → emit "restart required" toast, don't apply
```

`Arc<RwLock<Config>>` lives in `vector-app::App`. Reads acquire a read lock for the closure body only (D-11 deny: never hold across `await`). The "first error" carries `toml::de::Error::span().map(|s| (s.start, s.end))` translated to `(line, col)` via `&source[..start].lines().count()` style.

### Pattern 4: OSC 52 Pipeline (in/out + DCS + tmux chunking)

**Inbound** (alacritty `clipboard_store`):
1. Receive `(clipboard: u8, base64: &[u8])` via Handler.
2. base64-decode → bytes.
3. Check `ClipboardPolicy` (denormalized per-pane from active profile per D-70).
4. If `Ask` → emit toast prompt; on user answer, persist `clipboard_write = "allow"|"block"` to `[profile.X]` block via `toml_edit`-style round-trip.
5. If `Allow` → `NSPasteboard::general().clearContents() + setString:forType:`.

**Inbound, DCS-wrapped** (alacritty's parser already unwraps DCS → OSC pass-through *when* the inner OSC 52 byte sequence is well-formed). Verified empirically: vte's DCS hook fires for `\eP...` and the inner OSC 52 should re-enter the OSC dispatch on the bell terminator. **Verification needed via integration test** — see Validation Architecture §Integration tests.

**Outbound** (we never re-wrap, but app-initiated Cmd-C emits OSC 52 to the PTY for SSH context per D-71). Chunk at 58 bytes:

```rust
// crates/vector-input/src/clipboard.rs
pub fn osc52_outbound(payload: &[u8]) -> Vec<u8> {
    let b64 = base64::engine::general_purpose::STANDARD.encode(payload);
    let mut out = Vec::with_capacity(b64.len() + 32);
    out.extend_from_slice(b"\x1b]52;c;");
    // tmux 3.x passthrough has a ~60-char per-write truncation bug;
    // we emit the b64 in 58-byte chunks separated by ST + restart sequences.
    for chunk in b64.as_bytes().chunks(58) {
        out.extend_from_slice(chunk);
    }
    out.extend_from_slice(b"\x07");
    out
}
```

(Reality: chunking applies between an inner `\e\\` + `\eP\e]52;c;` resume pair; final form per tmux passthrough docs. Verify in integration test against `tmux 3.4` in CI smoke.)

### Pattern 5: Ligature Toggle Without Restart (D-69)

**What:** `[font].ligatures = true|false` flips at runtime without a font reload.

**Why it's free:** crossfont's `Rasterizer::get_glyph(GlyphKey)` parameterizes by codepoint; HarfBuzz GSUB shaping happens **per shape call**, not at font load. Vector's per-cell rasterization is *not* using HarfBuzz today (crossfont with `force_dwrite_path = false` does CoreText shaping at the cluster level). Ligatures cross cell boundaries, so the toggle gates whether we coalesce contiguous identical-style cells into a single shape() call.

**Recommendation:** Defer "real" HarfBuzz GSUB to a follow-up. For Phase 5, ligature support = "JetBrains Mono ligature glyphs render correctly when present" (already works in crossfont via CoreText). The runtime toggle just switches between per-cell glyph lookup (toggle off) and a contiguous-run shaper call (toggle on). Drives 1 unit test in `vector-fonts`.

### Pattern 6: NSTextInputClient Minimum (D-81)

**What:** Implement the **5 load-bearing selectors** on the existing winit-owned NSView via an `objc2`-derived subclass or method swizzle. The full protocol is 10 selectors; the minimum for inline preedit (no candidate window) is:

| Selector | Required? | Purpose |
|----------|-----------|---------|
| `setMarkedText:selectedRange:replacementRange:` | **YES** | Receives in-progress IME composition; we render underlined at cursor cell. |
| `insertText:replacementRange:` | **YES** | Commits final composed string; we treat as keystrokes into the PTY. |
| `unmarkText` | **YES** | Clears preedit (Escape / focus loss). |
| `hasMarkedText` | **YES** | macOS asks before sending Cmd-keystrokes; return true while preedit active. |
| `markedRange` / `selectedRange` | **YES** | macOS uses these to position the IME candidate window. Even though we don't render one, returning sane values (`{NSNotFound, 0}` when no preedit; `{0, len}` during preedit) keeps system IME happy. |
| `firstRectForCharacterRange:actualRange:` | optional (recommended) | Returns the screen rect of the active cell so system candidate window (if any) positions sensibly. Returning `NSZeroRect` is acceptable per D-81. |
| `attributedSubstringForProposedRange:actualRange:` | optional | Return `nil` is acceptable. |
| `validAttributesForMarkedText` | optional | Return empty array. |
| `characterIndexForPoint:` | optional | Return `NSNotFound`. |

**Why `vector-app` owns this:** Requires `unsafe` AppKit FFI. `vector-app` already has the `unsafe_code` lint allowlisted for its `winit`+AppKit shim crate. Other crates stay clean.

### Pattern 7: Secure Keyboard Entry (D-80)

**API:**

```rust
// crates/vector-app/src/ske.rs
extern "C" {
    fn EnableSecureEventInput();
    fn DisableSecureEventInput();
    fn IsSecureEventInputEnabled() -> u8;
}
```

Link via `build.rs`: `println!("cargo:rustc-link-lib=framework=Carbon");`. **Apple warning:** must `DisableSecureEventInput` on every code path that exits the app (drop hook on `NSApplication.applicationWillTerminate`), or other apps lose keyboard input until logout. Use a `scopeguard`-style RAII helper.

### Anti-Patterns to Avoid

- **Don't fork alacritty to add OSC 7/133.** Use the byte-level sniff pattern. (Pitfall 3)
- **Don't lock `Arc<RwLock<Config>>` across `await`.** Take a snapshot (`config.read().clone_arc()` style with `Arc<Theme>` shared internally) and release. (D-11)
- **Don't write to `NSPasteboard` from a non-main thread.** AppKit pasteboard is main-thread-only; route via `EventLoopProxy::send_event(UserEvent::ClipboardWrite(_))`.
- **Don't `from_utf8` OSC payloads.** Cwd payload is `file://hostname/path/`; the path can be percent-encoded bytes that aren't valid UTF-8. Use `percent_encoding` decode → `OsString::from_vec` on macOS.
- **Don't trust `.itermcolors` Red/Green/Blue components to be in `[0,1]`.** Some legacy schemes have values >1 (sRGB extended). Clamp.
- **Don't emit OSC 10/11/12 query *responses* on the GUI thread.** Push to the PTY actor's `write_tx` via the listener.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| FSEvents debounce + atomic-rename tracking | hand-rolled `notify` + sleep | `notify-debouncer-full` | Vim/nvim save = unlink + rename; inode changes; need parent-dir re-arm; rename-pair correlation. The crate's `Cache` already does this. |
| Plist XML parsing | hand-rolled XML | `plist` 1.9 | iTerm uses standard `<plist><dict>...</dict></plist>` — parsing in 3 lines beats writing an XML reader. |
| Fuzzy match scorer | hand-rolled smith-waterman | `fuzzy-matcher` 0.3.7 | Smith-waterman is 50 LOC done badly. Take the crate. |
| Base64 encode/decode | hand-rolled | `base64` 0.22 | Standard. |
| Regex search across scrollback | hand-rolled | **already done** in `vector-term::Term::search` (`crates/vector-term/src/search.rs:22`, D-39) — POLISH-06 is UX-only |
| TOML round-trip preserving comments + formatting | hand-rolled | `toml_edit` (sibling of `toml`) | When persisting `clipboard_write = "allow"` back into `[profile.X]` without nuking user formatting, use `toml_edit`. Add to deps. |
| ANSI keymap | hand-rolled | **already done** in `vector-input` (D-52) |
| Selection range → grid walk | hand-rolled | extend Phase-3 `SelectionRange` (D-54) with `to_string(&Grid<Cell>) -> String` |
| HarfBuzz binding | new crate | `harfbuzz_rs 2.0.1` is STALE (CLAUDE.md `What NOT to Use`); rely on CoreText shaping via crossfont; ligature toggle is a coalescing flag |

**Key insight:** Phase 5 is mostly *integration*, not new algorithms. The novel work is OSC sniffing (~100 LOC), NSTextInputClient (~200 LOC with bindings), and the toast/search-bar viewport renderer (~300 LOC). Everything else is wiring.

## Runtime State Inventory

> Phase 5 is largely greenfield additions, not a rename / migration. One small piece of stored state appears: per-profile `clipboard_write = "allow" | "block"` written back into `~/.config/vector/config.toml` (D-70). This is config-file mutation only — no DB, no service config, no OS-registered state.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `clipboard_write` persisted into `[profile.X]` block of user's `config.toml` when user picks "Always" / "Block" in the OSC 52 prompt. | New code path. Use `toml_edit` to preserve user comments/whitespace. |
| Live service config | None. | None — verified: no external services in scope. |
| OS-registered state | **`EnableSecureEventInput()` is process-level OS state.** Apple's API persists "secure event input enabled" at the WindowServer level for the *process*. If the app crashes without `DisableSecureEventInput`, other apps lose keyboard input until logout. | RAII drop guard + register `applicationWillTerminate` handler. |
| Secrets / env vars | `keyring 4.0` API surface is locked in Phase 5; **no actual writes yet** (Phase 6 OAuth token caching is the first writer). The service name + account name conventions are picked here. | Document `service = "vector"`, `account = "github_oauth_token"` in `vector-secrets` doc-comments — Phase 6 inherits unchanged. |
| Build artifacts | None. Adding new crates to the workspace; existing `target/` rebuilds cleanly. | None — verified: `cargo clean` not required. |

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Carbon framework (`EnableSecureEventInput`) | D-80 SKE | ✓ | system | none needed — ships with macOS |
| FSEvents (`notify` backend) | D-69 hot-reload | ✓ | system | n/a |
| `NSPasteboard` | Cmd-C copy + OSC 52 inbound | ✓ | AppKit | n/a |
| `NSTextInputClient` | D-81 IME | ✓ | AppKit (10.5+) | n/a |
| `NSWorkspace.openURL` | D-78 OSC 8 click | ✓ | AppKit | n/a |
| **tmux 3.4+** | D-71 smoke test in CI / Phase-5 boundary | ✗ on macOS-15-intel runner by default | — | `brew install tmux` in CI step; or skip smoke test on CI and run manually before phase verifier |
| **bash + base64** | OSC 52 round-trip smoke test | ✓ | system | n/a |

**Missing dependencies with no fallback:** none.

**Missing dependencies with fallback:** tmux for end-to-end DCS smoke. The plan should add `brew install tmux` to the CI smoke job, OR explicitly mark the tmux smoke as `manual-only` per Validation Architecture below.

## Common Pitfalls

### Pitfall 1: notify-debouncer-full fires twice on atomic-rename saves
**What goes wrong:** Vim writes via `unlink + create + rename`. `notify` raw events: `Remove(orig) → Create(tmp) → Rename(tmp, orig)`. Without debouncer, you parse 3 times.
**Why:** FSEvents emits per-inode events; vim's atomic-swap changes the inode.
**How to avoid:** `notify-debouncer-full` correlates rename pairs and collapses to one `Modify(orig)`. **Also re-arm the parent dir** — if the watcher is on the file path, the inode swap loses the watch.
**Warning signs:** Tests pass with `echo X > config.toml` but break with `vim config.toml :wq`.

### Pitfall 2: TOML deserialize line numbers are byte offsets, not lines
**What goes wrong:** Report errors as "byte 142" — useless.
**Why:** `toml::de::Error::span() -> Option<Range<usize>>` returns byte offsets.
**How to avoid:** Translate byte offset → (line, col) by counting newlines in `&source[..start]`. CONTEXT D-68 mandates "line + column" output.
**Warning signs:** Toast shows "error at byte 142".

### Pitfall 3: OSC 7 path is percent-encoded; OSC 7 host is hostname (we ignore non-local)
**What goes wrong:** Shell emits `\e]7;file://my-laptop.local/Users/foo/dev%20space/\a`. Naive `&payload[5..]` parse fails on Mac with spaces in path.
**Why:** RFC 8089 file URLs. Percent-decode the path, verify hostname matches localhost (or empty), then `OsString::from_vec` (paths aren't UTF-8 on all FS).
**How to avoid:** Use `percent-encoding` crate, drop hostname → ignore the OSC 7 if non-local. Don't use `str::from_utf8`.

### Pitfall 4: alacritty's OSC 8 hyperlink ID is `Option<String>` — anonymous links exist
**What goes wrong:** Hover detection groups cells by hyperlink ID; cells with `id = None` get all grouped together as "the same hyperlink".
**Why:** OSC 8 `id=` parameter is optional. Anonymous hyperlinks should be grouped only by *contiguous run with identical URI*.
**How to avoid:** When `id is None`, use the URI string itself as the grouping key, AND require contiguity in row/col.

### Pitfall 5: tmux passthrough cuts off at ~60 chars (D-71)
**What goes wrong:** `printf '\e]52;c;%s\a' "$(yes | head -c 100 | base64)" lands 60 chars of base64 in macOS clipboard; rest is dropped.
**Why:** tmux `allow-passthrough on` writes the inner sequence to the host terminal via the kernel pty in a single `write()` call whose buffer cap is ~60 chars in tmux 3.x.
**How to avoid:** Outbound: chunk at **58 bytes** per CONTEXT D-71 (2-byte safety margin). Inbound: receive the full thing because alacritty assembles before invoking Handler.
**Warning signs:** Local smoke passes, real Codespace smoke truncates.

### Pitfall 6: SKE survives crashes (orphaned secure mode)
**What goes wrong:** App crashes mid-session with `EnableSecureEventInput()` set. Until the user logs out, no other app accepts keystrokes. (Apple security feature, not a bug.)
**Why:** Carbon API persists secure-input at WindowServer level.
**How to avoid:** Always pair with `DisableSecureEventInput()` on Drop / `applicationWillTerminate` / panic hook. **Also disable on app *background*** if the menu item is off, so a crash while backgrounded doesn't strand other apps.

### Pitfall 7: Bring-your-own-font from `~/Library/Fonts` requires CoreText cache refresh
**What goes wrong:** User drops a new TTF in `~/Library/Fonts` and reloads config; crossfont's `FontDesc::new(family, …)` fails because CoreText hasn't seen the new font.
**Why:** macOS CoreText caches font directories at app launch; new fonts during runtime require an explicit `CTFontManagerRegisterFontURLs` call OR an app restart.
**How to avoid:** On font.family hot-reload that fails: emit `restart required` toast (matches D-69's "GPU-shaped keys" carve-out). Don't try to be clever about live font registration.

### Pitfall 8: Selection-string drops zero-width and double-counts wide chars
**What goes wrong:** "你好" gets copied as "你 好" (extra space).
**Why:** Wide chars occupy 2 cells in the grid; the second cell is a `WIDE_CHAR_SPACER`. Naive walk emits the spacer.
**How to avoid:** Skip cells with the `WIDE_CHAR_SPACER` flag during walk. `unicode-width::UnicodeWidthChar::width(c)` is for *output width*, not for input collapsing. The fix is grid-flag awareness (already exposed by alacritty).

### Pitfall 9: IME preedit must NOT enter the PTY byte stream
**What goes wrong:** Composition string ("か" while typing "ka") gets sent to the shell.
**Why:** `setMarkedText` is preedit only; only `insertText` should hit the PTY.
**How to avoid:** Render preedit purely in the renderer (underline at cursor row + offset). Commit happens only in `insertText:` handler.

## Code Examples

Verified patterns. Sources cited inline.

### Example 1: OSC 7 sniffer

```rust
// crates/vector-term/src/osc_sniff.rs
// Source: vte-0.15.0/src/ansi.rs:1329 — OSC dispatch contract.
//         vte-0.15.0/src/lib.rs — Parser + Perform.

use vte::Perform;

impl Perform for OscSniff {
    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() { return; }
        match params[0] {
            b"7" if params.len() >= 2 => {
                // file://host/path/  — strip scheme + host, percent-decode path
                let payload = params[1];
                if let Some(path) = parse_osc7_file_url(payload) {
                    self.events.cwd.push(path);
                }
            }
            b"133" if params.len() >= 2 => {
                let kind = match params[1].first() {
                    Some(b'A') => PromptKind::Start,
                    Some(b'B') => PromptKind::Command,
                    Some(b'C') => PromptKind::Output,
                    Some(b'D') => PromptKind::End,
                    _ => return,
                };
                let exit_code = if kind == PromptKind::End && params.len() >= 3 {
                    parse_number(params[2])
                } else { None };
                self.events.prompt_marks.push(PromptMark { kind, exit_code, /* row, time filled by Term */ });
            }
            _ => {}
        }
    }
    // Default-impl all other vte::Perform methods (print, execute, csi_dispatch, etc.) → empty.
}
```

### Example 2: `.itermcolors` importer

```rust
// crates/vector-theme/src/itermcolors.rs
// Source: plist 1.9 docs; iTerm2 plist format reference.

use plist::Value;

#[derive(Debug, Clone, Copy)]
pub struct Rgb { pub r: u8, pub g: u8, pub b: u8 }

pub fn parse_itermcolors(bytes: &[u8]) -> Result<Palette, ThemeError> {
    let value: Value = plist::from_bytes(bytes)?;
    let dict = value.as_dictionary().ok_or(ThemeError::NotADict)?;

    let mut palette = Palette::default();
    let mut ansi: [Rgb; 16] = [Rgb::default(); 16];

    for (key, v) in dict {
        let d = v.as_dictionary().ok_or_else(|| ThemeError::Field(key.clone()))?;
        let rgb = read_rgb(d).map_err(|_| ThemeError::Field(key.clone()))?;
        match key.as_str() {
            k if k.starts_with("Ansi ") && k.ends_with(" Color") => {
                if let Some(n) = k.trim_start_matches("Ansi ").trim_end_matches(" Color").parse::<usize>().ok() {
                    if n < 16 { ansi[n] = rgb; }
                }
            }
            "Foreground Color" => palette.fg = rgb,
            "Background Color" => palette.bg = rgb,
            "Cursor Color"     => palette.cursor = rgb,
            "Selection Color"  => palette.selection = rgb,
            "Bold Color"       => palette.bold = rgb,
            other => tracing::warn!(key = %other, "unknown .itermcolors key, ignored"),
        }
    }
    palette.ansi = ansi;
    Ok(palette)
}

fn read_rgb(d: &plist::Dictionary) -> Result<Rgb, ThemeError> {
    let r = d.get("Red Component").and_then(Value::as_real).unwrap_or(0.0);
    let g = d.get("Green Component").and_then(Value::as_real).unwrap_or(0.0);
    let b = d.get("Blue Component").and_then(Value::as_real).unwrap_or(0.0);
    Ok(Rgb {
        r: (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        g: (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        b: (b.clamp(0.0, 1.0) * 255.0).round() as u8,
    })
}
```

### Example 3: notify-debouncer-full watcher

```rust
// crates/vector-config/src/watcher.rs
// Source: notify-debouncer-full docs.rs (0.5.x); notify 8 API.

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::{path::Path, time::Duration};
use tokio::sync::mpsc;

pub fn spawn_watcher(
    config_path: &Path,
    themes_dir: &Path,
    tx: mpsc::Sender<ConfigEvent>,
) -> anyhow::Result<impl Drop> {
    let mut debouncer = new_debouncer(
        Duration::from_millis(150),
        None,
        move |result: DebounceEventResult| {
            match result {
                Ok(events) => {
                    for ev in events {
                        // collapse to a single ConfigDirty regardless of event count
                        let _ = tx.try_send(ConfigEvent::Dirty(ev.paths));
                    }
                }
                Err(errs) => tracing::warn!(?errs, "notify watcher errors"),
            }
        },
    )?;

    // Watch the parent dir of the config file too — atomic-rename saves
    // (vim's :w) replace the inode; watching the parent catches the create.
    debouncer.watcher().watch(
        config_path.parent().unwrap_or(Path::new(".")),
        RecursiveMode::NonRecursive,
    )?;
    debouncer.watcher().watch(themes_dir, RecursiveMode::NonRecursive)?;
    Ok(debouncer) // Drop = unwatch
}
```

### Example 4: NSTextInputClient minimum (D-81)

```rust
// crates/vector-app/src/ime.rs (sketch — actual impl is objc2 macros)
// Source: Apple NSTextInputClient ref; objc2-app-kit 0.3 binding patterns.

use objc2::{declare_class, msg_send_id, ClassType};
use objc2_app_kit::{NSTextInputClient, NSView};
use objc2_foundation::{NSAttributedString, NSRange, NSRect};

declare_class!(
    pub struct VectorInputView { /* ivars: ime_state: RefCell<ImeState> */ }
    unsafe impl ClassType for VectorInputView {
        type Super = NSView;
        const NAME: &'static str = "VectorInputView";
    }
    unsafe impl NSTextInputClient for VectorInputView {
        #[method(setMarkedText:selectedRange:replacementRange:)]
        fn set_marked_text(&self, text: &NSAttributedString, sel_range: NSRange, _replace: NSRange) {
            // Render text under cursor cell, underlined.
            self.ivars().ime_state.borrow_mut().set_preedit(text.string().to_string(), sel_range);
            self.setNeedsDisplay(true);
        }
        #[method(insertText:replacementRange:)]
        fn insert_text(&self, text: &NSObject, _replace: NSRange) {
            // Commit to PTY as keystroke bytes.
            let s = ns_object_as_string(text);
            self.ivars().write_tx.try_send(s.into_bytes()).ok();
            self.ivars().ime_state.borrow_mut().clear();
        }
        #[method(unmarkText)] fn unmark(&self) { self.ivars().ime_state.borrow_mut().clear(); }
        #[method(hasMarkedText)] fn has_marked(&self) -> bool { self.ivars().ime_state.borrow().is_active() }
        #[method(markedRange)] fn marked_range(&self) -> NSRange { self.ivars().ime_state.borrow().marked_range() }
        #[method(selectedRange)] fn selected_range(&self) -> NSRange { NSRange::new(usize::MAX, 0) /* NSNotFound */ }
        #[method(firstRectForCharacterRange:actualRange:)]
        fn first_rect(&self, _r: NSRange, _ar: *mut NSRange) -> NSRect {
            self.ivars().cursor_screen_rect()  // best-effort
        }
        // characterIndexForPoint, attributedSubstringForProposedRange, validAttributesForMarkedText:
        // accept defaults (NSNotFound / nil / empty array) — return statements omitted.
    }
);
```

### Example 5: TOML schema + line/col error

```rust
// crates/vector-config/src/schema.rs
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ConfigFile {
    pub default: ProfileBlock,
    #[serde(default)]
    pub profile: BTreeMap<String, ProfileBlock>,
    #[serde(default)]
    pub keybind: Vec<KeyBind>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct ProfileBlock {
    pub kind: Option<Kind>,           // local / codespace / dev_tunnel — only on [profile.X]
    pub theme: Option<String>,
    pub tint:  Option<String>,        // "#RRGGBB"
    pub font:  Option<FontCfg>,
    pub appearance: Option<Appearance>,
    pub clipboard_write: Option<ClipboardPolicy>,
    pub secure_keyboard_entry: Option<bool>,
    pub env: Option<BTreeMap<String, String>>,
    pub startup_command: Option<String>,
    pub codespace_name: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum Kind { Local, Codespace, DevTunnel }

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct FontCfg {
    pub family: Option<String>,
    pub size:   Option<f32>,
    pub ligatures: Option<bool>,
}

pub fn parse(source: &str) -> Result<ConfigFile, ConfigError> {
    toml::from_str(source).map_err(|e| {
        let (line, col) = e.span().map(|s| byte_to_line_col(source, s.start)).unwrap_or((0, 0));
        ConfigError { line, col, message: e.message().to_owned() }
    })
}

fn byte_to_line_col(src: &str, byte: usize) -> (usize, usize) {
    let prefix = &src[..byte.min(src.len())];
    let line = prefix.chars().filter(|c| *c == '\n').count() + 1;
    let col  = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    (line, col)
}
```

### Example 6: Profile picker (Cmd-Shift-P)

```rust
// crates/vector-app/src/profile_picker.rs
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

pub struct ProfilePicker {
    matcher: SkimMatcherV2,
    profiles: Vec<String>,
}

impl ProfilePicker {
    pub fn matches(&self, query: &str) -> Vec<(i64, &str)> {
        let mut out: Vec<_> = self.profiles.iter()
            .filter_map(|p| self.matcher.fuzzy_match(p, query).map(|score| (score, p.as_str())))
            .collect();
        out.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        out
    }
}
```

### Example 7: Workspace `[lints]` inheritance (D-83 sub-item 1)

Already done at workspace level. Per-crate add:

```toml
# crates/vector-config/Cargo.toml (and every other crate)
[lints]
workspace = true
```

For `vector-app` (existing `unsafe_code` allowlist):

```toml
[lints.rust]
unsafe_code = "allow"  # AppKit FFI: NSTextInputClient, SKE, NSPasteboard
[lints.clippy]
pedantic = { level = "warn", priority = -1 }
await_holding_lock = "deny"
```

### Example 8: Path-dep version arch-lint (D-83 sub-item 2)

```rust
// tests/path_deps_have_versions.rs (workspace-level integration test, or per crate)
use std::path::PathBuf;

#[test]
fn path_deps_have_versions() {
    let manifest = std::fs::read_to_string(env!("CARGO_MANIFEST_PATH"))
        .expect("read Cargo.toml");
    let parsed: toml::Value = toml::from_str(&manifest).unwrap();

    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(deps) = parsed.get(section).and_then(|v| v.as_table()) else { continue };
        for (name, spec) in deps {
            let Some(t) = spec.as_table() else { continue };
            let has_path = t.contains_key("path");
            let has_version = t.contains_key("version");
            assert!(
                !has_path || has_version,
                "dep `{name}` in {section} has `path` but no `version` — \
                 cargo-deny bans will FAIL on publish. Add version = \"X.Y\".",
            );
        }
    }
}
```

### Example 9: pre-commit `cargo deny` step (D-83 sub-item 3)

```yaml
# .pre-commit-config.yaml
- repo: local
  hooks:
    - id: cargo-deny
      name: cargo deny
      entry: cargo deny check bans licenses sources advisories
      language: system
      pass_filenames: false
      stages: [pre-commit]
```

### Example 10: `cargo-machete` in CI (D-83 sub-item 4)

```yaml
# .github/workflows/ci.yml — add a new job
unused-deps:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: bnjbvr/cargo-machete@v0.7  # pin
      # Fails the build on any unused workspace dep.
```

### Example 11: tmux DCS smoke test fixture

```bash
# tests/smoke/osc52_tmux.sh  (manual smoke; CI optional via brew install tmux)
set -euo pipefail
tmux new-session -d -s vector-test -x 80 -y 24
tmux set-option -t vector-test -g allow-passthrough on
tmux send-keys -t vector-test 'printf "\eP\e]52;c;%s\a\e\\" "$(printf "tmux passthrough OK" | base64)"' Enter
sleep 0.5
# Verify NSPasteboard contains the string:
pbpaste | grep -q "tmux passthrough OK"
tmux kill-session -t vector-test
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `cocoa-rs` + `objc 0.2` | `objc2` + `objc2-app-kit` | 2024 onwards | Workspace already on `objc2 0.6.4`; D-81 IME impl uses `declare_class!` + `NSTextInputClient` derive. |
| `harfbuzz_rs` for shaping | CoreText shaping via `crossfont 0.9` | Phase 3 decision (D-50) | Phase 5 ligature toggle is a coalescing flag, not a new shaper. |
| Hand-rolled `notify` debounce | `notify-debouncer-full` | 2023+ | Saves correlating rename events. |
| `tokio::fs::watch` | n/a | — | tokio has no FS watcher. Use `notify` on a blocking thread + mpsc → EventLoopProxy. |
| OSC 7 via shell-integration script only | OSC 7 captured live, fallback to `proc_pidinfo` | D-79 (this phase) | New-pane cwd works without shell-integration. |

**Deprecated / outdated:**
- `arboard`: avoid on macOS (per CLAUDE.md spirit — minimize deps); use `NSPasteboard` directly.
- `harfbuzz_rs`: stale since 2021; do not adopt.
- `cocoa` / `cocoa-foundation`: superseded by `objc2-app-kit`.

## Open Questions

1. **Does `vte 0.15`'s DCS parser pass through to the inner OSC 52 automatically?**
   - What we know: vte's `hook`/`put`/`unhook` cover DCS state; the standard `tmux` DCS-wrap is `\eP\e]52;c;DATA\a\e\\`. Reading `ansi.rs`, DCS parameters drive `unhook`-time decoding for specific DCS final bytes; arbitrary "DCS as transport for OSC" may not auto-unwrap.
   - What's unclear: Does Vector need to manually peel the `\eP ... \e\\` envelope before feeding alacritty, or does alacritty's vte do it?
   - Recommendation: **Write an integration test as the first task of the OSC 52 plan.** Feed `\eP\e]52;c;aGVsbG8=\a\e\\` to a `Term` and assert `clipboard_store` Handler fires. If it doesn't, add a thin DCS unwrap layer in `osc_sniff.rs` that detects `ESC P` → wraps until `ESC \\` → re-feeds the inner bytes.

2. **CoreText font registration for newly-added user fonts during runtime.**
   - What we know: macOS caches font directories at app launch.
   - What's unclear: Whether `CTFontManagerRegisterFontURLs(_ urls: CFArray, _ scope: CTFontManagerScope, _ enabled: Bool)` from a Swift/Cocoa main thread will pick up files dropped into `~/Library/Fonts` mid-session.
   - Recommendation: **Phase 5 stays out of this** — D-69 already classifies `font.family` change as `restart required`. Defer to v2.

3. **Tint stripe implementation choice (D-75): `NSVisualEffectView` overlay vs extending per-cell tint uniform.**
   - **Recommendation:** Use a **new dedicated render pass** with a single quad over the top 24-32 px of the window content area. Reasons:
     - `NSVisualEffectView` is for blur/material backgrounds; it doesn't compose with wgpu's `CAMetalLayer`. Layering an AppKit subview over a Metal layer fights the renderer.
     - Extending the per-cell tint uniform pollutes the cell shader for a banner that isn't a cell.
     - A dedicated stripe pass: one wgpu pipeline, one quad, one solid-color uniform. ~80 LOC. Reuses Plan 03-03 cell pipeline scaffolding.
   - Open: window-server titlebar vs Vector-owned area. Vector currently uses `NSWindow.titlebarAppearsTransparent = false` (default); the stripe paints *inside* the content view, *under* the tab bar, *over* the top edge of the active pane. Confirm during planning by inspecting `AppWindow` titlebar geometry.

4. **`tmux 3.4+` in CI vs. manual-only.**
   - What we know: `macos-15-intel` and `macos-14` runners don't pre-install tmux 3.4+ (3.3 typically); `brew install tmux` works but adds ~20s.
   - Recommendation: Add `brew install tmux` to a dedicated smoke job; mark OSC 52 DCS round-trip as **integration test in CI** (not manual-only). The end-to-end Codespace round-trip remains manual-only.

5. **`notify-debouncer-full` version (0.5 stable vs 0.8 rc).**
   - Recommendation: pin **0.5 line** (or whatever the resolver picks given `notify = "8"`). The 0.8 RC adds caching improvements not needed at Phase 5 scale.

## Validation Architecture

> Nyquist Dimension 8 — mandatory because `workflow.nyquist_validation` is not disabled.

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (workspace-native; no external runner) |
| Config file | `Cargo.toml` workspace `[workspace.dependencies]` |
| Quick run command | `cargo test --workspace --tests --no-fail-fast` |
| Full suite command | `cargo test --workspace --all-targets --no-fail-fast && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check` |
| Phase-boundary integration command | `cargo test --workspace --all-targets -- --include-ignored` (the tmux smoke is `#[ignore]` by default; enabled in a dedicated CI job with `brew install tmux` prereq) |
| Lint entry per CLAUDE.md | `make lint` (canonical; or `cargo clippy --workspace --all-targets -- -D warnings`) |
| Manual smoke matrix | `.planning/phases/05-polish-local-daily-driver/05-VALIDATION.md §"Manual-Only Verifications"` (font hot-swap toast, `.itermcolors` import, IME preedit, SKE toggle, Codespace tmux round-trip) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| POLISH-01 | TOML parses with `deny_unknown_fields` | unit | `cargo test -p vector-config schema::parse_rejects_unknown_field` | ❌ Wave 0 |
| POLISH-01 | Profile `[profile.X]` overrides `[default]` flatly (no deep merge) | unit | `cargo test -p vector-config schema::profile_overrides_flat` | ❌ Wave 0 |
| POLISH-01 | Parse error reports (line, col) | unit | `cargo test -p vector-config loader::error_line_col` | ❌ Wave 0 |
| POLISH-01 | Hot-reload debounce ≥ 150 ms quiescent | integration (tempfile + tokio time) | `cargo test -p vector-config watcher::debounce_150ms` | ❌ Wave 0 |
| POLISH-01 | Atomic-rename save (simulate `unlink+rename`) fires exactly one ConfigDirty | integration | `cargo test -p vector-config watcher::atomic_rename_single_event` | ❌ Wave 0 |
| POLISH-01 | On parse error, last-good Config preserved | integration | `cargo test -p vector-config apply::parse_error_keeps_last_good` | ❌ Wave 0 |
| POLISH-02 | crossfont rasterizes bundled JetBrains Mono ligature glyph | unit | `cargo test -p vector-fonts ligature_glyph_present` | ❌ Wave 0 |
| POLISH-02 | Ligature toggle off → per-cell glyphs only | unit | `cargo test -p vector-fonts ligature_toggle_off` | ❌ Wave 0 |
| POLISH-02 | Nerd Font glyph (PUA codepoint) renders without fallback substitution | unit | `cargo test -p vector-fonts nerd_font_codepoint_renders` | ❌ Wave 0 |
| POLISH-02 | BYO-font from `~/Library/Fonts` returns `restart required` (D-69) | unit | `cargo test -p vector-config apply::font_family_change_requires_restart` | ❌ Wave 0 |
| POLISH-03 | Vector Light + Vector Dark builtins load | unit | `cargo test -p vector-theme builtins_loadable` | ❌ Wave 0 |
| POLISH-03 | `.itermcolors` plist parses with all 16 ANSI + FG/BG/Cursor/Sel/Bold | unit | `cargo test -p vector-theme itermcolors::parses_full_scheme` (fixture: Solarized-Dark.itermcolors) | ❌ Wave 0 |
| POLISH-03 | `.itermcolors` unknown key warns + continues | unit | `cargo test -p vector-theme itermcolors::unknown_key_warns` | ❌ Wave 0 |
| POLISH-03 | macOS appearance change → `effectiveAppearance` flips Palette | unit (mock) | `cargo test -p vector-theme appearance::dark_light_flip` | ❌ Wave 0 |
| POLISH-04 | OSC 7 sniffer extracts `file:///Users/foo/dev/` correctly | unit | `cargo test -p vector-term osc_sniff::osc7_file_url_parses` | ❌ Wave 0 |
| POLISH-04 | OSC 7 with percent-encoded path decodes | unit | `cargo test -p vector-term osc_sniff::osc7_percent_encoded` | ❌ Wave 0 |
| POLISH-04 | OSC 8 hyperlink with `id=` groups multi-cell run | unit | `cargo test -p vector-term hyperlink::id_groups_run` | ❌ Wave 0 |
| POLISH-04 | OSC 8 anonymous (no id) groups by URI + contiguity | unit | `cargo test -p vector-term hyperlink::anonymous_by_uri` | ❌ Wave 0 |
| POLISH-04 | OSC 8 scheme not in allowlist → logged + ignored | unit | `cargo test -p vector-term hyperlink::scheme_allowlist` | ❌ Wave 0 |
| POLISH-04 | OSC 10/11/12 query → `Event::PtyWrite` payload matches xterm reply format | unit | `cargo test -p vector-term listener::osc10_query_response` | ❌ Wave 0 |
| POLISH-04 | OSC 133;A/B/C/D append to `prompt_marks` ring | unit | `cargo test -p vector-term osc_sniff::osc133_marks` | ❌ Wave 0 |
| POLISH-04 | Prompt mark ring caps at 1000 (D-79) | unit | `cargo test -p vector-term osc_sniff::prompt_ring_1000` | ❌ Wave 0 |
| POLISH-05 | OSC 52 base64 raw → `clipboard_store` Handler fires | unit | `cargo test -p vector-term osc52::raw_clipboard_store` | ❌ Wave 0 |
| POLISH-05 | OSC 52 DCS-wrapped (`\eP\e]52;c;…\a\e\\`) → `clipboard_store` fires | integration | `cargo test -p vector-term osc52::dcs_wrapped_round_trip` | ❌ Wave 0 |
| POLISH-05 | Outbound OSC 52 chunks at 58 bytes (D-71) | unit | `cargo test -p vector-input clipboard::outbound_58_byte_chunks` | ❌ Wave 0 |
| POLISH-05 | OSC 52 read query → denied (D-70 v1) | unit | `cargo test -p vector-term osc52::read_denied` | ❌ Wave 0 |
| POLISH-05 | Tmux DCS smoke (real `tmux 3.4`) | integration (CI: `#[ignore]` + dedicated job with `brew install tmux`) | `cargo test -p vector-term --test osc52_tmux -- --ignored` | ❌ Wave 0 |
| POLISH-06 | `Term::search` returns matches (existing API) | unit (exists) | `cargo test -p vector-term search` | ✅ |
| POLISH-06 | Smart-case: all-lowercase query → case-insensitive (D-77) | unit | `cargo test -p vector-app search_bar::smart_case_lower` | ❌ Wave 0 |
| POLISH-06 | Smart-case: any-uppercase query → case-sensitive | unit | `cargo test -p vector-app search_bar::smart_case_upper` | ❌ Wave 0 |
| POLISH-06 | Cache caps at 1000, beyond shows `1000+` lazy step | unit | `cargo test -p vector-app search_bar::cache_1000_lazy` | ❌ Wave 0 |
| POLISH-06 | `Esc` restores prior selection | integration | `cargo test -p vector-app search_bar::esc_restores_selection` | ❌ Wave 0 |
| POLISH-07 | Profile schema parses local / codespace / dev_tunnel | unit | `cargo test -p vector-config schema::profile_kinds_parse` | ❌ Wave 0 |
| POLISH-07 | Phase-5 wires `kind = "local"` end-to-end (LocalDomain spawn) | integration | `cargo test -p vector-mux profile_local_spawn` | ❌ Wave 0 |
| POLISH-07 | `kind = "codespace"` parses + shows `⚠ Phase 6+` label, spawn no-ops | unit | `cargo test -p vector-app profile_picker::codespace_warning_label` | ❌ Wave 0 |
| POLISH-07 | Cmd-Shift-P fuzzy match returns expected ranking | unit | `cargo test -p vector-app profile_picker::fuzzy_ranking` | ❌ Wave 0 |
| POLISH-07 | Tint stripe quad geometry matches `[0..24px, content_top]` | unit | `cargo test -p vector-render tint_stripe::geometry` | ❌ Wave 0 |
| POLISH-08 | Secure Keyboard Entry toggle calls `EnableSecureEventInput` (FFI mock) | unit | `cargo test -p vector-app ske::toggle_calls_carbon` | ❌ Wave 0 |
| POLISH-08 | Drop / panic hook always calls `DisableSecureEventInput` | unit | `cargo test -p vector-app ske::raii_disables_on_drop` | ❌ Wave 0 |
| POLISH-08 | `setMarkedText` does NOT enter PTY byte stream | unit | `cargo test -p vector-app ime::preedit_not_to_pty` | ❌ Wave 0 |
| POLISH-08 | `insertText` writes UTF-8 bytes to PTY | unit | `cargo test -p vector-app ime::commit_to_pty` | ❌ Wave 0 |
| POLISH-08 | `unmarkText` clears preedit | unit | `cargo test -p vector-app ime::unmark_clears` | ❌ Wave 0 |
| Cmd-N (D-82) | Spawns new ungrouped NSWindow with `[default]` profile and `$HOME` cwd | integration | manual-only (NSWindow lifecycle hard to assert headless); `cargo test -p vector-app cmd_n::spawns_default_profile_$home` covers config path | ❌ Wave 0 |
| Cmd-C (D-53/54) | Selection-string extracts wide chars correctly (`你好`) | unit | `cargo test -p vector-input selection::wide_chars_collapse` | ❌ Wave 0 |
| Cmd-C | Selection-string strips trailing whitespace per line | unit | `cargo test -p vector-input selection::trailing_ws_stripped` | ❌ Wave 0 |
| Cmd-C | Rectangular selection uses `\n` newlines | unit | `cargo test -p vector-input selection::rect_uses_newline` | ❌ Wave 0 |
| D-83 #1 | Every workspace crate has `[lints] workspace = true` | unit (workspace-level) | `cargo test --test workspace_lints_inheritance` | ❌ Wave 0 |
| D-83 #2 | Path deps have `version =` field | unit (per crate or workspace-level) | `cargo test --test path_deps_have_versions` | ❌ Wave 0 |
| D-83 #3 | `cargo deny check` runs in pre-commit | smoke (manual) | `pre-commit run cargo-deny --all-files` | manual |
| D-83 #4 | `cargo-machete` in CI | smoke (CI job) | `.github/workflows/ci.yml::unused-deps` green on PR | CI-only |

### Sampling Rate

- **Per task commit:** `cargo test --workspace --tests --no-fail-fast` (quick — D-83 added arch-lints run here).
- **Per wave merge:** `cargo test --workspace --all-targets && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check && cargo deny check`.
- **Phase gate (before `/gsd:verify-work`):** add `-- --include-ignored` to pick up the tmux + IME ignored integration tests, AND complete the manual smoke matrix in `05-VALIDATION.md`.

### Manual-Only Verifications (to be enumerated in 05-VALIDATION.md)

| Item | Why Manual | How to Verify |
|------|-----------|---------------|
| Font hot-swap toast appears on font.family change | NSWindow toast rendering | Edit `config.toml`, save, observe banner |
| `.itermcolors` drop-and-go (Solarized-Dark) | Live FSEvents on real macOS | Drop file in `~/.config/vector/themes/`, set `theme = "Solarized-Dark"`, save, observe palette flip |
| IME preedit (Japanese, Pinyin) underlines under cursor | NSTextInputClient driven by real IME source | Switch Input Source → Hiragana; type "ka" → preedit underlined; Enter commits |
| SKE toggle disables event capture in 1Password browser plugin | Cross-app verification | Toggle on; type in 1Password autofill field; verify it doesn't see Vector's keystrokes |
| Tmux DCS round-trip on a real Codespace | Network + tmux 3.4 + remote PTY | `ssh` to Codespace, `tmux new -A -s vector`, `printf "\eP\e]52;c;%s\a\e\\" "$(printf hi | base64)"`, verify macOS clipboard via `pbpaste` |
| Cmd-Shift-P picker behavior under 50+ profiles | UX smell test | Generate config with 50 profiles, open picker, type to filter |
| Cmd-N spawns ungrouped window | NSWindow tabbing mode behavior | Cmd-N twice; verify two separate windows, no tab merge |

### Wave 0 Gaps

- [ ] `crates/vector-config/tests/schema_and_loader.rs` — covers POLISH-01 parse + line/col
- [ ] `crates/vector-config/tests/watcher_debounce.rs` — covers POLISH-01 debounce + atomic-rename
- [ ] `crates/vector-config/tests/apply_pipeline.rs` — covers POLISH-01 last-good + font-restart classification
- [ ] `crates/vector-theme/tests/itermcolors.rs` + fixture `tests/fixtures/Solarized-Dark.itermcolors` — covers POLISH-03 importer
- [ ] `crates/vector-theme/tests/builtins.rs` + `appearance.rs` — covers POLISH-03 builtins + appearance
- [ ] `crates/vector-term/tests/osc_sniff.rs` — covers POLISH-04 OSC 7 + 133 sniffer
- [ ] `crates/vector-term/tests/hyperlinks.rs` — covers POLISH-04 OSC 8 id + anonymous grouping + allowlist
- [ ] `crates/vector-term/tests/dynamic_color_response.rs` — covers POLISH-04 OSC 10/11/12 PtyWrite reply
- [ ] `crates/vector-term/tests/osc52.rs` — covers POLISH-05 raw + DCS-wrapped + read-denied
- [ ] `crates/vector-term/tests/osc52_tmux.rs` (`#[ignore]` by default) — covers POLISH-05 real tmux round-trip; CI job with `brew install tmux`
- [ ] `crates/vector-input/tests/clipboard.rs` — covers POLISH-05 58-byte chunking
- [ ] `crates/vector-input/tests/selection_string.rs` — covers Cmd-C wide chars + trailing ws + rect newlines
- [ ] `crates/vector-app/tests/search_bar.rs` — covers POLISH-06 smart-case + cache cap + esc restore
- [ ] `crates/vector-app/tests/profile_picker.rs` — covers POLISH-07 fuzzy + label + tint
- [ ] `crates/vector-app/tests/ske.rs` — covers POLISH-08 toggle + RAII disable
- [ ] `crates/vector-app/tests/ime.rs` — covers POLISH-08 preedit not-to-PTY + commit + unmark
- [ ] `tests/workspace_lints_inheritance.rs` (top-level) — D-83 #1 arch-lint
- [ ] `tests/path_deps_have_versions.rs` (top-level) — D-83 #2 arch-lint
- [ ] `.pre-commit-config.yaml` — D-83 #3 cargo-deny hook
- [ ] `.github/workflows/ci.yml::unused-deps` job — D-83 #4 cargo-machete
- [ ] `05-VALIDATION.md` (planner generates) — enumerates all manual-only items above
- [ ] Framework install: none — `cargo test` is workspace-native and already wired

## Sources

### Primary (HIGH confidence)

- **alacritty_terminal 0.26 source:** `/Users/ashutosh/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/alacritty_terminal-0.26.0/src/term/mod.rs` — `Handler` impl lines 1662 (`set_color`), 1675 (`dynamic_color_sequence`), 1705 (`clipboard_store`), 1726 (`clipboard_load`), 1874 (`set_hyperlink`), 2221 (`set_title`).
- **alacritty_terminal `EventListener`:** `event.rs:103` — `send_event(Event::PtyWrite(...))` is the PTY response path.
- **vte 0.15 OSC dispatch:** `/Users/ashutosh/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/vte-0.15.0/src/ansi.rs:1329-1523` — exact table of which OSC codes are handled (`0/2/4/8/10/11/12/22/50/52/104/110/111/112`); OSC 7 and OSC 133 fall through to `unhandled`.
- **Existing `vector-term` crate:** `crates/vector-term/src/{term.rs, parser.rs, listener.rs, search.rs}` — current wrapper + `NoopListener` to replace + `search()` API D-39.
- **Vector workspace `Cargo.toml`:** confirms current pins for `alacritty_terminal 0.26`, `vte` (transitive), `crossfont 0.9`, `wgpu 29`, `winit 0.30.13`, `objc2 0.6.4`, `tokio 1.52.3`.
- **CLAUDE.md project instructions:** stack constraints + don't-use list + lint flow.

### Secondary (MEDIUM confidence)

- [iTerm2 Color Schemes README](https://github.com/mbadolato/iTerm2-Color-Schemes/blob/master/README.md) — confirms `.itermcolors` key set.
- [iTerm2 Pro.itermcolors example](https://raw.githubusercontent.com/mbadolato/iTerm2-Color-Schemes/master/schemes/Pro.itermcolors) — XML plist `<key>Ansi 0 Color</key><dict><key>Red Component</key><real>0.0</real>…</dict>` confirms float `[0,1]` components.
- [iTerm2 Color Properties Reference](https://deepwiki.com/mbadolato/iTerm2-Color-Schemes/2.3-color-properties-reference) — confirms full key set incl. Bold / Selection.
- [notify-debouncer-full docs (0.7)](https://docs.rs/notify-debouncer-full/) — `new_debouncer(Duration, Option<NotifyFilter>, closure)` signature; `DebounceEventResult = Result<Vec<DebouncedEvent>, Vec<Error>>`.
- crates.io versions: `cargo search` output 2026-05-12 for `notify`, `notify-debouncer-full`, `plist`, `fuzzy-matcher`, `base64`, `keyring`, `toml`, `nucleo-matcher`.
- Apple `NSTextInputClient` reference + Apple Secure Keyboard Entry guide (linked in CONTEXT.md canonical_refs).

### Tertiary (LOW confidence — flagged for verification during planning)

- `vte 0.15` DCS pass-through behavior to inner OSC 52 — needs the **Open Question #1** integration test as the first task.
- CoreText runtime font registration (Pitfall 7 / Open Question #2) — Phase 5 sidesteps; mark as v2.
- Exact tmux 3.4 passthrough cut-off byte count — empirical "~60"; CONTEXT D-71 picks 58 with safety margin (HIGH confidence on the mitigation, MEDIUM on the exact byte).

## Metadata

**Confidence breakdown:**
- **Standard Stack:** HIGH — all versions verified via `cargo search` on 2026-05-12; alternatives ruled out by reading existing CLAUDE.md "What NOT to Use".
- **OSC architecture:** HIGH — read vte 0.15 source directly; OSC 7 + 133 dispatch path empirically confirmed missing.
- **`.itermcolors` schema:** HIGH — read example file from mbadolato/iTerm2-Color-Schemes; matches DeepWiki property reference.
- **NSTextInputClient minimum:** MEDIUM — Apple documents 10 selectors; the 5-selector minimum reflects field experience from kitty + Alacritty's IME shim. Plan should validate against a real Pinyin/Hiragana session.
- **tmux DCS pass-through unwrap:** MEDIUM — flagged as Open Question #1 + first integration test.
- **CoreText live font registration:** LOW — explicitly punted to v2 per D-69.
- **`notify-debouncer-full` atomic-rename behavior:** MEDIUM — crate documents `Cache` correlation but rename-handling caveats are noted in `notify`'s known problems. Validate via integration test simulating `unlink + rename` (Pitfall 1 entry above).

**Research date:** 2026-05-12
**Valid until:** 2026-06-12 (30 days; crate versions are stable, but `notify-debouncer-full` 0.8 may stabilize and become a better default — re-check before Phase 5 starts if more than 30 days have elapsed).
