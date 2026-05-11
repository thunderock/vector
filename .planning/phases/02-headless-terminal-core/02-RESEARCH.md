# Phase 2: Headless Terminal Core - Research

**Researched:** 2026-05-11
**Domain:** VT/ANSI terminal core + local PTY + `Domain`/`PtyTransport` trait surface, headless pass-through binary
**Confidence:** HIGH for stack + trait shape + fixture pattern; MEDIUM for raw-mode pass-through rendering loop (one of several reasonable approaches; locked in by D-36 but exact tick cadence is Claude's discretion)

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-36 — `vector-headless` is a pass-through proxy**, not a snapshot tool or test-only fixture. It spawns `$SHELL` (fallback `/bin/zsh`), connects stdin/stdout to a `portable-pty` master via raw-mode bridging, parses output through `alacritty_terminal::Term`, and renders the grid back to the parent terminal each tick. Ctrl-D exits cleanly; child-process exit also exits cleanly. Binary lives at `crates/vector-app/src/bin/vector-headless.rs` (or own crate `crates/vector-headless/` — researcher/planner picks).
- **D-37 — Hand-author ~20–40 fixtures.** No `vttest`/`esctest`. No Alacritty corpus vendoring. Each fixture is a `(input_bytes, expected_grid_state)` pair as a Rust unit test inside `vector-term`. Corpus completes in <1s. Covers CSI/OSC/DCS dispatch, partial-UTF-8 reads, alt-screen DECSET 1049, scroll regions DECSTBM, tab stops, ED/EL erase, SGR truecolor, grapheme-cluster width (emoji ZWJ + East Asian width).
- **D-38 — Full trait + LocalDomain + stubbed remote domains.** `PtyTransport` and `Domain` traits ship in their **final** shape in Phase 2. `LocalDomain` is fully implemented atop `portable-pty`. `CodespaceDomain` + `DevTunnelDomain` files contain the type + `impl Domain` with `unimplemented!("Phase 7")` / `unimplemented!("Phase 8")` bodies — so Phases 7/8 only fill bodies, never reshape contracts.
- **D-39 — Library-only `search(&regex) -> Vec<Match>` API.** Phase 2 tests synthesize 10k+ line scrollbacks and assert match counts/positions. User-facing search bar (Cmd-F overlay, highlighted matches, jump-to-match) is **deferred to Phase 5**.

### Claude's Discretion

- Shell selection logic — `$SHELL` env var → `/etc/passwd` lookup → fallback `/bin/zsh`.
- Default grid size — 80×24 or 100×30; CLI flags `--cols`/`--rows` to override; watch parent terminal `SIGWINCH` in pass-through mode.
- Tracing levels — workspace-standard `tracing` crate; add `--debug-parser` if needed; no new logging infrastructure.
- Lifecycle on shell exit — child PID death → drain remaining PTY output → render final grid → exit 0. No zombies (verified in `ps`).
- Error reporting — `anyhow` at binary boundary; `thiserror` for library-level errors callers may want to match on (per Phase 1 pattern).
- Where the binary lives — `crates/vector-app/src/bin/vector-headless.rs` is canonical Cargo; planner may move to its own crate if `vector-app`'s winit/objc2 deps pull in things headless shouldn't have. **Research recommendation below: own crate.**
- Mux scope — Phase 2 does NOT pre-create `Pane`/`Tab`/`Window` in `vector-mux`. Headless binary wires `LocalDomain` → single `alacritty_terminal::Term` directly. Mux abstraction lands Phase 4.

### Deferred Ideas (OUT OF SCOPE)

- Search UI overlay → Phase 5.
- `Pane`/`Tab`/`Window` types in `vector-mux` → Phase 4.
- Vendoring Alacritty corpus or integrating `vttest`/`esctest`.
- Bracketed-paste / mouse-mode / DECSCUSR **input-event generation** — Phase 2 parser must recognize and track the modes (per CORE-06), but emitting mouse→SGR-1006 bytes is Phase 3's input layer.
- `CodespaceDomain` / `DevTunnelDomain` bodies → Phases 7, 8.
- Sixel, Kitty graphics, image protocols → out of scope per ROADMAP.
- Custom terminfo → out of scope; advertise `TERM=xterm-256color`.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CORE-01 | VT parser passes basic xterm conformance corpus (CSI/OSC/DCS dispatch, partial-UTF-8, DECSET 1049 alt-screen, DECSTBM scroll regions, tab stops, ED/EL erase) | `alacritty_terminal 0.26` `Term` + `vte` (transitive) handles all of these via the standard `vte::Perform`/`Handler` impl. Phase only needs hand-authored unit tests that feed bytes and assert grid state. See §"Code Examples" and §"Validation Architecture". |
| CORE-02 | 24-bit truecolor + 256-color SGR; grapheme-cluster cell width (East Asian + emoji ZWJ) | `alacritty_terminal` cells carry `Color::Indexed(u8)` / `Color::Spec(Rgb)`; grapheme width handled internally via `unicode-width` + `WIDE_CHAR` / `WIDE_CHAR_SPACER` / `LEADING_WIDE_CHAR_SPACER` flags on `Cell`. ZWJ sequences cluster correctly. Tests assert by inspecting `Cell::fg` / `Cell::bg` and checking that the spacer cell exists where expected. |
| CORE-03 | Scrollback ≥ 10,000 lines + regex search across history | `Term::new` takes a `Config { scrolling_history: usize, .. }` — set ≥ 10_000. `alacritty_terminal::term::search::RegexSearch::new(&str)` builds forward+backward DFAs; `Term::regex_search_left/right(&mut RegexSearch, start: Point, end: Point) -> Option<Match>` (where `Match = RangeInclusive<Point>`) finds matches; `RegexIter::new(start, end, direction, term, regex)` iterates all matches. **`vector-term::search()` wraps this** — see Question 6. |
| CORE-04 | Local PTY spawns user's login shell, propagates SIGWINCH on resize, survives child-exit cleanly (no zombies) | `portable-pty::native_pty_system()` → `PtySystem::openpty(PtySize)` → `(PtyPair { master, slave })`; `slave.spawn_command(CommandBuilder::new(shell))` returns `Box<dyn Child>`; `master.resize(PtySize)` issues `TIOCSWINSZ` → kernel SIGWINCH. **Pitfall 7 mitigations** below. |
| CORE-05 | `TERM=xterm-256color` advertised; zero Vector-specific terminfo quirks | Set via `CommandBuilder::env("TERM", "xterm-256color")` before `spawn_command`. No further work. |
| CORE-06 | Parser recognizes bracketed-paste (mode 2004), mouse modes 1000/1002/1003 with SGR 1006, DECSCUSR cursor-shape escapes | `alacritty_terminal` already dispatches these into mode flags + cursor-shape state on `Term`. Phase 2 tests assert state-after-feeding (e.g., `term.mode().contains(TermMode::BRACKETED_PASTE)` after `\x1b[?2004h`). Per CONTEXT, **emission** of bytes the other direction is Phase 3. |

</phase_requirements>

## Summary

Phase 2 is the lowest-risk Rust phase of the project: every load-bearing decision (parser, PTY abstraction, regex search) was already locked by ARCHITECTURE.md + STACK.md and is now backed by D-36..D-39. The phase wraps two mature crates — `alacritty_terminal 0.26` and `portable-pty 0.9` — and ships a focused fixture corpus plus a pass-through binary that doubles as the manual smoke harness for vim/tmux/htop.

Three things must be gotten right and are non-obvious:

1. **The PTY → tokio bridge.** `portable-pty`'s reader is blocking; it must run on `tokio::task::spawn_blocking` and push bytes into an `mpsc::Sender<Vec<u8>>`. The parser task drains the channel. Never call PTY reads on a tokio worker (Pitfall 7).
2. **Feed `&[u8]` to the parser; never decode UTF-8 at the boundary.** `alacritty_terminal`'s parser is byte-oriented and handles partial-UTF-8 internally (Pitfall 4). The headless binary that bridges stdin→PTY can also pass bytes through unchanged.
3. **`PtyTransport` and `Domain` trait shapes are locked here for the rest of v1.** Phases 4/7/8/9 plug into these. The trait surface must be expressive enough for `russh::ChannelMsg` streams (Phase 7) and `tokio-tungstenite` WebSocket-wrapped SSH (Phase 8) without re-shaping. Recommendation: model the trait on byte streams (`async fn read(&mut self, buf: &mut [u8]) -> Result<usize>`) rather than `AsyncRead`/`AsyncWrite` pin-boxing — concrete and testable, no `dyn AsyncRead` lifetime gymnastics.

**Primary recommendation:** Put the headless binary in its own crate (`crates/vector-headless/`) so it inherits the workspace-lints + `tests/no_tokio_main.rs` discipline cleanly without pulling `vector-app`'s `winit`/`objc2-app-kit`/`raw-window-handle`/`objc2-quartz-core` deps into a non-GUI binary. Cost: one new `Cargo.toml` + one `tests/no_tokio_main.rs`. Benefit: clean dep graph, headless builds in CI/Linux developer machines (useful for fixture-only iteration), and the architecture-lint per-crate count guard in `ci.yml` lines 71–79 enforces the test-file pairing.

## Standard Stack

### Core (new in Phase 2)

| Library | Version | Verified | Purpose | Why Standard |
|---------|---------|----------|---------|--------------|
| `alacritty_terminal` | 0.26.0 | 2026-04-06 (crates.io API) | VT parser + grid + scrollback + regex search | Battle-tested xterm parser since 2017. Re-exports `vte` (no separate parser dep). Ships `term::search::RegexSearch` + `Term::regex_search_left/right` + `RegexIter` — CORE-03 falls out for free. MSRV 1.85 (we're on 1.88). |
| `portable-pty` | 0.9.0 | 2025-02-11 (crates.io API) | Cross-platform local PTY | Authored by WezTerm's Wez Furlong. Handles `posix_openpt`/`forkpty`/macOS controlling-terminal edge cases per Pitfall 7. Blocking reader/writer — bridge to tokio via `spawn_blocking`. |
| `regex` | 1.12.3 | 2026-02-03 (crates.io API) | Regex compilation for `vector-term::search` | Used **inside** `alacritty_terminal::term::search::RegexSearch::new(&str)`; we expose a `regex::Regex` at the `vector-term` API boundary for ergonomics + reuse. |

### Supporting (transitive or for the headless binary)

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `vte` | 0.15.0 | VT escape parser | **Transitive only** via `alacritty_terminal` — do not declare directly. Pitfall 1: never roll our own. |
| `unicode-width` | 0.2.2 | East Asian width + grapheme cell-width tables | **Transitive only** via `alacritty_terminal`. Tests may declare it explicitly when asserting expected cell widths for CORE-02 fixtures. |
| `crossterm` | 0.29.0 | Parent-terminal raw mode for `vector-headless` | **Only inside the headless binary** for `terminal::enable_raw_mode()` / `terminal::size()` / SIGWINCH stream. Single-purpose use; **not** added to library crates. Alternative: hand-roll `termios` via `nix` — adds a second platform dep + `unsafe`, not worth it for one binary. |
| `clap` | 4.6.1 | CLI flags for `--cols` / `--rows` / `--debug-parser` in `vector-headless` | Phase 1 xtask already uses clap-derive; same pattern. |
| `tokio` | 1.52.3 (existing) | Async runtime + `spawn_blocking` + `mpsc` channels | Existing workspace dep. **Confirm `process` feature** in `vector-pty` if we ever need `tokio::process::Child` for non-PTY children (we don't — `portable-pty`'s `Child` is sufficient). |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `alacritty_terminal` | Hand-roll `vte` parser + custom grid | **Don't.** Pitfall 1 — six weeks of escape-sequence work, multiple CVE classes already solved by Alacritty's dispatch table. |
| `alacritty_terminal` | `wezterm-term` (vendored) | Not on crates.io; vendoring is maintenance burden. STACK.md ruled out. |
| `portable-pty` | Raw `nix::pty::posix_openpt` + `unsafe { fork() }` + `setsid()` | Pitfall 7. The PTY/controlling-terminal/process-group dance is decades old. Don't. |
| `portable-pty` | `tokio_pty_process` | Older, less complete, no Windows path (we want optional Windows later per STACK.md). |
| `crossterm` for raw mode | Direct `nix::sys::termios::tcsetattr` | Adds `nix` + `unsafe`. `crossterm` is one binary-local dep that does it cleanly. |
| `regex` exposed at API | Re-export `alacritty_terminal::term::search::RegexSearch` | RegexSearch is a stateful DFA builder, not a query. `regex::Regex` is the ergonomic surface; we compile it internally to `RegexSearch` per search call (or cache by `regex.as_str()` key). |
| Pass-through render loop using full `\x1b[H` repaint | Damage-tracked diff against previous grid render | **Defer to Phase 3.** D-36 is a smoke harness, not a performance demo. Full-repaint per tick at ~30 Hz for an 80×24 grid is ~12 KB of ANSI per tick — negligible for the human-driver use case. |

**Installation (workspace `Cargo.toml [workspace.dependencies]` additions):**

```toml
alacritty_terminal = "0.26"
portable-pty = "0.9"
regex = "1"
unicode-width = "0.2"           # only if tests declare it
crossterm = "0.29"              # binary-local; not workspace
clap = { version = "4", features = ["derive"] }   # binary-local
```

**Version verification (run before plan execution):**

```bash
npm view alacritty_terminal version       # n/a — Rust
cargo info alacritty_terminal
cargo info portable-pty
cargo info regex
```

All four versions in the table were verified live against `https://crates.io/api/v1/crates/{name}` on 2026-05-11 and match STACK.md.

## Architecture Patterns

### Recommended Project Structure

```
crates/
├── vector-term/                         # VT/grid/search library
│   ├── src/
│   │   ├── lib.rs                       # pub use Term, Grid, Cell; pub fn search()
│   │   ├── term.rs                      # thin wrapper around alacritty_terminal::Term
│   │   ├── parser.rs                    # owns vte::Parser; feeds Term
│   │   ├── search.rs                    # Regex -> RegexSearch -> Vec<Match>
│   │   └── listener.rs                  # impl alacritty_terminal::event::EventListener (no-op or mpsc)
│   └── tests/
│       ├── no_tokio_main.rs             # existing arch-lint (Phase 1)
│       ├── conformance_csi.rs           # CORE-01 fixtures (CSI cursor/SGR/ED/EL)
│       ├── conformance_osc.rs           # CORE-01 fixtures (OSC title/colors)
│       ├── conformance_dcs.rs           # CORE-01 fixtures (DCS dispatch)
│       ├── conformance_modes.rs         # CORE-01 (DECSET 1049 alt-screen) + CORE-06 (1006/2004/DECSCUSR mode state)
│       ├── conformance_scroll.rs        # CORE-01 (DECSTBM scroll regions, tab stops)
│       ├── conformance_utf8.rs          # CORE-01 partial-UTF-8 split-across-reads
│       ├── conformance_color.rs         # CORE-02 24-bit truecolor + 256-color SGR
│       ├── conformance_width.rs         # CORE-02 emoji ZWJ + East Asian wide
│       └── conformance_search.rs        # CORE-03 10k-line scrollback + regex search
│
├── vector-pty/                          # Local PTY library
│   ├── src/
│   │   ├── lib.rs                       # pub use LocalPty
│   │   └── local.rs                     # spawn portable_pty + spawn_blocking reader
│   └── tests/
│       ├── no_tokio_main.rs             # existing arch-lint
│       └── lifecycle.rs                 # spawn `echo hello`, assert exit, no zombie
│
├── vector-mux/                          # Trait surface only in Phase 2
│   ├── src/
│   │   ├── lib.rs                       # pub use Domain, PtyTransport
│   │   ├── domain.rs                    # trait Domain + LocalDomain (full impl)
│   │   ├── transport.rs                 # trait PtyTransport
│   │   ├── codespace_domain.rs          # stub: unimplemented!("Phase 7")
│   │   └── devtunnel_domain.rs          # stub: unimplemented!("Phase 8")
│   └── tests/
│       ├── no_tokio_main.rs             # existing arch-lint
│       └── trait_object_safety.rs       # compile-time test: Box<dyn PtyTransport>, Box<dyn Domain>
│
├── vector-headless/                     # NEW CRATE — pass-through proxy binary
│   ├── Cargo.toml                       # [[bin]] name = "vector-headless"; deps: vector-term, vector-pty, vector-mux, tokio, anyhow, tracing, clap, crossterm
│   ├── src/
│   │   ├── main.rs                      # raw-mode + tokio runtime; spawns LocalDomain; render loop
│   │   ├── bridge.rs                    # stdin->PTY writer task; PTY->parser reader task
│   │   └── render.rs                    # ANSI emit of grid -> parent terminal each tick
│   └── tests/
│       └── no_tokio_main.rs             # arch-lint (allowlist `src/main.rs` for block_on)
```

**Why a separate `vector-headless` crate** (not `vector-app/src/bin/vector-headless.rs`): `vector-app` currently depends on `winit`, `objc2-app-kit`, `objc2-foundation`, `objc2-quartz-core`, `raw-window-handle`. A headless binary in that crate would compile all of those even though it uses none, and would inherit `vector-app`'s `#![allow(unsafe_code)]` boundary. A new crate inherits workspace lints cleanly + adds one row to the CI's per-crate test-file count guard (ci.yml lines 71–79 will then expect 15 instead of 14).

**Cost of new crate:** one `Cargo.toml`, one `tests/no_tokio_main.rs`, one line added to `Cargo.toml [workspace] members`, the ci.yml `crates_count` will auto-update via `ls -d crates/vector-*`. Plan must update `crates/*/tests/no_tokio_main.rs` count expectations if anything is hard-coded — verify: it isn't (the guard counts dynamically).

### Pattern 1: `PtyTransport` Trait — Locked in Phase 2

**What:** Byte-stream transport abstraction. Local PTY, future `russh` channel, future Dev Tunnel WebSocket all implement it.

**When to use:** Any pane that wraps a `Term`. Phase 2 has exactly one user: `LocalDomain::spawn() -> Box<dyn PtyTransport>`.

**Locked signature (proposed):**

```rust
// crates/vector-mux/src/transport.rs
use anyhow::Result;
use tokio::sync::mpsc;

/// Byte-stream transport for a single shell session.
///
/// Reads are pushed (not pulled) into a caller-supplied mpsc channel so the
/// parser task can drive on `recv().await` without ownership wrangling over
/// pinned async readers. Writes are sync at the trait level — implementations
/// internally route to a `tokio::sync::mpsc::Sender<Vec<u8>>` consumed by a
/// per-transport writer task.
#[async_trait::async_trait]
pub trait PtyTransport: Send + 'static {
    /// Resize the underlying transport. For local PTY this calls
    /// `MasterPty::resize(PtySize { rows, cols, pixel_width, pixel_height })`,
    /// which the kernel translates into SIGWINCH for the foreground pgrp.
    fn resize(&mut self, rows: u16, cols: u16, px_w: u16, px_h: u16) -> Result<()>;

    /// Write bytes toward the shell. Buffered internally.
    async fn write(&mut self, bytes: &[u8]) -> Result<()>;

    /// Take the receiving end of the output channel. Called once at startup;
    /// subsequent calls return `None`.
    fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>>;

    /// Best-effort transport kind for diagnostics and tab tint.
    fn kind(&self) -> TransportKind;

    /// Wait for the underlying shell/channel to exit. Returns the exit status
    /// when available, or `None` if the transport has no notion of exit.
    async fn wait(&mut self) -> Result<Option<i32>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Local,
    Codespace,
    DevTunnel,
}
```

**Why this shape (not `Pin<Box<dyn AsyncRead + Send>>`):**

1. Avoids object-safety contortions with `AsyncRead`/`AsyncWrite` and their `Pin<&mut Self>` receivers.
2. Maps cleanly onto `russh::ChannelMsg` (Phase 7) — that stream is also push-based via a tokio channel.
3. Maps cleanly onto WebSocket frames (Phase 8) — `tokio-tungstenite` is also push-based.
4. Lets the local PTY use `spawn_blocking` + channel send without forcing the trait into pseudo-async land.

**Tradeoff:** `async_trait` macro adds a ~3 KB compile cost per impl, dyn dispatch on `write`. Negligible.

### Pattern 2: `Domain` Trait + LocalDomain Full Impl

**What:** Per WezTerm/ARCHITECTURE.md, a `Domain` knows how to **spawn** a `PtyTransport`. Phase 2 ships `Domain` + `LocalDomain` fully; `CodespaceDomain` + `DevTunnelDomain` are bodies-only stubs.

**Locked signature (proposed):**

```rust
// crates/vector-mux/src/domain.rs
use anyhow::Result;
use crate::transport::PtyTransport;

#[derive(Debug, Clone)]
pub struct SpawnCommand {
    /// Argv. None means "use the user's login shell".
    pub argv: Option<Vec<String>>,
    /// Working directory. None means "inherit".
    pub cwd: Option<std::path::PathBuf>,
    /// Initial PTY rows / cols.
    pub rows: u16,
    pub cols: u16,
    /// Extra env vars; TERM=xterm-256color is added by LocalDomain itself.
    pub env: Vec<(String, String)>,
}

#[async_trait::async_trait]
pub trait Domain: Send + Sync {
    /// Open a new shell session. Returns a transport that the caller wires
    /// to a `vector_term::Term`.
    async fn spawn(&self, cmd: SpawnCommand) -> Result<Box<dyn PtyTransport>>;

    /// Human-readable label for logs and (later) tab UI.
    fn label(&self) -> String;

    /// True if the underlying connection is live. LocalDomain always returns
    /// true; remote domains will track session liveness.
    fn is_alive(&self) -> bool;

    /// Re-establish the underlying transport. LocalDomain returns Ok(()) (a
    /// fresh `spawn` is sufficient). Remote domains implement this in Phase 9.
    async fn reconnect(&self) -> Result<()>;
}
```

**LocalDomain implementation (Phase 2 ships the full body):**

```rust
pub struct LocalDomain {
    shell: PathBuf,  // resolved from $SHELL / /etc/passwd / fallback /bin/zsh
}

impl LocalDomain {
    pub fn new() -> Result<Self> { /* shell-resolve */ }
}

#[async_trait::async_trait]
impl Domain for LocalDomain {
    async fn spawn(&self, cmd: SpawnCommand) -> Result<Box<dyn PtyTransport>> {
        let pty = vector_pty::LocalPty::spawn(&self.shell, cmd)?;
        Ok(Box::new(pty))
    }
    fn label(&self) -> String { "local".into() }
    fn is_alive(&self) -> bool { true }
    async fn reconnect(&self) -> Result<()> { Ok(()) }
}
```

**Remote-domain stubs (Phase 2 ships compiling bodies that panic at runtime):**

```rust
// crates/vector-mux/src/codespace_domain.rs
pub struct CodespaceDomain { /* fields TBD Phase 7 */ }
#[async_trait::async_trait]
impl Domain for CodespaceDomain {
    async fn spawn(&self, _: SpawnCommand) -> Result<Box<dyn PtyTransport>> {
        unimplemented!("Phase 7: SSH transport + Codespaces connect")
    }
    fn label(&self) -> String { "codespace".into() }
    fn is_alive(&self) -> bool { false }
    async fn reconnect(&self) -> Result<()> { unimplemented!("Phase 9") }
}
```

Same shape for `DevTunnelDomain` with "Phase 8" / "Phase 9".

### Pattern 3: PTY → tokio Bridge (the dedicated-I/O-thread pattern from Phase 1, re-applied)

```rust
// crates/vector-pty/src/local.rs (excerpt)
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize, PtyPair};
use tokio::sync::mpsc;

pub struct LocalPty {
    master: Box<dyn MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send>,
    writer_tx: mpsc::Sender<Vec<u8>>,
    reader_rx: Option<mpsc::Receiver<Vec<u8>>>,
}

impl LocalPty {
    pub fn spawn(shell: &Path, cmd: SpawnCommand) -> Result<Self> {
        let pair: PtyPair = native_pty_system().openpty(PtySize {
            rows: cmd.rows, cols: cmd.cols, pixel_width: 0, pixel_height: 0,
        })?;

        let mut builder = CommandBuilder::new(shell);
        if let Some(cwd) = cmd.cwd { builder.cwd(cwd); }
        builder.env("TERM", "xterm-256color");                    // CORE-05
        for (k, v) in cmd.env { builder.env(k, v); }

        let child = pair.slave.spawn_command(builder)?;
        // Drop the slave fd in the parent so SIGHUP propagates correctly on close.
        drop(pair.slave);

        // Reader: blocking read in spawn_blocking, push to mpsc.
        let (reader_tx, reader_rx) = mpsc::channel::<Vec<u8>>(64);
        let mut reader = pair.master.try_clone_reader()?;
        tokio::task::spawn_blocking(move || {
            let mut buf = vec![0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,                                // EOF on child exit
                    Ok(n) => {
                        let chunk = buf[..n].to_vec();
                        // Blocking send — natural backpressure (Pitfall 7 / ANTI 6).
                        if reader_tx.blocking_send(chunk).is_err() { break; }
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        });

        // Writer: drain mpsc, write to master.
        let (writer_tx, mut writer_rx) = mpsc::channel::<Vec<u8>>(64);
        let mut writer = pair.master.take_writer()?;
        tokio::task::spawn_blocking(move || {
            while let Some(bytes) = writer_rx.blocking_recv() {
                if writer.write_all(&bytes).is_err() { break; }
            }
        });

        Ok(Self { master: pair.master, child, writer_tx, reader_rx: Some(reader_rx) })
    }
}

#[async_trait::async_trait]
impl PtyTransport for LocalPty {
    fn resize(&mut self, rows: u16, cols: u16, px_w: u16, px_h: u16) -> Result<()> {
        self.master.resize(PtySize { rows, cols, pixel_width: px_w, pixel_height: px_h })?;
        Ok(())                                                     // kernel emits SIGWINCH (CORE-04)
    }
    async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer_tx.send(bytes.to_vec()).await?;
        Ok(())
    }
    fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>> { self.reader_rx.take() }
    fn kind(&self) -> TransportKind { TransportKind::Local }
    async fn wait(&mut self) -> Result<Option<i32>> {
        // portable_pty::Child::wait is blocking — wrap in spawn_blocking.
        let child = std::mem::replace(&mut self.child, /* unreachable sentinel */);
        let status = tokio::task::spawn_blocking(move || child.wait()).await??;
        Ok(status.exit_code().map(|c| c as i32))
    }
}
```

Two design notes for the planner:

- **`blocking_send` (not `try_send`).** A runaway `cat /dev/urandom` should backpressure into the kernel via the PTY's flow control, not OOM us with an unbounded mpsc. Bounded channel + `blocking_send` is the WezTerm pattern.
- **`drop(pair.slave)` in the parent.** If the parent keeps the slave fd open, closing the master doesn't terminate the child cleanly — `read()` blocks forever and the child becomes a zombie (Pitfall 7).

### Pattern 4: vector-term as a Thin Wrapper

```rust
// crates/vector-term/src/term.rs
use alacritty_terminal::{
    Term as AlacrittyTerm,
    term::{Config, test::TermSize},
    vte::ansi::Processor,
    event::{Event, EventListener, VoidListener},
};

pub struct Term {
    inner: AlacrittyTerm<NoopListener>,
    parser: Processor,
}

impl Term {
    pub fn new(cols: u16, rows: u16, scrollback: usize) -> Self {
        let mut config = Config::default();
        config.scrolling_history = scrollback;                     // CORE-03
        let dims = TermSize::new(cols.into(), rows.into());        // or hand-roll Dimensions impl
        let inner = AlacrittyTerm::new(config, &dims, NoopListener);
        let parser = Processor::new();                             // owns the vte::Parser internally
        Self { inner, parser }
    }

    /// Feed raw bytes from the PTY. Never decode UTF-8 here (Pitfall 4).
    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.inner, bytes);
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let dims = TermSize::new(cols.into(), rows.into());
        self.inner.resize(dims);
    }

    pub fn grid(&self) -> &alacritty_terminal::grid::Grid<alacritty_terminal::term::cell::Cell> {
        self.inner.grid()
    }
}

struct NoopListener;
impl EventListener for NoopListener {
    fn send_event(&self, _: Event) { /* Phase 2 ignores events; Phase 4 mux will route */ }
}
```

**Note for planner:** `alacritty_terminal::vte::ansi::Processor` is the public entry point that owns the `vte::Parser` and routes calls into the `Handler` trait that `Term` implements. The signature `processor.advance(&mut term, bytes)` was confirmed against `alacritty_terminal/src/event_loop.rs`. If 0.26 reorganized this to expose `vte::Parser` directly, the planner should match against the actual exported path — `cargo doc --open` on the dep will resolve it definitively.

### Pattern 5: Search API

```rust
// crates/vector-term/src/search.rs
use regex::Regex;
use alacritty_terminal::term::search::{RegexSearch, RegexIter};
use alacritty_terminal::index::{Point, Direction};

#[derive(Debug, Clone)]
pub struct Match {
    pub start_row: i32,                 // negative = scrollback; 0+ = visible grid
    pub start_col: u16,
    pub end_row: i32,
    pub end_col: u16,
}

impl Term {
    /// Find all matches across visible grid + scrollback. Phase 2's CORE-03 surface.
    pub fn search(&self, regex: &Regex) -> Vec<Match> {
        let mut dfa = match RegexSearch::new(regex.as_str()) {
            Ok(d) => d,
            Err(_) => return vec![],
        };
        let start = self.inner.topmost_line();
        let end   = self.inner.bottommost_line();
        let start_pt = Point::new(start, 0.into());
        let end_pt   = Point::new(end, (self.inner.columns() - 1).into());
        RegexIter::new(start_pt, end_pt, Direction::Right, &self.inner, &mut dfa)
            .map(|m| Match {
                start_row: m.start().line.0 as i32,
                start_col: m.start().column.0 as u16,
                end_row:   m.end().line.0   as i32,
                end_col:   m.end().column.0 as u16,
            })
            .collect()
    }
}
```

Note: API signatures here (`topmost_line`, `bottommost_line`, `Point::new`, `Direction::Right`) were inferred from the search-module structure. Planner must verify against `cargo doc` for `alacritty_terminal 0.26` — these names have churned across alacritty versions. If shapes differ, the **public surface** (`vector_term::Term::search(&Regex) -> Vec<Match>`) is what we promise to callers; internal plumbing flexes.

### Pattern 6: Pass-Through Proxy Rendering Loop (vector-headless)

D-36 locks the **shape**: stdin → PTY (raw bytes); PTY → parser → grid → ANSI emit to parent's stdout. The cadence is Claude's discretion. Recommended simple approach:

```rust
// crates/vector-headless/src/main.rs (skeleton)
fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    let (cols, rows) = parent_size_or(args.cols.unwrap_or(80), args.rows.unwrap_or(24));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {                                         // single allowlisted block_on
        crossterm::terminal::enable_raw_mode()?;
        let _guard = scopeguard::guard((), |_| {
            let _ = crossterm::terminal::disable_raw_mode();
        });

        let domain = LocalDomain::new()?;
        let mut transport = domain.spawn(SpawnCommand {
            argv: None, cwd: None, rows, cols, env: vec![],
        }).await?;
        let mut reader = transport.take_reader().expect("first call");

        let mut term = vector_term::Term::new(cols, rows, 10_000);

        // Three tasks: stdin → PTY; PTY → parser; periodic render.
        let writer_tx = /* clone of transport's write channel */;
        tokio::spawn(stdin_to_pty(writer_tx));

        let mut render_tick = tokio::time::interval(Duration::from_millis(33));   // ~30 Hz
        loop {
            tokio::select! {
                Some(chunk) = reader.recv() => { term.feed(&chunk); }            // mark dirty
                _ = render_tick.tick()      => { render_grid_to_stdout(&term)?; }
                status = transport.wait()   => { render_grid_to_stdout(&term)?; break; }
            }
        }
        Ok::<(), anyhow::Error>(())
    })
}
```

**Rendering ANSI emit:** simplest correct approach — each tick, write `\x1b[H` (cursor home) + `\x1b[2J` (clear screen) + for each row, emit cells as `\x1b[{r};1H` then per-cell `\x1b[38;2;{r};{g};{b}m\x1b[48;2;{r};{g};{b}m{ch}` then `\x1b[0m` at end of row. Final `\x1b[{r};{c}H` to position cursor where `Term` says it is. ~12 KB per repaint on 80×24, acceptable for human-driver iteration.

**Better (planner's call if time permits):** track per-row dirty flags and only re-emit dirty rows. `alacritty_terminal::grid::Grid` exposes `display_offset()` + line iteration; per-row dirty tracking can be Phase 3's renderer concern. **Recommendation: ship full-repaint in Phase 2; defer damage tracking to Phase 3** where it pays for itself against wgpu draw calls.

### Anti-Patterns to Avoid

- **`String::from_utf8_lossy` on PTY chunks** — Pitfall 4. Feed `&[u8]` directly to `Processor::advance`.
- **Direct `read()` on master in a tokio worker thread** — Pitfall 7. Use `spawn_blocking`.
- **Holding a `Term` lock across `.await`** — Anti-Pattern 5, D-11 lints it (`clippy::await_holding_lock = "deny"` at workspace level). The `vector-term` API is `&mut self` — no mutex needed inside the crate. If a Mutex appears around `Term`, it lives at the caller boundary in `vector-headless` (and later `vector-mux`). Pattern: lock, mutate, drop, then await. Compile will reject the wrong shape.
- **Storing `String` per cell** — `alacritty_terminal::term::cell::Cell` stores `char` + flags. Stay with that. Performance Trap 7 from PITFALLS.md.
- **Rolling raw `nix::pty::*` for the PTY** — Pitfall 7. Use `portable-pty`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| VT escape-sequence state machine | Custom Williams parser | `alacritty_terminal` (transitively `vte`) | Pitfall 1 — six weeks of escape edge cases, CVE-class bugs in shipping terminals. Locked by ROADMAP. |
| PTY allocation, controlling-tty, forkpty | Raw `nix` syscalls | `portable-pty 0.9` | Pitfall 7 — decades of Unix subtleties. Authored by WezTerm's maintainer. |
| Partial-UTF-8 reassembly across reads | Buffer + `from_utf8` loop | Just feed bytes to the parser | Pitfall 4 — parser handles it internally. |
| Regex search over grid + scrollback | Build my own DFA + walk cells | `alacritty_terminal::term::search::RegexSearch` + `RegexIter` | Already optimized; supports forward+backward DFAs; published API. |
| Grapheme cell-width tables | Code-point match tables | `unicode-width` (via `alacritty_terminal`) | CORE-02 falls out for free. Pitfall 2. |
| Parent-terminal raw-mode toggling | `unsafe { termios }` | `crossterm 0.29` (binary-local only) | One dep for one binary; alternative is `nix` + `unsafe`. |
| Mode flag tracking (bracketed-paste, mouse 1006/1002/1003, DECSCUSR shape) | Hand-roll dispatch | `alacritty_terminal` already tracks these on `Term::mode()` and cursor state | CORE-06 falls out. Tests just assert state. |

**Key insight:** Phase 2 is a **glue phase**, not an authoring phase. Every difficult problem in this domain has been solved by `alacritty_terminal` + `portable-pty`. The work is: wire them together, lock the trait surface for downstream phases, and prove correctness with a focused fixture corpus. Resist any urge to "do it ourselves."

## Runtime State Inventory

> Phase 2 is greenfield code on three empty crate stubs (`vector-term`, `vector-pty`, `vector-mux`). No existing data, no service config, no OS state to migrate.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — three crates are empty `lib.rs` stubs with no persistence. | None |
| Live service config | None — phase ships a CLI binary, no services. | None |
| OS-registered state | None — `vector-headless` is a run-to-exit binary; no daemon, launchd, or pm2 registration. | None |
| Secrets/env vars | `TERM=xterm-256color` is **set into the child** by `LocalDomain`, not consumed from environment. `$SHELL` is read from environment with fallback. No secrets. | None |
| Build artifacts | New `crates/vector-headless/` (if planner adopts recommendation) adds a binary target. CI's per-crate `tests/no_tokio_main.rs` file-count guard auto-tracks via `ls -d crates/vector-*`. | None — guard is dynamic |

**Architecture-lint contract update:** if `vector-headless` becomes its own crate, ensure its `tests/no_tokio_main.rs` has `BLOCK_ON_ALLOWLIST = &["src/main.rs"]` (the single `rt.block_on(async { ... })` at entry, mirroring `vector-app`'s allowlist). The existing CI grep (ci.yml line 65) already excludes `crates/**/tests/no_tokio_main.rs`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` / `rustc 1.88.0` | Workspace builds | ✓ | cargo 1.88.0, rustc 1.88.0 (verified on dev box 2026-05-11) | — |
| `/bin/zsh` (default `$SHELL` on macOS 13+) | `vector-headless` smoke test | ✓ (macOS ships zsh by default since Catalina) | system | `/bin/bash` (also always present) |
| Parent terminal supports raw-mode + ANSI (any modern terminal: iTerm/ghostty/Apple Terminal) | `vector-headless` interactive use | ✓ | — | If parent doesn't support raw-mode, `crossterm::terminal::enable_raw_mode()` returns Err — handle gracefully. |
| `vim`, `tmux`, `htop`, `less` for manual real-world fixtures | Manual smoke testing per CONTEXT §specifics | ⚠ `vim` and `less` ship with macOS; `tmux` and `htop` are Homebrew installs | — | If absent, fixtures auto-skip in CI; manual user verification covers them. |
| GitHub Actions `macos-14` runner (existing CI) | Running cargo test --workspace in CI | ✓ | — | — |
| `lipo`, `cargo-bundle`, `create-dmg` | **NOT needed in Phase 2** — these only matter when building the .app/.dmg, which Phase 2 doesn't touch | n/a | — | — |

**Missing dependencies with no fallback:** None — Phase 2 has zero external service dependencies.
**Missing dependencies with fallback:** None.

## Common Pitfalls

### Pitfall 1: Holding the `Term` across `.await`

**What goes wrong:** Renderer or input task can't acquire the lock; eventual deadlock or UI stall.
**Why it happens:** Easy to write `let mut t = mutex.lock(); t.feed(bytes); other_io.await;` in the bridge code.
**How to avoid:** D-11 workspace lint catches it (`clippy::await_holding_lock = "deny"`). Pattern: lock → mutate → drop → await. Phase 2's `Term` API is `&mut self`; the only mutex appears at the headless-binary's parser-task boundary — keep its critical sections synchronous.
**Warning signs:** clippy lint fires in CI.

### Pitfall 2: Reader task survives Term destruction

**What goes wrong:** Tokio task holding a `Sender<Vec<u8>>` keeps running after the Term/headless process should exit, pinning the runtime.
**Why it happens:** No explicit drop or cancel. PTY reader is in `spawn_blocking` and is essentially uncancellable.
**How to avoid:** `LocalPty` drops the master fd on `Drop` → reader's `read()` returns EOF → reader task exits cleanly. **Verify by `Drop` impl** explicitly closing the master file descriptor or by ensuring `Box<dyn MasterPty>` is dropped before the headless binary exits.
**Warning signs:** Process lingers after Ctrl-D; `ps` shows a Rust process orphaned.

### Pitfall 3: Forgetting to `drop(pair.slave)` in the parent

**What goes wrong:** Closing the master doesn't terminate the child; `Child::wait` hangs; zombie shell on exit.
**Why it happens:** `PtyPair` owns both ends; if the parent keeps the slave open, the kernel doesn't deliver EOF when the master closes.
**How to avoid:** `let _ = pair.slave;` or `drop(pair.slave)` immediately after `spawn_command` consumes it. CORE-04 success-criterion #4 (`no zombie shells on exit`) tests this explicitly via `ps`.
**Warning signs:** Integration test for CORE-04 fails; `ps` shows `<defunct>` shell after headless exit.

### Pitfall 4: `crossterm::terminal::enable_raw_mode()` not restored on panic

**What goes wrong:** Headless panics mid-run; user's terminal stays in raw mode; user sees jumbled output and has to type `reset` blind.
**How to avoid:** Wrap in `scopeguard::guard` (or a custom `Drop` struct) that calls `disable_raw_mode()` on unwind. `tracing::error!` then resume_unwind.
**Warning signs:** A `cargo run --bin vector-headless` that fails leaves the parent terminal broken.

### Pitfall 5: SIGWINCH from parent terminal not propagated

**What goes wrong:** User resizes their iTerm window while `vector-headless` runs. The headless process keeps the old grid size; the child shell renders to 80 cols on a 120-col display.
**How to avoid:** In the headless binary, install a SIGWINCH handler (via `tokio::signal::unix::signal(SignalKind::window_change())`) and on every signal: `crossterm::terminal::size()` to get parent dims, `term.resize(cols, rows)`, `transport.resize(cols, rows, 0, 0)`. Per Pitfall 7 in PITFALLS.md, the kernel then SIGWINCHes the child pgrp automatically.
**Warning signs:** Resizing the parent terminal while running `vim` inside vector-headless doesn't reflow.

### Pitfall 6: Trait shape ossifies prematurely

**What goes wrong:** D-38 commits us to a `PtyTransport` shape that compiles for LocalPty in Phase 2 but turns out to be awkward when Phase 7's `russh::Channel` arrives — reshaping then breaks every downstream caller.
**How to avoid:** The proposed shape (mpsc-based read, async write, sync resize) is **deliberately byte-oriented** and matches the actual data-flow shapes of both `russh::ChannelStream` (push-based via channel) and `tokio-tungstenite` (WebSocket frames push-based). Stress-test the shape by writing a **compile-time** mock impl for `CodespaceDomain::spawn` that returns a `MockRusshChannel: PtyTransport` placeholder — if it compiles, the shape is sufficient. Don't actually wire russh in Phase 2.
**Warning signs:** Phase 7 planning surfaces "we can't actually impl PtyTransport for russh Channel without changing the trait."

### Pitfall 7: Regex search performance on 10k+ scrollback

**What goes wrong:** Naive regex search materializes the entire scrollback into one giant string (each line is `cols` chars wide × 10k rows = ~800 KB). Compile + match dominates fixture runtime.
**How to avoid:** `alacritty_terminal::term::search::RegexSearch` already runs as a streaming DFA over the grid in-place — no string materialization. **Use it.** Don't write `(0..10000).map(|r| grid_row_to_string(r)).collect::<String>()` style code. Confirms <100 ms for 10k-line search per CORE-03.
**Warning signs:** `cargo test conformance_search` takes >2 s.

### Pitfall 8: alacritty_terminal API drift between minor versions

**What goes wrong:** Code examples in this doc assume specific paths (`alacritty_terminal::vte::ansi::Processor`, `term::test::TermSize`, `index::Point`, `Direction`). Across alacritty releases these have moved.
**How to avoid:** Planner runs `cargo doc -p alacritty_terminal --open` after `cargo build` and verifies actual paths in 0.26.0 before writing tasks. If anything moved, public API of `vector-term` (`Term::new`, `feed`, `resize`, `grid`, `search`) does NOT change — only internals.
**Warning signs:** Compile fails on `use alacritty_terminal::...`.

## Code Examples

### Example 1: Conformance fixture — CSI cursor + SGR + ED

```rust
// crates/vector-term/tests/conformance_csi.rs
use vector_term::Term;

#[test]
fn echo_hello_lands_in_cell_0_0() {
    // ROADMAP success criterion #1 — the canonical smoke test.
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"hello");
    let cell = &term.grid()[(0, 0).into()];
    assert_eq!(cell.c, 'h');
}

#[test]
fn sgr_truecolor_24bit_foreground() {
    // CORE-02: \x1b[38;2;255;128;0m sets RGB foreground.
    use alacritty_terminal::vte::ansi::Color;
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"\x1b[38;2;255;128;0mX\x1b[0m");
    let cell = &term.grid()[(0, 0).into()];
    assert!(matches!(cell.fg, Color::Spec(rgb) if rgb.r == 255 && rgb.g == 128 && rgb.b == 0));
}

#[test]
fn ed_2_clears_visible_grid_not_scrollback() {
    // CORE-01: ED 2 leaves scrollback intact (matches xterm; PITFALLS UX section).
    let mut term = Term::new(80, 24, 1000);
    for i in 0..50 { term.feed(format!("line {i}\n").as_bytes()); }
    term.feed(b"\x1b[2J");
    // Scrollback retains lines 0..25 (50 lines emitted; ~24 visible cleared, rest in history).
    // Assert at least one historical row still contains "line 5".
    let needle = regex::Regex::new(r"line 5").unwrap();
    let matches = term.search(&needle);
    assert!(!matches.is_empty(), "ED 2 must not clear scrollback");
}
```

### Example 2: Partial-UTF-8 split across reads

```rust
// crates/vector-term/tests/conformance_utf8.rs
#[test]
fn utf8_multibyte_split_across_two_feeds() {
    // CORE-01: a 3-byte UTF-8 sequence (世 = U+4E16 = E4 B8 96) split across reads.
    let mut term = Term::new(80, 24, 1000);
    term.feed(&[0xE4, 0xB8]);          // first 2 bytes
    term.feed(&[0x96]);                // continuation
    let cell = &term.grid()[(0, 0).into()];
    assert_eq!(cell.c, '世');
}
```

### Example 3: DECSET 1049 alt-screen save/restore (vim's escape pattern)

```rust
#[test]
fn decset_1049_alt_screen_isolates_primary() {
    let mut term = Term::new(80, 24, 1000);
    term.feed(b"primary content\n");
    term.feed(b"\x1b[?1049h");                       // enter alt screen + save cursor
    term.feed(b"alt content");
    // Primary should be invisible while in alt screen.
    term.feed(b"\x1b[?1049l");                       // exit alt screen + restore cursor
    // After exit, primary content is back at row 0.
    let cell = &term.grid()[(0, 0).into()];
    assert_eq!(cell.c, 'p');
}
```

### Example 4: 10k scrollback + regex search (CORE-03)

```rust
// crates/vector-term/tests/conformance_search.rs
#[test]
fn ten_thousand_lines_regex_search_finds_match() {
    let mut term = Term::new(80, 24, 10_001);                       // headroom
    for i in 0..10_001 { term.feed(format!("line {i}\r\n").as_bytes()); }
    let re = regex::Regex::new(r"^line 9999$").unwrap();
    let matches = term.search(&re);
    assert_eq!(matches.len(), 1);
    // Optional: assert match location is near the bottom of scrollback.
    assert!(matches[0].start_col == 0);
}
```

### Example 5: Local PTY lifecycle (CORE-04)

```rust
// crates/vector-pty/tests/lifecycle.rs
#[tokio::test(flavor = "multi_thread")]   // arch-lint allowlist via #[tokio::test]? — NO, see note below
async fn spawn_echo_and_collect_output() {
    /* … */
}
```

**Architecture-lint note:** `#[tokio::test]` is on the D-08 forbid list. Integration tests that need an async runtime must build it manually:

```rust
#[test]
fn spawn_echo_and_collect_output() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2).enable_all().build().unwrap();
    rt.block_on(async {
        // … actual test body
    });
}
```

…AND the test file is under `crates/vector-pty/tests/lifecycle.rs`, which the existing arch-lint test (`tests/no_tokio_main.rs`) only scans for `src/`. Integration tests under `tests/*.rs` other than `no_tokio_main.rs` are **not** scanned, so `Builder::new_multi_thread()` is allowed there. Verified in `crates/vector-term/tests/no_tokio_main.rs` — `scan_dir` walks `src/` only.

But to be safe and consistent: keep test runtime construction inside the test body, not at module scope.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hand-rolled vt100/xterm state machine | `vte` + `alacritty_terminal` | ~2017 (Alacritty 0.x) | Saves 6+ weeks of work; Pitfall 1. |
| Synchronous PTY in main loop | `spawn_blocking` + mpsc bridge | tokio 1.0+ | Bounded backpressure; UI never blocks (Pitfall 7). |
| `String::from_utf8_lossy` on PTY chunks | Feed bytes directly to parser | always | Pitfall 4; correctness, not perf. |
| Raw `nix::fork() + posix_openpt()` | `portable-pty` | ~2018 (WezTerm public) | Pitfall 7. |
| `harfbuzz_rs` for shaping | `cosmic-text` / `crossfont` | — | **Phase 2 doesn't touch fonts** — listed for awareness only; shaping is Phase 3. |
| Embedded scripting config (Lua/Rhai) | TOML + serde | — | **Out of scope for Phase 2** — config is Phase 5. |

**Deprecated/outdated (do not use in Phase 2):**

- `tokio_pty_process` — superseded by `portable-pty`.
- `vte 0.10..0.14` direct deps — let `alacritty_terminal` pin its own `vte`.
- `String::from_utf8_lossy` on PTY chunks — destroys partial sequences.

## Open Questions

1. **`alacritty_terminal::Processor` exact path in 0.26.**
   - What we know: `Processor` exists; `processor.advance(&mut term, bytes)` is the byte-feed entry per Alacritty's `event_loop.rs`.
   - What's unclear: Exact module path in **0.26** — possibly `alacritty_terminal::vte::ansi::Processor` or simply `alacritty_terminal::vte::Parser` paired with `Term` as the `Perform` impl. **0.26 reorganized the public surface.**
   - Recommendation: First plan task runs `cargo add alacritty_terminal@0.26 && cargo doc --open -p alacritty_terminal` and pins the exact import paths in a 1-line "spike" SUMMARY. Adjust `vector-term::Term::feed` to match. Public API surface unchanged.

2. **`TermSize` / `Dimensions` trait — public vs. test-only.**
   - What we know: `Term::new<D: Dimensions>` takes anything implementing the `Dimensions` trait (`fn columns() -> usize; fn screen_lines() -> usize`). `alacritty_terminal::term::test::TermSize` exists.
   - What's unclear: Is `TermSize` exposed in non-test builds in 0.26?
   - Recommendation: Hand-roll a 5-line `struct VectorDims { cols: usize, rows: usize }` impl. Avoids depending on a `test::` module. Trivial.

3. **`Cell::fg` / `Cell::bg` color enum exact path.**
   - What we know: cells carry color attrs; truecolor support is mature.
   - What's unclear: Is it `alacritty_terminal::vte::ansi::Color`, `term::color::Color`, or both? `Color::Spec(Rgb)` vs `Color::Rgb(Rgb)` — naming has churned.
   - Recommendation: Same spike as Q1. `cargo doc --open` resolves it once.

4. **Per-row dirty flag exposure for damage tracking in pass-through render.**
   - What we know: `Grid` tracks dirty internally for the alacritty renderer.
   - What's unclear: Is dirty exposed publicly in 0.26, or do we diff cells ourselves?
   - Recommendation: **Phase 2 ignores this** — D-36 doesn't need damage tracking; full-repaint is fine. Phase 3 renderer is the right place to confront this.

5. **`async_trait` workspace dep.**
   - We need it for `Domain::spawn` and `PtyTransport::write`/`wait`.
   - Recommendation: Add `async-trait = "0.1"` at `[workspace.dependencies]`. Standard, well-maintained, used by tokio/axum ecosystem.

## Environment Availability

(Already detailed above under Step 2.6 outputs. No changes.)

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust built-in) + workspace-wide `cargo test --workspace --tests` (existing CI invocation at `.github/workflows/ci.yml` line 60) |
| Config file | None — uses `[[test]]` auto-discovery in each crate's `tests/` directory |
| Quick run command (single test) | `cargo test -p vector-term --test conformance_csi echo_hello -- --nocapture` |
| Per-crate run | `cargo test -p vector-term --tests` (≤ 1s for full conformance corpus per D-37) |
| Full suite command | `cargo test --workspace --tests` (existing CI command — Phase 2 adds tests, doesn't change the invocation) |
| Coverage tool | Not required for Phase 2; HARDEN phase (Phase 10) may add `cargo-llvm-cov`. |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CORE-01 | CSI cursor/SGR/ED/EL dispatch | unit | `cargo test -p vector-term --test conformance_csi` | ❌ Wave 0 — create `crates/vector-term/tests/conformance_csi.rs` |
| CORE-01 | OSC title/colors dispatch | unit | `cargo test -p vector-term --test conformance_osc` | ❌ Wave 0 |
| CORE-01 | DCS dispatch (state-machine + termination) | unit | `cargo test -p vector-term --test conformance_dcs` | ❌ Wave 0 |
| CORE-01 | DECSET 1049 alt-screen save/restore | unit | `cargo test -p vector-term --test conformance_modes alt_screen` | ❌ Wave 0 |
| CORE-01 | DECSTBM scroll regions | unit | `cargo test -p vector-term --test conformance_scroll regions` | ❌ Wave 0 |
| CORE-01 | Tab stops (HTS, CHT, TBC) | unit | `cargo test -p vector-term --test conformance_scroll tabs` | ❌ Wave 0 |
| CORE-01 | Partial-UTF-8 split across reads | unit | `cargo test -p vector-term --test conformance_utf8` | ❌ Wave 0 |
| CORE-02 | 24-bit truecolor SGR (38;2;r;g;b) | unit | `cargo test -p vector-term --test conformance_color truecolor` | ❌ Wave 0 |
| CORE-02 | 256-color SGR (38;5;n) | unit | `cargo test -p vector-term --test conformance_color indexed_256` | ❌ Wave 0 |
| CORE-02 | Emoji ZWJ grapheme cluster width | unit | `cargo test -p vector-term --test conformance_width emoji_zwj` | ❌ Wave 0 |
| CORE-02 | East Asian wide character cell width | unit | `cargo test -p vector-term --test conformance_width east_asian` | ❌ Wave 0 |
| CORE-03 | 10,000+ line scrollback regex search | unit | `cargo test -p vector-term --test conformance_search ten_thousand_lines` | ❌ Wave 0 |
| CORE-03 | Library `search(&Regex) -> Vec<Match>` API exists | doctest / unit | `cargo test -p vector-term --doc` and `cargo test -p vector-term search_api_shape` | ❌ Wave 0 |
| CORE-04 | Local PTY spawns shell, echo round-trips | integration | `cargo test -p vector-pty --test lifecycle spawn_echo` | ❌ Wave 0 — create `crates/vector-pty/tests/lifecycle.rs` |
| CORE-04 | Resize propagates SIGWINCH to child | integration | `cargo test -p vector-pty --test lifecycle resize_propagates` (spawn `stty size`, write SIGWINCH, read stty output before+after) | ❌ Wave 0 |
| CORE-04 | No zombie processes on exit | integration | `cargo test -p vector-pty --test lifecycle no_zombies` (parse `ps` output before+after) | ❌ Wave 0 |
| CORE-05 | `TERM=xterm-256color` advertised | integration | `cargo test -p vector-pty --test lifecycle term_env_var` (spawn `printenv TERM`) | ❌ Wave 0 |
| CORE-06 | Bracketed-paste mode 2004 sets state | unit | `cargo test -p vector-term --test conformance_modes bracketed_paste` (feed `\x1b[?2004h`, assert `term.mode().contains(...)`) | ❌ Wave 0 |
| CORE-06 | Mouse modes 1000/1002/1003 set state | unit | `cargo test -p vector-term --test conformance_modes mouse_modes` | ❌ Wave 0 |
| CORE-06 | DECSCUSR cursor-shape state | unit | `cargo test -p vector-term --test conformance_modes cursor_shape` | ❌ Wave 0 |
| (D-38) | `Box<dyn PtyTransport>` and `Box<dyn Domain>` are object-safe | compile-time | `cargo test -p vector-mux --test trait_object_safety` (compile-fails-test pattern: instantiate the boxes) | ❌ Wave 0 |
| (D-36) | `vector-headless` spawns and echo lands in cell (0,0) | smoke (binary) | `cargo run --bin vector-headless -- --cols 80 --rows 24` then manual `echo hello` (manual-only — see below) | n/a — manual smoke matrix |
| (D-36) | `vim`, `tmux`, `htop`, `less +F` run cleanly inside headless | manual | (per CONTEXT §specifics) — manual visual inspection by user before phase verifier closes | n/a — manual gate |

**Manual-only justification:** D-36's pass-through proxy is fundamentally a human-driver UX — the value is "do real-world TUIs render correctly?" Automating `vim`-inside-`vector-headless`-inside-CI would require a nested PTY harness that mocks keyboard input — high cost, low signal vs. a 60-second manual smoke. CORE-01..06 acceptance is fully covered by the automated corpus above; the manual gate is for D-36's UX claim only.

### Sampling Rate

- **Per task commit:** `cargo test -p {crate-under-edit} --tests` (only the crate being edited; <1s per D-37)
- **Per wave merge:** `cargo test --workspace --tests` (existing CI command; <30s expected including arch-lint + grep redundancy)
- **Phase gate:** `cargo test --workspace --tests` green + manual smoke matrix (vim, tmux, htop, less +F running inside `cargo run --bin vector-headless`) before `/gsd:verify-work`

### Wave 0 Gaps

Tasks the planner must schedule before any implementation:

- [ ] `crates/vector-term/tests/conformance_csi.rs` — covers CORE-01 CSI cases
- [ ] `crates/vector-term/tests/conformance_osc.rs` — covers CORE-01 OSC dispatch
- [ ] `crates/vector-term/tests/conformance_dcs.rs` — covers CORE-01 DCS dispatch
- [ ] `crates/vector-term/tests/conformance_modes.rs` — covers CORE-01 DECSET 1049 + CORE-06 (mode state assertions)
- [ ] `crates/vector-term/tests/conformance_scroll.rs` — covers CORE-01 DECSTBM + tab stops
- [ ] `crates/vector-term/tests/conformance_utf8.rs` — covers CORE-01 partial-UTF-8
- [ ] `crates/vector-term/tests/conformance_color.rs` — covers CORE-02 colors
- [ ] `crates/vector-term/tests/conformance_width.rs` — covers CORE-02 emoji + East Asian width
- [ ] `crates/vector-term/tests/conformance_search.rs` — covers CORE-03 search
- [ ] `crates/vector-pty/tests/lifecycle.rs` — covers CORE-04 + CORE-05
- [ ] `crates/vector-mux/tests/trait_object_safety.rs` — covers D-38 trait shape lock-in
- [ ] **Workspace dep additions** in `Cargo.toml [workspace.dependencies]`: `alacritty_terminal = "0.26"`, `portable-pty = "0.9"`, `regex = "1"`, `async-trait = "0.1"`.
- [ ] **If new `vector-headless` crate**: `crates/vector-headless/{Cargo.toml, src/main.rs, tests/no_tokio_main.rs}` + line in workspace `members`. The `tests/no_tokio_main.rs` for the binary crate uses `BLOCK_ON_ALLOWLIST = &["src/main.rs"]` matching `vector-app`'s pattern.
- [ ] No framework install needed — `cargo test` is built-in.

## Sources

### Primary (HIGH confidence)

- [`alacritty_terminal 0.26.0` docs.rs](https://docs.rs/alacritty_terminal/0.26.0/alacritty_terminal/) — main types, modules, `Term::new<D: Dimensions>` signature, `event::EventListener` trait, `term::Config` fields (incl. `scrolling_history: usize`)
- [`alacritty_terminal 0.26.0` `Term` struct](https://docs.rs/alacritty_terminal/0.26.0/alacritty_terminal/term/struct.Term.html) — confirmed constructor + `grid()` accessor + `resize<S: Dimensions>`
- [`alacritty_terminal 0.26.0` `term::search::RegexSearch`](https://docs.rs/alacritty_terminal/0.26.0/alacritty_terminal/term/search/struct.RegexSearch.html) — confirmed `RegexSearch::new(&str) -> Result<RegexSearch, Box<BuildError>>`
- [Alacritty `event_loop.rs` master](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/event_loop.rs) — confirmed `state.parser.advance(&mut **terminal, &buf[..unprocessed])` pattern, blocking PTY reader, mutex-guarded Term mutations
- [Alacritty `term/search.rs` master](https://github.com/alacritty/alacritty/blob/master/alacritty_terminal/src/term/search.rs) — confirmed `Match = RangeInclusive<Point>`, `Term::regex_search_left/right`, `RegexIter::new(start, end, direction, term, regex)`
- [`portable-pty 0.9.0` docs.rs](https://docs.rs/portable-pty/0.9.0/portable_pty/) — confirmed `PtySystem` / `PtyPair` / `MasterPty` / `SlavePty` / `Child` / `CommandBuilder` / `PtySize` API surface
- [`portable-pty 0.9.0` `MasterPty` trait](https://docs.rs/portable-pty/0.9.0/portable_pty/trait.MasterPty.html) — confirmed `resize`, `try_clone_reader`, `take_writer`, `get_size` are synchronous (`Result`-returning, not future-returning)
- [crates.io API live verification 2026-05-11](https://crates.io/api/v1/crates/) for `alacritty_terminal 0.26.0` (2026-04-06), `portable-pty 0.9.0` (2025-02-11), `regex 1.12.3` (2026-02-03), `unicode-width 0.2.2` (2025-10-06), `crossterm 0.29.0` (2025-04-05), `clap 4.6.1` (2026-04-15)
- `.planning/research/ARCHITECTURE.md` — `Domain`/`Pane`/`PtyTransport` pattern (WezTerm-derived); threading model with `EventLoopProxy` + dedicated I/O thread
- `.planning/research/STACK.md` — workspace dep pins; alternatives table; `winit + objc2-app-kit` rationale
- `.planning/research/PITFALLS.md` §1 (no custom VT parser), §4 (`&[u8]` to parser), §7 (PTY signal/resize), §11 (config sprawl — out of scope this phase but informs no-DSL discipline)
- `docs/adr/0002-winit-tokio-threading.md` — D-09 threading pattern; one allowlisted `block_on` per binary
- `docs/adr/0003-architecture-lint-mechanism.md` — D-08 per-crate `tests/no_tokio_main.rs` + CI grep redundancy
- `.planning/phases/01-foundation-ci-dmg-pipeline/01-VERIFICATION.md` — Phase 1 deliverables confirmed; CI grep allowlist excludes `crates/**/tests/no_tokio_main.rs`

### Secondary (MEDIUM confidence)

- [WezTerm `mux` crate workspace](https://github.com/wezterm/wezterm) — `Domain` trait shape (reference impl, not vendored)
- WebFetch of `alacritty_terminal` 0.26.0 module docs — confirms exposed types but does NOT show full method signatures or 0.26's exact module-path reorganization (see Open Question 1)

### Tertiary (LOW confidence — flagged for verification)

- Exact module path for `Processor` in `alacritty_terminal 0.26` — Open Question 1; planner verifies via `cargo doc` before writing the import statement.
- `TermSize` exposure outside `test` module in 0.26 — Open Question 2; planner hand-rolls 5-line `Dimensions` impl as workaround if not exposed.

## Project Constraints (from CLAUDE.md)

These project-level directives are non-negotiable. Tasks must honor them:

- **Workflow / do-not-push:** Commit each logical stage separately; **do not `git push`**. User reviews diffs asynchronously and pushes. (Repeated in `CLAUDE.md` §Constraints.)
- **Lint discovery order:** No `Makefile` or `justfile` exists. Fall back to `.github/workflows/ci.yml` (verified above): `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings` are the canonical lint commands. Tests: `cargo test --workspace --tests`. Use these verbatim.
- **Comments:** Succinct, one-line max, only when WHY is non-obvious. No block docstrings unless explicitly asked.
- **Tech stack discipline:** Rust workspace, `wgpu` for GPU (Phase 3, not 2), `alacritty_terminal` via the existing wrapper crate, `tokio` multi-thread runtime, no `tokio::main` (D-08 enforced).
- **Scope discipline:** Resist scope creep. Anything not on the v1 list (see REQUIREMENTS.md §"Out of Scope") defaults to deferral. Specifically rejected for Phase 2: Sixel/Kitty graphics, image protocols, custom terminfo, Lua/Python/JS scripting, plugins, IME, file browser, search UI.
- **macOS only for v1; macOS 13 baseline.** Phase 2 code is portable Rust except `vector-headless`'s `crossterm` raw-mode + SIGWINCH integration — also works on Linux as a developer-convenience side-benefit, but CI targets macOS 14 / macOS 15-Intel only.
- **GSD workflow:** Phase 2 implementation runs under `/gsd-execute-phase 2` after planning closes. No direct repo edits outside GSD.
- **Unsafe code:** `unsafe_code = "deny"` workspace-wide. `vector-app` opts in via `#![allow(unsafe_code)]` for AppKit FFI. `vector-term`, `vector-pty`, `vector-mux`, `vector-headless` must **NOT** opt in — they should compile clean with no `unsafe`.
- **`clippy::pedantic` warn + `clippy::await_holding_lock` deny.** D-11 catches the most common terminal-emulator bug class (Anti-Pattern 5).

## Metadata

**Confidence breakdown:**

- Standard stack: **HIGH** — versions verified live against crates.io API on 2026-05-11; matches STACK.md.
- Architecture (`PtyTransport` + `Domain` shapes, threading pattern): **HIGH** — extends Phase 1's locked threading model + ARCHITECTURE.md's WezTerm-derived pattern. Trait shape stress-tested mentally against Phases 7 + 8 data flows.
- Pitfalls: **HIGH** — five of seven phase-specific pitfalls are direct re-statements of PITFALLS.md items (§1, §4, §7, §11, plus Anti-Pattern 5). Two are novel to Phase 2 (trait ossification, raw-mode restoration on panic) — both straightforward.
- Fixture corpus + Validation Architecture: **HIGH** — maps every CORE-NN requirement to a named test file + command. Manual gate justified.
- Library API specifics (`Processor` path, `TermSize` public, `Point` constructor): **MEDIUM** — flagged in Open Questions 1–3. Public surface of `vector-term::Term` does NOT depend on these resolving any particular way. A 30-minute `cargo doc` spike at the top of Phase 2 nails them down.
- Pass-through render loop cadence (30 Hz, full-repaint): **MEDIUM** — locked by D-36 to "render each tick"; tick rate + dirty-tracking is Claude's discretion. The recommendation (30 Hz full-repaint) is defensible for the human-driver use case but not the only valid choice. Phase 3's renderer will revisit.

**Research date:** 2026-05-11
**Valid until:** 2026-06-10 (30 days — stack is mature and slow-moving; `alacritty_terminal` minor bumps every ~6 months; `portable-pty` annual)
