# Phase 7: SSH Transport + Codespaces Connect — Research

**Researched:** 2026-05-19
**Domain:** SSH transport over `gh codespace ssh --stdio` subprocess + russh client + CodespaceDomain
**Confidence:** HIGH on transport mechanics + GitHub APIs + integration seams; MEDIUM on the exact russh 0.60 channel-API surface (verified by Eugeny/russh examples but not Context7-fetched per call site).

## Summary

Phase 7 wires a **single-process, subprocess-based SSH transport** into Vector. The user clicks Connect on an Available codespace in the Phase 6 picker; the app spawns `gh codespace ssh -c {name} --stdio` as a child process, treats its stdin/stdout as a bidirectional raw-SSH byte pipe, drives a `russh 0.60` client over that pipe, opens a session channel, requests a PTY with the pane's initial rows/cols, exec's the user's login shell, and exposes the channel's bidirectional stream as a `Box<dyn PtyTransport>` plugged into the existing per-pane PTY actor router (Plan 04-03). Resize round-trips via russh `Channel::window_change`.

The architecture intentionally **defers the native russh+gRPC implementation to v1.x (CS-V2-01)**. `gh --stdio` is the v1 transport; it handles the port-16634 gRPC dance, the Codespaces relay tunnel, and the OAuth-derived ephemeral SSH cert internally. We hand it a `-c {name}` and an `-i {ed25519_keyfile}` and it returns a TCP-equivalent SSH byte stream on stdin/stdout. Vector then speaks SSH itself — that's how we get `window-change`, exit-status, and (in Phase 9) channel-level reconnect without re-parsing protocol details.

Vector generates an ed25519 keypair on first connect, registers the public half via `POST /user/keys` (scope: `write:public_key`), and stores the private key at `~/.ssh/vector_codespace_ed25519` (mode 0600). Subsequent connects reuse it. Tab tint and "remote" badge are driven by `TransportKind::Codespace` flowing from the transport up to the tab title and `TintStripePipeline` color uniform — both Phase 4/5 plumbing already exists.

**Primary recommendation:** Three-plan structure — (1) `vector-ssh` russh client wrapper + stdio-piped subprocess transport + tests against a real `gh` subprocess; (2) SSH keypair generation + `/user/keys` registration in `vector-codespaces`; (3) `CodespaceDomain::spawn` + wire-up in `app.rs::codespaces_connect_selected` + tab tint + "remote" badge.

## User Constraints (from CONTEXT.md)

CONTEXT.md does not exist for Phase 7. Treat the **ROADMAP Phase 7 entry** as the binding constraint set:

### Locked Decisions

- **v1 SSH transport = subprocess `gh codespace ssh --stdio`.** Not native russh+gRPC over port 16634.
- **`gh` CLI is a hard runtime dependency for v1.** No fallback to plain `ssh` — Codespaces SSH is not plain TCP SSH; it rides a tunneled relay only `gh` (or the Microsoft Dev-Tunnels SDK) currently speaks in production.
- **Stack additions: `russh 0.60`, `vector-ssh` crate impl, `CodespaceDomain` impl.** No new crates beyond these.
- **Host-key trust uses the API-provided fingerprint, not TOFU bypass** (Pitfall 15). The codespace's SSH host fingerprint is reachable via `GET /user/codespaces/{name}` → `connection.host_key_fingerprint` (already in the OctocrabAPI). The `Handler::check_server_key` impl must validate against that, not return `Ok(true)` blindly.
- **`pty-req` sends initial cols/rows; resize sends `window-change`** (Pitfall 7).
- **Phase 7 is `Domain::reconnect()` body unimplemented** — that's Phase 9. CodespaceDomain ships `unimplemented!("Phase 9")` for reconnect just like the Phase-2 stub.

### Claude's Discretion

