---
phase: 08-vs-code-remote-tunnels-connect
plan: 04
type: execute
wave: 2
depends_on: [01]
files_modified:
  - crates/vector-tunnels/src/api.rs
  - crates/vector-tunnels/src/model.rs
  - crates/vector-tunnels/src/transport.rs
  - crates/vector-tunnels/src/domain.rs
  - crates/vector-tunnels/tests/list_tunnels.rs
  - crates/vector-tunnels/tests/transport_protocol.rs
  - crates/vector-mux/src/devtunnel_domain.rs
autonomous: true
requirements:
  - DT-02
  - DT-03
  - DT-04
user_setup: []
must_haves:
  truths:
    - "`vector-tunnels::api::list_tunnels(auth)` returns tunnels filtered to ONLY those with `vector-agent: true` label (D-10)"
    - "Tunnel display name strips the `vector-` prefix at the API boundary (D-09)"
    - "`vector-tunnels::transport::DevTunnelTransport` implements `vector_mux::PtyTransport` with TransportKind::DevTunnel (D-A4)"
    - "On connect: open relay channel via SDK, send AgentMessage::OpenPty with protocol_version: 1, await Opened, then biased-select pump (resize > write > read) bridging client mpsc channels to JSON-framed Data/Resize/Exit frames"
    - "DevTunnelDomain (in vector-mux) Phase-7-style spawn returns Box<dyn PtyTransport> ready for create_tab_async_with_transport (D-A4)"
    - "Provider-aware Authorization header (D-06): GitHub → `github gho_...`; Microsoft → `Bearer <jwt>`"
  artifacts:
    - path: "crates/vector-tunnels/src/api.rs"
      provides: "DevTunnelsApi: list_tunnels, get_tunnel, get_access_token (connect-scope); reqwest-based; provider-aware auth header"
      min_lines: 80
    - path: "crates/vector-tunnels/src/model.rs"
      provides: "TunnelRecord, TunnelEndpoint, AuthProvider, display_name helpers"
      min_lines: 40
    - path: "crates/vector-tunnels/src/transport.rs"
      provides: "DevTunnelTransport impl PtyTransport; connect(tunnel, auth, rows, cols) constructor"
      min_lines: 150
    - path: "crates/vector-tunnels/src/domain.rs"
      provides: "DevTunnelDomain shim that delegates to transport::connect (Phase 7 trait shape)"
    - path: "crates/vector-mux/src/devtunnel_domain.rs"
      provides: "REPLACE Phase 2 unimplemented!() stub with a thin wrapper that requires a transport-injection rather than spawning directly (vector-mux stays free of vector-tunnels dep per WIN-04)"
  key_links:
    - from: "vector-tunnels::api::list_tunnels"
      to: "https://global.rel.tunnels.api.visualstudio.com/api/v1/tunnels"
      via: "GET with provider-aware Authorization header"
      pattern: "global\\.rel\\.tunnels\\.api\\.visualstudio\\.com"
    - from: "DevTunnelTransport::connect"
      to: "tunnels::connections::RelayTunnelClient"
      via: "SDK's connect + open_channel + AsyncRead/AsyncWrite stream"
      pattern: "RelayTunnelClient"
    - from: "DevTunnelTransport (impl PtyTransport)"
      to: "vector_mux::transport::TransportKind::DevTunnel"
      via: "kind() return value"
      pattern: "TransportKind::DevTunnel"
    - from: "AuthProvider variant"
      to: "Authorization header format"
      via: "format_header() method dispatch — D-06"
      pattern: "github |Bearer "
---

<objective>
Implement the Mac-side `vector-tunnels` crate: Dev Tunnels Management REST (list + get + connect-scope token), `DevTunnelTransport` that opens a relay channel via the SDK and speaks the JSON agent protocol against `vector-tunnel-agent`, and the `DevTunnelDomain` glue that hands a `Box<dyn PtyTransport>` to `Mux::create_tab_async_with_transport`.

