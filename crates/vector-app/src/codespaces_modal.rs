//! CS-01..03 / UI-SPEC §5.2 — Codespaces picker NSPanel.
//!
//! 640 px fixed width. Rows show state + repo + branch + last-used.
//! Connect / Save / Start dispatched via keyboard shortcuts (see app.rs).
//! Full target/selector wiring of per-row action buttons is deferred to
//! Plan 06-07 UAT feedback.

use std::path::PathBuf;
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSBackingStoreType, NSColor, NSFont, NSPanel, NSTextAlignment, NSTextField, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use tokio_util::sync::CancellationToken;

use vector_codespaces::{Codespace, CodespaceState};

use crate::relative_time::{humanize, state_color, state_label};

pub enum LoadState {
    Loading,
    Ready(Arc<Vec<Codespace>>),
    Error(String),
}

pub struct CodespacesPickerModal {
    panel: Retained<NSPanel>,
    rows_container_frame: NSRect,
    row_fields: parking_lot::Mutex<Vec<Retained<NSTextField>>>,
    footer: Retained<NSTextField>,
    state: LoadState,
    selected_index: Option<usize>,
    filter_text: String,
    /// One token shared by all in-flight poll tasks. Cancelled on dismiss.
    pub poll_cancel: CancellationToken,
}

impl CodespacesPickerModal {
    pub fn show(mtm: MainThreadMarker) -> Self {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(640.0, 480.0));
        let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable;
        let panel: Retained<NSPanel> = NSPanel::initWithContentRect_styleMask_backing_defer(
            mtm.alloc::<NSPanel>(),
            frame,
            style,
            NSBackingStoreType::Buffered,
            false,
        );
        panel.setTitle(&NSString::from_str("Codespaces"));
        panel.setLevel(objc2_app_kit::NSFloatingWindowLevel);
        panel.center();

