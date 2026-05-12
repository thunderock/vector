//! POLISH-01 / POLISH-05 / UI-SPEC §5.4 — toast banner state machine.
//! Info: 36 px, auto-dismiss 5 s. Action: 56 px, until-dismissed.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToastMode {
    Info,
    Action { buttons: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct ToastBanner {
    pub text: String,
    pub mode: ToastMode,
    pub shown_at: Instant,
    pub focused_button: usize,
}

impl ToastBanner {
    pub const INFO_DISMISS_AFTER: Duration = Duration::from_secs(5);

    pub fn info(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            mode: ToastMode::Info,
            shown_at: Instant::now(),
            focused_button: 0,
        }
    }

    pub fn action(text: impl Into<String>, buttons: Vec<String>) -> Self {
        Self {
            text: text.into(),
            mode: ToastMode::Action { buttons },
            shown_at: Instant::now(),
            focused_button: 0,
        }
    }

    #[must_use]
    pub fn should_auto_dismiss(&self, now: Instant) -> bool {
        matches!(self.mode, ToastMode::Info)
            && now.duration_since(self.shown_at) >= Self::INFO_DISMISS_AFTER
    }

    /// UI-SPEC §5.4: info = 36 px, action = 56 px.
    #[must_use]
    pub fn height_px(&self) -> u32 {
        match self.mode {
            ToastMode::Info => 36,
            ToastMode::Action { .. } => 56,
        }
    }
}

/// At most ONE toast visible. New replaces old.
#[derive(Default)]
pub struct ToastStack {
    current: Option<ToastBanner>,
}

impl ToastStack {
    pub fn show(&mut self, t: ToastBanner) {
        self.current = Some(t);
    }
    pub fn dismiss(&mut self) {
        self.current = None;
    }
    pub fn tick(&mut self, now: Instant) {
        if let Some(t) = &self.current {
            if t.should_auto_dismiss(now) {
                self.current = None;
            }
        }
    }
    #[must_use]
    pub fn current(&self) -> Option<&ToastBanner> {
        self.current.as_ref()
    }
}