Purpose: this is the Mac half of D-A1 Path 2c. Combined with Plan 08-03 (agent) and Plan 08-05 (auth wiring) and Plan 08-06 (picker UI), DT-02/03/04 close.
Output: a transport that returns `TransportKind::DevTunnel`, plug-compatible with Phase 7 mux integration. 6+ unit tests against wiremock + a duplex-stream protocol test.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/phases/08-vs-code-remote-tunnels-connect/08-CONTEXT.md
@.planning/phases/08-vs-code-remote-tunnels-connect/08-RESEARCH.md
@crates/vector-mux/src/transport.rs
@crates/vector-mux/src/mux.rs
@crates/vector-mux/src/devtunnel_domain.rs
@crates/vector-ssh/src/transport.rs
@crates/vector-tunnel-protocol/src/lib.rs

<interfaces>
From vector-mux/src/transport.rs (Phase 2 D-38):
```rust
#[async_trait::async_trait]
pub trait PtyTransport: Send + 'static {
    fn resize(&mut self, rows: u16, cols: u16, px_w: u16, px_h: u16) -> Result<()>;
    async fn write(&mut self, bytes: &[u8]) -> Result<()>;
    fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>>;
    fn kind(&self) -> TransportKind;
    async fn wait(&mut self) -> Result<Option<i32>>;
}
pub enum TransportKind { Local, DevTunnel }
```

From vector-mux/src/mux.rs:
```rust
pub async fn create_tab_async_with_transport(
    &self,
    window_id: WindowId,
    transport: Box<dyn PtyTransport>,
    rows: u16, cols: u16,
) -> Result<(TabId, PaneId)>;
```

From vector-ssh/src/transport.rs (mirror this shape for biased select):
- writer task owns master writer; mpsc::Receiver<Vec<u8>> for outbound bytes
- reader task owns master reader; mpsc::Sender<Vec<u8>> for inbound bytes
- resize: oneshot or atomic; biased select picks resize > write > read

From vector-tunnel-protocol::AgentMessage — Plan 08-01.

