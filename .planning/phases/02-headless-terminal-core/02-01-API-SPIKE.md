# Phase 2 — alacritty_terminal 0.26 API Spike

**Resolved:** 2026-05-11
**Tool:** `cargo doc -p alacritty_terminal --no-deps` + direct source inspection at `~/.cargo/registry/src/.../alacritty_terminal-0.26.0/`

## Confirmed Import Paths

| RESEARCH.md placeholder | Actual 0.26 path | Notes |
|-------------------------|-------------------|-------|
| `alacritty_terminal::vte::ansi::Processor` | `alacritty_terminal::vte::ansi::Processor` | byte-feed entry; `advance<H: Handler>(&mut self, &mut H, bytes: &[u8])`. `Processor` re-exported via the root `pub use vte;` in alacritty_terminal's lib.rs (line 20). |
| `alacritty_terminal::term::test::TermSize` | `alacritty_terminal::term::test::TermSize` (public, but in `test` module) — **decision: hand-roll `VectorDims`** | TermSize lives in `pub mod test` per `term/mod.rs:2415-2448`. Accessible at runtime but the `test` namespace signals intent; hand-rolling a 5-line `Dimensions` impl avoids semantic coupling to a test helper. |
| `alacritty_terminal::vte::ansi::Color` | `alacritty_terminal::vte::ansi::Color` | Variant for truecolor: **`Color::Spec(Rgb)`** (confirmed at `vte-0.15.0/src/ansi.rs:1128-1132`). Other variants: `Color::Named(NamedColor)`, `Color::Indexed(u8)`. `Cell::fg: Color` per `term/cell.rs:136`. |
| `Config.scrolling_history` | `Config.scrolling_history: usize` | Confirmed at `term/mod.rs:336`, default 10000 at `:359`. Used directly by `Term::new` at `:414`. |
| `term::search::RegexSearch` | `alacritty_terminal::term::search::RegexSearch` | Confirmed at `term/search.rs:25-32`. Constructor: `RegexSearch::new(&str) -> Result<RegexSearch, Box<BuildError>>` (unchanged from research). |
| `index::Point`, `index::Direction` | `alacritty_terminal::index::Point<L = Line, C = Column>`, `alacritty_terminal::index::Direction` | Confirmed at `index.rs:18` (Direction) and `:50` (Point). Unchanged from research. |

## Hand-roll Dimensions

`Dimensions` trait lives at `alacritty_terminal::grid::Dimensions` (`grid/mod.rs:486`). Three required methods: `total_lines`, `screen_lines`, `columns`.

```rust
struct VectorDims { cols: usize, rows: usize }

impl alacritty_terminal::grid::Dimensions for VectorDims {
    fn total_lines(&self) -> usize { self.rows }
    fn screen_lines(&self) -> usize { self.rows }
    fn columns(&self) -> usize { self.cols }
}
```

Notes:
- `total_lines` for the initial `Term::new` size is just `rows` (scrollback is allocated separately via `Config.scrolling_history`).
- `last_column`, `topmost_line`, `bottommost_line`, `history_size` are provided by the default trait impl — do NOT override.

## Downstream Impact

- Plan 02-02 (vector-term wrapper) uses these paths in `src/term.rs`, `src/parser.rs`, `src/search.rs`. The temporary `_api_probe` module in `crates/vector-term/src/lib.rs` proves the spike findings compile against real 0.26 — Plan 02-02 will replace it with the real `Term` wrapper.
- Public `vector_term::Term` API (`new`, `feed`, `resize`, `grid`, `search`) does NOT change regardless of spike outcome.
- For Plan 02-02 fixture authoring: `cell.fg` matches `Color::Spec(rgb)` for 24-bit truecolor; `Color::Indexed(n)` for SGR 38;5;n; `Color::Named(NamedColor::Foreground)` for reset.
- `Processor` requires a `Timeout` type parameter with default `StdSyncHandler` — call sites can use `Processor::<StdSyncHandler>::new()` or rely on the default.

## Open Question Resolutions

- **Q1 (Processor path)** — RESOLVED. `alacritty_terminal::vte::ansi::Processor`. No drift from research.
- **Q2 (TermSize exposure)** — RESOLVED. Public in `test` module but we hand-roll `VectorDims` per recommendation.
- **Q3 (Color variant naming)** — RESOLVED. `Color::Spec(Rgb)` confirmed; `Cell.fg: Color`. Matches the RESEARCH.md "Code Examples" §1 fixture as written.
- **Bonus (Config.scrolling_history)** — CONFIRMED. Field name is `scrolling_history: usize`, default 10000.
