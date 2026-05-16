//! Plan 05-16 Task 2 — chrome render orchestration correctness tests.
//!
//! Tests verify (a) per-App-state draw decisions (ChromeDrawPlan) and
//! (b) the W6 order invariant: draw-call byte offsets in app.rs are
//! monotonically increasing per UI-SPEC §11.

use std::collections::BTreeMap;
use std::sync::{atomic::AtomicBool, Arc};

use tokio::sync::mpsc;
use vector_app::app::{App, PaneRectPx};
use vector_config::{ConfigFile, Kind, ProfileBlock};

fn make_app() -> App {
    let (write_tx, _write_rx) = mpsc::channel::<Vec<u8>>(64);
    let (resize_tx, _resize_rx) = mpsc::channel::<(u16, u16)>(8);
    let lpm = Arc::new(AtomicBool::new(false));
    App::new(write_tx, resize_tx, lpm)
}

fn cfg_with_tint(tint: &str) -> std::sync::Arc<ConfigFile> {
    let mut profile = BTreeMap::new();
    profile.insert(
        "default".to_owned(),
        ProfileBlock {
            kind: Some(Kind::Local),
            tint: Some(tint.to_owned()),
            ..ProfileBlock::default()
        },
    );
    std::sync::Arc::new(ConfigFile {
        default: ProfileBlock::default(),
        profile,
        ..ConfigFile::default()
    })
}

/// Summary of which chrome surfaces should draw given the current App state.
/// A pure snapshot from App fields — no wgpu involved.
#[allow(clippy::struct_excessive_bools)]
pub struct ChromeDrawPlan {
    pub draw_tint: bool,
    pub draw_search_bar: bool,
    pub draw_toast: bool,
    pub draw_picker: bool,
}

/// Compute the ChromeDrawPlan from the given App state (pure, no side effects).
pub fn chrome_draw_plan(app: &App) -> ChromeDrawPlan {
    ChromeDrawPlan {
        draw_tint: app.active_profile_tint_rgba_pub().is_some(),
        // MEDIUM-2: search bar only draws if open AND active_pane_rect is Some.
        draw_search_bar: app.search_bar_open() && app.active_pane_rect_is_some(),
        draw_toast: app.toasts_has_current(),
        draw_picker: app.profile_picker_open(),
    }
}

// ─── test helpers — additional accessors needed by this test ────────────────

trait AppTestExt {
    fn active_pane_rect_is_some(&self) -> bool;
    fn toasts_has_current(&self) -> bool;
}

impl AppTestExt for App {
    fn active_pane_rect_is_some(&self) -> bool {
        self.active_pane_rect_pub().is_some()
    }
    fn toasts_has_current(&self) -> bool {
        self.toasts_current_pub().is_some()
    }
}

// ─── tests ──────────────────────────────────────────────────────────────────

/// Default App → all chrome surfaces hidden.
#[test]
fn default_state_all_false() {
    let app = make_app();
    let plan = chrome_draw_plan(&app);
    assert!(!plan.draw_tint, "tint should not draw by default");
    assert!(
        !plan.draw_search_bar,
        "search bar should not draw by default"
    );
    assert!(!plan.draw_toast, "toast should not draw by default");
    assert!(!plan.draw_picker, "picker should not draw by default");
}

/// SearchBar open but active_pane_rect None → draw_search_bar == false (MEDIUM-2).
#[test]
fn search_bar_open_no_rect_does_not_draw() {
    let mut app = make_app();
    app.do_toggle_search();
    assert!(
        app.search_bar_open(),
        "search bar should be open after toggle"
    );
    app.set_active_pane_rect_for_test(None);
    let plan = chrome_draw_plan(&app);
    assert!(
        !plan.draw_search_bar,
        "search bar must not draw without active_pane_rect"
    );
}

/// SearchBar open + active_pane_rect Some → draw_search_bar == true.
#[test]
fn search_bar_open_with_rect_draws() {
    let mut app = make_app();
    app.do_toggle_search();
    assert!(app.search_bar_open());
    app.set_active_pane_rect_for_test(Some(PaneRectPx {
        x_px: 0.0,
        y_px: 0.0,
        w_px: 800.0,
        h_px: 600.0,
    }));
    let plan = chrome_draw_plan(&app);
    assert!(
        plan.draw_search_bar,
        "search bar must draw when open + rect known"
    );
}

/// ProfilePicker open → draw_picker == true.
#[test]
fn picker_open_draws() {
    let mut app = make_app();
    let cfg = std::sync::Arc::new(ConfigFile::default());
    app.set_current_config(cfg);
    app.do_open_profile_picker();
    let plan = chrome_draw_plan(&app);
    assert!(plan.draw_picker, "picker must draw when open");
}

/// Toast shown → draw_toast == true.
#[test]
fn toast_shown_draws() {
    let mut app = make_app();
    app.show_toast_for_test("test toast");
    let plan = chrome_draw_plan(&app);
    assert!(plan.draw_toast, "toast must draw when current is Some");
}

/// Config with tint on active profile → draw_tint == true.
#[test]
fn tint_color_configured_draws() {
    let mut app = make_app();
    let cfg = cfg_with_tint("#7a3aaf");
    app.set_current_config(cfg);
    let plan = chrome_draw_plan(&app);
    assert!(
        plan.draw_tint,
        "tint must draw when active profile has tint color"
    );
}

/// W6 order test: assert draw-call byte offsets in app.rs are monotonically increasing
/// per UI-SPEC §11 (tint → search_bar → toast → picker.scrim → picker.modal).
#[test]
fn chrome_draw_order_matches_ui_spec_section_11() {
    let src_path = concat!(env!("CARGO_MANIFEST_DIR"), "/src/app.rs");
    let src =
        std::fs::read_to_string(src_path).expect("app.rs must be readable for the W6 order test");

    let markers = [
        "chrome.tint.draw(",
        "chrome.search_bar.draw(",
        "chrome.toast.draw(",
        "chrome.picker.draw_scrim(",
        "chrome.picker.draw_modal(",
    ];

    let offsets: Vec<Option<usize>> = markers.iter().map(|m| src.find(m)).collect();

    for (i, m) in markers.iter().enumerate() {
        assert!(
            offsets[i].is_some(),
            "W6: draw marker not found in app.rs: `{m}`"
        );
    }

    for i in 0..markers.len() - 1 {
        let a = offsets[i].unwrap();
        let b = offsets[i + 1].unwrap();
        assert!(
            a < b,
            "W6 UI-SPEC §11 order violated: `{}` (offset {a}) must appear BEFORE `{}` (offset {b})",
            markers[i],
            markers[i + 1]
        );
    }
}