- **Exact russh client architecture** (single shared client + multiplexed channels, vs one client per pane). Recommend one russh client per `CodespaceDomain::spawn` call for v1 simplicity — matches per-pane lifecycle, sidesteps multiplexing bugs. Multi-channel reuse is a v1.x optimization.
- **Key file path** — recommend `$HOME/.ssh/vector_codespace_ed25519` (mirrors `gh`'s own `automatic-id` path naming) with `0o600` perms. Don't use the keychain — `gh --stdio` needs a file path via `-i` argv.
- **"Remote" badge UI** — recommend a `[remote]` text suffix on the tab title (string-level, reuses existing `format_tab_title`) plus the existing `TintStripePipeline` color uniform set from the active profile's `tint` field. No new wgpu pipeline.
- **gh subprocess error surfacing** — recommend: on spawn failure or non-zero exit code in first 5 seconds, route an error toast through `EventLoopProxy<UserEvent>` and tear down the half-created pane.

### Deferred Ideas (OUT OF SCOPE)

- **Native russh+gRPC over port 16634** — explicitly v1.x (CS-V2-01).
- **Port-forwarding panel / "PORTS" tab** — v2 (RDEV-V2-01).
- **`Domain::reconnect()` body** — Phase 9.
- **Tmux auto-attach (`tmux new -A -s vector-{profile-id}`)** — Phase 9 (PERSIST-03).
- **Codespace lifecycle from inside the app (create/delete/rebuild)** — v2 (RDEV-V2-03).
- **SSH agent integration** — v2. Vector manages exactly one ed25519 key for codespaces, no agent socket.
- **Multi-codespace key registration scoping** — out of scope. One key registers for all the user's codespaces (it's a user-level key, not per-codespace).

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CS-04 | Connect opens remote shell in a Vector pane via subprocess `gh codespace ssh --stdio` | `--stdio` flag verified in `gh` 2.92 binary (`gh codespace ssh --stdio` → `requires explicit --codespace`); fossies.org source confirms it pipes raw SSH protocol over stdin/stdout to a port-forwarded sshd. Wire as `tokio::process::Command` → `(ChildStdin, ChildStdout)` → russh `connect_stream`. |
| CS-05 | Vector generates and registers an ed25519 keypair per machine; no manual ssh-add | `ssh-key` crate (pure-Rust ed25519 PEM/OpenSSH key generation, no OpenSSL dep) + `POST /user/keys` (scope `write:public_key`) + filesystem store at `~/.ssh/vector_codespace_ed25519`. Add `admin:public_key` to OAuth scopes at Phase 6 boundary or piggyback on Phase 6's existing token if `write:public_key` is already granted by `gh`'s default CLI scopes. |
| CS-06 | Connected tab is visually distinct: tinted tab + "remote" badge in title | Reuses existing `TintStripePipeline` (Plan 05-08 / D-75) — set color from active profile's `tint` field on first PaneOutput from a Codespace transport. Tab title gains a `[remote]` suffix via `format_tab_title` extension that takes `TransportKind`. |
| CS-07 | Resize propagates `window-change` so remote `vim`/`tmux` reflow within 1s | russh `Channel::window_change(cols, rows, px_w, px_h)` from inside `PtyTransport::resize` — fits the existing trait shape exactly. Resize already debounced at the App layer (50ms, D-49). |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `russh` | 0.60.2 | Async pure-Rust SSH client over arbitrary `AsyncRead + AsyncWrite + Unpin + Send` stream | Project-locked in `.planning/research/STACK.md`; active maintenance under @Eugeny; pure-Rust + tokio-native; used internally by Microsoft Dev-Tunnels Rust SDK. **MUST be added to workspace deps — not currently in `Cargo.toml`.** Required features: default (server feature optional; we only need client). |
| `russh-keys` | 0.60.x (matches russh) | Optional companion for keypair parse/serialize | Re-exports key types russh uses. We may not need this directly if `ssh-key` is used for generation; russh accepts `PrivateKey` from `ssh-key`. **Confirm at Wave-0 spike**: either russh re-exports `ssh-key` or russh-keys is the bridge. |
| `ssh-key` | 0.6.x | Pure-Rust ed25519/RSA OpenSSH-format keypair generation + PEM serialize | No OpenSSL dependency; emits OpenSSH `id_ed25519` and `id_ed25519.pub` formats byte-identical to `ssh-keygen -t ed25519 -N ""`. Used by russh itself for key parsing. Verify version compat with russh 0.60 at Wave 0. |
| `tokio::process::Command` | (tokio workspace pin 1.52.3) | Spawn `gh codespace ssh --stdio` child with piped stdin/stdout | Native to tokio; gives `ChildStdin: AsyncWrite`, `ChildStdout: AsyncRead`. Combine into a single `Stream { stdin: ChildStdin, stdout: ChildStdout }` type for `russh::client::connect_stream`. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `octocrab` | 0.50.0 (workspace) | `POST /user/keys` for SSH key registration | New `register_ssh_key(title, key)` helper in `vector-codespaces::auth` (or `client`). Octocrab does not natively wrap `/user/keys` for some endpoints — fall back to `reqwest` direct (the codebase already does this for `/user/codespaces` per `device_flow.rs:271`). |
| `reqwest` | 0.12 (workspace) | Direct REST fallback for `POST /user/keys` and `GET /user/codespaces/{name}` (for host fingerprint) | Same pattern as `list_codespaces_direct`. |
| `tokio` | 1.52.3 (workspace) | Existing async runtime | No additions. |
| `anyhow` + `thiserror` | (workspace) | Error handling — `vector-ssh::SshError` typed via `thiserror`; `Domain::spawn` returns `Result<_, anyhow::Error>` per locked trait shape | Mirror existing `vector-pty::PtyError`. |
| `zeroize` | 1 (workspace) | Wipe private key material on drop | Already a workspace dep; use `Zeroizing<Vec<u8>>` for the in-memory key bytes during keypair generation. |
| `tracing` | (workspace) | Structured logs for ssh handshake / channel lifecycle | Heavy `tracing::debug!` on handshake, `tracing::info!` on connect/exit, `tracing::warn!` on host-key mismatch. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| russh over `gh --stdio` subprocess | Plain `tokio::process::Command::new("gh").arg("cs").arg("ssh")` with PTY | Loses programmatic `window-change` (relies on SIGWINCH propagating through gh subprocess → relay → remote sshd; works but flaky on disconnect); loses exit-code observability; loses the channel-level seam Phase 9 needs for reconnect. **Rejected.** |
| russh over `gh --stdio` | OpenSSH `ssh` binary with `ProxyCommand "gh cs ssh -c X --stdio -- -i K"` | Two subprocesses (ssh + gh) instead of one (gh), and we'd parse `ssh`'s stderr for window-change confirmations. Slower spawn, more failure surface. **Rejected.** |
| `ssh-key 0.6` for keygen | `osshkeys`, `openssh-keys` | `ssh-key` is the most-active pure-Rust crate and is what russh itself depends on internally. Avoids version skew. |
| `gh` CLI as runtime dep | Vendor a stripped-down Go-to-Rust port of `gh cs`'s tunnel client | 100x more effort. v1.x territory per CS-V2-01. |
| One russh client per `spawn` | Singleton russh client + multiplexed channels per codespace | Simpler, isolates failures per pane, matches Phase-4 per-pane PtyTransport lifetime, avoids cross-pane noisy-neighbor bugs. Cost: one `gh` subprocess + one TCP-equivalent connection per pane (acceptable for v1; tmux on remote solves persistence in Phase 9). |
| Tab tint via new pipeline | Reuse `TintStripePipeline` (D-75) | Pipeline already exists, already wired to the active profile's `tint`. Codespace tabs already get `tint = "#7a3aaf"` when saved as a profile (see `app.rs:541`). Phase 7's job is to make sure tint is applied when connected from the picker too, not just from the profile list. |

**Installation (workspace `Cargo.toml` additions):**

```toml
# [workspace.dependencies]
russh = "0.60"
ssh-key = { version = "0.6", default-features = false, features = ["ed25519", "alloc", "rand_core"] }
# optional: russh-keys if russh 0.60 still exports the trait separately
```

**Version verification** (run before plan-writing — researcher could not exec from sandbox):

```bash
npm view russh version  # N/A — Rust
cargo search russh --limit 1
cargo search ssh-key --limit 1
```

Expected published versions per upstream README + STACK.md: `russh 0.60.2` (2026-04-29), `ssh-key 0.6.x` (latest pre-Apr 2026). **Confirm at Wave 0 spike** — if russh has shifted to 0.61+, re-check `connect_stream` signature.

## Architecture Patterns

### Recommended Project Structure

