//! HARDEN-03 D-11 runtime gate (vector-codespaces side).
//!
//! Static gate at vector-arch-tests/tests/no_token_in_debug_or_log.rs (D-29)
//! handles compile-time bans. This file is the RUNTIME complement: it
//! exercises GitHub OAuth code paths with `tracing` captured to an in-memory
//! buffer and asserts that zero token-shaped strings reach the output.
//!
//! Self-verifying: the fake token literally matches the regex
//! `gho_[A-Za-z0-9_]+`, so if the manual `Debug for Tokens` redaction ever
//! regresses (e.g. switched back to `#[derive(Debug)]`), this test FAILS.

use std::io::Write;
use std::sync::{Arc, Mutex};

use regex::Regex;
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;
use vector_codespaces::Tokens;
use zeroize::Zeroizing;

const FAKE_GITHUB_TOKEN: &str = "gho_FAKE_TOKEN_FOR_TESTING_1234567890abcdef";
const FAKE_GITHUB_REFRESH: &str = "ghr_FAKE_REFRESH_FOR_TESTING_abcdef0123456789";
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
fn github_oauth_tokens_debug_does_not_leak() {
    let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
    let writer = StringWriter(buf.clone());

    let subscriber = tracing_subscriber::fmt()
        .with_writer(writer)
        .with_max_level(Level::TRACE)
        .with_ansi(false)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let tokens = Tokens {
            access: Zeroizing::new(FAKE_GITHUB_TOKEN.to_string()),
            refresh: Some(Zeroizing::new(FAKE_GITHUB_REFRESH.to_string())),
        };
        tracing::debug!(?tokens, "constructed tokens (must be redacted)");
        tracing::info!(tokens = ?tokens, "info-level log of tokens");
        tracing::warn!("warn-level message referencing {:?}", tokens);
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
fn github_oauth_tokens_format_does_not_leak_via_string_format() {
    // A second surface: many callers stringify before logging. Confirm the
    // Display/Debug formatter never exposes raw tokens.
    let tokens = Tokens {
        access: Zeroizing::new(FAKE_GITHUB_TOKEN.to_string()),
        refresh: Some(Zeroizing::new(FAKE_GITHUB_REFRESH.to_string())),
    };
    let dbg = format!("{tokens:?}");
    let dbg_pretty = format!("{tokens:#?}");
    let re = Regex::new(TOKEN_SHAPE_RE).unwrap();
    assert!(!re.is_match(&dbg), "Debug leaked token: {dbg}");
    assert!(
        !re.is_match(&dbg_pretty),
        "pretty Debug leaked token: {dbg_pretty}"
    );
}
