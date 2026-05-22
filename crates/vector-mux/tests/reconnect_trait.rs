//! Wave 0 — trait-shape verification for Phase 9 reconnect.
//!
//! Confirms `Domain::reconnect_one_shot` is object-safe, `LocalDomain`
//! returns `Ok(None)`, and `DevTunnelDomain` still panics with the
//! Phase-9-Plan-02 forward-reference message.

use std::path::PathBuf;
use std::sync::Arc;

use vector_mux::{DevTunnelDomain, Domain, LocalDomain};

#[test]
fn reconnect_one_shot_trait_is_object_safe() {
    let _: Arc<dyn Domain> = Arc::new(LocalDomain::with_shell(PathBuf::from("/bin/bash")));
}

#[test]
fn local_domain_reconnect_returns_none() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let d = LocalDomain::with_shell(PathBuf::from("/bin/bash"));
        let out = d.reconnect_one_shot(24, 80).await.expect("Ok variant");
        assert!(out.is_none(), "LocalDomain reconnect must return Ok(None)");
    });
}

#[test]
#[should_panic(expected = "Phase 9 Plan 02: ReconnectableDevTunnelDomain")]
fn devtunnel_domain_reconnect_unimplemented() {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let d = DevTunnelDomain::new();
        let _ = d.reconnect_one_shot(24, 80).await;
    });
}
