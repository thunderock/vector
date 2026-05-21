---
phase: 08-vs-code-remote-tunnels-connect
plan: 03
type: execute
wave: 2
depends_on: [01]
files_modified:
  - crates/vector-tunnel-agent/Cargo.toml
  - crates/vector-tunnel-agent/src/main.rs
  - crates/vector-tunnel-agent/src/cli.rs
  - crates/vector-tunnel-agent/src/auth.rs
  - crates/vector-tunnel-agent/src/host.rs
  - crates/vector-tunnel-agent/src/session.rs
  - crates/vector-tunnel-agent/src/token_cache.rs
  - crates/vector-tunnel-agent/tests/protocol_codec.rs
  - crates/vector-tunnel-agent/tests/session_lifecycle.rs
autonomous: true
requirements:
  - DT-01
  - DT-03
user_setup:
  - service: github-or-microsoft-oauth-on-remote
    why: "Agent first-run device flow (D-07) — user opens device-code URL on any browser; token persisted to ~/.config/vector/agent-token mode 0600 on the Linux remote box"
    env_vars: []
    dashboard_config: []
must_haves:
  truths:
    - "Running `vector-tunnel-agent` on a Linux box registers a Dev Tunnel labeled `vector-agent: true` (D-10) prefixed with `vector-{hostname}` (D-09) under the user's GitHub or Microsoft identity"
    - "First run prints the device code + verification URL on stdout (D-07) and persists the resulting token to ~/.config/vector/agent-token mode 0600"
    - "On each incoming relay channel, the agent reads JSON-framed `open_pty` from the wire, spawns $SHELL via portable-pty, and pumps bytes through `data` frames bidirectionally"
    - "Resize and exit frames are handled per D-13 message contract"
    - "Protocol-version mismatch returns `{\"op\":\"error\",\"reason\":\"protocol_version_mismatch\"}` and the channel closes (D-15)"
    - "ONE shell per tunnel connection (D-14) — no multiplexing in v1"
    - "Graceful shutdown (SIGTERM/SIGINT) drops the tunnel host, kills child shell, and exits 0"
  artifacts:
    - path: "crates/vector-tunnel-agent/src/main.rs"
      provides: "Binary entry point — async runtime + CLI dispatch + tracing init"
    - path: "crates/vector-tunnel-agent/src/cli.rs"
      provides: "subcommand dispatch: run (default), --reauth, --status, --version"
    - path: "crates/vector-tunnel-agent/src/auth.rs"
      provides: "Agent-side OAuth device flow + token persistence to ~/.config/vector/agent-token"
    - path: "crates/vector-tunnel-agent/src/host.rs"
      provides: "RelayTunnelHost lifecycle: register tunnel (label vector-agent + name vector-{hostname}), accept relay connections, dispatch to session handler"
      min_lines: 100
    - path: "crates/vector-tunnel-agent/src/session.rs"
      provides: "Per-channel session: protocol_version handshake, PTY spawn via portable-pty, biased select(resize, write, read), exit propagation"
      min_lines: 120
  key_links:
    - from: "vector-tunnel-agent::host::register_tunnel"
      to: "tunnels::RelayTunnelHost (vendored SDK)"
      via: "create_tunnel + add_relay_host with label vector-agent: true and name vector-{hostname}"
      pattern: "vector-agent: true"
    - from: "vector-tunnel-agent::session::run"
      to: "portable_pty::native_pty_system"
      via: "PTY spawn + bidirectional bridge to AgentMessage::{Data, Resize, Exit}"
      pattern: "native_pty_system"
    - from: "vector-tunnel-agent::session::handle_open_pty"
      to: "vector_tunnel_protocol::AgentMessage::Opened"
      via: "reply with protocol_version: 1 + session id"
      pattern: "AgentMessage::Opened"
---

<objective>
Ship the `vector-tunnel-agent` Linux binary: Dev Tunnel registration via `microsoft/dev-tunnels rs/` as a Host, OAuth device flow (D-07) on first run with token persistence to disk (mode 0600), JSON-framed protocol loop per D-12/13/14/15 with portable-pty session spawn on each accepted relay channel.

