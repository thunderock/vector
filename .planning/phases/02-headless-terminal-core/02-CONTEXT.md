# Phase 02: Headless Terminal Core - Context

**Gathered:** 2026-05-11
**Status:** Ready for planning

<domain>
## Phase Boundary

A library + headless binary that:

1. Wraps `alacritty_terminal 0.26` to parse VT escape sequences into an in-memory grid + scrollback.
2. Wraps `portable-pty 0.9` to spawn a local PTY-attached child shell, propagate `SIGWINCH`, and survive child-process exit cleanly.
3. Locks in the `PtyTransport` and `Domain` trait contracts that Phases 4/7/8/9 depend on.
4. Ships `cargo run --bin vector-headless` as a pass-through proxy: the user's `$SHELL` runs inside, output bytes round-trip through `alacritty_terminal`, and the grid is rendered back to the parent terminal each tick. Manually driveable with vim/tmux/htop as real-world fixtures.
5. Passes a focused VT conformance corpus (~20–40 hand-authored fixtures) covering CORE-01/02/03/06.

**Out of scope (by ROADMAP):** GPU rendering, native AppKit window, Sixel/Kitty graphics, custom terminfo, multi-pane mux, search UI.

</domain>

<decisions>
## Implementation Decisions

### Binary surface

- **D-36:** `vector-headless` is a **pass-through proxy**, not a snapshot tool or test-only fixture. It spawns `$SHELL` (fallback `/bin/zsh`), connects stdin/stdout to a `portable-pty` master via raw-mode bridging, parses output through `alacritty_terminal::Term`, and renders the grid back to the parent terminal each tick. Ctrl-D exits cleanly; child-process exit also exits cleanly. The binary lives at `crates/vector-app/src/bin/vector-headless.rs` (or its own `crates/vector-headless/` crate — researcher/planner picks based on existing scaffolding).

### VT conformance corpus

- **D-37:** **Roll our own focused fixtures.** ~20–40 `(input_bytes, expected_grid_state)` test pairs covering exactly the CSI/OSC/DCS/DECSET 1049/DECSTBM/ED/EL/partial-UTF-8/SGR-truecolor/grapheme-cluster-width cases listed in CORE-01/02/06. No vendoring of Alacritty's test suite; no `vttest`/`esctest` integration. The corpus runs in `cargo test` and finishes in <1s. Each fixture is a small Rust unit test that pushes bytes into `Term` and asserts grid state.

### Domain / PtyTransport trait shape

- **D-38:** **Full trait + LocalDomain + stubbed remote domains.** `PtyTransport` and `Domain` traits ship with their final shape locked in Phase 2. `LocalDomain` is fully implemented (uses `portable-pty`). `CodespaceDomain` and `DevTunnelDomain` ship as files containing the type + `impl Domain` with `unimplemented!("Phase N")` bodies — so Phases 7/8 just fill bodies, never reshape contracts. Cost: ~30 LOC of stubs. Benefit: every later phase compiles against the same trait surface.

### Scrollback search interface

