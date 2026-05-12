//! Phase 5 ForwardingListener — forwards alacritty Event variants to channels.
//! Replaces the Phase-2 no-op listener.
//!
//! Non-blocking by design (`try_send` + `tracing::warn!` on full channel) so
//! that the renderer thread never stalls; PtyWrite replies (OSC 10/11/12) may
//! drop under sustained load — acceptable per CLAUDE.md "don't block main".

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::term::ClipboardType;
use alacritty_terminal::vte::ansi::Rgb;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum ClipboardEvent {
    Store(ClipboardType, String),
    /// OSC 52 read: D-70 denies reads in v1.
    LoadDenied,
}

pub struct ForwardingListener {
    pub write_tx: mpsc::Sender<Vec<u8>>,
    pub clipboard_tx: mpsc::Sender<ClipboardEvent>,
}

impl ForwardingListener {
    fn send_pty(&self, bytes: Vec<u8>) {
        if let Err(e) = self.write_tx.try_send(bytes) {
            tracing::warn!(
                ?e,
                "PTY write channel full or closed; dropping reply (likely OSC 10/11/12)"
            );
        }
    }
}

impl EventListener for ForwardingListener {
    fn send_event(&self, event: Event) {
        match event {
            Event::PtyWrite(s) => self.send_pty(s.into_bytes()),
            // Alacritty 0.26 routes OSC 10/11/12 (and 4 ; n ; ?) queries through
            // `Event::ColorRequest(idx, fmt)` — NOT `PtyWrite`. The listener
            // must invoke the callback with a color value to produce the reply.
            // Index 256=fg, 257=bg, 258=cursor; we return sensible defaults so
            // shell-side dark-mode detection (vim, neovim) round-trips. Real
            // theme integration is Plan 05-07's job.
            Event::ColorRequest(idx, fmt) => {
                let color = default_color_for(idx);
                let reply = fmt(color);
                self.send_pty(reply.into_bytes());
            }
            Event::ClipboardStore(kind, data) => {
                let _ = self.clipboard_tx.try_send(ClipboardEvent::Store(kind, data));
            }
            Event::ClipboardLoad(_, _) => {
                // D-70: OSC 52 reads always denied in v1; never invoke the callback.
                let _ = self.clipboard_tx.try_send(ClipboardEvent::LoadDenied);
            }
            _ => {}
        }
    }
}

/// Sensible defaults for OSC 10/11/12 replies; Plan 05-07 replaces with theme lookup.
fn default_color_for(idx: usize) -> Rgb {
    match idx {
        // 256 = Foreground, 258 = Cursor — light gray
        256 | 258 => Rgb {
            r: 0xeb,
            g: 0xeb,
            b: 0xeb,
        },
        // 257 = Background — near-black
        257 => Rgb {
            r: 0x18,
            g: 0x18,
            b: 0x18,
        },
        _ => Rgb { r: 0, g: 0, b: 0 },
    }
}
