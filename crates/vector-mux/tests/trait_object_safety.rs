//! D-38: Box<dyn PtyTransport> and Box<dyn Domain> are object-safe.
//!
//! Compile-time check: if these lines compile, the traits are object-safe.
//! The end-to-end test also proves CORE-04/05 reach through the trait surface.

use vector_mux::{DevTunnelDomain, Domain, LocalDomain, PtyTransport};

#[test]
fn pty_transport_is_object_safe() {
    // If `Box<dyn PtyTransport>` is not a valid type, this fn signature fails to compile.
    fn accepts_boxed(b: Box<dyn PtyTransport>) {
        drop(b);
    }
    let f: fn(Box<dyn PtyTransport>) = accepts_boxed;
    assert!(std::ptr::fn_addr_eq(
        f,
        accepts_boxed as fn(Box<dyn PtyTransport>)
    ));
}

#[test]
fn domain_is_object_safe() {
    fn accepts_boxed(b: Box<dyn Domain>) {
        drop(b);
    }
    let f: fn(Box<dyn Domain>) = accepts_boxed;
    assert!(std::ptr::fn_addr_eq(
        f,
        accepts_boxed as fn(Box<dyn Domain>)
    ));
}

#[test]
fn local_domain_constructs_when_shell_resolves() {
    if std::path::Path::new("/bin/zsh").exists()
        || std::path::Path::new("/bin/bash").exists()
        || std::env::var("SHELL").is_ok()
    {
        let d = LocalDomain::new().expect("LocalDomain::new on a host with a shell");
        assert_eq!(d.label(), "local");
        assert!(d.is_alive());
    }
}

// WIN-04: vector-mux must remain free of russh transitively. Remote transports
// are installed via `Mux::create_tab_async_with_transport` from outside crates.

#[test]
fn devtunnel_domain_compiles_with_unimplemented_body() {
    let d = DevTunnelDomain::new();
    assert_eq!(d.label(), "dev_tunnel");
    assert!(!d.is_alive());
}

#[test]
#[should_panic(expected = "create_tab_async_with_transport")]
fn devtunnel_spawn_panics_with_phase_marker() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let d = DevTunnelDomain::new();
        let _ = d.spawn(vector_mux::SpawnCommand::default()).await;
    });
}

#[test]
fn local_domain_spawn_yields_reader_and_clean_exit() {
    // End-to-end CORE-04/05 reachability through the trait surface.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let d = LocalDomain::with_shell(std::path::PathBuf::from("/bin/sh"));
        let cmd = vector_mux::SpawnCommand {
            argv: Some(vec!["sh".into(), "-c".into(), "echo hi".into()]),
            cwd: None,
            rows: 24,
            cols: 80,
            env: vec![],
        };
        let mut t = d.spawn(cmd).await.expect("spawn");
        let mut rx = t.take_reader().expect("take_reader first call");
        let mut collected = Vec::new();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(3);
        while let Ok(Some(chunk)) = tokio::time::timeout_at(deadline, rx.recv()).await {
            collected.extend_from_slice(&chunk);
        }
        let status = t.wait().await.expect("wait");
        assert_eq!(status, Some(0), "exit status via trait surface");
        assert!(
            String::from_utf8_lossy(&collected).contains("hi"),
            "expected 'hi' in collected output, got {:?}",
            String::from_utf8_lossy(&collected)
        );
    });
}