- **D-39:** **Library API only.** `vector-term` exposes a `fn search(&self, regex: &Regex) -> Vec<Match>` (or equivalent — exact signature researcher's call) that returns row/col spans of every match across history. Phase 2 tests synthesize 10k+ line scrollbacks and assert match counts/positions. The user-facing search bar (Cmd-F overlay, highlighted matches, jump-to-match navigation) is **deferred to Phase 5 (Polish)** since it's a GPU + input-handling concern.

### Claude's Discretion

The following are downstream-agent calls — researcher/planner pick the best approach without re-asking the user:

- **Shell selection logic** — read `$SHELL`, fall back to `/etc/passwd` lookup, final fallback `/bin/zsh`. Standard practice.
- **Default grid size** — sensible default (80×24 or 100×30); CLI flags `--cols`/`--rows` to override. Watch parent terminal's `SIGWINCH` if running in pass-through mode.
- **Tracing levels for parser** — use the workspace-standard `tracing` crate; add `--debug-parser` flag if needed for diagnostics. No new logging infrastructure.
- **Lifecycle on shell exit** — child PID death → drain remaining PTY output → render final grid → exit 0. No zombie processes (verified in `ps`).
- **Error reporting style** — `anyhow` at the binary boundary; `thiserror` for library-level errors that callers may want to match on. Per project pattern from Phase 1.
- **Where the binary file lives** — `crates/vector-app/src/bin/vector-headless.rs` is the canonical Cargo pattern; planner may move it to its own crate if `vector-app`'s dependencies pull in things the headless binary shouldn't have.
- **Mux scope boundary** — Phase 2 does NOT pre-create `Pane`/`Tab`/`Window` types in `vector-mux`. The headless binary wires `LocalDomain` directly to a single `alacritty_terminal::Term`. Mux abstraction lands in Phase 4.

### Folded Todos

No backlog todos matched Phase 2 (no `/gsd:todo match-phase 2` matches at session start).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Architecture & Patterns

- `.planning/research/ARCHITECTURE.md` — `vector-term` ↔ `vector-mux` ↔ `vector-pty` boundary; `PtyTransport` + `Domain` trait shapes; rationale for "terminal core never branches on transport"
- `.planning/research/STACK.md` — pinned crate versions: `alacritty_terminal 0.26`, `portable-pty 0.9`, `vte 0.15` (transitive)
- `.planning/research/PITFALLS.md` §"Pitfall 1" — never roll a custom VT parser
- `.planning/research/PITFALLS.md` §"Pitfall 4" — feed `&[u8]` to the parser, never `from_utf8_lossy` on PTY chunks
- `.planning/research/PITFALLS.md` §"Pitfall 7" — PTY signal/resize handling via `portable-pty`

### Phase 1 carryover

- `docs/adr/0002-winit-tokio-threading.md` — winit on main, tokio on background, `EventLoopProxy::send_event` only cross-thread signal (D-09/D-10/D-11). vector-headless doesn't use winit but the tokio threading discipline still applies for the PTY I/O bridge.
- `docs/adr/0003-architecture-lint-mechanism.md` — per-crate `tests/no_tokio_main.rs` forbids `#[tokio::main]`, `Builder::new_current_thread()`, and `Runtime::block_on(` outside an allowlist; CI grep redundancy. vector-headless and vector-term must respect this.
- `.planning/phases/01-foundation-ci-dmg-pipeline/01-VERIFICATION.md` — Phase 1 deliverables verified-by-construction + operationally validated 2026-05-11

### Project-level

- `.planning/PROJECT.md` — Core value, Out of Scope (no Sixel/Kitty/custom terminfo)
- `.planning/REQUIREMENTS.md` §CORE — CORE-01 through CORE-06 acceptance criteria
- `.planning/ROADMAP.md` §"Phase 2: Headless Terminal Core" — goal + success criteria + stack additions
- `CLAUDE.md` — "do not push" workflow; succinct comments; lint discovery order

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`crates/vector-term/src/lib.rs`** (empty stub from Phase 1) — target crate for the VT/grid layer. Has the per-crate `no_tokio_main` architecture-lint test and workspace lint inheritance.
- **`crates/vector-pty/src/lib.rs`** (empty stub from Phase 1) — target crate for the local PTY wrapper.
- **`crates/vector-mux/src/lib.rs`** (empty stub from Phase 1) — target crate for `Domain` + `PtyTransport` trait definitions. Phase 2 ships the traits + LocalDomain; bodies for Pane/Tab/Window land in Phase 4.
- **`xtask/src/{dmg,release}.rs`** — already-shipping reference for `xshell::cmd!` + tokio-free CLI patterns. vector-headless can follow the same shape (anyhow + clap-derive).
- **`crates/vector-app/src/{main,app,tick}.rs`** — Phase 1 working reference for the winit + tokio dedicated-I/O-thread pattern. vector-headless's PTY bridge mirrors the structure but without winit.

### Established Patterns

- **Workspace dependencies (`Cargo.toml [workspace.dependencies]`):** `tokio = { features = ["rt-multi-thread", "macros", "time", "sync"] }`, `anyhow`, `thiserror`, `tracing`, `tracing-subscriber`. Add `alacritty_terminal`, `portable-pty`, and `regex` (for CORE-03) at workspace level so all consumers share a single version.
- **`Cargo.toml [workspace.lints]`:** clippy::pedantic + `clippy::await_holding_lock = "deny"` (D-11). Phase 2 code must pass these.
- **`tests/no_tokio_main.rs` per crate:** Phase 1's architecture lint. New crates `vector-term`/`vector-pty` already have these. If we add a new crate (e.g., `vector-headless` as its own crate), it must inherit the same test.

### Integration Points

- **vector-headless ↔ vector-term:** the binary owns `alacritty_terminal::Term` and pumps PTY bytes through it.
- **vector-term ↔ vector-pty:** indirect via the `PtyTransport` trait in `vector-mux`. Neither crate depends directly on the other.
- **vector-mux ↔ vector-pty:** `LocalDomain` (in vector-mux) constructs a `LocalPty` (in vector-pty) and returns it as `Box<dyn PtyTransport>`.
- **vector-pty ↔ tokio:** PTY reads happen on a `spawn_blocking` task; bytes flow to vector-term via `tokio::sync::mpsc`. Mirrors the Phase 1 tick pattern.

</code_context>

<specifics>
## Specific Ideas

- **Manual driver test:** `vector-headless` should be runnable with `vim`, `htop`, `tmux`, and `less +F` inside as real-world fixtures — these stress mode 1049 (alt-screen save/restore), DECSTBM (scroll regions), partial-UTF-8 (htop's box-drawing chars), and SGR truecolor (vim themes). If any of these visibly corrupt or hang, the parser is wrong.
- **`echo hello in cell (0,0)`:** the canonical smoke test from ROADMAP success criterion #1. Should be both a unit test (assert `term.grid()[(0, 0)].c == 'h'`) AND visible in pass-through mode.
- **Scrollback fixture:** synthesize 10,001 lines like `format!("line {n}\n")` and assert that `term.search(/line 9999/)` returns exactly one match at the expected row/col. Standard regex search via the `regex` crate.

</specifics>

<deferred>
## Deferred Ideas

- **Search UI (Cmd-F overlay, highlighted matches, jump-to-match navigation)** — Phase 5 (Polish). CORE-03's regex-search-across-history requirement is met by the library API in Phase 2; the user-facing surface needs GPU + input handling.
- **`Pane`/`Tab`/`Window` types in vector-mux** — Phase 4 (Mux). Phase 2 ships only the `Domain` + `PtyTransport` traits.
- **Vendoring Alacritty's full test corpus or integrating `vttest`/`esctest`** — out of scope; D-37 rolls our own focused fixtures. Revisit if conformance gaps surface in real-world use.
- **Bracketed paste / mouse modes / DECSCUSR input handling** — CORE-06 says the parser must set the right internal state. Actual input-event generation (mouse → SGR 1006 bytes back to the PTY, paste-key → mode-2004 wrapper) is downstream of Phase 3's input layer.
- **`CodespaceDomain` / `DevTunnelDomain` bodies** — Phases 7 and 8. Stubs ship in Phase 2 so trait contracts are locked.
- **Sixel, Kitty graphics protocol, image protocols generally** — ROADMAP Out of Scope.
- **Custom terminfo** — ROADMAP Out of Scope. Vector advertises `TERM=xterm-256color` and stays compatible.

### Reviewed Todos (not folded)

None — no pending todos matched Phase 2 scope.

</deferred>

---

*Phase: 02-headless-terminal-core*
*Context gathered: 2026-05-11*
