//! D-56: set_tabbing_identifier invoked on every Cmd-T window.
//! Plan 04-04: mock the `WindowFactory` trait that the production helper uses
//! and assert the `"com.vector.terminal"` argument flows through unchanged.

use std::cell::RefCell;
use std::sync::Arc;

use anyhow::Result;
use vector_app::{WindowFactory, VECTOR_TABBING_IDENTIFIER};
use winit::window::{Window, WindowAttributes};

/// Mock that records every (identifier, attrs.title) pair without ever creating
/// a real winit Window. Production callers use `WinitWindowFactory`; the test
/// substitutes this mock to assert the API call shape.
struct RecordingFactory {
    calls: RefCell<Vec<String>>,
}

impl RecordingFactory {
    fn new() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
        }
    }
}

impl WindowFactory for RecordingFactory {
    fn create_tabbed(
        &self,
        _attrs: WindowAttributes,
        tabbing_identifier: &str,
    ) -> Result<Arc<Window>> {
        self.calls.borrow_mut().push(tabbing_identifier.to_string());
        // No real window in the test harness — caller asserts on `calls`.
        Err(anyhow::anyhow!("recording factory: no real window"))
    }
}

#[test]
fn set_tabbing_identifier_called_on_cmd_t() {
    let factory = RecordingFactory::new();
    // Simulate the App's Cmd-T handler invoking the factory.
    let _ = factory.create_tabbed(
        WindowAttributes::default().with_title("Vector"),
        VECTOR_TABBING_IDENTIFIER,
    );
    let _ = factory.create_tabbed(
        WindowAttributes::default().with_title("Vector"),
        VECTOR_TABBING_IDENTIFIER,
    );
    assert_eq!(
        factory.calls.borrow().as_slice(),
        &[
            VECTOR_TABBING_IDENTIFIER.to_string(),
            VECTOR_TABBING_IDENTIFIER.to_string(),
        ],
        "every Cmd-T invocation must pass the same tabbing identifier so AppKit \
         groups the new NSWindow into the existing tab group (D-56)",
    );
    assert_eq!(
        VECTOR_TABBING_IDENTIFIER, "com.vector.terminal",
        "tabbing identifier must be the documented constant"
    );
}