```
crates/vector-ssh/
├── Cargo.toml          # russh + ssh-key + tokio + tracing + thiserror + anyhow
└── src/
    ├── lib.rs          # re-exports
    ├── client.rs       # SshClient: wraps russh::client::Handle, opens session channel
    ├── transport.rs    # SshChannelTransport: impl PtyTransport over russh Channel
    ├── stdio_stream.rs # AsyncRead+AsyncWrite adapter over (ChildStdin, ChildStdout)
    ├── handler.rs      # impl russh::client::Handler with check_server_key
    └── error.rs        # SshError

crates/vector-codespaces/src/
├── client/mod.rs       # existing; add register_ssh_key + get_codespace_with_connection
└── ssh_keys.rs         # NEW: KeyManager — generate/load/store ed25519 key

crates/vector-mux/src/
└── codespace_domain.rs # impl spawn(): glues KeyManager + gh subprocess + SshClient

crates/vector-app/src/
├── app.rs              # codespaces_connect_selected: dispatch to codespace_actor
└── codespace_actor.rs  # NEW: tokio task that runs CodespaceDomain::spawn → installs pane
```

### Pattern 1: Subprocess-as-AsyncStream (the critical pattern)

**What:** Wrap `tokio::process::Child`'s stdin/stdout pair into a single struct that implements `AsyncRead + AsyncWrite + Unpin + Send`, suitable for `russh::client::connect_stream`.

**When to use:** Anytime we proxy SSH through a ProxyCommand-style external tunneler. This is the exact pattern OpenSSH's `ssh -o ProxyCommand=...` uses internally; we just do it in pure Rust.

**Example:**

```rust
// Source: pattern from tokio + russh; verify against russh 0.60 docs at Wave 0
// crates/vector-ssh/src/stdio_stream.rs
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::process::{ChildStdin, ChildStdout};

pub struct ChildStdioStream {
    stdout: ChildStdout, // AsyncRead
    stdin:  ChildStdin,  // AsyncWrite
}

impl ChildStdioStream {
    pub fn new(stdout: ChildStdout, stdin: ChildStdin) -> Self {
        Self { stdout, stdin }
    }
}

impl AsyncRead for ChildStdioStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdout).poll_read(cx, buf)
    }
}

impl AsyncWrite for ChildStdioStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.stdin).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdin).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdin).poll_shutdown(cx)
    }
}
```

### Pattern 2: russh Channel as PtyTransport

**What:** Once `russh::client::connect_stream(config, stream, handler).await` returns a `Handle`, call `handle.channel_open_session().await?`, then `channel.request_pty(...)`, `channel.exec(...)` (or `channel.request_shell(true)`). Convert the resulting `Channel<Msg>` into the bidirectional `AsyncRead + AsyncWrite` stream via `channel.into_stream()` (verify exact method name on russh 0.60).

**When to use:** This is `CodespaceDomain::spawn`'s entire body.

**Example (sketch — verify signatures at Wave 0):**

```rust
// crates/vector-ssh/src/client.rs (sketch)
use russh::client::{self, Config, Handle, Handler};
use russh::keys::PrivateKey;
use russh::{ChannelMsg, Disconnect};

pub struct SshClient {
    handle: Handle<MyHandler>,
}

impl SshClient {
    pub async fn connect_over<S>(
        stream: S,
        username: &str,
        identity: PrivateKey,
        host_key_fingerprint: String,
    ) -> Result<Self, SshError>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let config = std::sync::Arc::new(Config::default());
        let handler = MyHandler { expected_fp: host_key_fingerprint };
        let mut handle = client::connect_stream(config, stream, handler).await?;
        let authed = handle.authenticate_publickey(username, std::sync::Arc::new(identity)).await?;
        if !authed.success() { return Err(SshError::AuthFailed); }
        Ok(Self { handle })
    }

    pub async fn open_pty_shell(&self, rows: u16, cols: u16)
        -> Result<russh::Channel<russh::client::Msg>, SshError>
    {
        let mut chan = self.handle.channel_open_session().await?;
        chan.request_pty(true, "xterm-256color", cols.into(), rows.into(), 0, 0, &[]).await?;
        chan.request_shell(true).await?;
        Ok(chan)
    }
}
```

### Pattern 3: SshChannelTransport — adapter into the existing PtyTransport trait

**What:** Implement `vector_mux::PtyTransport` over a russh `Channel<Msg>` plus a `tokio::sync::mpsc::Sender<Vec<u8>>` reader bridge plus a held `Handle` (so the SSH session outlives the spawn call).

**When:** This is the value `CodespaceDomain::spawn` returns; the Phase-4 pty_actor router consumes it byte-identical to LocalTransport.

**Sketch:**

```rust
// crates/vector-ssh/src/transport.rs (sketch)
use async_trait::async_trait;
use russh::ChannelMsg;
use tokio::sync::mpsc;
use vector_mux::{PtyTransport, TransportKind};

pub struct SshChannelTransport {
    channel:    Option<russh::Channel<russh::client::Msg>>, // taken into reader task on first take_reader
    reader_rx:  Option<mpsc::Receiver<Vec<u8>>>,
    writer:     mpsc::Sender<Vec<u8>>,                       // routes writes back into the channel task
    resize_tx:  mpsc::UnboundedSender<(u16, u16)>,           // window_change requests
    _gh_child:  Option<tokio::process::Child>,               // hold the gh subprocess; drop = SIGKILL gh
    _handle:    russh::client::Handle<crate::handler::MyHandler>, // hold the russh handle
}

#[async_trait]
impl PtyTransport for SshChannelTransport {
    fn resize(&mut self, rows: u16, cols: u16, px_w: u16, px_h: u16) -> Result<()> {
        self.resize_tx.send((rows, cols)).map_err(|e| anyhow::anyhow!(e))
    }
    async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer.send(bytes.to_vec()).await.map_err(|e| anyhow::anyhow!(e))
    }
    fn take_reader(&mut self) -> Option<mpsc::Receiver<Vec<u8>>> {
        self.reader_rx.take()
    }
    fn kind(&self) -> TransportKind { TransportKind::Codespace }
    async fn wait(&mut self) -> Result<Option<i32>> {
        // Wait for the channel-task join handle to return the exit-status.
        ...
    }
}
```