From SDK (`microsoft/dev-tunnels/rs/src/connections/relay_tunnel_client.rs` — verify at exec time):
- `RelayTunnelClient::connect(&endpoint, &access_token) -> Result<ClientRelayHandle>`
- `ClientRelayHandle::connect_to_port(port: u16) -> Result<PortConnection>`
- `PortConnection::into_rw() -> PortConnectionRW (AsyncRead+AsyncWrite)`
- For the agent's case we don't `connect_to_port` (the agent is NOT exposing a TCP port — it accepts russh channels directly). Confirm with the SDK: there may be a `connect_to_channel(name)` or similar. If the SDK only supports port-style channels, the agent in Plan 08-03 must `add_port_raw` with a vector-defined port (e.g. 32100); document the chosen port in this plan's SUMMARY.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Management REST + model + filter to vector-agent tunnels</name>
  <files>crates/vector-tunnels/src/api.rs, crates/vector-tunnels/src/model.rs, crates/vector-tunnels/tests/list_tunnels.rs</files>
  <read_first>
    - crates/vector-codespaces/src/client/mod.rs (Phase 6 REST client — pattern for reqwest + 401 silent-refresh + manual Debug)
    - crates/vector-tunnels/tests/fixtures/dev_tunnels_list.json (Plan 08-01 — 5-record fixture)
    - crates/vector-tunnels/src/api.rs (Plan 08-01 placeholder — replace)
    - 08-RESEARCH.md §"Listing the user's tunnels via REST" + §"Filtering to `code tunnel`-created hosts" code examples
  </read_first>
  <behavior>
    - Test 1 (list filters to vector-agent label): given the 5-tunnel fixture (2 tagged, 3 untagged), `list_tunnels(auth)` returns exactly 2 records.
    - Test 2 (display_name strips prefix): a tunnel named `vector-corp-dev-box-42` has `record.display_name() == "corp-dev-box-42"`. A tunnel named `vector-` returns `""`. A tunnel without the prefix (defensive) returns its full name.
    - Test 3 (auth header — GitHub): `AuthProvider::GitHub("gho_xxx").format_header() == "github gho_xxx"`.
    - Test 4 (auth header — Microsoft): `AuthProvider::Microsoft("jwt").format_header() == "Bearer jwt"`.
    - Test 5 (401 handling): wiremock returns 401 for first call; the api returns `Err(ApiError::Unauthorized)` immediately — this plan does NOT implement silent refresh (that's the actor's job; Plan 08-06 wires the actor to call MicrosoftAuth::refresh on Unauthorized).
    - Test 6 (last-seen parsing): record exposes `last_updated: chrono::DateTime<Utc>` parsed from the ISO 8601 `lastUpdatedAt` field; on missing/garbled field, return None.
  </behavior>
  <action>
    Step 1 — `crates/vector-tunnels/src/model.rs`:
    ```rust
    use chrono::{DateTime, Utc};
    use serde::Deserialize;

    /// On-wire tunnel record. Microsoft's contract; fields named per their API.
    #[derive(Deserialize, Clone)]
    pub struct TunnelRecord {
        #[serde(rename = "tunnelId")]   pub tunnel_id: String,
        pub name: Option<String>,
        #[serde(default)]               pub labels: Vec<String>,
        #[serde(rename = "endpoints", default)] pub endpoints: Vec<TunnelEndpoint>,
        #[serde(rename = "lastUpdatedAt")] pub last_updated_at: Option<String>,
    }

    impl std::fmt::Debug for TunnelRecord {
        // Minimal — no secrets to redact but be terse.
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("TunnelRecord")
                .field("tunnel_id", &self.tunnel_id)
                .field("name", &self.name)
                .field("labels_n", &self.labels.len())
                .field("endpoints_n", &self.endpoints.len())
                .finish()
        }
    }

    impl TunnelRecord {
        pub const VECTOR_AGENT_LABEL: &str = "vector-agent: true";
        pub const VECTOR_NAME_PREFIX: &str = "vector-";

        pub fn is_vector_agent(&self) -> bool {
            self.labels.iter().any(|l| l == Self::VECTOR_AGENT_LABEL)
        }

        /// D-09: registration name is `vector-{hostname}`; picker displays without prefix.
        pub fn display_name(&self) -> String {
            let name = self.name.as_deref().unwrap_or("");
            name.strip_prefix(Self::VECTOR_NAME_PREFIX).unwrap_or(name).to_string()
        }

        pub fn last_updated(&self) -> Option<DateTime<Utc>> {
            self.last_updated_at.as_deref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.with_timezone(&Utc))
        }
    }

    #[derive(Deserialize, Clone, Debug)]
    pub struct TunnelEndpoint {
        #[serde(rename = "hostId")]         pub host_id: String,
        #[serde(rename = "clientRelayUri")] pub client_relay_uri: String,
        #[serde(rename = "hostPublicKeys", default)] pub host_public_keys: Vec<String>,
    }

    /// Provider tag — drives Authorization header format per D-06.
    pub enum AuthProvider {
        GitHub(String),     // raw gho_... or ghp_...
        Microsoft(String),  // JWT
    }
    impl AuthProvider {
        pub fn format_header(&self) -> String {
            match self {
                Self::GitHub(t) => format!("github {t}"),
                Self::Microsoft(t) => format!("Bearer {t}"),
            }
        }
    }
    // Manual Debug for AuthProvider — never leak token bytes.
    impl std::fmt::Debug for AuthProvider {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::GitHub(_)    => f.write_str("AuthProvider::GitHub(<token>)"),
                Self::Microsoft(_) => f.write_str("AuthProvider::Microsoft(<token>)"),
            }
        }
    }
    ```

    Step 2 — `crates/vector-tunnels/src/api.rs`:
    ```rust
    use crate::model::{AuthProvider, TunnelRecord};
    use serde::Deserialize;
    use thiserror::Error;

    pub const TUNNELS_BASE_URL: &str = "https://global.rel.tunnels.api.visualstudio.com";

    #[derive(Debug, Error)]
    pub enum ApiError {
        #[error("HTTP: {0}")]    Http(#[from] reqwest::Error),
        #[error("unauthorized")] Unauthorized,
        #[error("forbidden")]    Forbidden,
        #[error("not found")]    NotFound,
        #[error("api: {status}: {body}")] Other { status: u16, body: String },
    }

    pub struct DevTunnelsApi {
        http: reqwest::Client,
        base_url: String,
    }
    // Manual Debug — http client and base_url are not secrets but keep terse.
    impl std::fmt::Debug for DevTunnelsApi {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("DevTunnelsApi").field("base_url", &self.base_url).finish()
        }
    }

    impl DevTunnelsApi {
        pub fn new() -> Self { Self::with_base_url(TUNNELS_BASE_URL.into()) }

        pub fn with_base_url(base_url: String) -> Self {
            Self {
                http: reqwest::Client::builder()
                    .user_agent(concat!("Vector/", env!("CARGO_PKG_VERSION")))
                    .build()
                    .expect("reqwest client"),
                base_url,
            }
        }

        /// List all tunnels under the auth identity, filtered to `vector-agent: true`.
        pub async fn list_tunnels(&self, auth: &AuthProvider) -> Result<Vec<TunnelRecord>, ApiError> {
            let url = format!("{}/api/v1/tunnels", self.base_url);
            let resp = self.http.get(&url)
                .header("Authorization", auth.format_header())
                .send().await?;
            match resp.status().as_u16() {
                401 => Err(ApiError::Unauthorized),
                403 => Err(ApiError::Forbidden),
                s if (200..300).contains(&s) => {
                    #[derive(Deserialize)] struct Body { value: Vec<TunnelRecord> }
                    let b: Body = resp.json().await?;
                    Ok(b.value.into_iter().filter(|t| t.is_vector_agent()).collect())
                }
                s => Err(ApiError::Other { status: s, body: resp.text().await.unwrap_or_default() }),
            }
        }

        /// Fetch connect-scope access token for a tunnel.
        pub async fn get_access_token(&self, auth: &AuthProvider, tunnel_id: &str) -> Result<String, ApiError> {
            let url = format!("{}/api/v1/tunnels/{}/access?scopes=connect", self.base_url, tunnel_id);
            let resp = self.http.post(&url)
                .header("Authorization", auth.format_header())
                .send().await?;
            if resp.status().as_u16() == 401 { return Err(ApiError::Unauthorized); }
            if !resp.status().is_success() {
                return Err(ApiError::Other { status: resp.status().as_u16(), body: resp.text().await.unwrap_or_default() });
            }
            #[derive(Deserialize)] struct Body { token: String }
            Ok(resp.json::<Body>().await?.token)
        }
    }
    ```

    Step 3 — `crates/vector-tunnels/tests/list_tunnels.rs`: replace Plan 08-01 #[ignore] stubs.
    Use `wiremock::MockServer` to serve `/api/v1/tunnels`; load the fixture from disk:
    ```rust
    let fixture = std::fs::read_to_string("tests/fixtures/dev_tunnels_list.json").unwrap();
    Mock::given(method("GET")).and(path("/api/v1/tunnels"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(fixture, "application/json"))
        .mount(&server).await;
    let api = DevTunnelsApi::with_base_url(server.uri());
    let tunnels = api.list_tunnels(&AuthProvider::GitHub("gho_test".into())).await.unwrap();
    assert_eq!(tunnels.len(), 2);
    ```
    Land Tests 1, 2, 5 here. Tests 3+4 live in a unit-tests module inside `src/model.rs`. Test 6 lives in `model.rs` tests too.
  </action>
  <verify>
    <automated>cargo test -p vector-tunnels --test list_tunnels &amp;&amp; cargo test -p vector-tunnels --lib model &amp;&amp; cargo test -p vector-arch-tests --tests &amp;&amp; cargo clippy -p vector-tunnels --all-targets -- -D warnings &amp;&amp; ! grep -E "#\\[derive\\([^)]*Debug" crates/vector-tunnels/src/api.rs &amp;&amp; ! grep -E "impl Debug for AuthProvider \\{ // derived" crates/vector-tunnels/src/model.rs</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test -p vector-tunnels --test list_tunnels` reports >= 3 passed
    - `cargo test -p vector-tunnels --lib` reports >= 3 passed (model unit tests for auth header + last_updated + display_name)
    - `grep -c "VECTOR_AGENT_LABEL" crates/vector-tunnels/src/model.rs` >= 1
    - `grep -c "vector-agent: true" crates/vector-tunnels/src/model.rs` >= 1
    - `grep -c "VECTOR_NAME_PREFIX" crates/vector-tunnels/src/model.rs` >= 1
    - `grep -c "format!(\"github" crates/vector-tunnels/src/model.rs` >= 1 (D-06 GitHub prefix)
    - `grep -c "format!(\"Bearer" crates/vector-tunnels/src/model.rs` >= 1 (D-06 Microsoft prefix)
    - `grep -c "global.rel.tunnels.api.visualstudio.com" crates/vector-tunnels/src/api.rs` >= 1
    - `! grep -E "#\\[derive\\([^)]*Debug" crates/vector-tunnels/src/api.rs` exit 0 (no derived Debug)
    - `cargo clippy -p vector-tunnels --all-targets -- -D warnings` exit 0
    - `cargo test -p vector-arch-tests --tests` 0 failed
  </acceptance_criteria>
  <done>List endpoint + filter + display-name + last-seen + provider-aware auth header all green against wiremock + unit tests. Pitfall 14 holds.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: DevTunnelTransport (PtyTransport impl) + agent protocol pump</name>
  <files>crates/vector-tunnels/src/transport.rs, crates/vector-tunnels/src/domain.rs, crates/vector-tunnels/tests/transport_protocol.rs, crates/vector-mux/src/devtunnel_domain.rs</files>
  <read_first>
    - crates/vector-ssh/src/transport.rs (entire file — SshChannelTransport is the canonical pattern; DevTunnelTransport mirrors its shape but bridges to JSON frames instead of SSH channel writes)
    - crates/vector-mux/src/transport.rs (PtyTransport trait + TransportKind)
    - crates/vector-mux/src/devtunnel_domain.rs (Phase 2 stub — replace `unimplemented!("Phase 8")` body)
    - crates/vector-tunnel-protocol/src/lib.rs (AgentMessage enum)
    - crates/vector-tunnel-agent/src/session.rs (Plan 08-03 — the protocol contract we connect to)
  </read_first>
  <behavior>
    - Test 1 (handshake): construct a `DevTunnelTransport` against a `tokio::io::duplex(8192)` test pair. From the test side, read the first frame and assert it is `AgentMessage::OpenPty { protocol_version: 1, rows: 24, cols: 80, .. }`.
    - Test 2 (Opened reply): after sending `AgentMessage::Opened { protocol_version: 1, session: "s-1".into() }` back, `DevTunnelTransport::new_with_stream(...)` returns Ok and `take_reader()` yields Some(rx).
    - Test 3 (protocol mismatch): if the agent replies with `AgentMessage::Error { reason: "protocol_version_mismatch" }`, the transport constructor returns `Err(TransportError::ProtocolVersion)`.
    - Test 4 (write path): calling `transport.write(b"hello\n").await.unwrap()` produces an outbound frame on the wire that decodes to `AgentMessage::Data { session: "s-1", bytes: b"hello\n".to_vec() }`.
    - Test 5 (read path): a `AgentMessage::Data { session: "s-1", bytes: b"out".to_vec() }` written by the test to the wire arrives on the reader mpsc as `Vec<u8> = b"out"`.
    - Test 6 (resize): `transport.resize(42, 120, 0, 0).unwrap()` produces a `AgentMessage::Resize { session, rows: 42, cols: 120 }` on the wire within 1 polling tick.
    - Test 7 (exit): when the test side writes `AgentMessage::Exit { session, code: 0 }` and drops the stream, `transport.wait().await.unwrap() == Some(0)` and `take_reader()` channel closes shortly thereafter.
    - Test 8 (kind): `transport.kind() == TransportKind::DevTunnel`.
  </behavior>
  <action>
    Step 1 — `crates/vector-tunnels/src/transport.rs`:

    Mirror `vector-ssh/src/transport.rs` shape. Key differences:
    - We bridge over a generic `AsyncRead + AsyncWrite` stream (from the SDK's PortConnectionRW) instead of a russh::Channel.
    - Reads/writes go through the JSON line codec (newline-delimited per D-12).
    - On `resize()`, push a `Resize` frame onto an internal mpsc rather than calling kernel ioctl.

    Skeleton (~200 LOC):
    ```rust
    use crate::model::{AuthProvider, TunnelRecord};
    use anyhow::{Context, Result};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, AsyncRead, AsyncWrite, BufReader};
    use tokio::sync::{mpsc, oneshot};
    use tokio::task::JoinHandle;
    use vector_mux::transport::{PtyTransport, TransportKind};
    use vector_tunnel_protocol::{AgentMessage, PROTOCOL_VERSION};

    #[derive(thiserror::Error, Debug)]
    pub enum TransportError {
        #[error("io: {0}")] Io(#[from] std::io::Error),
        #[error("protocol error: {0}")] Protocol(String),
        #[error("protocol version mismatch — update agent")] ProtocolVersion,
        #[error("agent returned error: {0}")] AgentError(String),
        #[error("disconnected")] Disconnected,
    }

    pub struct DevTunnelTransport {
        session_id: String,
        write_tx: mpsc::Sender<AgentMessage>,
        read_rx: Option<mpsc::Receiver<Vec<u8>>>,
        exit_rx: Option<oneshot::Receiver<Option<i32>>>,
        // join handle for the pump task; dropped on `Drop` ⇒ cancellation
        _pump: JoinHandle<()>,
    }

    impl std::fmt::Debug for DevTunnelTransport {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("DevTunnelTransport").field("session_id", &self.session_id).finish()
        }
    }

    impl DevTunnelTransport {
        /// Test seam — constructs the transport from any AsyncRead+AsyncWrite pair.
        /// Production path goes through `connect(tunnel, auth, rows, cols)` which
        /// builds the stream from the SDK and calls into this constructor.
        pub async fn new_with_stream<S>(stream: S, rows: u16, cols: u16) -> Result<Self, TransportError>
        where S: AsyncRead + AsyncWrite + Send + Unpin + 'static
        {
            let (mut reader, mut writer) = tokio::io::split(stream);
            // Step 1: send OpenPty handshake.
            let open = AgentMessage::OpenPty { protocol_version: PROTOCOL_VERSION, rows, cols, shell: None };
            let mut buf = serde_json::to_string(&open).unwrap(); buf.push('\n');
            writer.write_all(buf.as_bytes()).await?;

            // Step 2: read first frame (must be Opened or Error).
            let mut buf_reader = BufReader::new(reader);
            let mut line = String::new();
            buf_reader.read_line(&mut line).await?;
            let session_id = match serde_json::from_str::<AgentMessage>(line.trim_end())
                .map_err(|e| TransportError::Protocol(e.to_string()))?
            {
                AgentMessage::Opened { protocol_version, session } => {
                    if protocol_version != PROTOCOL_VERSION { return Err(TransportError::ProtocolVersion); }
                    session
                }
                AgentMessage::Error { reason } if reason == "protocol_version_mismatch" =>
                    return Err(TransportError::ProtocolVersion),
                AgentMessage::Error { reason } => return Err(TransportError::AgentError(reason)),
                other => return Err(TransportError::Protocol(format!("expected Opened, got {other:?}"))),
            };

            // Step 3: spawn pump task. Owns the split reader+writer. Channels:
            //   - write_rx (mpsc) ⇒ outbound AgentMessage to wire
            //   - read_tx  (mpsc) ⇒ inbound PTY bytes to consumer (via take_reader)
            //   - exit_tx  (oneshot) ⇒ child exit code
            let (write_tx, mut write_rx) = mpsc::channel::<AgentMessage>(64);
            let (read_tx,  read_rx)      = mpsc::channel::<Vec<u8>>(64);
            let (exit_tx,  exit_rx)      = oneshot::channel::<Option<i32>>();
            let session = session_id.clone();

            let pump = tokio::spawn(async move {
                let mut line = String::new();
                loop {
                    tokio::select! {
                        biased;   // resize/write commands prioritized over read

                        msg = write_rx.recv() => {
                            match msg {
                                Some(m) => {
                                    let mut s = serde_json::to_string(&m).unwrap(); s.push('\n');
                                    if writer.write_all(s.as_bytes()).await.is_err() { break; }
                                }
                                None => break,   // closed by Drop
                            }
                        }

                        n = buf_reader.read_line(&mut line) => {
                            match n {
                                Ok(0) => break,   // EOF
                                Ok(_) => {
                                    let parsed: serde_json::Result<AgentMessage> =
                                        serde_json::from_str(line.trim_end());
                                    match parsed {
                                        Ok(AgentMessage::Data { session: s, bytes }) if s == session => {
                                            if read_tx.send(bytes).await.is_err() { break; }
                                        }
                                        Ok(AgentMessage::Exit { session: s, code }) if s == session => {
                                            let _ = exit_tx.send(Some(code));
                                            break;
                                        }
                                        Ok(AgentMessage::Error { reason }) => {
                                            tracing::warn!("agent error: {reason}");
                                            break;
                                        }
                                        Ok(_) => {}   // ignore foreign frames
                                        Err(e) => {
                                            tracing::warn!("frame decode: {e}");
                                            break;
                                        }
                                    }
                                    line.clear();
                                }
                                Err(_) => break,
                            }
                        }
                    }
                }
            });

            Ok(Self { session_id, write_tx, read_rx: Some(read_rx), exit_rx: Some(exit_rx), _pump: pump })
        }

        /// Production constructor: build SDK relay client, open channel, then delegate.
        pub async fn connect(
            tunnel: &TunnelRecord,
            access_token: String,   // connect-scope, fetched by API::get_access_token
            rows: u16,
            cols: u16,
        ) -> Result<Self, TransportError> {
            // VERIFY at exec time: exact SDK call sequence. Sketch:
            //   let endpoint = tunnel.endpoints.first().context("no endpoint")?;
            //   let client = tunnels::connections::RelayTunnelClient::connect(endpoint, &access_token).await?;
            //   let stream = client.connect_to_port(AGENT_PORT).await?.into_rw();
            //   Self::new_with_stream(stream, rows, cols).await
            //
            // AGENT_PORT must match what vector-tunnel-agent's host.rs registers via SDK's
            // add_port_raw. Pick 32100 (high, unprivileged, not in common-services range).
            // Document the constant in this plan's SUMMARY.
            todo!("wire SDK calls at exec time after reading rs/src/connections/relay_tunnel_client.rs")
        }
    }

    #[async_trait::async_trait]
    impl PtyTransport for DevTunnelTransport {
        fn resize(&mut self, rows: u16, cols: u16, _pw: u16, _ph: u16) -> anyhow::Result<()> {
            let msg = AgentMessage::Resize { session: self.session_id.clone(), rows, cols };
            // try_send is sufficient — resize requests are rare and small.
            self.write_tx.try_send(msg).map_err(|e| anyhow::anyhow!("resize send: {e}"))
        }
        async fn write(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
            let msg = AgentMessage::Data { session: self.session_id.clone(), bytes: bytes.to_vec() };
            self.write_tx.send(msg).await.map_err(|e| anyhow::anyhow!("write send: {e}"))
        }
        fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>> { self.read_rx.take() }
        fn kind(&self) -> TransportKind { TransportKind::DevTunnel }
        async fn wait(&mut self) -> anyhow::Result<Option<i32>> {
            match self.exit_rx.take() {
                Some(rx) => Ok(rx.await.unwrap_or(None)),
                None => Ok(None),
            }
        }
    }
    ```

    Step 2 — `crates/vector-tunnels/src/domain.rs`:
    ```rust
    use crate::model::{AuthProvider, TunnelRecord};
    use crate::transport::DevTunnelTransport;
    use crate::api::DevTunnelsApi;
    use vector_mux::transport::PtyTransport;

    /// Convenience: end-to-end "connect to tunnel" using API + transport. Used by
    /// the picker actor (Plan 08-06) — vector-mux stays free of vector-tunnels dep.
    pub async fn connect_tunnel(
        api: &DevTunnelsApi,
        auth: &AuthProvider,
        tunnel: &TunnelRecord,
        rows: u16,
        cols: u16,
    ) -> anyhow::Result<Box<dyn PtyTransport>> {
        let token = api.get_access_token(auth, &tunnel.tunnel_id).await?;
        let t = DevTunnelTransport::connect(tunnel, token, rows, cols).await?;
        Ok(Box::new(t))
    }
    ```

    Step 3 — `crates/vector-mux/src/devtunnel_domain.rs`:
    Replace the existing `unimplemented!("Phase 8: Dev Tunnels Integration (spike-gated)")` body with a clearer comment that this stub is DEFERRED PERMANENTLY in favor of `vector-tunnels::domain::connect_tunnel`. This preserves WIN-04: vector-mux stays free of vector-tunnels dep. Keep the Domain impl skeleton but `spawn` continues to return `unimplemented!("Use vector_tunnels::domain::connect_tunnel + Mux::create_tab_async_with_transport")` — Plan 08-06's actor will call `create_tab_async_with_transport` directly, NEVER routing through `DevTunnelDomain::spawn`.

    Update the doc comment at top of devtunnel_domain.rs:
    ```rust
    //! DevTunnelDomain — Phase 8 leaves this as a stub by design.
    //!
    //! Per WIN-04 (Phase 4 D-38): vector-mux must not depend on vector-tunnels.
    //! Phase 8 Dev Tunnel sessions are installed via `vector-tunnels::domain::connect_tunnel`
    //! producing a `Box<dyn PtyTransport>` that callers pass to
    //! `Mux::create_tab_async_with_transport`. The Domain trait method below remains
    //! `unimplemented!()` and is unreachable in v1.
    ```

    Step 4 — `crates/vector-tunnels/tests/transport_protocol.rs`:
    Land all 8 tests from `<behavior>`. Use `tokio::io::duplex(8192)` to bridge the transport's stream to a test-controlled "wire end" — same harness pattern as vector-ssh tests.
  </action>
  <verify>
    <automated>cargo build -p vector-tunnels &amp;&amp; cargo test -p vector-tunnels --test transport_protocol &amp;&amp; cargo test -p vector-mux --tests &amp;&amp; cargo clippy -p vector-tunnels --all-targets -- -D warnings &amp;&amp; cargo test -p vector-arch-tests --tests &amp;&amp; grep -q "TransportKind::DevTunnel" crates/vector-tunnels/src/transport.rs &amp;&amp; grep -q "PROTOCOL_VERSION" crates/vector-tunnels/src/transport.rs &amp;&amp; grep -q "biased" crates/vector-tunnels/src/transport.rs &amp;&amp; ! grep -q "use vector_tunnels" crates/vector-mux/src/devtunnel_domain.rs</automated>
  </verify>
  <acceptance_criteria>
    - `cargo build -p vector-tunnels` exit 0
    - `cargo test -p vector-tunnels --test transport_protocol` reports >= 8 passed
    - `cargo test -p vector-mux --tests` 0 failed (no regression)
    - `grep -c "TransportKind::DevTunnel" crates/vector-tunnels/src/transport.rs` >= 1
    - `grep -c "PROTOCOL_VERSION" crates/vector-tunnels/src/transport.rs` >= 1
    - `grep -c "biased" crates/vector-tunnels/src/transport.rs` >= 1 (resize-priority select)
    - `grep -c "impl PtyTransport for DevTunnelTransport" crates/vector-tunnels/src/transport.rs` >= 1
    - `grep -c "impl std::fmt::Debug for DevTunnelTransport" crates/vector-tunnels/src/transport.rs` >= 1
    - `! grep "vector_tunnels::" crates/vector-mux/src/devtunnel_domain.rs` exit 0 (vector-mux stays free of vector-tunnels dep — WIN-04)
    - `! grep "vector-tunnels" crates/vector-mux/Cargo.toml` exit 0 (no dep added to vector-mux)
    - `cargo clippy -p vector-tunnels --all-targets -- -D warnings` exit 0
    - `cargo test -p vector-arch-tests --tests` 0 failed
  </acceptance_criteria>
  <done>DevTunnelTransport implements PtyTransport with TransportKind::DevTunnel, handshakes via OpenPty/Opened, biased-select pump bridges JSON frames to mpsc channels, vector-mux stays free of vector-tunnels dep per WIN-04.</done>
</task>

</tasks>

<verification>
- `make lint` exit 0
- `make test` exit 0 (workspace tests; >= 11 passed in vector-tunnels alone)
- `! grep -E "vector-tunnels|vector_tunnels" crates/vector-mux/{src,Cargo.toml}` exit 0 (WIN-04 preserved)
- `cargo test -p vector-arch-tests --tests` 0 failed
</verification>

<success_criteria>
- list_tunnels filters to `vector-agent: true` label and returns TunnelRecord with `display_name()` stripping the `vector-` prefix
- DevTunnelTransport implements PtyTransport, kind() == DevTunnel, biased-select pump correct
- Protocol version mismatch surfaces as TransportError::ProtocolVersion
- vector-mux stays free of vector-tunnels dep — Domain::spawn for DevTunnel is intentionally unreachable (WIN-04)
- Manual Debug on all token-bearing types; arch-lint passes
</success_criteria>

<output>
After completion, create `.planning/phases/08-vs-code-remote-tunnels-connect/08-04-SUMMARY.md` documenting:
- The exact AGENT_PORT chosen (default 32100 — confirm at exec time the SDK accepts it via add_port_raw)
- The exact SDK type paths used for RelayTunnelClient::connect / connect_to_port / into_rw (after reading the vendored SDK)
- Any deviation from the JSON protocol (e.g., if the SDK forced msgpack)
- Confirmation that vector-mux has zero deps on vector-tunnels (WIN-04 preserved)
</output>
