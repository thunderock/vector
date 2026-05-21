//! `DevTunnelTransport` — `vector_mux::PtyTransport` over the JSON agent
//! protocol (Plan 08-04 D-A4). Mirrors the biased-select pump pattern from
//! `vector-ssh::SshChannelTransport` (Phase 7) but speaks newline-delimited
//! JSON frames against `vector-tunnel-agent` instead of russh channel data.
//!
//! Production `connect()` builds the wire from the Microsoft Dev Tunnels
//! relay SDK; tests use `new_with_stream` against `tokio::io::duplex`.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use vector_mux::{PtyTransport, TransportKind};
use vector_tunnel_protocol::{AgentMessage, PROTOCOL_VERSION};

use crate::model::TunnelRecord;

/// Agent control-channel port. The agent registers this via the Dev Tunnels
/// SDK `add_port_raw`. 32100 was chosen: unprivileged, outside common-services
/// (>= 32768 is the dynamic range, but 32100 is high enough to avoid any
/// well-known service collision while leaving room for adjacent future channels).
pub const AGENT_PORT: u16 = 32100;

#[derive(thiserror::Error)]
pub enum TransportError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("protocol version mismatch — update agent")]
    ProtocolVersion,
    #[error("agent returned error: {0}")]
    AgentError(String),
    #[error("disconnected")]
    Disconnected,
}

// Manual Debug — never include payload bytes (Pitfall 14 discipline).
impl std::fmt::Debug for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => f.debug_tuple("Io").field(e).finish(),
            Self::Protocol(s) => f.debug_tuple("Protocol").field(s).finish(),
            Self::ProtocolVersion => f.write_str("ProtocolVersion"),
            Self::AgentError(s) => f.debug_tuple("AgentError").field(s).finish(),
            Self::Disconnected => f.write_str("Disconnected"),
        }
    }
}

pub struct DevTunnelTransport {
    session_id: String,
    write_tx: mpsc::Sender<AgentMessage>,
    read_rx: Option<mpsc::Receiver<Vec<u8>>>,
    exit_rx: Option<oneshot::Receiver<Option<i32>>>,
    /// Pump task — aborts on drop.
    _pump: JoinHandle<()>,
}

impl std::fmt::Debug for DevTunnelTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DevTunnelTransport")
            .field("session_id", &self.session_id)
            .finish_non_exhaustive()
    }
}

impl DevTunnelTransport {
    /// Test seam — constructs the transport from any AsyncRead+AsyncWrite pair.
    /// Production `connect` wraps an SDK-provided relay stream and delegates here.
    ///
    /// Handshake: send `OpenPty { protocol_version, rows, cols }`, read first
    /// frame. Must be `Opened { protocol_version }` with matching version, else
    /// `Err(TransportError::ProtocolVersion)` or `Err(TransportError::AgentError)`.
    #[allow(clippy::too_many_lines)]
    pub async fn new_with_stream<S>(stream: S, rows: u16, cols: u16) -> Result<Self, TransportError>
    where
        S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    {
        let (reader, mut writer) = tokio::io::split(stream);

        // Step 1: send OpenPty handshake.
        let open = AgentMessage::OpenPty {
            protocol_version: PROTOCOL_VERSION,
            rows,
            cols,
            shell: None,
        };
        let mut buf = serde_json::to_string(&open)
            .map_err(|e| TransportError::Protocol(format!("encode OpenPty: {e}")))?;
        buf.push('\n');
        writer.write_all(buf.as_bytes()).await?;
        writer.flush().await?;

        // Step 2: read first frame — must be Opened or Error.
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();
        let n = buf_reader.read_line(&mut line).await?;
        if n == 0 {
            return Err(TransportError::Disconnected);
        }
        let first: AgentMessage = serde_json::from_str(line.trim_end())
            .map_err(|e| TransportError::Protocol(format!("decode first frame: {e}")))?;
        let session_id = match first {
            AgentMessage::Opened {
                protocol_version,
                session,
            } => {
                if protocol_version != PROTOCOL_VERSION {
                    return Err(TransportError::ProtocolVersion);
                }
                session
            }
            AgentMessage::Error { reason } if reason == "protocol_version_mismatch" => {
                return Err(TransportError::ProtocolVersion);
            }
            AgentMessage::Error { reason } => return Err(TransportError::AgentError(reason)),
            other => {
                return Err(TransportError::Protocol(format!(
                    "expected Opened, got {other:?}"
                )));
            }
        };

        // Step 3: spawn pump task. biased select: resize/write before read.
        let (write_tx, mut write_rx) = mpsc::channel::<AgentMessage>(64);
        let (read_tx, read_rx) = mpsc::channel::<Vec<u8>>(64);
        let (exit_tx, exit_rx) = oneshot::channel::<Option<i32>>();
        let pump_session = session_id.clone();

        let pump = tokio::spawn(async move {
            let session = pump_session;
            let mut writer = writer;
            let mut buf_reader = buf_reader;
            let mut exit_tx = Some(exit_tx);
            let mut line = String::new();
            loop {
                tokio::select! {
                    biased; // resize/write > read

                    msg = write_rx.recv() => {
                        let Some(m) = msg else { break }; // dropped by Self::Drop
                        let Ok(mut s) = serde_json::to_string(&m) else { break };
                        s.push('\n');
                        if writer.write_all(s.as_bytes()).await.is_err() { break; }
                        if writer.flush().await.is_err() { break; }
                    }

                    n = buf_reader.read_line(&mut line) => {
                        match n {
                            Ok(0) | Err(_) => break, // EOF or io error
                            Ok(_) => {
                                let parsed: serde_json::Result<AgentMessage> =
                                    serde_json::from_str(line.trim_end());
                                match parsed {
                                    Ok(AgentMessage::Data { session: s, bytes }) if s == session => {
                                        if read_tx.send(bytes).await.is_err() { break; }
                                    }
                                    Ok(AgentMessage::Exit { session: s, code }) if s == session => {
                                        if let Some(tx) = exit_tx.take() {
                                            let _ = tx.send(Some(code));
                                        }
                                        break;
                                    }
                                    Ok(AgentMessage::Error { reason }) => {
                                        tracing::warn!("agent error: {reason}");
                                        break;
                                    }
                                    Ok(_) => {} // ignore foreign frames
                                    Err(e) => {
                                        tracing::warn!("frame decode: {e}");
                                        break;
                                    }
                                }
                                line.clear();
                            }
                        }
                    }
                }
            }
            if let Some(tx) = exit_tx.take() {
                let _ = tx.send(None);
            }
        });

        Ok(Self {
            session_id,
            write_tx,
            read_rx: Some(read_rx),
            exit_rx: Some(exit_rx),
            _pump: pump,
        })
    }