Purpose: this is the remote half of D-A1 Path 2c — without it Mac-side picker has nothing to connect to.
Output: a single binary that installs cleanly on Debian/Ubuntu (Plan 08-07 packages it) and serves a PTY shell to one connected Vector Mac client.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/phases/08-vs-code-remote-tunnels-connect/08-CONTEXT.md
@.planning/phases/08-vs-code-remote-tunnels-connect/08-RESEARCH.md
@crates/vector-tunnel-protocol/src/lib.rs
@crates/vector-pty/src/lib.rs
@crates/vector-ssh/src/transport.rs
@crates/vector-codespaces/src/auth/device_flow.rs

<interfaces>
From vector-tunnel-protocol (Plan 08-01):
```rust
pub const PROTOCOL_VERSION: u32 = 1;
pub enum AgentMessage {
    OpenPty { protocol_version: u32, rows: u16, cols: u16, shell: Option<String> },
    Opened  { protocol_version: u32, session: String },
    Data    { session: String, bytes: Vec<u8> },     // base64 on wire
    Resize  { session: String, rows: u16, cols: u16 },
    Exit    { session: String, code: i32 },
    Error   { reason: String },
    Unknown,
}
```

From vector-ssh/src/transport.rs — biased-select pattern to mirror in session.rs:
```rust
// resize is priority 1 (rare, never starve), write priority 2, read priority 3
tokio::select! {
    biased;
    Some(sz) = resize_rx.recv() => { /* TIOCSWINSZ */ }
    Some(buf) = write_rx.recv() => { /* PTY master write */ }
    n = pty_reader.read(&mut buf) => { /* emit Data frame */ }
}
```

From portable-pty (workspace dep):
```rust
let pty_system = portable_pty::native_pty_system();
let pair = pty_system.openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })?;
let child = pair.slave.spawn_command(cmd)?;
drop(pair.slave);  // Pitfall 3 (Phase 2 Plan 03) — close slave or zombie
let reader = pair.master.try_clone_reader()?;
let writer = pair.master.take_writer()?;
```

