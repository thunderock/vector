//! Phase 8 / Plan 08-05 Task 2 / UI-SPEC §S1 — DevTunnels picker NSPanel.
//!
//! Mirrors `crates/vector-app/src/codespaces_modal.rs` verbatim, swapping
//! UI-SPEC §Visual Diff vs Phase 6 strings.
//!
//! 640 × 480 px frame; rows container x=8 y=32 w=624 h=416; footer y=4 h=24.
//! Row height 22px (UI-SPEC §Spacing Scale exception). SF Mono 13pt rows;
//! SF Mono 11pt footer. Floating window level. Esc / Enter / ↑↓ / Cmd-S / R
//! per UI-SPEC §Interaction Contract.

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSFont, NSPanel, NSTextAlignment, NSTextField, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use tokio_util::sync::CancellationToken;

use crate::devtunnels_actor::TunnelView;

/// UI-SPEC §Spacing Scale (locked). Panel frame & subview rects.
pub const PANEL_W: f64 = 640.0;
pub const PANEL_H: f64 = 480.0;
pub const ROWS_X: f64 = 8.0;
pub const ROWS_Y: f64 = 32.0;
pub const ROWS_W: f64 = 624.0;
pub const ROWS_H: f64 = 416.0;
pub const FOOTER_X: f64 = 8.0;
pub const FOOTER_Y: f64 = 4.0;
pub const FOOTER_W: f64 = 624.0;
pub const FOOTER_H: f64 = 24.0;
pub const ROW_HEIGHT: f64 = 22.0;
pub const ROW_FONT_SIZE: f64 = 13.0;
pub const FOOTER_FONT_SIZE: f64 = 11.0;

/// UI-SPEC §Picker footer copy table (locked verbatim — never reword inline).
#[derive(Debug, Clone)]
pub enum FooterState {
    Loading,
    EmptySignedIn,
    NotSignedIn,
    SignedInOtherProvider { provider: String },
    ApiError { reason: String },
    Loaded { shown: usize, total: usize },
}

#[must_use]
pub fn footer_copy(state: &FooterState) -> String {
    match state {
        FooterState::Loading => "Loading Dev Tunnels…".to_string(),
        FooterState::EmptySignedIn => {
            "No Vector-agent tunnels yet. Install vector-tunnel-agent on a remote machine and run it.".to_string()
        }
        FooterState::NotSignedIn => {
            "Sign in with GitHub or Microsoft to list Dev Tunnels.".to_string()
        }
        FooterState::SignedInOtherProvider { provider } => format!(
            "No tunnels under your {provider} account. Switch providers or register one."
        ),
        FooterState::ApiError { reason } => {
            format!("Could not load tunnels: {reason}. Press R to retry.")
        }
        FooterState::Loaded { shown, total } => format!("{shown} of {total} tunnels."),
    }
}

/// UI-SPEC §Color — status dot color category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusColor {
    Live,
    Stale,
    Unreachable,
}

/// UI-SPEC §Row copy template + §Color (status dots). Single `●` glyph; color via
/// `setTextColor` on a leading sub-range. Categories: <5 min Live (green),
/// 5min..24h Stale (amber), older or unknown Unreachable (red).
#[must_use]
pub fn status_dot(last_seen_secs_ago: Option<u64>) -> (char, StatusColor) {
    let dot = '\u{25CF}'; // ●
    let color = match last_seen_secs_ago {
        Some(s) if s < 5 * 60 => StatusColor::Live,
        Some(s) if s < 24 * 60 * 60 => StatusColor::Stale,
        _ => StatusColor::Unreachable,
    };
    (dot, color)
}

/// UI-SPEC §Row copy template (locked layout):
/// `{status_dot}  {display_name}  {host}  ·  {last_seen}`
#[must_use]
pub fn format_row(view: &TunnelView) -> String {
    let (dot, _) = status_dot(view.last_seen_secs_ago);
    let last_seen = view.last_seen_secs_ago.map_or_else(
        || "never".to_string(),
        |s| crate::relative_time::humanize(i64::try_from(s).unwrap_or(0)),
    );
    format!(
        "{dot}  {}  {}  ·  {}",
        view.display_name, view.host, last_seen
    )
}

/// Modal context passed in at construction.
pub struct DevTunnelsModalCtx {
    pub poll_cancel: CancellationToken,
}

/// AppKit-backed picker panel. Holds the panel + footer label + row fields.
pub struct DevTunnelsPickerModal {
    panel: Retained<NSPanel>,
    rows_container_frame: NSRect,
    row_fields: parking_lot::Mutex<Vec<Retained<NSTextField>>>,
    footer: Retained<NSTextField>,
    views: Vec<TunnelView>,
    selected_index: Option<usize>,
    filter_text: String,
    pub poll_cancel: CancellationToken,
}