The internal "channel task" spawned on `connect` drives a single `tokio::select!` over (a) `channel.wait()` ChannelMsg::Data → push into reader_tx; (b) `writer_rx.recv()` → `channel.data(bytes).await`; (c) `resize_rx.recv()` → `channel.window_change(cols, rows, 0, 0).await`; (d) ChannelMsg::ExitStatus → record exit code, break loop. Exit ⇒ also `_gh_child.kill()` to release the subprocess.

### Pattern 4: Codespace action flow in app.rs

`codespaces_connect_selected` currently emits a placeholder toast (`app.rs:486-496`). Phase 7 replaces the body with:

1. Read the selected `Codespace` from `self.codespaces_modal`.
2. Dispatch to a new `crate::codespace_actor::spawn_codespace_connect(handle, proxy, client, codespace_name, mux_window_id, rows, cols)`.
3. The actor task: (a) builds/loads `KeyManager` → ensures ed25519 keypair on disk + registered with GitHub; (b) instantiates `CodespaceDomain::new(codespace_name, key_path, host_fingerprint)`; (c) calls `mux.create_tab_async_for_domain(mux_window_id, domain, rows, cols).await` (NEW helper) — analogous to the existing `create_tab_async` but takes an arbitrary `Domain` trait object instead of using `default_domain`; (d) installs the pane + spawn the per-pane actor via `router.spawn_pane(pane_id, transport)`; (e) on error, emit `UserEvent::ToastInfo("connect failed: {e}")`.

### Anti-Patterns to Avoid

