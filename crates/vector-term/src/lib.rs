//! VT parser + grid + scrollback. Filled in Phase 2 atop `alacritty_terminal`.

use anyhow::Result;

pub trait Terminal: Send {}

#[allow(dead_code, unused_imports)]
fn _force_anyhow_use() -> Result<()> {
    Ok(())
}

#[allow(dead_code, unused_imports)]
mod _api_probe {
    // PROBE: confirms Plan 02-01 spike-resolved paths compile under alacritty_terminal 0.26.
    // Replaced by Plan 02-02. See .planning/phases/02-headless-terminal-core/02-01-API-SPIKE.md.
    use alacritty_terminal::Term;
    use alacritty_terminal::grid::Dimensions;
    use alacritty_terminal::index::{Direction, Point};
    use alacritty_terminal::term::Config;
    use alacritty_terminal::term::search::RegexSearch;
    use alacritty_terminal::vte::ansi::{Color, Processor};

    struct VectorDims {
        cols: usize,
        rows: usize,
    }

    impl Dimensions for VectorDims {
        fn total_lines(&self) -> usize {
            self.rows
        }
        fn screen_lines(&self) -> usize {
            self.rows
        }
        fn columns(&self) -> usize {
            self.cols
        }
    }

    fn _probe() {
        let _config = Config::default();
        let _ = Config { scrolling_history: 10_000, ..Config::default() };
        let _dims = VectorDims { cols: 80, rows: 24 };
        let _ = std::marker::PhantomData::<Term<alacritty_terminal::event::VoidListener>>;
        let _ = std::marker::PhantomData::<Processor>;
        let _ = std::marker::PhantomData::<Color>;
        let _ = std::marker::PhantomData::<RegexSearch>;
        let _ = std::marker::PhantomData::<Point>;
        let _ = std::marker::PhantomData::<Direction>;
    }
}
