//! Phase 6 / AUTH-01 — tokio task driver for the device-flow state machine.
//!
//! Bridges to the main thread via `EventLoopProxy<UserEvent>`. The cancel flag
//! lets the modal's `Cancel sign-in` button abort an in-flight poll without
//! blocking on a network round-trip.
//!
//! Pitfall 14: token material is held in `Zeroizing<String>` end-to-end; only
//! the public GitHub login (e.g. "octocat") leaves this module via UserEvent.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::runtime::Handle;
use winit::event_loop::EventLoopProxy;
use zeroize::Zeroizing;

use vector_codespaces::{AuthError, GitHubAuth, TokenStore};

use crate::UserEvent;

/// Handle returned by `spawn_device_flow` so the modal's Cancel button can
/// abort. Cheaply clonable; shared across the actor task and the modal.
#[derive(Clone)]
pub struct AuthCancellation {
    flag: Arc<AtomicBool>,
}

impl AuthCancellation {
    /// Create a fresh, un-cancelled flag.
    #[must_use]
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

impl Default for AuthCancellation {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for AuthCancellation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthCancellation")
            .field("is_cancelled", &self.is_cancelled())
            .finish()
    }
}

/// Spawn the device-flow state machine on `handle`. Returns the cancel handle
/// the modal stashes so its Cancel button can abort.
pub fn spawn_device_flow(handle: &Handle, proxy: EventLoopProxy<UserEvent>) -> AuthCancellation {
    let cancel = AuthCancellation::new();
    let cancel_clone = cancel.clone();
    let proxy_for_supervisor = proxy.clone();
    handle.spawn(async move {
        // Supervisor: run the flow in a child task so panics surface as
        // JoinError instead of silently aborting the tokio worker. On panic,
        // emit AuthFailed with the panic payload so the UI can show it.
        let inner = tokio::spawn(async move { run_flow(proxy, cancel_clone).await });
        if let Err(err) = inner.await {
            if err.is_panic() {
                let payload = err.into_panic();
                let msg = panic_payload_to_string(&payload);
                tracing::error!(panic_msg = %msg, "auth_actor panicked");
                let _ = proxy_for_supervisor.send_event(UserEvent::AuthFailed {
                    reason: format!("internal error: {msg}"),
                });
            } else {
                tracing::error!(?err, "auth_actor join error");
            }
        }
    });
    cancel
}

fn panic_payload_to_string(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}

async fn run_flow(proxy: EventLoopProxy<UserEvent>, cancel: AuthCancellation) {
    tracing::info!("auth_actor: stage=new");
    let auth = match GitHubAuth::new() {
        Ok(a) => a,
        Err(e) => {
            let _ = proxy.send_event(UserEvent::AuthFailed {
                reason: format!("init: {e}"),
            });
            return;
        }
    };

    tracing::info!("auth_actor: stage=request_device_code");
    let (display, details) = match auth.request_device_code().await {
        Ok(v) => v,
        Err(e) => {
            let _ = proxy.send_event(UserEvent::AuthFailed {
                reason: format!("device_code: {e}"),
            });
            return;
        }
    };

    // Translate the `Instant`-based expires_at to a SystemTime so the modal
    // (which lives on the main thread) can render an absolute countdown
    // without holding a tokio-runtime reference.
    let remaining = display
        .expires_at
        .saturating_duration_since(std::time::Instant::now());
    let expires_at = SystemTime::now() + remaining;
    let _ = proxy.send_event(UserEvent::AuthDisplayCode {
        user_code: display.user_code.clone(),
        verification_uri: display.verification_uri.clone(),
        expires_at,
        interval_secs: display.interval_secs,
    });

    tracing::info!("auth_actor: stage=poll_for_token (display code emitted)");
    // Race the oauth poll against the cancellation flag. The flag is polled
    // every 200 ms — well below the device-flow interval (5 s typical) so the
    // user perceives Cancel as instant.
    let poll_fut = auth.poll_for_token(details);
    tokio::pin!(poll_fut);
    let token_result = loop {
        tokio::select! {
            result = &mut poll_fut => break result,
            () = tokio::time::sleep(Duration::from_millis(200)) => {
                if cancel.is_cancelled() {
                    let _ = proxy.send_event(UserEvent::AuthFailed {
                        reason: "cancelled".into(),
                    });
                    return;
                }
            }
        }
    };
    let tokens = match token_result {
        Ok(t) => t,
        Err(AuthError::Expired) => {
            let _ = proxy.send_event(UserEvent::AuthFailed {
                reason: "expired".into(),
            });
            return;
        }
        Err(AuthError::Cancelled) => {
            let _ = proxy.send_event(UserEvent::AuthFailed {
                reason: "cancelled".into(),
            });
            return;
        }
        Err(e) => {
            let _ = proxy.send_event(UserEvent::AuthFailed {
                reason: format!("oauth: {e}"),
            });
            return;
        }
    };

    tracing::info!("auth_actor: stage=save_tokens (poll returned ok)");
    // Persist tokens BEFORE emitting AuthCompleted so the menu rebuild path
    // sees a populated Keychain.
    let store = TokenStore::new();
    if let Err(e) = store.save_access(&tokens.access) {
        let _ = proxy.send_event(UserEvent::AuthFailed {
            reason: format!("keychain: {e}"),
        });
        return;
    }
    if let Some(refresh) = &tokens.refresh {
        // Refresh-save failure is non-fatal — refresh tokens are an optional
        // courtesy from GitHub; access alone is enough to drive the picker.
        let _ = store.save_refresh(refresh);
    }

    tracing::info!("auth_actor: stage=fetch_login");
    // Fetch @login via plain reqwest (auth.fetch_user_login) instead of
    // octocrab — octocrab's tower::buffer service has panicked in the field
    // on this single call site, and we don't need octocrab's surface here.
    let login = fetch_login(&auth, &tokens.access)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, "fetch_login failed; falling back to 'unknown'");
            "unknown".to_string()
        });
    tracing::info!(login = %login, "auth_actor: stage=emit_completed");
    let _ = proxy.send_event(UserEvent::AuthCompleted { user_login: login });
}

async fn fetch_login(auth: &GitHubAuth, token: &Zeroizing<String>) -> Result<String, String> {
    auth.fetch_user_login(token)
        .await
        .map_err(|e| e.to_string())
}
