//! POLISH-01 watcher: notify-debouncer-full with 150 ms quiescent debounce.
//! D-69: 150 ms debounce. Pitfall 1: parent-dir watch handles atomic-rename inode swap.
//! D-73: themes dir watched non-recursively.

use crate::ConfigEvent;
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::{path::Path, sync::mpsc, time::Duration};

/// Spawn a debounced FS watcher over `config_path` (parent dir) + `themes_dir`.
/// Collapses every flush window into a single `ConfigEvent::Dirty { paths }`.
pub fn spawn_watcher(
    config_path: &Path,
    themes_dir: &Path,
    tx: mpsc::Sender<ConfigEvent>,
) -> anyhow::Result<impl Drop> {
    let mut debouncer = new_debouncer(
        Duration::from_millis(150),
        None,
        move |result: DebounceEventResult| match result {
            Ok(events) => {
                let mut paths: Vec<std::path::PathBuf> =
                    events.into_iter().flat_map(|e| e.paths.clone()).collect();
                paths.sort();
                paths.dedup();
                if !paths.is_empty() {
                    let _ = tx.send(ConfigEvent::Dirty { paths });
                }
            }
            Err(errs) => {
                let msg = format!("notify watcher errors: {errs:?}");
                tracing::warn!("{msg}");
                let _ = tx.send(ConfigEvent::Error(msg));
            }
        },
    )?;

    // Pitfall 1: watch the PARENT dir — atomic-rename swaps the file's inode.
    if let Some(parent) = config_path.parent() {
        debouncer.watch(parent, RecursiveMode::NonRecursive)?;
    }
    // D-73: themes dir, non-recursive (no subdirs by contract).
    if themes_dir.exists() {
        debouncer.watch(themes_dir, RecursiveMode::NonRecursive)?;
    }
    Ok(debouncer)
}