From vendored `tunnels` SDK (workspace dep added Plan 08-01):
Look at `microsoft/dev-tunnels/rs/src/connections/relay_tunnel_host.rs` for the Host API surface. Key APIs (verify exact signatures at execution time):
- `tunnels::management::TunnelManagementClient::new(...)`
- `mgmt.create_tunnel(Tunnel { labels, ... })` to register
- `tunnels::connections::RelayTunnelHost::new(tunnel, mgmt)` + `host.connect(...)` to open WS relay listener
- The host yields an inbound channel stream per accepted client — that's what we hand to `session::run`.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Agent CLI + first-run device flow + token persistence</name>
  <files>crates/vector-tunnel-agent/Cargo.toml, crates/vector-tunnel-agent/src/main.rs, crates/vector-tunnel-agent/src/cli.rs, crates/vector-tunnel-agent/src/auth.rs, crates/vector-tunnel-agent/src/token_cache.rs</files>
  <read_first>
    - crates/vector-tunnel-agent/src/main.rs (Plan 08-01 stub — replace with full async runtime entry)
    - crates/vector-codespaces/src/auth/device_flow.rs (GitHub device flow reference — agent reuses this shape locally, NOT depending on vector-codespaces crate to keep agent binary small)
    - crates/vector-tunnels/src/auth/device_flow_microsoft.rs (Plan 08-02 — Microsoft device flow reference)
  </read_first>
  <behavior>
    - Test 1 (CLI dispatch): `vector-tunnel-agent --version` prints `vector-tunnel-agent X.Y.Z` and exits 0.
    - Test 2 (CLI dispatch): `vector-tunnel-agent --help` prints subcommand list including `run`, `--reauth`, `--status`, `--version`.
    - Test 3 (token persistence path): `token_cache::token_path()` returns `~/.config/vector/agent-token` resolving `$HOME` correctly; under `$XDG_CONFIG_HOME=/tmp/foo` returns `/tmp/foo/vector/agent-token`.
    - Test 4 (token persistence permissions): after `token_cache::save(provider, refresh_token, access_token, expires_at)`, the file mode is exactly 0o600 (no group/world read).
    - Test 5 (token load + provider tag): after save then load, the returned struct contains the provider tag (`Provider::GitHub` or `Provider::Microsoft`) and the same tokens.
    - Test 6 (token absent): `load()` on non-existent path returns `Ok(None)` not Err.
    - Test 7 (corrupted token file): `load()` on garbled bytes returns `Err(AgentTokenError::Corrupted)` with a clear error message.
  </behavior>
  <action>
    Step 1 — extend `crates/vector-tunnel-agent/Cargo.toml` (already created in Plan 08-01) — add dependencies:
    ```toml
    [dependencies]
    # existing from 08-01 plus:
    clap = { version = "4", features = ["derive"] }   # add to workspace.dependencies if not there yet
    nix = { version = "0.29", features = ["user", "fs"] }
    dirs = "5"  # $HOME / XDG_CONFIG_HOME resolution
    hostname = "0.4"
    chrono = { workspace = true }
    tokio-util = { workspace = true }
    tokio-tungstenite = { workspace = true }
    tunnels = { workspace = true }   # vendored SDK from Plan 08-01
    ```
    If `clap`/`dirs`/`hostname`/`nix` are not in `[workspace.dependencies]` yet, add them there with the versions above. Prefer workspace-level pins.

    Step 2 — `crates/vector-tunnel-agent/src/cli.rs` (NEW):
    ```rust
    use clap::Parser;

    #[derive(Parser, Debug)]
    #[command(name = "vector-tunnel-agent", version, about = "Vector Tunnel Agent — Dev Tunnels host for PTY shells")]
    pub struct Cli {
        #[command(subcommand)]
        pub command: Option<Command>,
    }

    #[derive(Parser, Debug)]
    pub enum Command {
        /// Run the agent (default). Registers a tunnel and serves PTY shells.
        Run,
        /// Re-authenticate (clears stored token, prompts for fresh device flow).
        Reauth,
        /// Print current registration status (tunnel id, label, last seen).
        Status,
    }
    ```
    Manual Debug REQUIRED on any struct holding `device_code` / `user_code` / token bytes. Per arch-lint.

    Step 3 — `crates/vector-tunnel-agent/src/auth.rs` (NEW):
    Mirror GitHubAuth + MicrosoftAuth shape but condensed (~150 LOC). Two providers: `Provider::GitHub` and `Provider::Microsoft`. Per D-07/D-08:
    - On first run, prompt user to pick provider (stdin: "Sign in with [G]itHub or [M]icrosoft? ").
    - Drive device flow against picked provider's endpoints.
    - On success, write token via `token_cache::save`.
    - Manual `Debug` impls (Pitfall 14).

    Endpoints:
    - GitHub: `https://github.com/login/device/code` + `https://github.com/login/oauth/access_token`, scopes `read:user`
    - Microsoft: same as Plan 08-02 (`common` authority, `46da2f7e-b5ef-422a-9a4e-fb5e1cb7da14/.default`)

    Reuse identical polling loop logic as Phase 6 / Plan 08-02 (slow_down doubling, authorization_pending continues, expired returns DeviceCodeExpired). Print the verification URL + code to stdout in a format mirroring Phase 6:
    ```
    To sign in, open https://github.com/login/device in a browser and enter:

        ABCD-1234

    Waiting for sign-in… (code expires in 14:59)
    ```

    Step 4 — `crates/vector-tunnel-agent/src/token_cache.rs` (NEW):
    ```rust
    use std::fs;
    use std::io;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use serde::{Deserialize, Serialize};

    pub fn token_path() -> PathBuf {
        // Honor XDG_CONFIG_HOME first, then ~/.config.
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("vector").join("agent-token");
        }
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".config").join("vector").join("agent-token")
    }

    #[derive(Serialize, Deserialize, Clone)]
    pub struct CachedToken {
        pub provider: Provider,
        pub access_token: String,
        pub refresh_token: Option<String>,
        pub expires_at_unix: u64,
    }

    impl std::fmt::Debug for CachedToken {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("CachedToken")
                .field("provider", &self.provider)
                .field("has_refresh", &self.refresh_token.is_some())
                .field("expires_at_unix", &self.expires_at_unix)
                .finish()
        }
    }

    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Provider { GitHub, Microsoft }

    #[derive(thiserror::Error, Debug)]
    pub enum AgentTokenError {
        #[error("io: {0}")] Io(#[from] io::Error),
        #[error("corrupted token file: {0}")] Corrupted(String),
    }

    pub fn save(t: &CachedToken) -> Result<(), AgentTokenError> {
        let path = token_path();
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
            // Tighten directory mode 0700.
            fs::set_permissions(dir, fs::Permissions::from_mode(0o700))?;
        }
        let json = serde_json::to_string(t).map_err(|e| AgentTokenError::Corrupted(e.to_string()))?;
        // Write atomically: temp file + rename, then chmod 0600.
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, json)?;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn load() -> Result<Option<CachedToken>, AgentTokenError> {
        let path = token_path();
        if !path.exists() { return Ok(None); }
        let content = fs::read_to_string(&path)?;
        let t: CachedToken = serde_json::from_str(&content)
            .map_err(|e| AgentTokenError::Corrupted(e.to_string()))?;
        Ok(Some(t))
    }

    pub fn clear() -> Result<(), AgentTokenError> {
        let path = token_path();
        if path.exists() { fs::remove_file(&path)?; }
        Ok(())
    }
    ```

    Step 5 — `crates/vector-tunnel-agent/src/main.rs` — full async entry:
    ```rust
    use clap::Parser;
    use tracing_subscriber::EnvFilter;

    mod auth;
    mod cli;
    mod host;
    mod session;
    mod token_cache;

    fn main() -> anyhow::Result<()> {
        // tokio multi-thread runtime — agent is server-shaped, needs work-stealing.
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,tunnels=warn,russh=warn")))
            .init();

        let cli = cli::Cli::parse();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        rt.block_on(async {
            match cli.command.unwrap_or(cli::Command::Run) {
                cli::Command::Run => host::run().await,
                cli::Command::Reauth => {
                    token_cache::clear()?;
                    host::run().await   // run() will see no token and trigger device flow
                }
                cli::Command::Status => host::status().await,
            }
        })
    }
    ```

    Step 6 — `crates/vector-tunnel-agent/tests/auth_token_cache.rs` (NEW):
    Land Tests 3–7 from `<behavior>`. Use `tempfile::TempDir` + override `$XDG_CONFIG_HOME` for the duration of each test. (Tests 1–2 are integration-style; gate them as `#[ignore]` with bodies that `assert_cmd::Command::cargo_bin("vector-tunnel-agent")` — pull in `assert_cmd = "2"` as dev-dep — or skip if `cargo build` already validated CLI structure.)
  </action>
  <verify>
    <automated>cargo build -p vector-tunnel-agent &amp;&amp; cargo test -p vector-tunnel-agent --test auth_token_cache &amp;&amp; ./target/debug/vector-tunnel-agent --version &amp;&amp; cargo test -p vector-arch-tests --tests &amp;&amp; ! grep -E "#\\[derive\\([^)]*Debug" crates/vector-tunnel-agent/src/token_cache.rs</automated>
  </verify>
  <acceptance_criteria>
    - `cargo build -p vector-tunnel-agent` exit 0
    - `./target/debug/vector-tunnel-agent --version` prints `vector-tunnel-agent` followed by a semver string; exit 0
    - `./target/debug/vector-tunnel-agent --help` includes substrings `run`, `reauth`, `status`
    - `cargo test -p vector-tunnel-agent --test auth_token_cache` >= 5 passed
    - `grep -c "0o600" crates/vector-tunnel-agent/src/token_cache.rs` >= 1
    - `grep -c "0o700" crates/vector-tunnel-agent/src/token_cache.rs` >= 1
    - `grep -c "impl std::fmt::Debug for CachedToken" crates/vector-tunnel-agent/src/token_cache.rs` >= 1
    - `cargo test -p vector-arch-tests --tests` 0 failed
    - `! grep -E "#\\[derive\\([^)]*Debug" crates/vector-tunnel-agent/src/token_cache.rs` exit 0 (no derived Debug on CachedToken — manual only)
    - `! grep -E "#\\[derive\\([^)]*Debug" crates/vector-tunnel-agent/src/auth.rs` exit 0
  </acceptance_criteria>
  <done>Agent CLI parses, device flow drives both providers, tokens persist to ~/.config/vector/agent-token at mode 0600 atomically.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: RelayTunnelHost registration + session lifecycle</name>
  <files>crates/vector-tunnel-agent/src/host.rs, crates/vector-tunnel-agent/src/session.rs, crates/vector-tunnel-agent/tests/protocol_codec.rs, crates/vector-tunnel-agent/tests/session_lifecycle.rs</files>
  <read_first>
    - crates/vector-tunnel-protocol/src/lib.rs (Plan 08-01 — AgentMessage enum + serde codec)
    - crates/vector-ssh/src/transport.rs (entire file — biased-select pattern: resize > write > read)
    - crates/vector-pty/src/lib.rs (LocalPty impl — reference for portable-pty usage, Drop semantics, mpsc bounded channels)
    - crates/vector-tunnel-agent/src/main.rs (Task 1 output — confirm module wiring)
  </read_first>
  <behavior>
    - Test 1 (codec round-trip): Encode `AgentMessage::OpenPty { protocol_version: 1, rows: 24, cols: 80, shell: None }` to a JSON line ending in `\n`; decode it back; field-equal.
    - Test 2 (codec partial frames): Feeding the codec `{"op":"open` then `_pty","protocol_version":1,"rows":24,"cols":80}\n{"op":"data"` should yield exactly ONE complete OpenPty message, no Data.
    - Test 3 (codec multiple frames): Feeding two complete frames concatenated yields TWO messages in order.
    - Test 4 (protocol version mismatch): When `OpenPty.protocol_version != 1`, the session emits `{"op":"error","reason":"protocol_version_mismatch"}` and the per-channel task returns.
    - Test 5 (PTY echo round-trip): Spawn a real PTY running `/bin/sh -c "echo hi"`, send `OpenPty { rows:24,cols:80,shell:Some("/bin/sh") }`, then a `Data { bytes: b"echo HELLO\n".to_vec() }` followed by an `Exit` after capturing 1 second of output; assert the captured Data frames include the bytes `HELLO`.
    - Test 6 (Resize forwards SIGWINCH): On `AgentMessage::Resize { rows:42, cols:120, ... }`, the PTY's master `resize` is called with rows=42 cols=120. Assert via a mock PtySize observer (or — if real PTY testing is heavy — gate as `#[ignore]` and assert via tracing-test that the resize log line includes `42x120`).
    - Test 7 (Exit on child exit): When the spawned shell exits naturally, the agent emits `AgentMessage::Exit { session, code: 0 }` and tears down the channel cleanly (no leftover tokio task per process-snapshot before/after).
  </behavior>
  <action>
    Step 1 — `crates/vector-tunnel-agent/src/host.rs` (NEW): tunnel registration + accept loop.
    Sketch (~120 LOC):
    ```rust
    use anyhow::Context;
    use crate::auth;
    use crate::token_cache::{self, Provider};
    use tunnels::management::TunnelManagementClient;
    use tunnels::contracts::{Tunnel, TunnelPort, /* etc */};   // verify exact paths at exec time
    use tunnels::connections::RelayTunnelHost;
    use std::collections::HashMap;

    /// Entry point. Loads cached token (or runs device flow), registers tunnel,
    /// opens relay host, accepts inbound channels and hands each to session::run.
    pub async fn run() -> anyhow::Result<()> {
        let token = ensure_token().await?;
        let mgmt = build_mgmt_client(&token)?;
        let tunnel = register_tunnel(&mgmt).await?;
        let mut host = RelayTunnelHost::new(/* ... */);
        let mut shutdown = tokio::signal::ctrl_c();   // also handle SIGTERM via tokio::signal::unix

        eprintln!("vector-tunnel-agent: tunnel '{}' registered. Waiting for connections.",
                  tunnel.name.as_deref().unwrap_or(""));

        loop {
            tokio::select! {
                accept = host.accept_next() => {
                    let stream = accept?;
                    tokio::spawn(crate::session::run(stream));   // one session per channel (D-14)
                }
                _ = &mut shutdown => {
                    tracing::info!("shutdown signal — closing tunnel");
                    host.close().await?;
                    return Ok(());
                }
            }
        }
    }

    async fn ensure_token() -> anyhow::Result<token_cache::CachedToken> {
        if let Some(t) = token_cache::load()? { return Ok(t); }
        auth::run_first_run_device_flow().await   // prints code, polls, persists, returns
    }

    async fn register_tunnel(mgmt: &TunnelManagementClient) -> anyhow::Result<Tunnel> {
        let hostname = hostname::get().context("hostname")?.to_string_lossy().to_string();
        let mut labels = HashMap::new();
        labels.insert("vector-agent".to_string(), "true".to_string());   // D-10 filter key
        let tunnel = Tunnel {
            name: Some(format!("vector-{}", hostname)),    // D-09 prefix
            labels: Some(labels),
            // tags, expiration, ports — defer to SDK defaults
            ..Default::default()
        };
        let created = mgmt.create_tunnel(&tunnel, /* options */).await?;
        Ok(created)
    }

    pub async fn status() -> anyhow::Result<()> {
        let tok = token_cache::load()?;
        match tok {
            None => { println!("not registered (run `vector-tunnel-agent` to register)"); }
            Some(t) => {
                println!("provider: {:?}", t.provider);
                println!("token expires_at_unix: {}", t.expires_at_unix);
                // Optionally fetch live tunnel status from Management API
            }
        }
        Ok(())
    }

    fn build_mgmt_client(t: &token_cache::CachedToken) -> anyhow::Result<TunnelManagementClient> {
        // Auth header per D-06:
        //   GitHub → "github gho_..."
        //   Microsoft → "Bearer <jwt>"
        // The SDK's Authorization enum has both variants; use Provider tag to dispatch.
        // Implementation: dispatch on the Provider tag.
        //   - AuthProvider::GitHub(t)    → use the SDK's GitHub-flavored Authorization
        //                                  constructor (e.g. `Authorization::GitHub(t.clone())`)
        //   - AuthProvider::Microsoft(t) → use `Authorization::Bearer(t.clone())`
        // Inspect the vendored SDK at execution time to confirm the exact constructor names
        // and module path — read `vendor/dev-tunnels-rs/src/management/mod.rs` (or equivalent
        // after `cargo vendor`) for the `Authorization` enum definition. If the SDK changes,
        // document the new path in this plan's SUMMARY.
        //
        // Pseudo-code shape (verify against actual SDK):
        //   let auth = match t.provider {
        //       Provider::GitHub    => Authorization::GitHub(t.access_token.clone()),
        //       Provider::Microsoft => Authorization::Bearer(t.access_token.clone()),
        //   };
        //   TunnelManagementClient::new(/* user-agent */, /* base */, Some(auth))
        unimplemented!("verify SDK Authorization variant names at execution time; see comment above")
    }
    ```

    The exact SDK type paths (`tunnels::contracts::Tunnel`, `TunnelManagementClient::create_tunnel` signature, `RelayTunnelHost::accept_next`) must be VERIFIED by reading `microsoft/dev-tunnels/rs/src/` at execution time. Treat the snippet above as architecture; adapt to the SDK's actual surface. If the SDK has changed since 2026-05-19 research, document the deltas in the Wave 0 spike sub-step before proceeding.

    Step 2 — `crates/vector-tunnel-agent/src/session.rs` (NEW): per-channel session.
    ```rust
    use anyhow::Result;
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    use tokio::sync::mpsc;
    use vector_tunnel_protocol::{AgentMessage, PROTOCOL_VERSION};

    /// Run one Vector session over a single relay channel (D-14: one shell per channel).
    pub async fn run<S>(stream: S) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let (read_half, mut write_half) = tokio::io::split(stream);
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();

        // Step 1: await OpenPty handshake (must be first frame).
        line.clear();
        reader.read_line(&mut line).await?;
        let msg: AgentMessage = serde_json::from_str(line.trim_end())?;
        let (rows, cols, shell) = match msg {
            AgentMessage::OpenPty { protocol_version, rows, cols, shell } => {
                if protocol_version != PROTOCOL_VERSION {
                    let err = AgentMessage::Error { reason: "protocol_version_mismatch".into() };
                    write_frame(&mut write_half, &err).await?;
                    return Ok(());
                }
                (rows, cols, shell)
            }
            _ => {
                let err = AgentMessage::Error { reason: "expected open_pty as first frame".into() };
                write_frame(&mut write_half, &err).await?;
                return Ok(());
            }
        };

        // Step 2: spawn PTY.
        let session_id = uuid_like_id();   // see helper below
        let pty = native_pty_system().openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })?;
        let mut cmd = CommandBuilder::new(shell.unwrap_or_else(|| std::env::var("SHELL").unwrap_or("/bin/sh".into())));
        cmd.env("TERM", "xterm-256color");
        let mut child = pty.slave.spawn_command(cmd)?;
        drop(pty.slave);   // Pitfall 3 (Phase 2 Plan 03) — close slave or zombie

        let mut master_writer = pty.master.take_writer()?;
        let mut master_reader = pty.master.try_clone_reader()?;
        let master_resize = pty.master;   // keep handle alive for resize

        // Step 3: handshake reply.
        let opened = AgentMessage::Opened { protocol_version: PROTOCOL_VERSION, session: session_id.clone() };
        write_frame(&mut write_half, &opened).await?;

        // Step 4: bidirectional pump (biased select: resize > write > read).
        // ... 60 LOC of tokio::select! with: read from `reader` for inbound frames,
        //     read from `master_reader` for outbound PTY bytes,
        //     periodic `child.try_wait()` check for exit.

        // On any error or exit: emit AgentMessage::Exit with the child's status.code()
        // and return.

        // IMPLEMENT THIS PUMP IN FULL — do not ship the placeholder above.
        //
        // Implementation: mirror the biased-select structure in `vector-ssh/src/transport.rs`.
        // See Tests 5 and 7 in this task's <behavior> block for the contract:
        //   - read from `master_reader` (PTY out) → encode AgentMessage::Data { session, bytes }
        //     → write to `write_half` (i.e. the wire / SDK channel write half) as a JSON line.
        //   - read from `reader` (BufReader over the wire's read half) → parse AgentMessage →
        //     dispatch: Data { bytes } → `master_writer.write_all(&bytes)`, Resize { rows, cols }
        //     → `master_resize.resize(PtySize { rows, cols, .. })`, Exit / Error → break.
        //   - both arms wrapped in `tokio::select!` with `biased;` ordering:
        //     1) resize-channel recv (priority — rare, must never starve)
        //     2) inbound wire frame (apply to PTY)
        //     3) PTY master read (emit Data frame outbound)
        //     4) `child.try_wait()` tick (every ~100ms) — on Some(status), emit
        //        AgentMessage::Exit { session, code: status.exit_code().unwrap_or(-1) } and break.
        // On any error path or after the loop ends: ensure `write_frame(&mut write_half,
        // &AgentMessage::Exit { session, code })` is sent before return. Drop order: writer
        // closed → wire EOF on client; child reaped via `child.wait().await`.
        Ok(())
    }

    async fn write_frame<W: tokio::io::AsyncWrite + Unpin>(w: &mut W, m: &AgentMessage) -> Result<()> {
        let mut s = serde_json::to_string(m)?;
        s.push('\n');
        w.write_all(s.as_bytes()).await?;
        Ok(())
    }

    fn uuid_like_id() -> String {
        // No uuid dep — use process-pid + nano-time, sufficient for v1 single-session.
        format!("s-{}-{}", std::process::id(), std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0))
    }
    ```
    The biased-select pump is the load-bearing piece — implement it following `crates/vector-ssh/src/transport.rs` shape exactly. Tests 4 + 5 + 6 + 7 prove the pump is correct.

    Step 3 — `crates/vector-tunnel-agent/tests/protocol_codec.rs`: replace Plan 08-01 stubs with real tests 1–4.

    Step 4 — `crates/vector-tunnel-agent/tests/session_lifecycle.rs` (NEW): Tests 5–7. Use a `tokio::io::duplex(8192)` pair to bridge a fake "wire" to the session::run call; the test owns the other end. Test 5+7 spawn a real `/bin/sh`; Test 6 may be `#[cfg(target_os = "linux")]`-gated since macOS PTY resize behavior differs slightly (the agent runs on Linux; this is fine).

    Test gating: `#[cfg(target_os = "linux")]` on real-PTY tests so `cargo test -p vector-tunnel-agent` on the developer's Mac doesn't run them (the binary is Linux-deployed; PTY-spawn tests pass on the Mac too in practice, but mark Linux as ground-truth). Mark a SUBSET of tests with `#[ignore]` if they require root/CAP_SYS_ADMIN — none should in v1.
  </action>
  <verify>
    <automated>cargo build -p vector-tunnel-agent &amp;&amp; cargo test -p vector-tunnel-agent --test protocol_codec &amp;&amp; cargo test -p vector-tunnel-agent --test session_lifecycle 2>&amp;1 | tail -20 &amp;&amp; cargo clippy -p vector-tunnel-agent --all-targets -- -D warnings &amp;&amp; grep -q "drop(pty.slave)\\|drop(pair.slave)" crates/vector-tunnel-agent/src/session.rs &amp;&amp; grep -q "biased" crates/vector-tunnel-agent/src/session.rs &amp;&amp; grep -q "vector-agent" crates/vector-tunnel-agent/src/host.rs &amp;&amp; grep -q "vector-{" crates/vector-tunnel-agent/src/host.rs</automated>
  </verify>
  <acceptance_criteria>
    - `cargo build -p vector-tunnel-agent` exit 0
    - `cargo test -p vector-tunnel-agent --test protocol_codec` reports >= 4 passed
    - `cargo test -p vector-tunnel-agent --test session_lifecycle` reports >= 3 passed (Test 5 + Test 7 + one of Test 4 or Test 6 minimum — the others may be `#[ignore]`-gated for Linux-only CI)
    - `cargo clippy -p vector-tunnel-agent --all-targets -- -D warnings` exit 0
    - `grep -c "drop(pty.slave)\\|drop(pair.slave)" crates/vector-tunnel-agent/src/session.rs` >= 1 (Pitfall 3 zombie-prevention)
    - `grep -c "biased" crates/vector-tunnel-agent/src/session.rs` >= 1 (biased select for resize priority)
    - `grep -c "\"vector-agent\"" crates/vector-tunnel-agent/src/host.rs` >= 1 (D-10 label key)
    - `grep -c "vector-" crates/vector-tunnel-agent/src/host.rs` >= 1 (D-09 name prefix)
    - `grep -c "TERM" crates/vector-tunnel-agent/src/session.rs` >= 1 (xterm-256color advertised — CORE-05 pattern)
  </acceptance_criteria>
  <done>Agent registers tunnel with `vector-agent: true` label + `vector-{hostname}` name, accepts relay channels, runs the JSON protocol against a real PTY shell with biased resize > write > read select. Zombie-shell prevention in place per Pitfall 3.</done>