- **DO NOT block the winit main thread on `gh` subprocess spawn or russh handshake.** All `CodespaceDomain::spawn` work runs on a tokio runtime task; the result returns to the main thread via `EventLoopProxy::send_event`. Same pattern as `Mux::create_tab_async` in Plan 04-03.
- **DO NOT `from_utf8_lossy` PTY bytes.** Feed raw `&[u8]` into `Term::feed` (Pitfall 4 retired in Phase 2; don't re-introduce here).
- **DO NOT hold a `parking_lot::Mutex` across an `.await`.** `clippy::await_holding_lock = "deny"` is workspace-wide. The russh channel reader/writer/resize tasks NEVER take the Mux's RwLock.
- **DO NOT bypass `check_server_key` by returning `Ok(true)`.** That's Pitfall 15 (TOFU bypass). Validate against the API-provided fingerprint.
- **DO NOT spawn `gh codespace ssh` (without `--stdio`).** That allocates a local PTY inside the subprocess and we lose programmatic `window-change`. Always pass `--stdio`.
- **DO NOT store the ed25519 private key in the macOS Keychain.** `gh --stdio` reads it via `-i {path}` argv — a Keychain-resident key would require an `ssh-agent` round-trip we don't want. File at `~/.ssh/vector_codespace_ed25519` with `0o600` (Unix; `ssh-keygen` convention).
- **DO NOT hand-roll SSH wire-format.** Use russh. This is exactly the "Don't Hand-Roll" call.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| SSH protocol framing, KEX, channels | A bespoke SSH client | `russh 0.60` | 3000+ lines of crypto + protocol code; Eugeny's russh is the active fork and tracks RFC 4254/4252/4253; powers Warpgate + Dev-Tunnels-Rust. |
| ed25519 keypair generation + OpenSSH PEM serialize | `process::Command::new("ssh-keygen")` | `ssh-key 0.6` | Avoids needing `ssh-keygen` on PATH; deterministic; round-trips byte-identical to OpenSSH format; russh consumes the resulting `PrivateKey` directly. |
| Codespaces relay/tunnel protocol | A native Rust port of `cli/cli/internal/codespaces/connection/` | `gh codespace ssh --stdio` subprocess | Explicit v1.x scope (CS-V2-01). Subprocess delivers a working tunnel today. |
| Subprocess stdin/stdout → async stream | An `Arc<Mutex<>>` + spawn-blocking-thread pattern | `tokio::process::Command` with `Stdio::piped()` returns `ChildStdin: AsyncWrite` + `ChildStdout: AsyncRead` natively | Tokio's process types implement the AsyncRead/AsyncWrite traits — no glue needed beyond a wrapper struct combining the two halves. |
| Tab tint pipeline | A new wgpu pipeline + shader | `TintStripePipeline` (Plan 05-08) | Already exists, already wired to the active profile's `tint`. Phase 7 just sets the color from the right place. |
| Tab title rendering | New title-bar text rendering | `format_tab_title` (vector-mux) + `winit::Window::set_title` | Already in use (`app.rs:1652`). Extend `format_tab_title` to take a `TransportKind` so it appends `[remote]` for non-Local. |

**Key insight:** Phase 7's entire novel surface is roughly 600-900 LOC across `vector-ssh` + `vector-codespaces::ssh_keys` + `vector-mux::codespace_domain` + `app::codespace_actor`. Every other subsystem (tab tint, pane router, resize debounce, OSC handling, Term feed, render pass) is already in place — Phase 7 plugs into existing seams, doesn't build new ones.

## Runtime State Inventory

Phase 7 is not a rename/refactor. **Section omitted as not applicable** — there are no pre-existing runtime-stored references to migrate, and the new state (key file, registered GitHub SSH key) is created during normal operation, not migrated.

## Common Pitfalls

### Pitfall 1: `gh` CLI not installed / outdated / unauthenticated

**What goes wrong:** `gh codespace ssh --stdio` fails with "command not found", or runs but blocks waiting for browser auth, or returns "unknown flag --stdio" on a sub-2.0 `gh` version.

**Why it happens:** Vector is bundled, `gh` is not. The user may not have `gh` installed; if they do, it may not be authenticated separately (or worse — authenticated as a different user than Vector's stored OAuth token).

**How to avoid:**
- **Pre-flight check at codespace_actor start:** probe `gh --version` and `gh auth status` synchronously; if missing/unauthed, surface a toast with install link.
- **Detect the `--stdio requires explicit --codespace` error message and pin a minimum `gh` version** (≥ 2.20 based on the flag's age; verify against `gh` changelog at Wave 0).
- **Set `GH_TOKEN={our_oauth_access_token}` in the subprocess env** so we don't rely on `gh auth login` separately. This is the documented `gh` override path.

**Warning signs:** Subprocess exits with code 4 (auth error) within 500ms of spawn; stderr contains "could not find gh".

### Pitfall 2: Codespace not in Available state when Connect clicked

**What goes wrong:** User clicks Connect on a Shutdown row; `gh --stdio` spawns the codespace cold-boot transparently but our 30-second deadline expires before sshd is reachable.

**How to avoid:** The picker already routes Shutdown → Start (CS-02). Phase 7's Connect path **MUST** check `codespace.state == Available` before spawning `gh`; if Starting, queue a "wait for state change" indication and bail; if Shutdown, dispatch the Start flow.

**Warning signs:** First 5s of subprocess output is empty → likely the codespace is still booting. Surface as a "codespace is starting…" toast and retry once.

### Pitfall 3: Host-key TOFU bypass (Pitfall 15 from PITFALLS.md)

**What goes wrong:** Implementer is tempted to return `Ok(true)` from `Handler::check_server_key` because "the gh subprocess already validated it" or "it's a fresh codespace, there's no prior fingerprint". Result: MITM attacker controlling the relay can substitute their own host key and the SSH session connects anyway.

**How to avoid:** Fetch the host key fingerprint from `GET /user/codespaces/{name}` (or whichever connection endpoint `cli/cli` uses — verify at Wave 0). Compare against the fingerprint russh's `check_server_key` callback receives. On mismatch: refuse the connection, raise a hard error, no toast-and-retry. **This is non-negotiable per ROADMAP Phase 7 Risks & notes.**

**Warning signs:** Code reviewer sees `fn check_server_key(...) -> ... { Ok(true) }` anywhere.

### Pitfall 4: Window-change not sent on first PaneOutput

**What goes wrong:** `gh --stdio` connects, russh handshake completes, sshd starts a shell — but the shell sees the default 80×24 PTY size set by the `request_pty` call. Then the user resizes the window. The Phase 4 resize debounce kicks in 50ms later, calls `PtyTransport::resize`, which sends `window_change`. But if the user never resizes, the shell stays at the initial cols/rows we passed.

**How to avoid:** Pass the pane's actual cols/rows into `request_pty` (NOT a default 80×24). The pane's dims are known at `CodespaceDomain::spawn` time — they're passed in via `SpawnCommand.rows`/`cols`.

**Warning signs:** Remote `tput cols` returns 80 when the local pane is wider; remote `vim` opens a tiny editor in the corner.

### Pitfall 5: gh subprocess zombies on pane close

**What goes wrong:** User closes a pane. `pty_actor`'s `JoinSet` notices, sends `PaneExited`. But the russh client + the gh subprocess + the underlying TCP relay live in a separate tokio task that doesn't get cancelled. Zombies pile up over a session.

**How to avoid:** `SshChannelTransport.wait()` MUST `_gh_child.kill().await` AND `handle.disconnect(Disconnect::ByApplication, "", "").await` on the way out. The transport's `Drop` must do best-effort `_gh_child.start_kill()` to cover unclean exit paths. **Verify with `ps -ef | grep "gh codespace ssh"` after closing 10 codespace panes.**

**Warning signs:** Activity Monitor shows residual `gh` processes after panes close.

### Pitfall 6: russh writer task starves under chatty output

**What goes wrong:** Channel-task uses non-biased `select!` over read/write/resize. Heavy output (`cat large.log` on remote) starves `window_change` and the SSH server thinks the window stayed small.

**How to avoid:** Mirror Plan 02-05/04-03 pattern — `biased` select! with resize > write > read priority order. Identical to `pty_actor::pane_io_loop`.

**Warning signs:** Resize takes seconds to reflect under load.

### Pitfall 7: SSH key registration race / 422 "key is already in use"

**What goes wrong:** First-connect on machine A registers key K. Machine B (or the same machine after a wipe) generates key K' and registers — fine. But if Vector deletes the local key file mid-session, regenerates, and tries to re-register, GitHub returns 422 "key already in use" if the same public key is presented (won't happen — different random key) OR a stale registration from a previous Vector install holds the title slot.

**How to avoid:** `POST /user/keys` payload `title` should be unique per machine (`format!("vector-{hostname}-{uuid}")`); on 422, fall back to fetching `GET /user/keys`, finding the entry by title, deleting via `DELETE /user/keys/{id}`, retrying once. Cap retries at 1 to avoid loops.

**Warning signs:** First connect after reinstall fails with 422 and no auto-recovery.

### Pitfall 8: OAuth scope missing `write:public_key`

**What goes wrong:** Phase 6 device flow requested scopes `codespace`, `read:user` only (see `device_flow.rs:134-135`). `POST /user/keys` returns 403 because the token lacks `write:public_key`.

**How to avoid:** **Decide at Plan 1**: do we extend Phase 6's scope set (forces re-auth on Phase 7 first-run for existing users) OR piggyback on `gh`'s own credential storage (let `gh` register the key via its automatic key path)? Recommend: **extend the scope set in `device_flow.rs:134` to include `write:public_key`** and document this as a forced re-auth note in the Phase 7 README. The forced re-auth is a one-time cost; users see "Vector needs to update its GitHub permissions" once.

**Warning signs:** First connect on an existing Phase-6 install returns 403 from `/user/keys`.

### Pitfall 9: Hidden `--stdio` flag may change

**What goes wrong:** `--stdio` is a hidden flag (`gh codespace ssh -h` doesn't list it). GitHub could rename or remove it in a `gh` minor update without breaking documented surface area. Vector breaks silently when users update `gh`.

**How to avoid:** Pin a minimum `gh` version (currently 2.92.0 on the dev machine; project-document `gh >= 2.40`); detect "unknown flag" stderr and surface an actionable error. Add a smoke test against the live `gh` binary in CI (best-effort; CI may not have `gh` installed — gate behind `GH_AVAILABLE` env).

**Warning signs:** Subprocess exits with "unknown flag: --stdio" stderr.

### Pitfall 10: PaneResized fires before the resize debounce, but `window_change` is async

**What goes wrong:** User drags window corner; 50ms debounce expires; main thread calls `router.send_resize(pane_id, rows, cols)`; the resize mpsc fires; the pane_io_loop calls `transport.resize(rows, cols, 0, 0)`. For LocalTransport this is sync (TIOCSWINSZ ioctl). For SshChannelTransport, `resize` enqueues a (rows, cols) tuple onto an unbounded channel for the channel-task to consume and call `channel.window_change(...).await`. If the channel task is wedged or the channel is back-pressured, the resize never reaches the remote.

**How to avoid:** Don't make `PtyTransport::resize` async (it isn't — the trait method is sync). Decouple: the trait's `resize` is `tokio::sync::mpsc::UnboundedSender::send` (sync); the channel-task drains and awaits `window_change` independently. Log if the unbounded channel grows beyond 8 — that's a wedge signal.

**Warning signs:** Remote `vim` doesn't reflow within 1s on resize; logs show growing resize backlog.

## Code Examples

### Spawn `gh codespace ssh --stdio` as an async stdin/stdout subprocess

```rust
// Source: tokio docs + gh `cli/cli` source verified at Wave 0
use tokio::process::Command;
use std::process::Stdio;

let mut child = Command::new("gh")
    .args([
        "codespace", "ssh",
        "--codespace", codespace_name,
        "--stdio",
        "--", "-i", key_path.to_str().unwrap(),
    ])
    .env("GH_TOKEN", access_token.as_str()) // override gh's own auth
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped()) // capture for diagnostic toast on failure
    .kill_on_drop(true)     // belt-and-braces zombie prevention
    .spawn()?;

let stdin  = child.stdin.take().ok_or_else(|| anyhow!("no stdin"))?;
let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout"))?;
let stream = ChildStdioStream::new(stdout, stdin);
```

### ed25519 keypair generation (ssh-key 0.6)

```rust
// Source: ssh-key crate docs — verify exact API at Wave 0
use ssh_key::{PrivateKey, Algorithm, LineEnding};

let key = PrivateKey::random(&mut rand::rngs::OsRng, Algorithm::Ed25519)?;
let openssh_priv = key.to_openssh(LineEnding::LF)?;            // -> Zeroizing<String>
let openssh_pub  = key.public_key().to_openssh()?;             // -> String, "ssh-ed25519 AAAA…"
std::fs::write(&priv_path, openssh_priv.as_bytes())?;
#[cfg(unix)] {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&priv_path, std::fs::Permissions::from_mode(0o600))?;
}
std::fs::write(&pub_path, openssh_pub.as_bytes())?;
```

### POST /user/keys (direct reqwest, matches device_flow.rs pattern)

```rust
let resp = http_client
    .post(format!("{api_base}/user/keys"))
    .header("Authorization", format!("Bearer {}", access_token.as_str()))
    .header("Accept", "application/vnd.github+json")
    .header("User-Agent", "Vector/0.1")
    .json(&serde_json::json!({
        "title": format!("vector-{}-{}", hostname, machine_uuid),
        "key":   openssh_pub,
    }))
    .send().await?;
// 201 Created on success; 422 if title clashes — retry with delete-then-add.
```

### russh handler with API-fingerprint validation

```rust
struct VectorHandler { expected_fp: String /* "SHA256:..." */ }

#[async_trait::async_trait]
impl russh::client::Handler for VectorHandler {
    type Error = russh::Error;
    async fn check_server_key(
        &mut self,
        server_pubkey: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // ssh-key API: fingerprint(HashAlg::Sha256).to_string() -> "SHA256:..."
        let actual_fp = server_pubkey.fingerprint(russh::keys::ssh_key::HashAlg::Sha256).to_string();
        let ok = actual_fp == self.expected_fp;
        if !ok {
            tracing::warn!(?actual_fp, expected = %self.expected_fp, "host key mismatch — refusing");
        }
        Ok(ok)
    }
}
```

### Wiring CodespaceDomain into Mux

The existing `Mux::create_tab_async` hard-codes `self.default_domain.spawn_local(...)`. Phase 7 needs a sibling helper that takes an arbitrary `Domain` trait object:

```rust
// crates/vector-mux/src/mux.rs — add:
pub async fn create_tab_async_with_domain(
    &self,
    window_id: WindowId,
    domain: &dyn Domain,
    cwd: Option<PathBuf>,
    rows: u16,
    cols: u16,
) -> Result<(TabId, PaneId)> {
    let transport = domain.spawn(SpawnCommand {
        argv: None, cwd, rows, cols, env: vec![],
    }).await?;
    let pane_id = self.allocate_pane_id();
    let term = Arc::new(Mutex::new(self.build_term(cols, rows, 10_000)));
    // `pid` and `master_fd` are Local-only; None for SSH.
    let pane = Arc::new(Pane::new(pane_id, term, transport, None, None));
    Ok(self.install_tab(window_id, pane, rows, cols))
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| OpenSSH + `gh cs ssh --config` + ProxyCommand | `gh cs ssh --stdio` direct (no OpenSSH dep) | gh 2.x feature (year tied to D-32 era of `cli/cli`) | We don't need `ssh` on PATH; pure Rust SSH; programmatic window-change. |
| ssh-key registration via `ssh-keygen` subprocess | `ssh-key` crate (pure Rust) | Pre-2025 | No `ssh-keygen` runtime dep; deterministic; round-trips ssh-keygen format. |
| Native russh+gRPC over port 16634 (the CS-V2-01 path) | Subprocess `gh --stdio` for v1 | Phase 7 scoping decision (this phase) | Eliminates the gnarliest protocol work; tradeoff is `gh` runtime dep. |

**Deprecated/outdated:**
- `thrussh` — predecessor of russh, unmaintained. **Do not use.**
- `ssh2` (libssh2 binding) — C dep, sync, less actively maintained. **Do not use.**
- `openssh` crate wrapping `ssh` binary — works for plain TCP SSH, can't handle the Codespaces relay tunnel. **Do not use.**

## Open Questions

1. **Does russh 0.60's `connect_stream` API name and signature exactly match what we wrote in the sketch?**
   - What we know: WebFetch of docs.rs/russh confirms `connect_stream(config, stream, handler)` where stream is `AsyncRead + AsyncWrite + Unpin + Send`. Version is 0.60.3 per the docs page.
   - What's unclear: Exact `Handle<H>` generic parameters; whether `client::Msg` is the right channel-type marker; whether `channel.into_stream()` exists or we use `channel.data()` / `channel.wait()` directly.
   - Recommendation: **Wave 0 spike** — write a 50-line `cargo build` smoke that calls each API call we plan to use, against a non-Codespace SSH server (`localhost` + native sshd). Catches API drift before plan-writing.

2. **Does the existing Phase 6 OAuth token have `write:public_key` scope?**
   - What we know: `device_flow.rs:134-135` requests `codespace` + `read:user`. Per Phase 6 SUMMARY notes, no other scopes were added.
   - What's unclear: Whether `gh`'s default device-flow client_id grants `write:public_key` implicitly (gh's default scope set is broader than ours).
   - Recommendation: Add `write:public_key` to the Phase 7 OAuth scope set; surface a "Vector needs to update its GitHub permissions" toast on first Phase 7 use that triggers a re-auth via the existing Phase 6 device-flow modal.

3. **Where does the host-key fingerprint come from?**
   - What we know: The `Codespace` API response includes connection details when fetched via the singular `GET /user/codespaces/{name}` (NOT the list endpoint). `cli/cli`'s `internal/codespaces/api/api.go` reads `connection.tunnel_properties.host_public_keys[]` (this is the dev-tunnels relay's host key — verified via Section 7 of STACK.md).
   - What's unclear: Whether the singular endpoint returns this same field for non-VS-Code-flow clients, and whether `octocrab`'s `Codespace` type exposes it (it doesn't in our current `model.rs`; we'd need a custom deserialize or a `reqwest` direct fetch).
   - Recommendation: **Wave 0 second spike** — `curl -H "Authorization: bearer $GH_TOKEN" https://api.github.com/user/codespaces/{name}` against a real Available codespace and inspect the JSON. Map the exact JSON path to a Rust field. Document in `model.rs`.

4. **What hostname / username does russh authenticate as?**
   - What we know: For `gh --stdio`, the username is typically `codespace` (the standard Codespaces-container user is `vscode` or `codespace`; verify which by inspecting `gh`'s automatic ssh config output).
   - What's unclear: Without checking a live connection, the exact username isn't certain.
   - Recommendation: Wave-0 spike step — run `gh codespace ssh --config` against a live codespace, read the generated `Host` block; `User` field is the answer.

5. **How does the Phase 6 codespaces_modal's "Connect" key dispatch (currently `app.rs:1888`) map to our actor?**
   - What we know: `app.rs:486` is the entry point; line 1888 dispatches it from a key handler.
   - What's unclear: Nothing — this is a straightforward refactor: replace the placeholder toast body with a `codespace_actor::spawn_connect(...)` call.
   - Recommendation: One-line plan task. Not a research gap.

## Environment Availability

| Dependency | Required By | Available (on dev machine) | Version | Fallback |
|------------|------------|---------------------------|---------|----------|
| `gh` CLI | CS-04 subprocess transport | ✓ | 2.92.0 (2026-04-28) | None — hard dep. v1 documents `gh` as a prerequisite in README. |
| `ssh-keygen` (system binary) | None — we use `ssh-key` crate instead | ✓ | (Apple OpenSSH 10.2p1) | n/a (not needed at runtime) |
| `ssh` (system binary) | None — we use `russh` instead | ✓ | OpenSSH_10.2p1 LibreSSL 3.3.6 | n/a (not needed at runtime) |
| Live GitHub Codespace in Available state | Manual smoke matrix (CS-04..07 end-to-end) | (developer-dependent) | n/a | Mock SSH server (e.g. russh's own example server) for unit tests; live codespace only for the manual smoke matrix gate. |
| Network reachability to `*.app.github.dev` + `api.github.com` | All runtime ops | (dev-dependent) | n/a | None — these are GitHub-controlled endpoints, required for the feature to work at all. |

**Missing dependencies with no fallback:**
- None on the researcher's dev machine. **Plan authors must verify on the user's target macOS dev machine that `gh` is installed; surface an install link in the first-run toast if not.**

**Missing dependencies with fallback:**
- None.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (standard libtest) + `wiremock` for HTTP mocks (already in workspace dev-deps) |
| Config file | None — workspace-level `[lints]` in `Cargo.toml` + `cargo test --workspace` |
| Quick run command | `cargo test -p vector-ssh -p vector-codespaces -p vector-mux --tests --no-fail-fast` |
| Full suite command | `cargo test --workspace --tests` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --all -- --check` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CS-04 | russh client connects over `ChildStdioStream` and reaches authed state | integration (Wave 0 spike against localhost sshd OR mock russh server) | `cargo test -p vector-ssh --test connect_stdio_stream` | ❌ Wave 0 |
| CS-04 | `gh codespace ssh --stdio` subprocess spawns cleanly with expected argv | unit (assert argv shape; mock the actual gh by substituting `/bin/cat` in tests) | `cargo test -p vector-ssh --test gh_subprocess_argv` | ❌ Wave 0 |
| CS-04 | End-to-end: click Connect → pane opens → `pwd` returns codespace cwd | **manual smoke** (live codespace required) | smoke matrix item — manual UAT | ❌ Wave 0 (smoke checklist file) |
| CS-05 | ssh-key crate generates a valid OpenSSH ed25519 keypair; pub key round-trips | unit | `cargo test -p vector-codespaces --test ssh_keys` | ❌ Wave 0 |
| CS-05 | `register_ssh_key` POSTs `/user/keys` correctly (mocked via wiremock); handles 422 dedup | integration | `cargo test -p vector-codespaces --test register_ssh_key` | ❌ Wave 0 |
| CS-05 | KeyManager creates `~/.ssh/vector_codespace_ed25519` with 0600 perms; reuses on second call | integration | `cargo test -p vector-codespaces --test key_manager_lifecycle` | ❌ Wave 0 |
| CS-06 | Tab title appends `[remote]` for Codespace TransportKind | unit | `cargo test -p vector-mux --test format_tab_title_remote` | ❌ Wave 0 |
| CS-06 | TintStripePipeline color uniform is set when active pane's transport is Codespace | unit | `cargo test -p vector-app --test tint_for_remote_pane` | ❌ Wave 0 |
| CS-07 | `SshChannelTransport::resize` enqueues (rows, cols) without panic | unit | `cargo test -p vector-ssh --test resize_enqueue` | ❌ Wave 0 |
| CS-07 | Channel task drains resize queue and calls `channel.window_change` | unit (mock russh Channel via trait abstraction OR integration against localhost sshd) | `cargo test -p vector-ssh --test window_change_dispatch` | ❌ Wave 0 |
| CS-07 | End-to-end resize: remote `tput cols` matches local pane cols after resize | **manual smoke** | smoke matrix item | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p vector-ssh -p vector-codespaces -p vector-mux --tests` (target ≤ 30 seconds)
- **Per wave merge:** `cargo test --workspace --tests && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all -- --check`
- **Phase gate:** Full suite green + manual smoke matrix (4-6 items) user-approved before `/gsd:verify-phase`.

### Wave 0 Gaps

- [ ] `crates/vector-ssh/Cargo.toml` — add russh + ssh-key + async-trait + tokio + thiserror + tracing + zeroize deps
- [ ] `crates/vector-ssh/src/{lib,client,transport,stdio_stream,handler,error}.rs` — all six modules new
- [ ] `crates/vector-ssh/tests/{connect_stdio_stream,gh_subprocess_argv,resize_enqueue,window_change_dispatch}.rs` — 4 test files
- [ ] `crates/vector-codespaces/src/ssh_keys.rs` — new KeyManager module
- [ ] `crates/vector-codespaces/src/auth/device_flow.rs` — add `write:public_key` scope (line 134 area)
- [ ] `crates/vector-codespaces/src/client/mod.rs` — add `register_ssh_key`, `get_codespace_with_connection`, dedupe-by-title flow
- [ ] `crates/vector-codespaces/tests/{ssh_keys,register_ssh_key,key_manager_lifecycle}.rs` — 3 test files
- [ ] `crates/vector-mux/src/codespace_domain.rs` — replace stub body; new fields (codespace_name, host_fingerprint, key_path, gh_token_handle, http_client_for_lazy_lookup_or_pass_at_construction)
- [ ] `crates/vector-mux/src/mux.rs` — add `create_tab_async_with_domain` helper
- [ ] `crates/vector-mux/src/pane.rs::format_tab_title` — extend signature to take `TransportKind`
- [ ] `crates/vector-app/src/codespace_actor.rs` — new module mirroring `codespaces_actor.rs` pattern
- [ ] `crates/vector-app/src/app.rs::codespaces_connect_selected` — replace placeholder body with actor dispatch
- [ ] `crates/vector-app/src/app.rs` UserEvent — add `CodespacePaneReady { mux_window_id, pane_id, transport, term }` so the main thread installs the spawned pane synchronously
- [ ] Workspace `Cargo.toml` — add `russh = "0.60"` + `ssh-key = "0.6"` to `[workspace.dependencies]`
- [ ] Manual smoke matrix file — `.planning/phases/07-ssh-transport-codespaces-connect/SMOKE.md` with 4-6 items (Connect → shell, pwd correct; resize → tput cols matches; vim reflows; remote tab visually tinted + [remote] badge; close pane → no gh zombie; second-connect skips re-registration).

## Sources

### Primary (HIGH confidence)

- `/Users/ashutosh/personal/vector/crates/vector-mux/src/{domain,transport,local_domain,codespace_domain,mux,pane}.rs` — locked Phase-2/4 trait shape (D-38) and the exact integration seam Phase 7 fills.
- `/Users/ashutosh/personal/vector/crates/vector-codespaces/src/auth/device_flow.rs` — Phase 6 OAuth + direct-reqwest pattern; reused for `/user/keys` registration.
- `/Users/ashutosh/personal/vector/crates/vector-app/src/{codespaces_actor,pty_actor,app}.rs` — actor pattern, EventLoopProxy ↔ main-thread handoff, codespaces_connect_selected stub.
- `/Users/ashutosh/personal/vector/Cargo.toml` (workspace) — confirms russh + ssh-key not yet declared; reqwest 0.12; tokio 1.52.3 features.
- [`gh codespace ssh` source — `pkg/cmd/codespace/ssh.go`](https://github.com/cli/cli/blob/trunk/pkg/cmd/codespace/ssh.go) — confirmed `--stdio` proxies a port-forwarded sshd to stdin/stdout as raw SSH protocol.
- [`russh::client` docs](https://docs.rs/russh/latest/russh/client/index.html) — `connect_stream` accepts `AsyncRead + AsyncWrite + Unpin + Send` stream; version 0.60.3.
- [GitHub Docs — REST endpoints for Git SSH keys](https://docs.github.com/en/rest/users/keys) — `POST /user/keys` requires `write:public_key` scope; ed25519 keys supported.
- [GitHub Docs — REST endpoints for Codespaces](https://docs.github.com/en/rest/codespaces/codespaces) — `GET /user/codespaces/{name}` returns connection details including host-key fingerprint via the tunnel-properties field.
- Local toolchain probe: `which gh` → `/opt/homebrew/bin/gh`; `gh --version` → 2.92.0 (2026-04-28).

### Secondary (MEDIUM confidence)

- [Hacksore gist + community examples](https://duckduckgo.com/?q=ProxyCommand+gh+cs+ssh+--stdio+-i) — confirms the canonical ProxyCommand form `gh cs ssh -c {name} --stdio -- -i {keypath}`. (WebSearch verified, not Context7.)
- [Eugeny/russh examples directory](https://github.com/Eugeny/russh/tree/main/russh/examples) — `client_exec_interactive.rs` shows `channel_open_session` → `request_pty` → `request_shell` pattern.
- [cli/cli issue #11206](https://github.com/cli/cli/issues/11206) — confirms port 16634 is the internal gRPC port used by the relay (relevant for the v1.x CS-V2-01 path, not v1).

### Tertiary (LOW confidence — needs validation at Wave 0)

- Exact `russh::Channel::window_change(...)` argument order and async-vs-sync nature (sketched here; verify at Wave 0 spike).
- Whether `gh` honors `GH_TOKEN` env override when `--stdio` is set (highly likely per `gh`'s general design; verify empirically).
- The exact JSON field path for the codespace host-key fingerprint in `GET /user/codespaces/{name}` (Open Question 3).

## Metadata

**Confidence breakdown:**

- Standard stack (russh, ssh-key, gh subprocess): HIGH — sourced from STACK.md + crates.io + verified `gh` binary on dev machine.
- Architecture (PtyTransport reuse, codespace_actor, mux helper): HIGH — read existing code; Phase 7 is glue, not new abstractions.
- Pitfalls: HIGH on host-key / zombie / scope / window-change priorities; MEDIUM on `gh --stdio` minimum version (need changelog inspection).
- Code examples: MEDIUM — three russh API signatures sketched, verify at Wave 0 spike before plan-writing.

**Research date:** 2026-05-19
**Valid until:** 2026-06-18 (30 days; revisit if `gh` major version bumps, or russh 0.61 ships).
