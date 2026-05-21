//! B2 / D-79 OSC 7 consumers — new-pane cwd inheritance + tab-title cwd-stem suffix.

use std::path::PathBuf;

use vector_mux::{format_tab_title, spawn_cwd_for_with_proc, PaneCwdView, TransportKind};

#[test]
fn tab_title_with_osc7_cwd_stem() {
    let cwd = PathBuf::from("/Users/me/vector");
    assert_eq!(
        format_tab_title("zsh", Some(&cwd), TransportKind::Local),
        "zsh: vector"
    );
}

#[test]
fn tab_title_without_osc7_falls_back() {
    assert_eq!(format_tab_title("zsh", None, TransportKind::Local), "zsh");
}

#[test]
fn tab_title_handles_root_path() {
    // Root has no file_name → bare process name.
    let cwd = PathBuf::from("/");
    assert_eq!(
        format_tab_title("zsh", Some(&cwd), TransportKind::Local),
        "zsh"
    );
}

#[test]
fn new_pane_inherits_cwd_from_osc7() {
    let view = PaneCwdView {
        cwd: Some(PathBuf::from("/Users/me/code")),
        pid: Some(1234),
    };
    let cwd = spawn_cwd_for_with_proc(&view, |_| None, || Some(PathBuf::from("/home")));
    assert_eq!(cwd, PathBuf::from("/Users/me/code"));
}

#[test]
fn new_pane_falls_back_to_proc_pidinfo() {
    let view = PaneCwdView {
        cwd: None,
        pid: Some(1234),
    };
    let cwd = spawn_cwd_for_with_proc(
        &view,
        |_| Some(PathBuf::from("/tmp/work")),
        || Some(PathBuf::from("/home")),
    );
    assert_eq!(cwd, PathBuf::from("/tmp/work"));
}

#[test]
fn new_pane_falls_back_to_home() {
    let view = PaneCwdView {
        cwd: None,
        pid: Some(1234),
    };
    let cwd = spawn_cwd_for_with_proc(&view, |_| None, || Some(PathBuf::from("/Users/test")));
    assert_eq!(cwd, PathBuf::from("/Users/test"));
}