</task>

</tasks>

<verification>
- `cargo build -p vector-tunnel-agent` exit 0 (binary builds for the developer's host arch)
- `./target/debug/vector-tunnel-agent --version` works
- `make lint` exit 0
- `make test` exit 0 (includes vector-tunnel-agent --tests; non-ignored tests passing)
- `cargo test -p vector-arch-tests --tests` (Pitfall 14) 0 failed
</verification>

<success_criteria>
- Agent binary builds and runs `--version` + `--help`
- First-run device flow prints code + URL + persists token mode 0600 atomically
- RelayTunnelHost registers tunnel with `vector-agent: true` label + `vector-{hostname}` name (D-09/D-10)
- Session module spawns PTY, handles OpenPty handshake with protocol_version: 1, biased-select pump with resize priority, emits Exit on child exit
- All token-bearing types use manual Debug
- Pitfall 3 zombie-shell prevention in place (drop slave)
</success_criteria>

<output>
After completion, create `.planning/phases/08-vs-code-remote-tunnels-connect/08-03-SUMMARY.md` documenting:
- The exact SDK API surface used (verified RelayTunnelHost::new / accept_next / close signatures — paste here from the vendored SDK at exec time)
- Any SDK API deviations from 08-RESEARCH.md
- Whether `clap` / `dirs` / `hostname` / `nix` were added at workspace level (and pinned versions)
- The Linux-only test count vs cross-platform test count
</output>