impl DevTunnelsPickerModal {
    pub fn show(mtm: MainThreadMarker, ctx: DevTunnelsModalCtx) -> Self {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(PANEL_W, PANEL_H));
        let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable;
        let panel: Retained<NSPanel> = NSPanel::initWithContentRect_styleMask_backing_defer(
            mtm.alloc::<NSPanel>(),
            frame,
            style,
            NSBackingStoreType::Buffered,
            false,
        );
        panel.setTitle(&NSString::from_str("Dev Tunnels"));
        panel.setLevel(objc2_app_kit::NSFloatingWindowLevel);
        panel.center();

        let rows_container_frame =
            NSRect::new(NSPoint::new(ROWS_X, ROWS_Y), NSSize::new(ROWS_W, ROWS_H));
        let footer = make_label(
            mtm,
            &footer_copy(&FooterState::Loading),
            NSRect::new(
                NSPoint::new(FOOTER_X, FOOTER_Y),
                NSSize::new(FOOTER_W, FOOTER_H),
            ),
            FOOTER_FONT_SIZE,
            true,
        );

        {
            let content = panel.contentView().expect("content");
            content.addSubview(&footer);
            panel.makeKeyAndOrderFront(None);
        }

        Self {
            panel,
            rows_container_frame,
            row_fields: parking_lot::Mutex::new(Vec::new()),
            footer,
            views: Vec::new(),
            selected_index: None,
            filter_text: String::new(),
            poll_cancel: ctx.poll_cancel,
        }
    }

    pub fn handle_loaded(&mut self, mtm: MainThreadMarker, views: Vec<TunnelView>) {
        let total = views.len();
        self.views = views;
        self.selected_index = if total == 0 { None } else { Some(0) };
        self.rerender(mtm);
        let shown = self.filtered_count();
        let footer = if total == 0 {
            footer_copy(&FooterState::EmptySignedIn)
        } else {
            footer_copy(&FooterState::Loaded { shown, total })
        };
        self.footer.setStringValue(&NSString::from_str(&footer));
    }

    pub fn handle_load_failed(&mut self, mtm: MainThreadMarker, reason: String) {
        self.views.clear();
        self.selected_index = None;
        self.rerender(mtm);
        self.footer
            .setStringValue(&NSString::from_str(&footer_copy(&FooterState::ApiError {
                reason,
            })));
    }

    pub fn handle_auth_required(&mut self, mtm: MainThreadMarker) {
        self.views.clear();
        self.selected_index = None;
        self.rerender(mtm);
        self.footer
            .setStringValue(&NSString::from_str(&footer_copy(&FooterState::NotSignedIn)));
    }

    #[must_use]
    pub fn selected(&self) -> Option<TunnelView> {
        self.filtered_indices()
            .into_iter()
            .nth(self.selected_index?)
            .and_then(|i| self.views.get(i).cloned())
    }

    pub fn select_next(&mut self, mtm: MainThreadMarker) {
        let n = self.filtered_count();
        if n == 0 {
            return;
        }
        let idx = self.selected_index.unwrap_or(0);
        self.selected_index = Some((idx + 1) % n);
        self.rerender(mtm);
    }

    pub fn select_prev(&mut self, mtm: MainThreadMarker) {
        let n = self.filtered_count();
        if n == 0 {
            return;
        }
        let idx = self.selected_index.unwrap_or(0);
        self.selected_index = Some(if idx == 0 { n - 1 } else { idx - 1 });
        self.rerender(mtm);
    }

    pub fn set_filter(&mut self, mtm: MainThreadMarker, q: String) {
        self.filter_text = q;
        self.selected_index = if self.filtered_count() == 0 {
            None
        } else {
            Some(0)
        };
        self.rerender(mtm);
    }

    fn filtered_indices(&self) -> Vec<usize> {
        let q = self.filter_text.to_lowercase();
        self.views
            .iter()
            .enumerate()
            .filter(|(_, v)| {
                if q.is_empty() {
                    return true;
                }
                let hay = format!("{} {}", v.display_name, v.host).to_lowercase();
                hay.contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn filtered_count(&self) -> usize {
        self.filtered_indices().len()
    }

    fn clear_rows(&self) {
        let mut fields = self.row_fields.lock();
        for f in fields.drain(..) {
            f.removeFromSuperview();
        }
    }

    fn rerender(&self, mtm: MainThreadMarker) {
        self.clear_rows();
        let content = self.panel.contentView().expect("content");
        let indices = self.filtered_indices();
        let mut fields = self.row_fields.lock();
        let top = self.rows_container_frame.origin.y + self.rows_container_frame.size.height;
        for (shown_idx, vi) in indices.into_iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let y = top - ((shown_idx + 1) as f64) * ROW_HEIGHT;
            let view = &self.views[vi];
            let label = format_row(view);
            let row_frame = NSRect::new(
                NSPoint::new(self.rows_container_frame.origin.x, y),
                NSSize::new(self.rows_container_frame.size.width, ROW_HEIGHT),
            );
            let row = make_label(mtm, &label, row_frame, ROW_FONT_SIZE, false);
            if self.selected_index == Some(shown_idx) {
                row.setBackgroundColor(Some(&NSColor::selectedControlColor()));
                row.setDrawsBackground(true);
            }
            content.addSubview(&row);
            fields.push(row);
        }
    }

    pub fn dismiss(&self) {
        self.poll_cancel.cancel();
        self.panel.orderOut(None);
    }

    pub fn is_key_window(&self) -> bool {
        self.panel.isKeyWindow()
    }
}

fn make_label(
    mtm: MainThreadMarker,
    text: &str,
    frame: NSRect,
    font_size: f64,
    muted: bool,
) -> Retained<NSTextField> {
    let f = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    f.setFrame(frame);
    f.setBezeled(false);
    f.setEditable(false);
    f.setAlignment(NSTextAlignment::Left);
    let font = NSFont::monospacedSystemFontOfSize_weight(font_size, 0.0_f64);
    f.setFont(Some(&font));
    if muted {
        f.setTextColor(Some(&NSColor::secondaryLabelColor()));
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(name: &str, host: &str, last_seen: Option<u64>) -> TunnelView {
        TunnelView {
            tunnel_id: format!("tid-{name}"),
            display_name: name.into(),
            host: host.into(),
            last_seen_secs_ago: last_seen,
        }
    }

    #[test]
    fn panel_frame_constants_lock_640x480() {
        assert!((PANEL_W - 640.0).abs() < f64::EPSILON);
        assert!((PANEL_H - 480.0).abs() < f64::EPSILON);
        assert!((ROWS_X - 8.0).abs() < f64::EPSILON);
        assert!((ROWS_Y - 32.0).abs() < f64::EPSILON);
        assert!((ROWS_W - 624.0).abs() < f64::EPSILON);
        assert!((ROWS_H - 416.0).abs() < f64::EPSILON);
        assert!((FOOTER_X - 8.0).abs() < f64::EPSILON);
        assert!((FOOTER_Y - 4.0).abs() < f64::EPSILON);
        assert!((FOOTER_W - 624.0).abs() < f64::EPSILON);
        assert!((FOOTER_H - 24.0).abs() < f64::EPSILON);
    }

    #[test]
    fn footer_copy_matches_ui_spec_verbatim() {
        // UI-SPEC §Picker footer copy — character-for-character.
        assert_eq!(footer_copy(&FooterState::Loading), "Loading Dev Tunnels…");
        assert_eq!(
            footer_copy(&FooterState::EmptySignedIn),
            "No Vector-agent tunnels yet. Install vector-tunnel-agent on a remote machine and run it."
        );
        assert_eq!(
            footer_copy(&FooterState::NotSignedIn),
            "Sign in with GitHub or Microsoft to list Dev Tunnels."
        );
        assert_eq!(
            footer_copy(&FooterState::SignedInOtherProvider {
                provider: "GitHub".into()
            }),
            "No tunnels under your GitHub account. Switch providers or register one."
        );
        assert_eq!(
            footer_copy(&FooterState::ApiError {
                reason: "503 Service Unavailable".into()
            }),
            "Could not load tunnels: 503 Service Unavailable. Press R to retry."
        );
        assert_eq!(
            footer_copy(&FooterState::Loaded { shown: 3, total: 5 }),
            "3 of 5 tunnels."
        );
    }

    #[test]
    fn format_row_template_no_vector_prefix() {
        let v = view("corp-dev-box-42", "corp-dev-box-42.host", Some(30));
        let line = format_row(&v);
        // No leading 'vector-' prefix per D-09.
        assert!(!line.contains("vector-"));
        // Template: `●  {name}  {host}  ·  {last_seen}`
        assert!(line.starts_with('\u{25CF}'));
        assert!(line.contains("corp-dev-box-42"));
        assert!(line.contains("corp-dev-box-42.host"));
        assert!(line.contains(" · "));
        assert!(line.contains("just now"));
    }

    #[test]
    fn status_dot_color_buckets() {
        assert_eq!(status_dot(Some(60)).1, StatusColor::Live);
        assert_eq!(status_dot(Some(300)).1, StatusColor::Stale); // exactly 5 min
        assert_eq!(status_dot(Some(86_400)).1, StatusColor::Unreachable);
        assert_eq!(status_dot(None).1, StatusColor::Unreachable);
    }
}