        let rows_container_frame = NSRect::new(NSPoint::new(8.0, 32.0), NSSize::new(624.0, 416.0));
        let footer = make_label(
            mtm,
            "loading codespaces…",
            NSRect::new(NSPoint::new(8.0, 4.0), NSSize::new(624.0, 24.0)),
            11.0,
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
            state: LoadState::Loading,
            selected_index: None,
            filter_text: String::new(),
            poll_cancel: CancellationToken::new(),
        }
    }

    pub fn handle_loaded(&mut self, mtm: MainThreadMarker, list: Arc<Vec<Codespace>>) {
        let is_empty = list.is_empty();
        self.state = LoadState::Ready(list);
        self.selected_index = if is_empty { None } else { Some(0) };
        self.rerender(mtm);
    }

    pub fn handle_load_failed(&mut self, mtm: MainThreadMarker, err: String) {
        self.state = LoadState::Error(err);
        self.rerender(mtm);
    }

    pub fn handle_state_change(
        &mut self,
        mtm: MainThreadMarker,
        name: &str,
        new_state: CodespaceState,
    ) {
        if let LoadState::Ready(list) = &self.state {
            let mut v: Vec<Codespace> = (**list).clone();
            if let Some(row) = v.iter_mut().find(|c| c.name == name) {
                row.state = new_state;
            }
            self.state = LoadState::Ready(Arc::new(v));
        }
        let footer_text = format!("polling {name} ({})…", state_label(new_state));
        self.footer
            .setStringValue(&NSString::from_str(&footer_text));
        self.rerender(mtm);
    }

    /// Returns the currently-selected Codespace (for Connect / Save / Start dispatch).
    #[must_use]
    pub fn selected(&self) -> Option<Codespace> {
        let LoadState::Ready(list) = &self.state else {
            return None;
        };
        self.selected_index.and_then(|i| list.get(i)).cloned()
    }

    pub fn select_next(&mut self, mtm: MainThreadMarker) {
        if let LoadState::Ready(list) = &self.state {
            if !list.is_empty() {
                let idx = self.selected_index.unwrap_or(0);
                let next = (idx + 1) % list.len();
                self.selected_index = Some(next);
                self.rerender(mtm);
            }
        }
    }

    pub fn select_prev(&mut self, mtm: MainThreadMarker) {
        if let LoadState::Ready(list) = &self.state {
            if !list.is_empty() {
                let idx = self.selected_index.unwrap_or(0);
                let prev = if idx == 0 { list.len() - 1 } else { idx - 1 };
                self.selected_index = Some(prev);
                self.rerender(mtm);
            }
        }
    }

    pub fn set_filter(&mut self, mtm: MainThreadMarker, q: String) {
        self.filter_text = q;
        self.rerender(mtm);
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

        let list = match &self.state {
            LoadState::Ready(list) => list,
            LoadState::Loading => {
                let row = make_label(
                    mtm,
                    "loading codespaces…",
                    self.rows_container_frame,
                    13.0,
                    true,
                );
                content.addSubview(&row);
                self.row_fields.lock().push(row);
                return;
            }
            LoadState::Error(_) => {
                let row = make_label(
                    mtm,
                    "could not fetch codespaces — check your connection",
                    self.rows_container_frame,
                    13.0,
                    true,
                );
                content.addSubview(&row);
                self.row_fields.lock().push(row);
                return;
            }
        };

        if list.is_empty() {
            let row = make_label(
                mtm,
                "no codespaces found",
                self.rows_container_frame,
                13.0,
                true,
            );
            content.addSubview(&row);
            self.row_fields.lock().push(row);
            self.footer
                .setStringValue(&NSString::from_str("0 codespaces"));
            return;
        }

        let q = self.filter_text.to_lowercase();
        let row_h: f64 = 44.0;
        let mut shown: usize = 0;
        let mut fields = self.row_fields.lock();
        let top = self.rows_container_frame.origin.y + self.rows_container_frame.size.height;
        for (idx, cs) in list.iter().enumerate() {
            if !q.is_empty() {
                let hay = format!(
                    "{} {} {}",
                    cs.repository.full_name,
                    cs.git_status.ref_name,
                    cs.display_name.as_deref().unwrap_or("")
                )
                .to_lowercase();
                if !hay.contains(&q) {
                    continue;
                }
            }
            #[allow(clippy::cast_precision_loss)]
            let y = top - ((shown + 1) as f64) * row_h;
            let label = format!(
                "  ● {state:<10}  {repo:<32}  {branch:<16}  {time}",
                state = state_label(cs.state),
                repo = cs.repository.full_name,
                branch = cs.git_status.ref_name,
                time = humanize(elapsed_since(cs.last_used_at).max(0)),
            );
            let row_frame = NSRect::new(
                NSPoint::new(self.rows_container_frame.origin.x, y),
                NSSize::new(self.rows_container_frame.size.width, row_h),
            );
            let row = make_label(mtm, &label, row_frame, 13.0, false);
            if self.selected_index == Some(idx) {
                row.setBackgroundColor(Some(&NSColor::selectedControlColor()));
                row.setDrawsBackground(true);
            }
            let _ = state_color(cs.state); // reserved for future badge swatch
            content.addSubview(&row);
            fields.push(row);
            shown += 1;
        }

        let count = list.len();
        let footer_text = format!(
            "{count} codespace{plural} · last refreshed just now",
            plural = if count == 1 { "" } else { "s" }
        );
        self.footer
            .setStringValue(&NSString::from_str(&footer_text));
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

fn elapsed_since(ts: chrono::DateTime<chrono::Utc>) -> i64 {
    let now = chrono::Utc::now();
    (now - ts).num_seconds()
}

/// Compute the config.toml path: $XDG_CONFIG_HOME/vector/config.toml or
/// $HOME/.config/vector/config.toml.
#[must_use]
pub fn config_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("vector").join("config.toml")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
            .join(".config")
            .join("vector")
            .join("config.toml")
    } else {
        PathBuf::from("./config.toml")
    }
}
