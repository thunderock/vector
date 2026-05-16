//! Phase 6 / CS-01..03 — tokio actor for picker REST calls.
//! All UI updates flow through EventLoopProxy<UserEvent>.

use std::sync::Arc;
use std::time::Duration;

use tokio::runtime::Handle;
use tokio_util::sync::CancellationToken;
use winit::event_loop::EventLoopProxy;

use vector_codespaces::{ClientError, CodespacesClient};

use crate::UserEvent;

/// One-shot list fetch. Emits CodespacesLoaded or CodespacesLoadFailed.
/// On `ClientError::Unauthenticated`, emits `UserEvent::AuthRequired`.
pub fn spawn_fetch_codespaces(
    handle: &Handle,
    proxy: EventLoopProxy<UserEvent>,
    client: Arc<CodespacesClient>,
) {
    handle.spawn(async move {
        match client.list_with_refresh().await {
            Ok(list) => {
                let _ = proxy.send_event(UserEvent::CodespacesLoaded(Arc::new(list)));
            }
            Err(ClientError::Unauthenticated) => {
                let _ = proxy.send_event(UserEvent::AuthRequired);
            }
            Err(e) => {
                tracing::warn!(error = %e, "codespaces_fetch_failed");
                let _ = proxy.send_event(UserEvent::CodespacesLoadFailed(e.to_string()));
            }
        }
    });
}

/// Poll one row until terminal state OR cancel. Each tick emits
/// `CodespaceStateChanged`.
pub fn spawn_poll_row(
    handle: &Handle,
    proxy: EventLoopProxy<UserEvent>,
    client: Arc<CodespacesClient>,
    name: &str,
    cancel: CancellationToken,
) {
    let name_for_task = name.to_string();
    handle.spawn(async move {
        let proxy_for_cb = proxy.clone();
        let name_cb = name_for_task.clone();
        let _ = client
            .poll_until_available(
                &name_for_task,
                cancel,
                move |state| {
                    let _ = proxy_for_cb.send_event(UserEvent::CodespaceStateChanged {
                        name: name_cb.clone(),
                        state,
                    });
                },
                Duration::from_secs(120),
            )
            .await;
    });
}

/// Start a Shutdown codespace then poll until Available. UI-SPEC §5.2 Start button.
pub fn spawn_start_then_poll(
    handle: &Handle,
    proxy: EventLoopProxy<UserEvent>,
    client: Arc<CodespacesClient>,
    name: String,
    cancel: CancellationToken,
) {
    handle.spawn(async move {
        match client.start(&name).await {
            Ok(()) | Err(ClientError::StartFailed { status: 409 }) => {
                // 409 already swallowed inside client.start; belt-and-braces.
                let _ = proxy.send_event(UserEvent::ToastInfo("starting codespace…".into()));
            }
            Err(ClientError::Unauthenticated) => {
                let _ = proxy.send_event(UserEvent::AuthRequired);
                return;
            }
            Err(e) => {
                let _ = proxy.send_event(UserEvent::ToastInfo(format!(
                    "could not start codespace — try again ({e})"
                )));
                return;
            }
        }
        // Now poll.
        let proxy_for_cb = proxy.clone();
        let name_cb = name.clone();
        let _ = client
            .poll_until_available(
                &name,
                cancel,
                move |state| {
                    let _ = proxy_for_cb.send_event(UserEvent::CodespaceStateChanged {
                        name: name_cb.clone(),
                        state,
                    });
                },
                Duration::from_secs(120),
            )
            .await;
    });
}

/// Construct a CodespacesClient from the Keychain-stored access token, or
/// None if no token is present (caller falls back to AuthRequired).
#[must_use]
pub fn build_client_from_keychain() -> Option<Arc<CodespacesClient>> {
    use vector_codespaces::{build_octocrab, TokenStore};
    let store = TokenStore::new();
    let access = store.load_access()?;
    let octo = build_octocrab(&access, None).ok()?;
    Some(Arc::new(CodespacesClient::new(octo)))
}
