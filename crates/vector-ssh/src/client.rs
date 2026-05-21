//! `SshClient` — owns a russh `Handle` after authentication.
//!
//! `connect_over` + `open_pty_shell` call `russh::client::connect_stream`
//! over any `AsyncRead+AsyncWrite` transport (TCP socket, Dev Tunnel relay
//! stream, subprocess stdio bridge, etc.).

use std::sync::Arc;

use russh::client::{self, Config, Handle, Msg};
use russh::keys::{PrivateKey, PrivateKeyWithHashAlg};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::SshError;
use crate::handler::VectorHandler;

pub struct SshClient {
    pub handle: Handle<VectorHandler>,
}

impl SshClient {
    /// Connect a russh client session over an arbitrary
    /// `AsyncRead + AsyncWrite` stream, perform public-key auth, and return
    /// the authenticated client.
    pub async fn connect_over<S>(
        stream: S,
        username: &str,
        identity: PrivateKey,
        host_key_fingerprint: String,
    ) -> Result<Self, SshError>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let config = Arc::new(Config::default());
        let handler = VectorHandler::new(host_key_fingerprint);
        let mut handle = client::connect_stream(config, stream, handler).await?;

        // Ed25519 ignores the hash alg; RSA would need an explicit choice.
        let key = PrivateKeyWithHashAlg::new(Arc::new(identity), None);
        let authed = handle.authenticate_publickey(username, key).await?;
        if !authed.success() {
            return Err(SshError::AuthFailed);
        }
        Ok(Self { handle })
    }

    /// Open a session channel, request a PTY with the given dimensions, and
    /// start a remote login shell. Returns the live channel for the transport
    /// task to drive.
    pub async fn open_pty_shell(
        &self,
        term: &str,
        rows: u16,
        cols: u16,
    ) -> Result<russh::Channel<Msg>, SshError> {
        let chan = self.handle.channel_open_session().await?;
        chan.request_pty(true, term, u32::from(cols), u32::from(rows), 0, 0, &[])
            .await?;
        chan.request_shell(true).await?;
        Ok(chan)
    }
}
