//! POLISH-01 watcher: notify-debouncer-full debounce + atomic-rename re-arm (D-69, Pitfall 1).

use std::{sync::mpsc, time::Duration};
use tempfile::TempDir;
use vector_config::{spawn_watcher, ConfigEvent};

#[test]
fn debounce_150ms() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let themes = dir.path().join("themes");
    std::fs::write(&cfg, "[default]\n").unwrap();
    std::fs::create_dir(&themes).unwrap();

    let (tx, rx) = mpsc::channel::<ConfigEvent>();
    let _w = spawn_watcher(&cfg, &themes, tx).unwrap();

    // Give the watcher enough time to arm even on a loaded CI runner.
    std::thread::sleep(Duration::from_millis(200));

    // 3 rapid writes with no inter-write sleep — all land well within the
    // 150 ms debounce window regardless of CI scheduler jitter.
    for i in 0..3 {
        std::fs::write(&cfg, format!("[default]\n# write {i}\n")).unwrap();
    }
    std::thread::sleep(Duration::from_millis(500)); // wait for debounce flush

    let mut count = 0;
    while let Ok(_ev) = rx.recv_timeout(Duration::from_millis(50)) {
        count += 1;
    }
    assert!(count >= 1, "watcher missed all events");
    assert!(
        count <= 2,
        "debounce collapsing failed: got {count} events for 3 rapid writes (D-69 mandates 150ms quiescent collapse)"
    );
}

#[test]
fn atomic_rename_single_event() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("config.toml");
    let tmp = dir.path().join("config.toml.tmp");
    let themes = dir.path().join("themes");
    std::fs::write(&cfg, "[default]\n").unwrap();
    std::fs::create_dir(&themes).unwrap();

    let (tx, rx) = mpsc::channel::<ConfigEvent>();
    let _w = spawn_watcher(&cfg, &themes, tx).unwrap();

    std::thread::sleep(Duration::from_millis(50)); // let watcher arm
                                                   // Simulate vim atomic-save: write to tmp, then rename onto config.toml (Pitfall 1).
    std::fs::write(&tmp, "[default]\n# atomic\n").unwrap();
    std::fs::rename(&tmp, &cfg).unwrap();
    std::thread::sleep(Duration::from_millis(350));

    let mut got_dirty = false;
    while let Ok(ev) = rx.recv_timeout(Duration::from_millis(50)) {
        if matches!(ev, ConfigEvent::Dirty { .. }) {
            got_dirty = true;
        }
    }
    assert!(
        got_dirty,
        "atomic-rename (Pitfall 1) MUST surface a Dirty event via parent-dir watch"
    );
}