    /// Production constructor — wires the Microsoft Dev Tunnels relay SDK to
    /// `new_with_stream`. **DEFERRED** until the SDK consumption decision lands
    /// (russh-0.37 vs 0.60 dual-version cost; see Plan 08-04 §"Phase 7 vector-ssh
    /// Status" risk). Plan 08-06 (picker actor) is the first caller; this
    /// method's body lands then.
    #[allow(clippy::unused_async, clippy::needless_pass_by_value)]
    pub async fn connect(
        _tunnel: TunnelRecord,
        _access_token: String,
        _rows: u16,
        _cols: u16,
    ) -> Result<Self, TransportError> {
        // VERIFY at SDK-consumption time: exact tunnels::connections::* paths.
        // Sketch (from microsoft/dev-tunnels/rs/src/connections/relay_tunnel_client.rs):
        //   let endpoint = tunnel.endpoints.first()
        //       .ok_or_else(|| TransportError::Protocol("no endpoint".into()))?;
        //   let client = tunnels::connections::RelayTunnelClient::connect(
        //       endpoint, &access_token).await?;
        //   let port_conn = client.connect_to_port(AGENT_PORT).await?;
        //   let stream = port_conn.into_rw();
        //   Self::new_with_stream(stream, rows, cols).await
        Err(TransportError::Protocol(
            "DevTunnelTransport::connect not yet wired — pending SDK consumption decision (Plan 08-06)".into()
        ))
    }
}

#[async_trait]
impl PtyTransport for DevTunnelTransport {
    fn kind(&self) -> TransportKind {
        TransportKind::DevTunnel
    }

    fn resize(&mut self, rows: u16, cols: u16, _pw: u16, _ph: u16) -> Result<()> {
        let msg = AgentMessage::Resize {
            session: self.session_id.clone(),
            rows,
            cols,
        };
        self.write_tx
            .try_send(msg)
            .map_err(|e| anyhow!("devtunnel resize tx: {e}"))
    }

    async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        let msg = AgentMessage::Data {
            session: self.session_id.clone(),
            bytes: bytes.to_vec(),
        };
        self.write_tx
            .send(msg)
            .await
            .map_err(|e| anyhow!("devtunnel write tx: {e}"))
    }

    fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>> {
        self.read_rx.take()
    }

    async fn wait(&mut self) -> Result<Option<i32>> {
        match self.exit_rx.take() {
            Some(rx) => Ok(rx.await.ok().flatten()),
            None => Ok(None),
        }
    }
}
