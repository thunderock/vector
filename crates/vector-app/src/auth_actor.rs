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

use vector_codespaces::{build_octocrab, AuthError, GitHubAuth, TokenStore};

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
    handle.spawn(async move {
        run_flow(proxy, cancel_clone).await;
    });
    cancel
}

async fn run_flow(proxy: EventLoopProxy<UserEvent>, cancel: AuthCancellation) {
    let auth = match GitHubAuth::new() {
        Ok(a) => a,
        Err(e) => {
            let _ = proxy.send_event(UserEvent::AuthFailed {
                reason: format!("init: {e}"),
            });
            return;
        }
    };

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

    // Fetch @login (best-effort). On failure, fall back to "unknown" rather
    // than aborting the whole flow — the user is authenticated either way.
    let login = fetch_login(&tokens.access)
        .await
        .unwrap_or_else(|_| "unknown".to_string());
    let _ = proxy.send_event(UserEvent::AuthCompleted { user_login: login });
}

async fn fetch_login(token: &Zeroizing<String>) -> Result<String, String> {
    let octo = build_octocrab(token, None).map_err(|e| e.to_string())?;
    let user: octocrab::models::Author = octo.current().user().await.map_err(|e| e.to_string())?;
    Ok(user.login)
}
