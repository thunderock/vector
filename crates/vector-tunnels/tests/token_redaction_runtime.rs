//! HARDEN-03 D-11 runtime gate (vector-tunnels side).
//!
//! Static gate at vector-arch-tests/tests/no_token_in_debug_or_log.rs (D-29)
//! handles compile-time bans. This file is the RUNTIME complement: it
//! exercises the Microsoft/DevTunnels auth surface with `tracing` captured
//! to an in-memory buffer and asserts that zero token-shaped strings reach
//! the output.
//!
//! Self-verifying: the fake bearer uses the `eyJ` JWT-header prefix that the
//! regex matches, so if `Debug for AuthProvider` ever regresses to
//! `#[derive(Debug)]` (or someone adds a tracing call site that logs the raw
//! token), this test FAILS.

use std::io::Write;
use std::sync::{Arc, Mutex};

use regex::Regex;
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;
use vector_tunnels::{AuthProvider, DevTunnelsApi};

const FAKE_MICROSOFT_JWT: &str =
    "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJGQUtFX1RPS0VOX0ZPUl9URVNUSU5HIn0.SIGNATURE";
const FAKE_GITHUB_TOKEN: &str = "gho_FAKE_TOKEN_FOR_TESTING_devtunnels_path_42";
const TOKEN_SHAPE_RE: &str = r"(gho_|ghp_|gha_|ghs_|eyJ[A-Za-z0-9_-]{10,})";

#[derive(Clone)]
struct StringWriter(Arc<Mutex<Vec<u8>>>);

impl Write for StringWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| std::io::Error::other("poisoned"))?
            .extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for StringWriter {
    type Writer = StringWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[test]
fn microsoft_devtunnels_auth_debug_does_not_leak() {
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let writer = StringWriter(buf.clone());
    let subscriber = tracing_subscriber::fmt()
        .with_writer(writer)
        .with_max_level(Level::TRACE)
        .with_ansi(false)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let api = DevTunnelsApi::new();
        let ms = AuthProvider::Microsoft(FAKE_MICROSOFT_JWT.to_string());
        let gh = AuthProvider::GitHub(FAKE_GITHUB_TOKEN.to_string());
        tracing::debug!(?api, ms_auth = ?ms, gh_auth = ?gh, "constructed api + auth providers (must redact)");
        tracing::info!("ms provider: {ms:?}");
        tracing::warn!("gh provider: {gh:?}");
    });

    let captured = {
        let g = buf.lock().unwrap();
        String::from_utf8_lossy(&g).into_owned()
    };
    let re = Regex::new(TOKEN_SHAPE_RE).unwrap();
    assert!(
        !re.is_match(&captured),
        "token-shaped string leaked into tracing output:\n{captured}"
    );
}

#[test]
fn auth_provider_format_does_not_leak_via_string_format() {
    let ms = AuthProvider::Microsoft(FAKE_MICROSOFT_JWT.to_string());
    let gh = AuthProvider::GitHub(FAKE_GITHUB_TOKEN.to_string());
    let ms_dbg = format!("{ms:?}");
    let gh_dbg = format!("{gh:?}");
    let re = Regex::new(TOKEN_SHAPE_RE).unwrap();
    assert!(
        !re.is_match(&ms_dbg),
        "Microsoft AuthProvider Debug leaked: {ms_dbg}"
    );
    assert!(
        !re.is_match(&gh_dbg),
        "GitHub AuthProvider Debug leaked: {gh_dbg}"
    );
}
