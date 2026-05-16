//! Input bridge: routes keymap-encoded bytes from main thread → I/O actor write channel,
//! and owns the click-drag SelectionState. Plan 03-04.

use tokio::sync::mpsc;
use vector_input::SelectionState;

pub struct InputBridge {
    pub selection: SelectionState,
    pub write_tx: mpsc::Sender<Vec<u8>>,
    pub resize_tx: mpsc::Sender<(u16, u16)>,
}

impl InputBridge {
    pub fn new(write_tx: mpsc::Sender<Vec<u8>>, resize_tx: mpsc::Sender<(u16, u16)>) -> Self {
        Self {
            selection: SelectionState::default(),
            write_tx,
            resize_tx,
        }
    }

    pub fn send_bytes(&self, bytes: Vec<u8>) {
        if let Err(err) = self.write_tx.try_send(bytes) {
            tracing::warn!(?err, "input write channel full or closed; dropping bytes");
        }
    }

    pub fn send_resize(&self, rows: u16, cols: u16) {
        if let Err(err) = self.resize_tx.try_send((rows, cols)) {
            tracing::warn!(?err, "input resize channel full or closed; dropping");
        }
    }
}
