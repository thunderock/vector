//! vector-headless: pass-through terminal proxy (D-36).
//!
//! Architecture:
//! - tokio multi-thread runtime owns: PTY bridge, stdin reader, SIGWINCH
//!   watcher, transport_actor (sole owner of `Box<dyn PtyTransport>`),
//!   render tick.
//! - Exactly one `rt.block_on(...)` lives in this file; the architecture-lint
//!   in `tests/no_tokio_main.rs` allowlists `main.rs` for that single call
//!   (D-09 / D-36).

mod bridge;
mod cli;
mod render;
mod sigwinch;

use std::sync::Arc;
use std::time::Duration;

use std::io::Write as _;

use anyhow::{Context, Result};
use clap::Parser;
use parking_lot::Mutex;
use tokio::sync::{mpsc, oneshot};
use tracing_subscriber::{fmt, EnvFilter};
use vector_mux::{Domain, LocalDomain, SpawnCommand};
use vector_term::Term;

use crate::bridge::{pump_pty_to_term, pump_stdin_to_pty, transport_actor, ResizeCmd, SharedTerm};

fn parent_size_or(default_cols: u16, default_rows: u16) -> (u16, u16) {
    crossterm::terminal::size().unwrap_or((default_cols, default_rows))
}

fn main() -> Result<()> {
    let cli = cli::Cli::parse();

    let filter = if cli.debug_parser {
        EnvFilter::new("vector_term=trace,vector_pty=trace,vector_mux=trace,vector_headless=trace")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn"))
    };
    fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(filter)
        .init();

    let (default_cols, default_rows) = parent_size_or(80, 24);
    let cols = cli.cols.unwrap_or(default_cols);
    let rows = cli.rows.unwrap_or(default_rows);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("vector-headless-worker")
        .build()
        .context("build tokio runtime")?;

    // Single allowlisted block_on (BLOCK_ON_ALLOWLIST = &["main.rs"]).
    rt.block_on(run(cols, rows, cli.scrollback))
}

async fn run(cols: u16, rows: u16, scrollback: usize) -> Result<()> {
    // Raw mode on parent; restore on every exit path including panic (Pitfall 4).
    crossterm::terminal::enable_raw_mode().context("enable_raw_mode")?;
    let _raw_guard = scopeguard::guard((), |()| {
        let _ = crossterm::terminal::disable_raw_mode();
        // Best-effort: clear screen + home cursor so the parent prompt isn't trashed.
        print!("\x1b[2J\x1b[H");
        let _ = std::io::stdout().flush();
    });

    // Construct LocalDomain + spawn PtyTransport. take_reader BEFORE handing
    // transport to the actor — `take_reader` needs `&mut self`.
    let domain = LocalDomain::new().context("LocalDomain::new (resolve shell)")?;
    let mut transport = domain
        .spawn(SpawnCommand {
            argv: None,
            cwd: None,
            rows,
            cols,
            env: vec![],
        })
        .await
        .context("Domain::spawn")?;
    let reader_rx = transport.take_reader().expect("take_reader first call");

    // Shared Term (parking_lot::Mutex; never locked across `.await`).
    let term: SharedTerm = Arc::new(Mutex::new(Term::new(cols, rows, scrollback)));

    // Actor channels + exit signal.
    let (write_tx, write_rx) = mpsc::channel::<Vec<u8>>(64);
    let (resize_tx, resize_rx) = mpsc::channel::<ResizeCmd>(16);
    let (exit_signal_tx, exit_signal_rx) = oneshot::channel::<()>();
    let (done_tx, done_rx) = oneshot::channel::<Result<Option<i32>>>();

    // transport_actor: sole owner of the transport.
    let _actor = tokio::spawn(transport_actor(transport, write_rx, resize_rx, done_tx));

    // stdin -> write_tx
    let _stdin_task = tokio::spawn(pump_stdin_to_pty(write_tx.clone()));

    // PTY -> Term; signals exit_signal_tx on reader EOF.
    let _pump_task = tokio::spawn(pump_pty_to_term(
        reader_rx,
        Arc::clone(&term),
        exit_signal_tx,
    ));

    // SIGWINCH watcher: term.resize() + resize_tx.send() to actor.
    let _winch_task = tokio::spawn(sigwinch::watch(Arc::clone(&term), resize_tx.clone()));

    // 30Hz render tick; exit on child EOF signal.
    let mut tick = tokio::time::interval(Duration::from_millis(33));
    let mut exit_signal_rx = exit_signal_rx;
    loop {
        tokio::select! {
            _ = tick.tick() => {
                render::render_grid_to_stdout(&term)?;
            }
            _ = &mut exit_signal_rx => {
                // Drain remaining output for ~100ms then paint final frame.
                tokio::time::sleep(Duration::from_millis(100)).await;
                render::render_grid_to_stdout(&term)?;
                break;
            }
        }
    }

    // Close actor channels so it can wait() and report.
    drop(write_tx);
    drop(resize_tx);
    let exit_status = match done_rx.await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            tracing::warn!("transport.wait error: {e}");
            None
        }
        Err(_) => None,
    };
    tracing::info!(?exit_status, "child exited; vector-headless shutting down");
    Ok(())
}
