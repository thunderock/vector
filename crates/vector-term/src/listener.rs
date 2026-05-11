//! Phase 2 NoopListener — Term events are dropped. Phase 4 mux will route.

use alacritty_terminal::event::{Event, EventListener};

pub(crate) struct NoopListener;

impl EventListener for NoopListener {
    fn send_event(&self, _: Event) {}
}
