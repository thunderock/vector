//! russh `Handler` impl. Plan 07-03 will extend; Plan 07-01 lands the
//! host-key check against the GitHub-API-supplied fingerprint (Pitfall 3).

// russh 0.60 vendors its own ssh-key fork; the Handler trait references
// `russh::keys::PublicKey`.
use russh::keys::{HashAlg, PublicKey};

/// Validates the server's host key against an expected SHA-256 fingerprint
/// supplied at connect time (e.g. from `GET /user/codespaces/{name}`).
pub struct VectorHandler {
    /// `"SHA256:..."` fingerprint as returned by the GitHub API.
    pub expected_fp: String,
}

impl VectorHandler {
    pub fn new(expected_fp: String) -> Self {
        Self { expected_fp }
    }
}

impl russh::client::Handler for VectorHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let actual_fp = server_public_key.fingerprint(HashAlg::Sha256).to_string();
        let ok = actual_fp == self.expected_fp;
        if !ok {
            tracing::warn!(
                actual = %actual_fp,
                expected = %self.expected_fp,
                "host key mismatch — refusing"
            );
        }
        Ok(ok)
    }
}
