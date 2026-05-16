//! POLISH-07 — Local profile → SpawnCommand → LocalDomain end-to-end.

use std::collections::BTreeMap;

use vector_config::{Kind, ProfileBlock};
use vector_mux::{Domain, LocalDomain, SpawnCommand};

/// Walk profile fields into a SpawnCommand. Real impl lives in vector-app
/// (Plan 05-10 will wire it from the picker selection); test inlines the
/// helper to avoid app-layer coupling.
fn spawn_command_for_profile(p: &ProfileBlock) -> SpawnCommand {
    let argv = p
        .startup_command
        .as_ref()
        .map(|c| vec!["/bin/sh".to_owned(), "-c".to_owned(), c.clone()]);
    let env = p
        .env
        .as_ref()
        .map(|e| e.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    SpawnCommand {
        argv,
        cwd: p.cwd_override.clone(),
        rows: 24,
        cols: 80,
        env,
    }
}

#[tokio::test(flavor = "current_thread")]
async fn profile_local_spawn() {
    let mut env = BTreeMap::new();
    env.insert("FOO".to_owned(), "bar".to_owned());
    let profile = ProfileBlock {
        kind: Some(Kind::Local),
        startup_command: Some("echo hi".to_owned()),
        env: Some(env),
        ..Default::default()
    };
    let cmd = spawn_command_for_profile(&profile);
    let domain = LocalDomain::new().expect("LocalDomain::new");
    let mut transport = domain.spawn(cmd).await.expect("spawn");
    let mut reader = transport.take_reader().expect("reader");

    // Drain output for up to 2 seconds; assert "hi" appears.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut combined = String::new();
    while std::time::Instant::now() < deadline {
        match tokio::time::timeout(std::time::Duration::from_millis(200), reader.recv()).await {
            Ok(Some(chunk)) => {
                combined.push_str(&String::from_utf8_lossy(&chunk));
                if combined.contains("hi") {
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => {
                if combined.contains("hi") {
                    break;
                }
            }
        }
    }
    assert!(
        combined.contains("hi"),
        "Local profile end-to-end: expected 'hi' in output, got {combined:?}"
    );
}
