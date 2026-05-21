# Phase 8: VS Code Remote Tunnels Connect — Research

**Researched:** 2026-05-19
**Domain:** Microsoft Dev Tunnels relay + VS Code CLI tunnel host protocol; SSH/russh client over a custom relay transport; GitHub OAuth auth to the tunnels service.
**Confidence:** HIGH on facts; HIGH on day-1 spike recommendation.

## Summary

> **DECISION LOCKED 2026-05-20:** Path 2 Variant 2c — Vector Tunnel Agent. Vector ships a user-space `vector-tunnel-agent` binary that the user installs on each remote box (same install model as `code tunnel`). The agent uses `microsoft/dev-tunnels rs/` as a Host, exposes a PTY via a Vector-controlled framed protocol on the relay channel. No sshd dependency, no VPN dependency, no vscode-remote protocol. ~3-4 calendar weeks of work. See `## Day-1 Spike Decision Recommendation` for the full breakdown.

## Research Findings Summary (pre-decision)

The phase's center of gravity is **DT-01 — the day-1 spike decision**. Research reveals a fact that re-frames the spike entirely:

**`code tunnel` on the remote does NOT expose an SSH shell endpoint.** When a user runs `code tunnel` on EC2/home/laptop, the process registers a Dev Tunnel and serves a **custom MessagePack-RPC protocol** on `CONTROL_PORT` inside that tunnel. The RPC has handlers like `handle_serve` (start VS Code Server), `handle_spawn` (piped stdio, **no PTY**), `handle_forward`, `handle_fs_read/write`, plus a challenge-response auth layer. There is **no `handle_spawn_pty`, no `handle_shell`, no `handle_open_terminal`** in the public source. The "SSH" Microsoft references in tunnel docs is the relay's *transport* layer (russh between client/host and the relay), not an SSH shell the user can attach to.

Concretely:
- The `code` CLI exposes `code tunnel`, `code tunnel kill/restart/status/rename/prune`, `code tunnel user {login,logout,show}`, `code tunnel service {install,uninstall}`. There is **no `code tunnel ssh`, no `code tunnel client`, no `code tunnel exec` subcommand**.
- The standalone `devtunnel` CLI has `devtunnel connect TUNNELID` — but that forwards **TCP ports**, it does not give you a shell. It exists for the "share my localhost:3000 with a teammate" use case.
- The Dev Tunnels Rust SDK (`microsoft/dev-tunnels/rs/`) is **actively maintained** (May 4 2026 added a "relay client", May 6 2026 "Use new local dnsname"), now has ✅ Host Connections (the README matrix is current as of 2026-05). It connects via WebSocket → russh, verifies host keys against `host_public_keys` returned by the tunnel API, and exposes `connect_to_port(port)` returning a russh channel. It still lacks ❌ Reconnection / SSH-level reconnect / Auto token refresh / SSH keep-alive — same gaps Vector's existing notes call out.
- The Rust SDK pins **russh 0.37.1 from crates.io** (verified in `rs/Cargo.lock` on `main` as of 2026-05-19). VS Code's own CLI patches russh-family crates to `microsoft/vscode-russh` (also 0.37.1). Our workspace uses **russh 0.60** (Phase 7 scaffolding). The conflict is real.

**Primary recommendation (revised 2026-05-20): SPIKE OUTCOME = (b) vendor SDK — Path 2 Variant 2a.** Continuation research (`## Continuation Research (2026-05-20)`) overturns the first-round (c) recommendation. The key fact the first round missed: the SDK's `connect_to_port(port)` returns a `PortConnection` whose `into_rw()` is an `AsyncRead+AsyncWrite` stream plug-compatible with `russh::client::connect_stream`. Combined with the `code tunnel` host's public `handle_forward(port, public)` RPC (verified in `port_forwarder.rs`), Vector can ask the user's `code tunnel`-hosted machine to expose port 22, then run vanilla SSH through it using Phase 7's existing `SshClient`/`SshChannelTransport` — no VS Code protocol reimplementation needed. Path 1 (vscode-remote protocol client) is rejected: greenfield Rust work, monthly version-mismatch breakage, 7–10 dev-weeks vs Path 2's 4–6.

The detailed recommendation, with invalidators, is in `## Day-1 Spike Decision Recommendation` below (revised section header). The first-round logic and the (c) defer fallback remain documented in the body in case Wave 2's msgpack-RPC mini-client creeps past its ~400 LOC budget.

## User Constraints (from CONTEXT.md)

CONTEXT.md does not exist for Phase 8 yet (researcher runs before `/gsd:discuss-phase`). Constraints below are derived from REQUIREMENTS.md (DT-01..04) + ROADMAP.md §Phase 8 + PROJECT.md "Out of Scope".

### Locked Decisions (from PROJECT.md + ROADMAP.md)
- VS Code Remote Tunnels only (not Codespaces). Pivot recorded 2026-05-19.
- The user runs `code tunnel` on their own remote machine; Vector attaches.
- Day-1 spike is mandatory before any integration code is written (DT-01).
- Defer-to-v2 is an acceptable spike outcome (DT-04 success criterion #5).
- SSH host-key trust uses the tunnel's API-provided fingerprint, not TOFU bypass.
- v1 transport must visually mark non-local panes (`[remote]` badge + tinted tab).

### Claude's Discretion
- Recommend among (a) subprocess `code tunnel client`, (b) vendor `microsoft/dev-tunnels/rs/`, (c) defer to v2. Invalidators listed.
- Decision-document format and contents (subject to user review).
- Whether to re-scope Phase 8 to a smaller "Vector Tunnel Agent" feature if spike picks (c) but user still wants tunnels in v1.

### Deferred Ideas (OUT OF SCOPE — from PROJECT.md + ROADMAP.md §Phase 8 Risks)
- Port-forwarding "PORTS" panel UX (v2 RDEV-V2-01).
- File transfer / scp UI (v2 RDEV-V2-02).
- Clean-room reverse engineering of the relay protocol or `code tunnel` RPC.
- Arbitrary SSH targets as first-class profiles (v2 RDEV-V2-04).
- Codespaces lifecycle (descoped entirely 2026-05-19).

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DT-01 | 1-2 day spike commits a written decision among (a)/(b)/(c) before any integration code | This whole document — especially §Day-1 Spike Decision Recommendation. Spike output is `.planning/research/spikes/dev-tunnels-decision.md`. |
| DT-02 | Signed-in user can list active Dev Tunnels in the picker | §"Dev Tunnels Management REST API" — `GET /api/v1/tunnels` with `Authorization: github <gho_token>` header works directly; no token exchange. Plus §"`code tunnel` Tunnel Tagging" — how to filter the list to *machines running `code tunnel`* (label-based). |
| DT-03 | Connecting to a Dev Tunnel opens a remote shell in a Vector pane | §"The Killer Finding: `code tunnel` Host Has No PTY Endpoint" — DT-03 is **unattainable against the `code tunnel` protocol as it exists today** without a clean-room RPC reimplementation (out-of-scope). DT-03 attainable only via (c) defer or via the alternative Vector-Tunnel-Agent path (§"Alternative Architecture: Vector Tunnel Agent"). |
| DT-04 | Dev Tunnel sessions are visually distinct (tinted tab + `[remote]` badge) | Already shipped by Phase 7: `TransportKind::DevTunnel` + `format_tab_title` + tint pipeline. Zero new research needed. Trivially satisfied if DT-03 ships at all. |

## Standard Stack

If the spike picks (a) subprocess: **no new Rust dependencies** — only requires `code` CLI present on the user's Mac. Existing tokio + reqwest + octocrab cover REST + spawn.

If the spike picks (b) vendor SDK: **the SDK is its own dep, plus the russh 0.37 patch decision.** Versions verified on crates.io and the Microsoft repo.

### Core (only if spike picks "vendor SDK")
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tunnels` (microsoft/dev-tunnels rs/) | git, pinned SHA `64048c1409ff56cb958b879de7ea069ec71edc8b` (or newer) | Dev Tunnels Management API + Relay Client (Tunnel Host too, as of 2026-05) | VS Code CLI itself uses this exact SHA. Active maintenance (commits within last 2 weeks). Not on crates.io — must vendor as git dep. |
| `russh` | **0.37.1** (forced by `tunnels` git dep) — OR — `0.60.2` (workspace) with `[patch.crates-io] russh = { git = "https://github.com/microsoft/vscode-russh", branch = "main" }` | Async pure-Rust SSH client | The `tunnels` SDK opens a russh client *inside* the relay WebSocket. Two compatibility paths exist; see §"russh 0.37 vs 0.60: Three Resolutions." |
| `tokio-tungstenite` | `0.29.x` (transitive via `tunnels`) | WebSocket transport to relay | Required by `tunnels`. Same version VS Code CLI uses. |
| Already in tree: `reqwest 0.12`, `tokio 1.52`, `octocrab 0.50`, `oauth2 5.0`, `ssh-key 0.6`, `keyring-core 1.0` | (verified by `cat workspace Cargo.toml`) | REST, async runtime, GitHub API, OAuth, key handling, Keychain | These cover all auth + Dev Tunnels Management REST surface. |

### Supporting (regardless of spike outcome)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `reqwest 0.12.x` (workspace pin) | already pinned | Direct calls to `https://global.rel.tunnels.api.visualstudio.com/api/v1/tunnels` | Use for tunnel listing if (a) subprocess can't surface the list cheaply, or to layer a fallback. |
| `serde / serde_json` | already pinned | Tunnel record deserialization | The list endpoint returns JSON. |
| `tracing` | already pinned | Diagnostics for tunnel connect failures | Critical — connect failures will be the #1 user-visible error in this phase. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Vendor `microsoft/dev-tunnels` rs/ git dep | Hand-roll Management API via `reqwest` | Saves the russh 0.37 conflict, but loses the WebSocket+russh relay transport — and the relay transport is the only documented path to actually *connect into* a tunnel. Hand-rolling the relay framing is clean-room RE, which is out-of-scope. |
| Subprocess `code tunnel client` | Subprocess `devtunnel connect` | `code tunnel client` does not exist as a command. `devtunnel connect` does — but it forwards ports, not shells. Neither path gives a PTY shell. |
| Workspace patch to vscode-russh | Accept dual russh 0.37 + 0.60 versions | Patch is cleaner (single russh in dep graph, ~3MB binary savings) but couples our build to a Microsoft fork's stability. vscode-russh has 16 stars, 0 releases, 5 forks — it's a fork-of-convenience, not a maintained library. Risk: if Microsoft archives or breaks it, we have to ship our own fork. |

**Installation** (only if spike picks (b)):
```toml
# Workspace Cargo.toml additions
[workspace.dependencies]
tunnels = { git = "https://github.com/microsoft/dev-tunnels", rev = "64048c1409ff56cb958b879de7ea069ec71edc8b", features = ["connections"] }

# EITHER accept dual russh:
# (no change — workspace russh = "0.60" stays; tunnels brings in 0.37 transitively)

# OR patch to vscode-russh:
# [patch.crates-io]
# russh = { git = "https://github.com/microsoft/vscode-russh", branch = "main" }
# russh-keys = { git = "https://github.com/microsoft/vscode-russh", branch = "main" }
# russh-cryptovec = { git = "https://github.com/microsoft/vscode-russh", branch = "main" }
```

**Version verification (2026-05-19):**
- `microsoft/dev-tunnels` rs/Cargo.toml — `russh = "0.37.1"`, `tokio = "1.20"`, `reqwest = "0.13"`, package `tunnels 0.1.0` (unchanged from initial discovery).
- Most recent commit to `rs/`: 2026-05-06 ("Use the new local dnsname (#630)"). Previous: 2026-05-04 ("rust: implement relay client (#626)") — significant new code.
- `microsoft/vscode/cli/Cargo.toml` — pins `tunnels` at `rev = "64048c1409ff56cb958b879de7ea069ec71edc8b"`, patches russh to vscode-russh `main`, tokio 1.52.
- `microsoft/vscode-russh` — version 0.37.1, default features `flate2` + `rs-crypto`, 284 commits on main, 16 stars.

## Architecture Patterns

### How a `code tunnel` host actually works (verified from source)

```
[user's remote box]
    │
    │ runs: code tunnel
    │
    ├─→ Registers a Dev Tunnel via Management API
    │   (POST https://global.rel.tunnels.api.visualstudio.com/api/v1/tunnels)
    │   with labels { "tunnel-type": "vscode-remote" } and host_public_keys.
    │
    ├─→ Opens a WebSocket "tunnel-relay-host" to relay
    │   (wss://<cluster>-data.rel.tunnels.api.visualstudio.com/...).
    │
    ├─→ Runs a russh **server** on top of that WebSocket
    │   (auth = "none" + token; channels carry the RPC stream).
    │
    └─→ On accepted channel:
        ├─→ Receives msgpack-RPC requests:
        │   handle_serve         → start VS Code Server (returns port for HTTP/WS access)
        │   handle_spawn         → spawn process with PIPED stdio (no PTY field)
        │   handle_spawn_cli     → spawn the `code` CLI itself
        │   handle_forward       → set up port forwarding
        │   handle_fs_*          → file operations
        │   handle_call_server_http → proxy HTTP to the local VS Code Server
        │   handle_challenge_*   → auth challenge-response
        │
        └─→ (NO handle_shell, NO handle_open_terminal, NO PTY in any RPC.)
```

A VS Code *client* (the actual VS Code IDE) connects by:
1. Listing tunnels via Management API.
2. Opening a "tunnel-relay-client" WebSocket to the same relay endpoint.
3. Running a russh **client** over that WebSocket.
4. Opening a russh channel.
5. Sending msgpack-RPC `handle_serve` to spawn the VS Code Server.
6. From there, all editor features (including the integrated terminal) go through **the VS Code Server's own protocol** (a different, larger, also-undocumented surface — `vscode-remote` IPC over the server's HTTP/WS port that `handle_serve` returned).

**There is no public, documented path for a non-VS-Code client to get a remote PTY shell from a `code tunnel` host.** The "open the app, pick a tunnel, get a shell" UX collides head-on with the protocol's actual surface.

### Alternative Architecture: Vector Tunnel Agent (the path that *could* work)

If the user is willing to install a small extra binary (`vector-tunnel-agent`) on their remote box alongside `code tunnel`, Vector could:

```
[user's remote box]
    │
    ├─→ runs: code tunnel           (their existing flow — unchanged)
    │
    └─→ runs: vector-tunnel-agent   (Vector's own binary)
            │
            └─→ Uses dev-tunnels rs/ SDK as a Host:
                ├─→ Creates a separate Dev Tunnel (labeled "vector-shell")
                ├─→ OR attaches as an additional port on the user's `code tunnel`
                ├─→ Listens for incoming relay-host SSH channels
                └─→ On channel open: pty-allocate, spawn $SHELL, biased select.

[Vector on Mac]
    │
    ├─→ Lists user's tunnels via Management API, filters label="vector-shell"
    ├─→ Opens "tunnel-relay-client" WebSocket
    ├─→ russh client connects, opens channel, request_pty + request_shell
    └─→ vector-ssh's existing SshChannelTransport handles the rest.
```

This is achievable with the SDK + existing Phase 7 scaffolding (`SshClient::connect_over` + `SshChannelTransport`), but it is **a different feature** than "attach to `code tunnel`". It requires:
- A new binary crate (`vector-tunnel-agent`) — additional CI/distribution surface.
- User must install + run it (one extra command on the remote — comparable to `code tunnel service install`).
- It piggybacks on Dev Tunnels for NAT traversal + relay + auth; everything else (PTY, shell mgmt) is our code.

**This is the v2 RDEV path the deferred-features list already gestures at.** Recommending it for v1 is a deliberate scope expansion — outside this phase's spike charter.

### Recommended Project Structure (if any code is written for Phase 8)

```
crates/
├── vector-devtunnels/        # NEW (only if spike picks (a) or (b))
│   ├── src/
│   │   ├── lib.rs            # public API: list_tunnels, connect_tunnel
│   │   ├── api.rs            # Management REST (reqwest)
│   │   ├── auth.rs           # GitHub token → tunnel API header formatting
│   │   └── transport.rs      # subprocess wrapper OR SDK adapter (spike-dependent)
│   └── tests/
│       └── list_tunnels.rs   # wiremock-backed REST tests
└── vector-codespaces/        # existing — rename to vector-github-auth?
    └── ...
```

If spike picks (c) defer: no new crate. Document and stop.

### Architecture Patterns from Existing Vector Code (reuse as-is)

| Pattern | Where | Reuse for Phase 8 |
|---------|-------|-------------------|
| GitHub OAuth Device Flow | `vector-codespaces/src/auth/device_flow.rs` | Reuse verbatim. Add Microsoft account login as v2; v1 = GitHub-only per PROJECT.md. |
| Keychain-backed token storage | `vector-secrets` + `TokenStore` | Reuse verbatim. Same `GITHUB_REFRESH_ACCOUNT` constant. |
| `Box<dyn PtyTransport>` install via Mux | `Mux::create_tab_async_with_transport` (mux.rs:436) | This is the seam; whatever transport Phase 8 produces, it goes through here. |
| `[remote]` tab badge | `format_tab_title` + `TransportKind::DevTunnel` | Already wired. Zero work. |
| Tint pipeline | `TintStripePipeline` from Phase 5 | Already wired for `Kind::DevTunnel` profile rows. |
| 401 silent-refresh chain | `CodespacesClient` (`client/mod.rs`) | Reuse the pattern verbatim if we ship REST list. |
| SSH channel transport over relay stream | `vector-ssh::SshClient::connect_over` + `SshChannelTransport` | **This is exactly the surface the dev-tunnels rs/ SDK's `connect_to_port` returns.** If spike picks (b), the existing scaffolding plugs in directly — modulo russh version conflict. |
| `ChildStdioStream` (AsyncRead+Write over subprocess) | `vector-ssh::stdio_stream.rs` | **Not useful for Phase 8.** There is no subprocess analogous to `gh codespace ssh --stdio`. `devtunnel connect` and `code tunnel` neither expose a stdio-SSH mode. |

### Anti-Patterns to Avoid
- **Hand-rolling the relay protocol.** Out of scope; not documented; will break on Microsoft's next ship.
- **Reimplementing VS Code's msgpack-RPC.** Clean-room RE territory. Out of scope per Phase 8 risks.
- **Listing tunnels with no label filter.** The user's tunnel list may include test/personal/scratch tunnels with no shell. Must filter to tunnels that have a `code tunnel`-style label OR (under the v2 agent path) a `vector-shell` label.
- **TOFU host-key acceptance.** Phase 7 already established the discipline; the dev-tunnels SDK's `host_public_keys` list is the API-attested fingerprint set. Use it directly.
- **Shipping (a) subprocess without verifying `code tunnel client` exists.** It doesn't. See `## Subprocess Path Reality Check`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| WebSocket connection to Dev Tunnels relay | Your own WS client + framing | `tunnels::connections::RelayTunnelClient::connect` (the SDK) | Pings, reconnect framing, cluster-id resolution, `HTTPS_PROXY` env handling all already there. |
| Dev Tunnels REST authentication | A `bearer` header guess | `Authorization: github <gho_token>` (the SDK's `Authorization::Bearer` with `as_header()` returning the `github` prefix — verified in `authorization.rs`) | The tunnel API has a custom 4-prefix scheme: `aad`, `github`, `tunnel`, `bearer`. Using the wrong one fails silently or with cryptic errors. |
| SSH host-key trust for tunnel connections | TOFU + pin-on-first-use | The `host_public_keys` array returned in the `TunnelEndpoint` JSON | These are server-attested keys signed by the tunnel service. We have an API-supplied trust anchor; use it. Phase 7's `VectorHandler` SHA-256 path applies cleanly once we feed it the right key. |
| Tunnel access token refresh | Manual 24h timer + ad-hoc refresh | (Currently no SDK support — known gap in matrix) | If we go path (b) we either (i) recreate tokens via Management API every ~50min via `octocrab`/REST or (ii) accept that long-running tunnels will need re-auth. **Critical: this is a deal-breaker for "transparent reconnect" (PERSIST-02 in Phase 9).** |
| MessagePack-RPC client for `code tunnel` | Custom msgpack codec + handler stubs | **Don't** — out of scope. Recommend (c) defer. | Reverse-engineering moving target. |
| Dev Tunnels CI install on the user's box | Bundling `devtunnel` or `code` | Document the prerequisite; detect at runtime via `which code` | Vector is a terminal, not a tunnel installer. |
| OAuth Device Flow | A custom polling loop | `oauth2 5.0` (already in tree via Phase 6) | Done. |
| Keychain token storage | A file write | `keyring-core 1.0` + `apple-native-keyring-store 1.0` (already in tree) | Done. |

**Key insight:** Almost every component the SDK provides has a known-good pattern in Vector already (russh client → channel → PTY transport via Phase 7). The single non-trivial novel piece is the **WebSocket-relay-to-russh-stream adapter**, which is what the SDK's `RelayTunnelClient::connect` returns. If we want this stack, we vendor the SDK; we do not roll our own relay client.

## Runtime State Inventory

Phase 8 is a greenfield/integration phase, not a rename/refactor. **No runtime state inventory required.** All Phase 7 reverts already landed (codespace_actor, KeyManager, etc. removed); no stale Keychain entries because Phase 7 didn't store any DT-specific secrets.

The only Keychain entry of concern is the GitHub OAuth refresh token at `service="vector", account="github_oauth_token"`. Reused as-is for tunnel API auth (the `github <token>` header). Zero migration.

## Common Pitfalls

### Pitfall 1: Confusing `code tunnel` and `devtunnel` CLIs
**What goes wrong:** Plans assume there's a single CLI named `code tunnel ssh` or similar.
**Why it happens:** Microsoft uses "Dev Tunnels" branding for *both* the service AND the standalone `devtunnel` CLI. Then VS Code embeds its own `code tunnel` subcommand which uses the same service but with different commands and a different host-side protocol.
**How to avoid:**
- `devtunnel CLI`: general-purpose, ports-forwarding-focused, `devtunnel connect` forwards a tunnel's ports to localhost. **No shell access.**
- `code tunnel` (subcommand of `code`): VS-Code-specific. Host-side runs the msgpack-RPC. **Also no public shell-channel.**
- They share auth (GitHub or Microsoft) and the same Dev Tunnels backend service.
**Warning signs:** Plans reference `code tunnel client` (does not exist) or assume `devtunnel connect` provides a shell.

### Pitfall 2: Assuming `handle_spawn` Gives You a PTY
**What goes wrong:** A plan based on the `code tunnel` host protocol assumes the `handle_spawn` RPC takes a `pty: true` flag and returns a PTY-attached process.
**Why it happens:** "Spawn" naming is misleading. The struct (`SpawnParams { command, args, cwd, env }`) is verified — no PTY field.
**How to avoid:** Verify against `vscode/cli/src/tunnels/protocol.rs` if you're tempted to design against this RPC. The integrated terminal in VS Code Remote Tunnels uses the VS Code Server's *own* protocol layer on top of the tunnel-relayed HTTP port that `handle_serve` allocates — not `handle_spawn`.
**Warning signs:** Plan tasks like "send msgpack-RPC `spawn` with pty=true and stream stdin/stdout".

### Pitfall 3: russh 0.37 vs 0.60 Dual-Version Conflict
**What goes wrong:** Vendoring `tunnels` brings in russh 0.37; our workspace uses russh 0.60. Cargo accepts both, doubles the russh footprint (~3MB), and **types from the two versions are not interoperable** — a `russh::Channel<Msg>` from 0.60 (what our `SshChannelTransport` consumes) is a different type than the SDK's 0.37 channel.
**Why it happens:** SDK pins russh tightly; Microsoft patches it via `[patch.crates-io]` in VS Code's own CLI to point at `vscode-russh` (their fork, also 0.37.1). We can't use 0.60 inside the SDK's call chain without either bumping the SDK or replacing its russh.
**How to avoid (in priority order):**
1. **Workspace-level `[patch.crates-io]` to `microsoft/vscode-russh`.** Single russh in the dep graph (0.37.1 across the workspace). Our existing `vector-ssh` (which uses russh 0.60 API surface) must be **downgraded** to 0.37 API or kept independent. Cost: Phase 7's `vector-ssh` was written against 0.60; expect API breaks on `Handler` trait (0.37 used `#[async_trait]`, 0.60 uses AFIT), `PrivateKeyWithHashAlg` (0.60 only), etc.
2. **Accept dual versions.** Use the SDK's russh-0.37 channel internally; have `SshChannelTransport` (russh 0.60) operate only on local-PTY-via-stdio adapters. Cost: ~3MB binary, two russh code paths, no shared SSH config.
3. **Fork the SDK** and bump it to russh 0.60. Cost: ongoing maintenance burden; we own a fork of a moving Microsoft repo.
**Warning signs:** `cargo tree -p russh` reports two versions; `cargo build` succeeds but type-mismatch errors appear in the plumbing between the SDK's channel and our transport.

### Pitfall 4: Authenticating to the Tunnel API With the Wrong Prefix
**What goes wrong:** Plans use `Authorization: Bearer gho_...` against `global.rel.tunnels.api.visualstudio.com`. Some endpoints accept it via the `bearer` variant of the `Authorization` enum, but others require `github gho_...` explicitly. Hard-to-debug 401s or 403s.
**Why it happens:** The Dev Tunnels REST API has FOUR Authorization header schemes: `aad <token>`, `github <token>`, `tunnel <token>`, `bearer <token>`. Each routes to different validation logic.
**How to avoid:** Always use `Authorization: github <gho_xxxx>` when our token comes from GitHub OAuth. Verify against the SDK's `authorization.rs`. (No exchange step is needed — GitHub bearer tokens flow directly with the `github` prefix.)
**Warning signs:** Sporadic 401s; some endpoints work and others don't with the same token.

### Pitfall 5: Tunnel Access Token Expiration During a Long Session
**What goes wrong:** A tunnel session is open for 4 hours; at the 24-hour-from-acquisition mark (or sooner — varies by token type), the tunnel access token expires and the russh channel dies. No auto-recovery.
**Why it happens:** Microsoft docs: *"The tokens expire after some time (currently 24 hours). Tokens can only be refreshed using an actual user identity..."* The Rust SDK feature matrix has ❌ for "Auto Tunnel Access Token Refresh."
**How to avoid:** If we ship Phase 8 with the SDK, we must implement our own token-refresh task that calls Management API every ~12-23 hours to re-issue a connect-scope token via `devtunnel token` equivalent (`POST /api/v1/tunnels/{id}/access` with scope=connect). Then either reconnect the SSH channel or warn the user.
**Warning signs:** "Pane went dead after several hours of idle" reports. No clean error.

### Pitfall 6: Tunnel-Listing Filter Confusion
**What goes wrong:** The picker shows the user's ENTIRE tunnel list (10-max account-wide), including their port-forwarding tunnels for web app dev, anonymous-access tunnels, expired tunnels.
**Why it happens:** `GET /api/v1/tunnels` returns all tunnels owned by the user; `code tunnel`-created tunnels are tagged with labels but not differentiated from `devtunnel host -p 3000` tunnels.
**How to avoid:** Filter the result to tunnels whose labels include a known `code tunnel` marker. Inspect the labels VS Code CLI sets — the rust dev_tunnels.rs uses labels for "version" and "platform". Document the exact filter we use; expect to revisit if VS Code changes label conventions.
**Warning signs:** Picker shows "frontend-dev (port 3000)" alongside the user's actual machine; user confused.

### Pitfall 7: NAT/Firewall Assumptions
**What goes wrong:** Plans assume the user's remote box has direct inbound connectivity.
**Why it happens:** SSH habits.
**How to avoid:** Dev Tunnels is **explicitly outbound-only** from both client and host — the relay sits between them. Tunnel works behind NAT and corporate firewalls so long as outbound HTTPS to `*.rel.tunnels.api.visualstudio.com` is allowed. **This is the feature.** Document it.
**Warning signs:** Plan mentions "open port 22 on remote" or "configure router port forwarding".

### Pitfall 8: Multi-Machine Name Collisions
**What goes wrong:** User registers two machines as `code tunnel` with the same hostname (default tunnel name is the machine hostname); list shows two entries with the same label, no way to disambiguate.
**Why it happens:** `code tunnel rename` exists but users forget. Also: `code tunnel` cap is 10; CLI auto-deletes a random unused one when a user creates an 11th — names can disappear.
**How to avoid:** Picker shows tunnel **id** (short hash) + label + last-seen. Treat label as a display name only.
**Warning signs:** Plans assume tunnel names are unique.

### Pitfall 9: Mistaking the Relay's SSH for an SSH Endpoint
**What goes wrong:** Plan reads Microsoft's "SSH inside the tunnel" docs and concludes there's a regular SSH server we can `ssh user@tunnel-host` into.
**Why it happens:** The docs deliberately use "SSH" to reassure users about encryption ("AES-256-CTR"); the protocol layer above is *not* a shell-access SSH server.
**How to avoid:** Internalize: "SSH" in the Dev Tunnels context is the *transport* between relay+client and relay+host; it carries application-defined channels. Standard `ssh` won't connect; `request_shell` returns nothing useful.
**Warning signs:** Plan task: "shell out to `ssh -o ProxyCommand=...`".

### Pitfall 10: Subprocess Path Reality Check
**What goes wrong:** The spike option (a) — "subprocess `code tunnel client`" — was the cheapest path on paper. But `code tunnel client` is not a real subcommand. The CLI exposes `code tunnel`, `code tunnel kill/restart/status/rename/prune/unregister`, `code tunnel user {login,logout,show}`, `code tunnel service {install,uninstall}` — no client-side connect subcommand.
**Why it happens:** PROJECT.md and the Phase 8 description were written before research nailed down the actual command surface.
**How to avoid:** Treat option (a) as **"subprocess `devtunnel connect TUNNELID` to forward a port"** if you must keep it on the table. That gives you a port-forwarded localhost endpoint, not a shell. The path stops there without further code.
**Warning signs:** Plan references `code tunnel client --tunnel <name>`.

## Code Examples

### Listing the user's tunnels via REST (works today, GitHub token only)
```rust
// Verified against tunnels/rs/src/management/* + Dev Tunnels security docs.
// Endpoint base: https://global.rel.tunnels.api.visualstudio.com
// Auth header format: "github <gho_token>"  (NOT "Bearer <token>")
use reqwest::Client;
use serde::Deserialize;

#[derive(Deserialize)]
struct TunnelList { value: Vec<TunnelRecord> }

#[derive(Deserialize)]
struct TunnelRecord {
    #[serde(rename = "tunnelId")] tunnel_id: String,
    name: Option<String>,
    labels: Option<Vec<String>>,
    #[serde(rename = "endpoints")] endpoints: Option<Vec<TunnelEndpoint>>,
    // ...
}

#[derive(Deserialize)]
struct TunnelEndpoint {
    #[serde(rename = "hostId")] host_id: String,
    #[serde(rename = "clientRelayUri")] client_relay_uri: String,
    #[serde(rename = "hostPublicKeys")] host_public_keys: Vec<String>,
}

async fn list_tunnels(http: &Client, gh_token: &str) -> anyhow::Result<Vec<TunnelRecord>> {
    let resp = http
        .get("https://global.rel.tunnels.api.visualstudio.com/api/v1/tunnels")
        .header("Authorization", format!("github {gh_token}"))
        .header("User-Agent", "Vector/0.1")
        .send()
        .await?;
    if resp.status() == 401 || resp.status() == 403 {
        return Err(anyhow::anyhow!("tunnel API rejected token (status {})", resp.status()));
    }
    let body: TunnelList = resp.json().await?;
    Ok(body.value)
}
// Source: tunnels/rs/src/management/http_client.rs + security docs.
```

### Filtering to `code tunnel`-created hosts
```rust
// Heuristic — labels are not a stable public contract. Verify against
// vscode/cli/src/tunnels/dev_tunnels.rs current label additions.
fn is_code_tunnel(t: &TunnelRecord) -> bool {
    t.labels.as_ref()
        .map(|labels| labels.iter().any(|l| l.starts_with("vscode-tunnel-") || l == "vscode-server-launcher"))
        .unwrap_or(false)
}
```

### What the SDK adapter would look like (path (b) only)
```rust
// Skeleton; verified shape from rs/src/connections/relay_tunnel_client.rs.
// Note: this uses russh 0.37 types (from the SDK), NOT our workspace russh 0.60.
use tunnels::management::{TunnelManagementClient, ...};
use tunnels::connections::RelayTunnelClient;

async fn connect_devtunnel(mgmt: &TunnelManagementClient, tunnel_id: &str) -> anyhow::Result<russh_037::Channel<...>> {
    let tunnel = mgmt.get_tunnel(tunnel_id, ...).await?;
    let endpoint = tunnel.endpoints.into_iter().next()
        .ok_or_else(|| anyhow::anyhow!("no relay endpoint"))?;
    let access_token = mgmt.get_tunnel_access_token(&tunnel, "connect").await?;
    let client = RelayTunnelClient::connect(&endpoint, &access_token).await?;
    // From here the SDK exposes connect_to_port() — but `code tunnel` doesn't
    // expose a PTY port. You'd open a port (CONTROL_PORT) and start msgpack-
    // RPCing into handle_spawn — and there is no PTY in handle_spawn. Stop here.
    todo!("no shell path forward against code tunnel host RPC")
}
```

### The path that WOULD work (v2 Vector-Tunnel-Agent — included for completeness)
```rust
// On the user's remote box (separate binary, runs alongside `code tunnel`):
use tunnels::connections::RelayTunnelHost;
use russh_037::server::{Server, Handler};

// Host a tunnel with label "vector-shell"; on each incoming channel, spawn a real PTY.
// (~150 lines of standard russh-server-with-portable-pty code.)

// On the Mac, in Vector:
use tunnels::connections::RelayTunnelClient;
let client = RelayTunnelClient::connect(&endpoint, &access_token).await?;
let stream = client.connect_to_port(VECTOR_SHELL_PORT).await?;
// stream is AsyncRead+AsyncWrite — feed it to vector-ssh::SshClient::connect_over.
let ssh = vector_ssh::SshClient::connect_over(stream, "vector-shell", identity, host_fp).await?;
let chan = ssh.open_pty_shell("xterm-256color", rows, cols).await?;
let transport = vector_ssh::SshChannelTransport::spawn(chan, ssh.handle, None);
mux.create_tab_async_with_transport(window_id, Box::new(transport), rows, cols).await?;
```

**This last snippet is the only currently-feasible "remote shell over Dev Tunnel" architecture for Vector.** It requires the user to install one extra binary on each remote machine. It is **not** what DT-02/03/04 ask for ("attach to `code tunnel`"); it is the v2 path the deferred-features list hints at.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| dev-tunnels rs/ had no Host Connections | rs/ has ✅ Host Connections in feature matrix | rs/src/connections/relay_tunnel_host.rs landed; matrix updated | Path (b) is more capable than Phase 8 risk-notes assume. |
| dev-tunnels rs/ russh internally pinned to 0.37 | **Still 0.37.1** (verified 2026-05-19) | No change in 2 years on this pin | The version conflict with our workspace 0.60 remains real. |
| dev-tunnels rs/ was "vendor at your own risk" | **Active maintenance** — May 6 2026 commit, "relay client" feature May 4 2026 | Last 4 weeks | Lower vendor risk than Phase 8 notes assumed. |
| VS Code CLI used a separate fork of dev-tunnels | VS Code CLI uses upstream `microsoft/dev-tunnels` at pinned SHA | Long-standing | The SHA-pinning pattern (vs version) is the right one. |
| Microsoft maintained `vscode-russh` as a separate fork | Still maintained — 284 commits, recent activity | — | Path-by-`[patch.crates-io]` to vscode-russh is viable. |

**Deprecated/outdated:**
- The "russh 0.37 means dev-tunnels rs/ is abandoned" framing in original PROJECT.md notes. **Wrong** as of 2026-05-19 — version pin reflects internal consistency with vscode-russh, not abandonment.
- The "the spike picks (a) subprocess `code tunnel client`" framing in REQUIREMENTS.md DT-01. **`code tunnel client` does not exist.** Spike option (a) needs to be re-articulated as "subprocess `devtunnel connect` for port forwarding only" — and then immediately deferred because port forwarding ≠ shell.

## Day-1 Spike Decision Recommendation

> **LOCKED 2026-05-20 by user decision** — recommendation revised TWICE during research:
> 1. First round: (c) defer-to-v2
> 2. Continuation: (b) Path 2 Variant 2a (SSH over tunnel-forwarded port)
> 3. **Final (locked by user):** **(b) Path 2 Variant 2c — Vector Tunnel Agent.** User's constraint: their target remote machines (Adobe company box + others) do not have sshd available without VPN, and the user does not want to depend on a VPN. Variants 2a/2b would still route through `code tunnel`'s `handle_forward(22)` but require sshd on the remote. Path 1 (vscode-remote protocol) rejected for greenfield Rust cost + ongoing Microsoft protocol-breakage burden. Variant 2c uses `microsoft/dev-tunnels/rs/` as a **Host** (Vector ships a user-space `vector-tunnel-agent` binary that the user installs on each remote box the same way they install `code tunnel`), exposes a PTY via a Vector-controlled JSON/msgpack protocol on top of the relay channel. No sshd needed; no vscode-remote needed.

### Locked Recommendation: (b) vendor SDK — Path 2 Variant 2c (Vector Tunnel Agent)

**Confidence: HIGH.** Locked by user 2026-05-20.

### The load-bearing facts (final)

1. **`microsoft/dev-tunnels/rs/` ships a working `RelayTunnelHost`** (verified at `rs/src/connections/relay_tunnel_host.rs`). The agent registers a Dev Tunnel under the user's GitHub identity, opens a relay WebSocket, and accepts incoming `connect_to_port` calls from clients. We own both sides of the wire above the SDK.

2. **GitHub bearer tokens authenticate the SDK directly** (`Authorization: github <gho_token>`) — no token exchange required. Existing Phase 6 OAuth flow is reused as-is.

3. **The agent protocol can be embarrassingly simple.** We control both client and host, so a small framed JSON or msgpack protocol over a single relay channel is sufficient: `open_pty {rows, cols, shell?}` → `opened {session}`; `data {session, bytes}` bidirectional; `resize {session, rows, cols}`; `exit {session, code}`. No vscode-remote, no SSH, no protocol negotiation. ~600-800 LOC end-to-end.

4. **`portable-pty 0.9` on the agent side handles every PTY concern** — already in workspace deps, already used by vector-pty, already cross-platform Linux/macOS.

5. **User constraint that drove the decision:** the user does not want to depend on a VPN, and their target remote machines (Adobe company box + personal) do not expose sshd to non-VPN networks. Variants 2a/2b would have worked if sshd were reachable; 2c is the only variant that needs nothing on the remote box except our own user-space binary.

6. **Path 1 (vscode-remote protocol) was the obvious-seeming alternative** but it requires reimplementing a Microsoft-internal IDE protocol with no maintained Rust reference and monthly breaking changes. Cursor/VS Code only get away with it because they ARE VS Code. Variant 2c reaches the same end-state with less code and zero protocol-breakage exposure.

### What the spike document should say

```markdown
# Dev Tunnels Decision (Phase 8 Spike)

**Date:** 2026-05-20
**Decision:** (b) Path 2 Variant 2c — Vector Tunnel Agent.
**Reason:** User's target machines (corporate + personal) cannot expose sshd
to non-VPN networks, and user does not want a VPN dependency. Variants 2a/2b
require sshd on the remote. Path 1 (vscode-remote) is a Microsoft-internal
protocol with no maintained Rust reference and monthly breaking changes.

Variant 2c uses `microsoft/dev-tunnels/rs/` as a Host. Vector ships a small
user-space `vector-tunnel-agent` binary the user installs on each remote box
the same way they install `code tunnel`. The agent exposes a PTY via a
Vector-controlled framed protocol on top of the relay channel. No sshd, no
vscode-remote, no VPN.

**v1 commitment:**
- New crate: `vector-tunnel-agent` (binary).
- New crate: `vector-devtunnels` (Mac-side client + Dev Tunnels Management REST + Picker integration).
- Vendor `microsoft/dev-tunnels` rs/ at pinned SHA + `[patch.crates-io] russh = vscode-russh`.
- Distribution surface grows: Linux x86_64 + aarch64 binaries for the agent, alongside Mac Universal DMG.

**Carry-over from Phase 7:**
- vector-ssh transport scaffolding stays in tree but is NOT used by Phase 8 (no SSH in the agent path). Phase 9 (persistence + reconnect) may reuse the russh client for plain-SSH future work.
- `Mux::create_tab_async_with_transport` is the install seam.
- `TransportKind::DevTunnel` + `[remote]` badge + tint pipeline are already wired.
```

### What would change the decision (invalidators)

- **SDK regression in `RelayTunnelHost`:** if `microsoft/dev-tunnels rs/` archives or breaks the Host API → re-evaluate (likely fall to plain SSH + VPN tolerance, or fork the SDK).
- **Adobe IT blocks `vector-tunnel-agent`:** if the user's company IT signs/approves `code tunnel` but blocks arbitrary user-space binaries → fall back to Path 1 reluctantly (vscode-remote + accept the cost) or wrap `code tunnel` as a subprocess if it ever gains a shell endpoint.
- **`code tunnel` ships a shell endpoint upstream:** check `vscode/cli/src/tunnels/protocol.rs` for `pty` field on `SpawnParams` → if present, Path 1 collapses to a thin RPC wrapper and becomes preferable.

### Plan-phase implications (locked path)

Wave structure for Path 2 Variant 2c (Vector Tunnel Agent):

| Wave | Work | Effort |
|------|------|--------|
| 0 | Vendor `microsoft/dev-tunnels` rs/ + apply `[patch.crates-io] russh = vscode-russh` for the workspace + verify build | ~3 days |
| 1 | `vector-tunnel-agent` binary crate: `RelayTunnelHost` registration, GitHub OAuth identity, `$SHELL` spawn via `portable-pty 0.9`, framed protocol (open_pty + data + resize + exit), graceful shutdown | ~1 week |
| 2 | `vector-devtunnels` crate on Mac side: list tunnels via Management API (`GET /api/v1/tunnels`), open client connection via `connect_to_port`, speak the agent protocol, return `Box<dyn PtyTransport>` plug-compatible with Phase 7's mux helper | ~1 week |
| 3 | Picker UI + connect actor in `vector-app` (mirror Phase 6 codespaces picker shape: `DevTunnelsPickerModal` NSPanel + `devtunnels_actor` tokio driver + `Cmd-Shift-T` keybind) | ~3-4 days |
| 4 | Linux x86_64 + aarch64 cross-compilation for `vector-tunnel-agent` in CI + GitHub Release artifacts + agent install docs | ~2-3 days |
| 5 | Manual smoke matrix on the user's actual Adobe box + a personal Mac/Linux machine: install agent, register, list, connect, resize vim, exit cleanly, no zombie agent process | ~2-3 days |

**Total: ~3-4 calendar weeks (5-7 dev-weeks of effort).**

### Plan-phase implications (revised)

Phase 8 ships DT-02/03/04 in v1 against Path 2 Variant 2a. Expected shape:

- **Wave 0:** Vendor SDK at pinned SHA + workspace `[patch.crates-io]` to `microsoft/vscode-russh` + smoke build that resolves a single russh version graph.
- **Wave 1:** `vector-devtunnels` crate skeleton — Management REST list-tunnels (label filter for `code tunnel` hosts) + tunnel-access-token issuance + connect-to-tunnel via SDK's `RelayTunnelClient`.
- **Wave 2:** msgpack-RPC mini-client: one method (`handle_forward`) over the SDK's `connect_to_port(CONTROL_PORT)` stream. ~400 LOC scope target; if it creeps to ~1000+ LOC, halt and reconsider Variant 2b. Includes the challenge-response auth handshake.
- **Wave 3:** SSH wiring — `SshClient::connect_over(PortConnectionRW, ...)` reusing Phase 7 unchanged. Add host-key TOFU-with-prompt UI (native AppKit modal, known_hosts read/write, ~/Library/Application Support/Vector/known_hosts).
- **Wave 4:** Token refresh task (12h JWT re-issue via Management API) + reconnect plumbing for Phase 9.
- **Wave 5:** Manual smoke matrix on real `code tunnel` host (EC2 Amazon Linux 2023, Mac home box behind NAT, corporate-firewalled laptop).

Crate additions: `vector-devtunnels` (new). Deps: `tunnels` (git, pinned SHA), `[patch.crates-io] russh = vscode-russh`, no other new top-level deps.

**Fallback paths if Wave 2 blocks:**
- 2b: User runs `devtunnel host -p 22` alongside `code tunnel`. Test Go-host interop first.
- 2c: Ship a `vector-tunnel-agent` binary (~150 LOC russh server + PTY). Highest cost, cleanest UX.

If a future investigation overrides back to (c) defer: Phase 8 collapses to the decision-document Wave (~1 day) and DT-02/03/04 move to v2.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `code` (VS Code CLI on Mac) | option (a) subprocess to inspect tunnel state | check `which code` at runtime | — | warn + disable DT features |
| `devtunnel` CLI on Mac | optional diagnostic; not strictly required for (a)/(b) | check `which devtunnel` | — | document install link |
| GitHub OAuth refresh token in Keychain | All paths | ✓ (Phase 6 wired) | — | — |
| Network egress to `*.rel.tunnels.api.visualstudio.com` | All paths | runtime check | — | clear error message; "your network blocks Dev Tunnels" |
| Network egress to `github.com` and `api.github.com` | OAuth + token-validation | runtime check | — | (Phase 6 already handles this) |
| `rust-toolchain.toml` at 1.88+ | All Rust paths | ✓ (Phase 1 pinned) | 1.88.0 | — |

**Missing dependencies with no fallback:**
- If spike picks (b) Vendor SDK: vscode-russh as a git patch source. Lives at `microsoft/vscode-russh`, no version pinning beyond branch. **Risk: a force-push would break our build.** Document a pinned SHA in the patch table.

**Missing dependencies with fallback:**
- `code` CLI on user's Mac is helpful but not strictly required if we go REST-direct for tunnel listing.

## Validation Architecture

**Per `.planning/config.json`:** `workflow.nyquist_validation = true`. Validation section is included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (Rust built-in) + `wiremock 0.6` for HTTP mocks (already in tree) |
| Config file | per-crate `Cargo.toml` `[[test]]` entries |
| Quick run command | `cargo test -p vector-devtunnels --tests --lib` (only exists if spike picks (a)/(b)) |
| Full suite command | `cargo test --workspace --tests` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DT-01 | Spike document committed to `.planning/research/spikes/dev-tunnels-decision.md` | manual-only | `test -f .planning/research/spikes/dev-tunnels-decision.md` (file-existence assertion in CI) | ❌ Wave 0 (document doesn't exist yet) |
| DT-02 | Picker lists user's tunnels filtered to `code tunnel` hosts | unit (REST shape) + manual (UI) | `cargo test -p vector-devtunnels list_tunnels` + smoke matrix | ❌ Wave 0 (only if (a)/(b) picked) |
| DT-03 | Connecting to a tunnel opens a remote shell | manual-only (live tunnel + remote shell) | smoke matrix step | ❌ (only if (a)/(b) picked AND user accepts scope expansion) |
| DT-04 | Connected pane is tinted + `[remote]` badge | unit (format_tab_title) | `cargo test -p vector-mux format_tab_title_remote_badge` | ✅ Phase 7 covered this already |

### Sampling Rate
- **Per task commit:** `cargo test --workspace --tests` (existing discipline; ~363+ tests pass).
- **Per wave merge:** Full suite + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo fmt --check` + arch-lint.
- **Phase gate:** Full suite green + smoke matrix sign-off (only if (a)/(b) picked + any plans written; otherwise spike-document existence + REQUIREMENTS/ROADMAP updates).

### Wave 0 Gaps
- [ ] `.planning/research/spikes/dev-tunnels-decision.md` — the spike output itself (DT-01)
- [ ] If spike picks (b): `crates/vector-devtunnels/` skeleton with workspace dep additions
- [ ] If spike picks (b): `crates/vector-devtunnels/tests/list_tunnels.rs` — wiremock-backed REST test
- [ ] No new framework install — `wiremock`, `tokio-test`, `reqwest` all in tree

*(If spike picks (c): "None — Wave 0 reduces to the decision document + REQUIREMENTS/ROADMAP updates.")*

## Open Questions

1. **Does the user accept (c) defer-to-v2, or do they want to override?**
   - What we know: The current `code tunnel` protocol cannot deliver DT-03 without scope expansion.
   - What's unclear: User's appetite for either deferring or expanding scope (Vector-Tunnel-Agent path).
   - Recommendation: Present the spike document to user; let user decide between (c) and the override-to-agent-path before plan-phase runs.

2. **If override to Vector-Tunnel-Agent path: which Linux target architectures must the agent binary support?**
   - What we know: Mac client is fixed (Apple Silicon + Intel). Remote boxes are typically Linux x86_64, increasingly arm64.
   - What's unclear: Whether user's actual remote machines include Windows hosts (some users `code tunnel` from Windows).
   - Recommendation: v1-of-agent ships Linux x86_64 + aarch64; Mac arm64 for local self-test. Windows defer.

3. **Tunnel-list filter robustness.**
   - What we know: VS Code CLI tags `code tunnel`-created tunnels with labels, but the label scheme is internal and may change.
   - What's unclear: Whether filtering by label is enough, or if we need to also check tunnel endpoint metadata.
   - Recommendation: Implement label filter behind a `is_code_tunnel(t)` function with a comment listing all known label values; revisit if VS Code rev-bumps.

4. **Tunnel API token TTL in practice.**
   - What we know: Microsoft docs say "currently 24 hours" — could change.
   - What's unclear: Whether the connect-scope token issued via `POST /tunnels/{id}/access` follows the same TTL or a different one.
   - Recommendation: Treat as 12-hour window for refresh planning. Make the refresh interval a config var.

## Project Constraints (from CLAUDE.md)

- **No emoji in files** unless requested. (Honored — none in this document.)
- **Comments succinct, only on non-obvious WHY.** Apply to any Phase 8 code.
- **Use project's existing lint/format rules.** `make lint` / `cargo fmt --check` / `cargo clippy --workspace --all-targets -- -D warnings` — established in Phase 1.
- **`do not push`.** All commits stay local; user pushes asynchronously.
- **Commit each logical stage separately.** Plan tasks ship one commit each per established Phase 1-7 discipline.
- **GSD workflow enforcement.** All file edits go through `/gsd-execute-phase`; no direct edits.
- **Scope discipline.** If a feature is not on the v1 list, default to deferring. This entire document leans on that constraint to justify (c).

## Sources

### Primary (HIGH confidence)
- [microsoft/dev-tunnels rs/Cargo.toml](https://raw.githubusercontent.com/microsoft/dev-tunnels/main/rs/Cargo.toml) — package version 0.1.0, russh 0.37.1 from crates.io, tokio 1.20, reqwest 0.13 (verified 2026-05-19)
- [microsoft/dev-tunnels rs/Cargo.lock](https://raw.githubusercontent.com/microsoft/dev-tunnels/main/rs/Cargo.lock) — russh 0.37.1 resolved from crates.io (line ~2570)
- [microsoft/dev-tunnels rs/src/lib.rs](https://github.com/microsoft/dev-tunnels/blob/main/rs/src/lib.rs) — exports `contracts`, `management`, `connections (cfg=connections)`
- [microsoft/dev-tunnels rs/src/connections/](https://github.com/microsoft/dev-tunnels/tree/main/rs/src/connections) — files: `errors.rs`, `io.rs`, `mod.rs`, `relay_tunnel_client.rs`, `relay_tunnel_host.rs`, `ws.rs`
- [microsoft/dev-tunnels rs/src/connections/relay_tunnel_client.rs](https://raw.githubusercontent.com/microsoft/dev-tunnels/main/rs/src/connections/relay_tunnel_client.rs) — WebSocket "tunnel-relay-client", russh client, `host_public_keys` verification, `connect_to_port`
- [microsoft/dev-tunnels rs/src/management/authorization.rs](https://raw.githubusercontent.com/microsoft/dev-tunnels/main/rs/src/management/authorization.rs) — Auth header schemes: `aad`, `github`, `tunnel`, `bearer`
- [microsoft/dev-tunnels rs/src/management/http_client.rs](https://raw.githubusercontent.com/microsoft/dev-tunnels/main/rs/src/management/http_client.rs) — uses `AuthorizationProvider`, standard `AUTHORIZATION` header
- [microsoft/dev-tunnels feature matrix (README)](https://github.com/microsoft/dev-tunnels) — Rust now has ✅ Management + Client + Host; ❌ Reconnection / SSH-reconnect / Token-refresh / Keep-alive
- [microsoft/dev-tunnels commits to rs/](https://github.com/microsoft/dev-tunnels/commits/main/rs) — May 6 2026 "Use new local dnsname", May 4 2026 "rust: implement relay client" (#626)
- [microsoft/vscode/cli/Cargo.toml](https://github.com/microsoft/vscode/blob/main/cli/Cargo.toml) — pins `tunnels` at `rev = "64048c1409ff56cb958b879de7ea069ec71edc8b"`, `[patch.crates-io]` russh family → `microsoft/vscode-russh`, tokio 1.52
- [microsoft/vscode/cli/src/tunnels/control_server.rs](https://github.com/microsoft/vscode/blob/main/cli/src/tunnels/control_server.rs) — all `handle_*` methods listed; NO `handle_spawn_pty`/`handle_shell`/`handle_open_terminal`
- [microsoft/vscode/cli/src/tunnels/protocol.rs](https://raw.githubusercontent.com/microsoft/vscode/main/cli/src/tunnels/protocol.rs) — `SpawnParams { command, args, cwd: Option<String>, env: HashMap<...> }` — NO PTY field
- [microsoft/vscode-russh](https://github.com/microsoft/vscode-russh) — version 0.37.1, 284 commits, default features `flate2` + `rs-crypto`, 16 stars
- [VS Code Remote Tunnels docs](https://code.visualstudio.com/docs/remote/tunnels) — official user-facing description; "SSH connection is created over the tunnel" refers to transport layer
- [Microsoft Dev Tunnels security](https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/security) — domains, token TTL (24h), `X-Tunnel-Authorization: tunnel <TOKEN>` header for service-port access
- [Microsoft Dev Tunnels CLI commands](https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/cli-commands) — full `devtunnel` CLI surface; `devtunnel connect TUNNELID` forwards ports, not shells
- [Phase 7 SUMMARY + STATE.md Plan 07-01](file:///Users/ashutosh/personal/vector/.planning/STATE.md) — `vector-ssh` scaffolding (`SshClient::connect_over`, `SshChannelTransport`, `ChildStdioStream`, `VectorHandler`), workspace deps `russh = "0.60"` + `ssh-key = "0.6"`
- [Vector `vector-codespaces/src/auth/device_flow.rs`](file:///Users/ashutosh/personal/vector/crates/vector-codespaces/src/auth/device_flow.rs) — existing OAuth Device Flow, scopes `codespace + read:user`, manual `Debug` discipline
- [Vector `vector-mux/src/transport.rs`](file:///Users/ashutosh/personal/vector/crates/vector-mux/src/transport.rs) — `TransportKind { Local, DevTunnel }`, `PtyTransport` trait
- [Vector `vector-mux/src/mux.rs`](file:///Users/ashutosh/personal/vector/crates/vector-mux/src/mux.rs) — `create_tab_async_with_transport` (line 436): installs `Box<dyn PtyTransport>` directly

### Secondary (MEDIUM confidence)
- [Luis Johnstone "VScode tunnels guide" (2025)](https://luisjohnstone.com/2025/07/vscode-tunnels-guide.html) — practical examples of `devtunnel list` + `devtunnel token --scopes connect` against tunnels created by `code tunnel`
- [DEV.to "Introducing VS Code Remote Tunnels"](https://dev.to/burkeholland/introducing-vs-code-remote-tunnels-connect-to-remote-machines-with-ease-3nlg) — corroborating user-facing model
- [Arm Learning Paths "VS Code Tunnels"](https://learn.arm.com/install-guides/vscode-tunnels/) — practical install + usage flow
- [GitHub issue microsoft/vscode-remote-release#8373 "code tunnel to local machine directly"](https://github.com/microsoft/vscode-remote-release/issues) — closed as out-of-scope, confirming Microsoft does not plan to make `code tunnel` directly client-attachable outside VS Code

### Tertiary (LOW confidence — flagged for validation)
- The exact label values VS Code CLI sets on `code tunnel`-created tunnels: derived from one inspection of `dev_tunnels.rs` (`add labels containing version + platform`); needs verification by listing a real user's tunnels and inspecting `labels` field before shipping a filter.
- Token TTL in practice (docs say 24h "currently"): not independently verified against a live tunnel.

## Metadata

**Confidence breakdown:**
- Standard stack versions: HIGH — all verified via crates.io + Microsoft repo direct read 2026-05-19.
- `code tunnel` protocol has no PTY: HIGH — verified against source of `control_server.rs` + `protocol.rs` on main branch.
- Spike recommendation (c) defer: HIGH — follows mechanically from the protocol finding + Phase 8 out-of-scope constraint.
- russh 0.37/0.60 conflict scope: HIGH — verified both Cargo.toml + Cargo.lock + VS Code CLI patch table.
- Vector-Tunnel-Agent alternative architecture: MEDIUM — design is sound but unproven for Vector specifically; would need its own Phase 8.5 spike if user wants to pursue it.
- Tunnel labels filtering: LOW — labels are not a stable contract; needs live verification before filtering ships.

**Research date:** 2026-05-19
**Valid until:** 2026-06-19 (30 days) — re-check `dev-tunnels/rs` commit log and `vscode/cli/src/tunnels/protocol.rs` for new RPC methods before any plan-phase if delayed.

---

## Continuation Research (2026-05-20): Path 1 vs Path 2

**Researcher mode:** feasibility (not ecosystem). The first round established stack selection and identified the protocol gap. This round answers a different question: **with the SDK as plumbing, what's the cheapest path to a remote PTY shell in Vector?**

The user's pushback ("Cursor and all other tools can connect to GitHub tunnels...and then all of them can launch a terminal") is correct. The first-round (c) recommendation was over-conservative. Re-reading the protocol files exposes a third option the first round didn't enumerate: **the `code tunnel` host already has a public `handle_forward(port, public)` RPC** — i.e., Vector can RPC the existing `code tunnel` to expose port 22, then SSH through it.

Two survivor paths emerge:

- **Path 1: VS Code Server protocol client** — call `handle_serve`, then speak vscode-remote on the forwarded HTTP/WS port. The "Cursor" approach.
- **Path 2: SSH over tunnel-forwarded port** — ask the tunnel host to forward port 22 (either via msgpack-RPC `handle_forward`, or by the user running a separate `devtunnel host -p 22`), then `russh::client::connect_stream` over the SDK's `PortConnectionRW`.

### Q1. VS Code remote-terminal protocol surface (Path 1)

**Source location (HIGH confidence):**

- `microsoft/vscode/src/vs/server/node/remoteTerminalChannel.ts` — the IPC channel the VS Code Server registers under the name `terminal`. Verified directly.
- `microsoft/vscode/src/vs/platform/terminal/common/terminal.ts` — defines `TerminalIpcChannels` (`localPty`, `ptyHost`, `ptyHostWindow`, `logger`, `heartbeat`) and the `IPtyService` interface that `createProcess()` lives on.
- `microsoft/vscode/src/vs/workbench/contrib/terminal/common/remote/remoteTerminalChannel.ts` — the **client-side** counterpart (the workbench's `IRemoteTerminalAttachTarget` / `RemoteTerminalChannelClient`). This is the closest thing to a wire-protocol reference document. (Not yet read but exists per the directory listing.)
- `microsoft/vscode/src/vs/platform/remote/common/remoteAgentConnection.ts` — the framing layer above the WebSocket: VS Code's `IPCRPCProtocol` (request/event/promise IDs, JSON-with-Buffer encoding).

**Channel command names (verified by reading `remoteTerminalChannel.ts`):**

The server registers a `terminal` IPC channel. The `call` methods (client → server) include:

| Method | Purpose |
|--------|---------|
| `$createProcess(args: ICreateTerminalProcessArguments)` | Create remote PTY |
| `$start(args)` | Start the created process |
| `$input(args)` | Send keystrokes |
| `$resize(args)` | SIGWINCH |
| `$shutdown(args)` | Kill the PTY |
| `$attachToProcess` / `$detachFromProcess` | Reconnect to an orphaned PTY (this is the persistence story) |
| `$listProcesses` | Enumerate PTYs (for reattach) |
| `$getInitialCwd` / `$getCwd` | Path queries |
| `$acknowledgeDataEvent` | Backpressure ack — the server stops sending data until the client acks |
| `$getDefaultSystemShell` / `$getProfiles` / `$getEnvironment` | Shell config discovery |
| `$updateTitle` / `$updateIcon` / `$updateProperty` | Terminal metadata mutation |
| `$setUnicodeVersion` | Width-table negotiation |

And `listen` events (server → client):
| Event | Purpose |
|-------|---------|
| `$onProcessDataEvent` | Output bytes |
| `$onProcessReadyEvent` | Process started, returns pid + cwd |
| `$onProcessExitEvent` | Exit code |
| `$onProcessReplayEvent` | Replay buffer for reconnect |
| `$onDidChangeProperty` | Property updates |

**This is a documented (in source) RPC.** It's not an obscure handshake; it's a structured IPC channel. Implementing a Rust client against the **terminal sub-channel only** is bounded scope.

### Q2. Existing Rust client implementations (Path 1)

**Verified by `crates.io` search for `vscode-remote` / `vscode-server` / `vscode-tunnel` / `code-server-client`:**

- ❌ **No published Rust crate implements the vscode-remote protocol.** Closest hits: `code-remote` (a session-selection wrapper around `code` CLI — not the protocol), `vscli` (devcontainer CLI launcher), `tauri-remote-ui` (unrelated).
- ❌ **No GitHub Rust project** implements a non-trivial vscode-remote client. Searched the obvious term spaces, no maintained candidates.

**Adjacent prior art (verified):**

- **Zed** (`zed-industries/zed`) — does NOT implement the vscode-remote protocol. Per Zed's official blog and DeepWiki: Zed shells out to the `ssh` binary, opens a control-master multiplex, downloads its own `remote_server` binary (compiled with `musl`), and speaks its own protobuf RPC (`proto/zed.proto`) over the multiplexed SSH channels. **Zed avoids the VS Code protocol entirely** — a strong signal about the cost/benefit.
- **`coder/code-server`** — implements the **server** side of vscode-remote (it's a fork of VS Code itself, running as a server). The documentation does not enumerate the wire protocol externally; the source IS the spec. Not a Rust reference; a TypeScript existence proof.
- **`gitpod-io/openvscode-server`** — same story as code-server: a server-side fork of VS Code.
- **Coder's Rust tooling** — none implements vscode-remote. Coder's Rust crates are around Tailscale/Wireguard plumbing (Coder Connect), not editor protocols.

**Conclusion (HIGH confidence):** A Rust vscode-remote terminal client is **greenfield work**. Zero existing Rust crate to vendor or fork. The reference implementation lives only inside the TypeScript codebase of VS Code itself.

### Q3. Protocol stability (Path 1)

**Findings:**

- VS Code Server announces a **protocol version number** at handshake. Clients with mismatched versions are refused with "Connection error: Version mismatch, client refused" (verified via `microsoft/vscode-remote-release` issues #533, #2374, #8582 — the latter reports a "from one day to the other" break in 2024-2025).
- Microsoft ships VS Code monthly. The vscode-remote protocol layer (`IRemoteAgentEnvironmentDTO`, `IPCRPCProtocol` framing, terminal channel signatures) is **internal-only — versioned for the editor's own client-server compatibility**, not as a public API. There is no compatibility commitment to third-party clients.
- The terminal channel's method shape has been stable in observable ways (the `$createProcess`/`$input`/`$resize`/`$shutdown` quartet has been there since the protocol's inception), but field additions, type-tag changes (e.g., `Buffer` vs `Uint8Array`), and ack-protocol tweaks (e.g., `$acknowledgeDataEvent` was added later for flow control) **do occur without notice**.
- The `vscode-remote-release` repo has multiple "client refused, version mismatch" issues per year (e.g., #8582 from 2024 reports a same-day break), confirming Microsoft routinely bumps the protocol.

**Implication for Vector:**

- Path 1 requires shipping a Rust client that pins to a snapshot of the protocol. Every VS Code Server release is a potential break.
- Mitigation strategies are bad:
  1. **Pin to an older VS Code Server version** — but the server is downloaded fresh per `code tunnel` session (via `handle_serve`'s `quality`/`commit` args). We can't control what server version the user's `code tunnel` host launches.
  2. **Track upstream and re-pin Vector monthly** — turns Vector into a maintenance treadmill against a vendor that doesn't want third-party clients.
  3. **Implement only the smallest subset and tolerate the minimum field set** — fragile against any rename or required-field addition.

**Confidence: HIGH** that this is a real, recurring break point. The "transparent to the user" UX Vector wants does not survive a monthly Microsoft release if we go Path 1.

### Q4. Minimum protocol subset (Path 1)

If a heroic engineer wanted to ship Path 1 anyway, here's the absolute minimum:

**Phase A — transport bring-up:**
1. WebSocket connect to the VS Code Server port forwarded over the dev tunnel.
2. The VS Code Server expects a connection-token query param (`?reconnectionToken=...` + `connectionToken`); both are returned by `handle_serve`.
3. Negotiate `IPCRPCProtocol` framing — fixed header layout with little-endian uint32 lengths + JSON payloads with `Buffer` escape encoding (`{ "$type": "Buffer", "data": [...] }`).
4. Send "Connection Auth" handshake message — the server replies with `ConnectionType.MessagePassthrough` accepted.

**Phase B — terminal-channel subset:**
5. `getChannel('terminal')` returns a logical sub-channel; subsequent calls are framed with channel ID + method ID.
6. `$createProcess` → returns `IPersistentTerminalProcess` ID. Args include `shellLaunchConfig`, `cols`/`rows`, `cwd`, `env`.
7. Subscribe to `$onProcessDataEvent` and `$onProcessExitEvent` (these are `listen` events, framed as event-emitter messages — distinct from call/return).
8. `$start(id)` — begin process.
9. `$input(id, data)` — outbound bytes.
10. `$resize(id, cols, rows)` — SIGWINCH.
11. `$acknowledgeDataEvent(id, charCount)` — every N KB of input, ack so the server keeps streaming. **Skipping this stalls the PTY.**
12. `$shutdown(id, immediate: bool)` — exit.

**Estimate of "minimal" code size:**

- IPCRPCProtocol framing + WS framing: ~600 LOC Rust (JSON + Buffer escape + length-prefix + event/call dispatch).
- Connection handshake + auth: ~200 LOC.
- Terminal channel client (12 message types): ~400 LOC.
- Glue to `PtyTransport`: ~150 LOC.
- Tests against a recorded WS dump: ~500 LOC.

**Total minimum:** ~1,800 LOC of new Rust, ~half of it framing infrastructure we own forever. Plus per-release validation work.

### Q5. End-to-end auth (Path 1)

```
GitHub OAuth Device Flow
  → GitHub access token (gho_...)
  → Authorization: github gho_... → Dev Tunnels Management API
  → POST /tunnels/{id}/access?scopes=connect
  → tunnel access token (tunnel JWT)
  → WebSocket "tunnel-relay-client" with `Sec-WebSocket-Protocol: tunnel-relay-client`,
    `Authorization: tunnel <jwt>`
  → russh client over WS, host-key check against tunnel.endpoint.host_public_keys
  → channel_open_direct_tcpip("127.0.0.1", CONTROL_PORT (=31545 by convention))
  → msgpack-RPC handle_challenge_issue/verify (token exchange — bypassable in same-tenant scenarios per protocol.rs reading)
  → handle_serve (downloads + spawns VS Code Server, returns server port)
  → handle_forward(server_port, public=false) — exposes the server's HTTP/WS as a forwarded port on the tunnel
  → SDK's connect_to_port(server_port) returns a PortConnection
  → upgrade to WebSocket on that channel (VS Code Server speaks HTTP-with-WS-upgrade on its port)
  → VS Code Server's own `connectionToken` handshake (separate from the tunnel token)
  → IPCRPCProtocol handshake → terminal channel → $createProcess → $start
```

**Five auth checkpoints**, three different token formats (GitHub OAuth, tunnel-access JWT, VS Code Server `connectionToken`). Each can independently expire/refresh.

### Q6. Cost estimate (Path 1)

**Honest range, Rust engineer fluent in async:**

- SDK vendoring + russh patch chain (one-time): 0.5 week.
- WS+IPCRPCProtocol framing: 1.5–2 weeks.
- Handshake + connection-auth + terminal channel client: 1.5–2 weeks.
- Glue to `PtyTransport` + integration with existing mux: 0.5 week.
- Reattach / `$onProcessReplayEvent` for Phase 9 persistence: 1 week.
- Token refresh task + reconnect logic for tunnel access token: 1 week.
- Manual smoke matrix + debugging the inevitable protocol-mismatch issues: 1–2 weeks.

**Total: 7–10 dev-weeks.** Plus ongoing 0.5-1 week/quarter to track upstream protocol breaks.

This is "a small Phase by itself." It exceeds Phase 8's scoping intent.

### Q7. SDK port-forward API (Path 2) — verified by reading source

**File:** `microsoft/dev-tunnels/rs/src/connections/relay_tunnel_client.rs` (read directly, lines 142-180).

**Method signature:**

```rust
impl ClientRelayHandle {
    pub async fn connect_to_port(&self, port: u16) -> Result<PortConnection, TunnelError> {
        let channel = self.session
            .channel_open_direct_tcpip("127.0.0.1", port as u32, "127.0.0.1", 0)
            .await
            .map_err(TunnelError::TunnelRelayDisconnected)?;
        Ok(PortConnection { channel })
    }
}
```

**The returned `PortConnection`:**

```rust
pub struct PortConnection {
    channel: russh::Channel<russh::client::Msg>,
}

impl PortConnection {
    pub fn into_rw(self) -> PortConnectionRW {
        PortConnectionRW(self.channel.into_stream())
    }
}

pub struct PortConnectionRW(russh::ChannelStream);
impl AsyncRead for PortConnectionRW { /* delegates to ChannelStream */ }
impl AsyncWrite for PortConnectionRW { /* delegates to ChannelStream */ }
```

**Conclusion (HIGH confidence):** `PortConnection::into_rw()` returns a value that is **exactly** what `russh::client::connect_stream(config, rw, handler)` consumes. The SDK gives us a plug-compatible byte stream for any TCP port on the tunnel host. End-to-end: `ClientRelayHandle::connect_to_port(22) → PortConnection → into_rw() → russh::client::connect_stream(..)`. No msgpack-RPC involvement at the Vector layer.

**Compatibility footnote (HIGH confidence):** The SDK's `relay_tunnel_client.rs` `Interoperability` doc-comment states: *"This client uses SSH `direct-tcpip` channels to connect to forwarded ports. This works with Rust and TypeScript/C# tunnel hosts. Go tunnel hosts do not currently handle `direct-tcpip` channels."* The VS Code CLI's `code tunnel` host is **Rust** (it links the same SDK). So we're in the compatible cell. The `devtunnel` standalone CLI is **Go** — connecting to a `devtunnel host`-served tunnel from our Rust SDK *may fail*. This is a real differentiator: connect to `code tunnel` hosts works; connect to user's pure `devtunnel host` hosts may not.

### Q8. Does the port have to be pre-registered? (Path 2)

**Two paths, both verified by source:**

**Path 2a — User runs `code tunnel` and Vector RPCs `handle_forward(22)`:**

- `microsoft/vscode/cli/src/tunnels/port_forwarder.rs` exposes `PortForwarding::forward(port: u16, privacy)` with the ONLY rejections being `CONTROL_PORT` and `AGENT_HOST_PORT`. Port 22 is allowed.
- `microsoft/vscode/cli/src/tunnels/protocol.rs` defines `ForwardParams { port: u16, public: bool }` and the dispatcher registers `handle_forward` at the RPC layer.
- **But:** invoking `handle_forward` requires speaking the msgpack-RPC handshake (challenge-response auth + correct message framing). That's the same RE-territory Path 1 hits — except here we only need to use ONE RPC, not the full terminal protocol. Estimate: ~400 LOC to send a single `forward` call (vs ~1,800 LOC for full Path 1).

**Path 2b — User runs `devtunnel host -p 22` separately (no `code tunnel` involved):**

- Verified by Microsoft Learn docs (`Manage dev tunnel ports`): `devtunnel create` → `devtunnel port create -p 22 --protocol auto` → `devtunnel host`. The tunnel persistently has port 22. Vector connects via SDK with no RPC involvement at all.
- **Critical caveat:** `devtunnel host` is a **Go** binary. Per the SDK's interop note (Q7), our Rust `RelayTunnelClient.connect_to_port` may not work against a Go host. **This is the single biggest unknown for Path 2b.** Must be tested before commitment.
- **Mitigation:** the user could instead run a Rust host process. The dev-tunnels rs/ SDK has `RelayTunnelHost::add_port_raw`, so a small "vector-tunnel-agent" binary that wraps the SDK's host side + spawns a local PTY directly is feasible. This is the "Alternative Architecture: Vector Tunnel Agent" the first-round research already gestured at — but in *that* world we don't even need a real sshd; the agent handles PTY natively.

**Best Path 2 variant:** Path 2a is the most user-friendly (user runs `code tunnel` once, the same flow they already use for VS Code) but requires us to implement the minimum msgpack-RPC client to call `handle_forward`. Path 2c (a thin "vector tunnel agent" using the SDK's host side) is the cleanest engineering but requires the user to install a second binary.

### Q9. Host-key trust UX (Path 2)

This is the most interesting question because Path 2 strips the dev-tunnels API's host-key attestation: the tunnel's `host_public_keys` is the **tunnel relay's** key, not the **remote box's sshd** key. Once you're inside the tunnel speaking to the remote sshd directly, sshd presents its OWN key, which the tunnel API doesn't know about.

**Sub-questions answered:**

1. **Does Dev Tunnels expose remote-box SSH host metadata?** No. The Dev Tunnels Management API treats the host as an opaque endpoint; it has no knowledge that sshd is running on port 22 or what sshd's host key is. Verified by reviewing `contracts/TunnelEndpoint` and `host_public_keys` field semantics — `host_public_keys` are for the relay session itself.

2. **TOFU vs prompt vs other?**
   - **TOFU (silently trust on first use):** wrong for v1. PROJECT.md and Phase 7 already established that host-key trust must be explicit. Even if the data path is encrypted by the tunnel transport, the SSH layer is independently encrypted and MITM-able if a relay-side attacker substitutes their own sshd.
   - **First-use prompt + cache, with explicit user confirmation:** the right answer. UX matches how ghostty, WezTerm, and standard `ssh` behave — show fingerprint, ask user to confirm, then pin in a known_hosts-equivalent store.
   - **Side-channel attestation (e.g., publishing the sshd key as a Dev Tunnels tag/label on the tunnel from the user's box during initial setup):** clever and zero-prompt, but requires the user to run a one-time setup helper. Filed as v2.

3. **How do ghostty/WezTerm handle ordinary SSH host-key UX?**
   - **ghostty**: shells out to `ssh` binary; inherits the user's existing `~/.ssh/known_hosts` and the standard `ssh` prompt UX ("The authenticity of host '...' can't be established. Are you sure you want to continue connecting (yes/no/[fingerprint])?").
   - **WezTerm**: uses its own `wezterm-ssh` crate (a russh wrapper); on unknown host key, prompts in a tab with the fingerprint and Yes/No/Once buttons; persists to `~/.config/wezterm/known_hosts`.
   - **Vector should mirror WezTerm's UX**: native modal prompt with fingerprint, three buttons (Trust & Save / Trust This Session / Cancel), pinned to `~/Library/Application Support/Vector/known_hosts` (single-line-per-entry hostname + key + comment, ssh-compatible format so users can paste from `ssh-keyscan`).

**Confidence: HIGH** on TOFU-with-prompt approach. Established pattern in the ecosystem; mirrors `ssh` behavior; consistent with Phase 7's "no TOFU bypass" discipline (this isn't TOFU — it's TOFU-with-explicit-confirmation).

### Q10. End-to-end auth (Path 2)

```
GitHub OAuth Device Flow
  → GitHub access token (gho_...)
  → Authorization: github gho_... → Dev Tunnels Management API
  → POST /tunnels/{id}/access?scopes=connect → tunnel access token (JWT)
  → WebSocket "tunnel-relay-client" with `Authorization: tunnel <jwt>`
  → russh client over WS, host-key check against tunnel.endpoint.host_public_keys
  → channel_open_direct_tcpip("127.0.0.1", 22, ...)
  → PortConnection::into_rw() → AsyncRead+AsyncWrite stream
  → russh::client::connect_stream(config, stream, VectorHandler with sshd's expected fp)
  → sshd presents host key, VectorHandler verifies against ~/.../known_hosts or prompts user
  → russh::client::authenticate_publickey(username, signing_key from Vector's keystore)
  → channel.request_pty(...) + channel.request_shell(...)
  → existing SshChannelTransport drives the bytes (Phase 7 scaffolding, unmodified)
```

**Three auth checkpoints, three independent token types:**
1. GitHub OAuth refresh (handled by Phase 6 silent-refresh chain).
2. Tunnel access JWT (must be refreshed every ~12h — same gap as Path 1).
3. SSH pubkey auth to sshd (handled by russh + ssh-key from Phase 7).

**Does this work without manual `ssh-add`?**

- Yes if Vector manages its own key material. Phase 7's `vector-secrets` already has `KeyManager` scaffolding (now removed but easy to re-add). Generate an ed25519 key on first run, store in Keychain via `keyring-core`, write the public half to `~/.ssh/authorized_keys` on the remote ONE TIME via a setup helper.
- Alternative: load `~/.ssh/id_ed25519` from disk via `ssh-key 0.6` (already in tree). Standard behavior.

**Recommendation:** load user's existing `~/.ssh/id_ed25519` (or `id_rsa` fallback) by default. If absent, prompt to generate-and-deploy a Vector-managed key.

### Q11. sshd default config on EC2/Linux

**Defaults that matter for a first-time pubkey connect from a freshly-deployed key (verified by sshd_config docs):**

| Setting | Default | Impact |
|---------|---------|--------|
| `PubkeyAuthentication` | `yes` | Pubkey auth is on. |
| `PasswordAuthentication` | `yes` (Debian/Ubuntu); `no` (Amazon Linux 2023, Fedora 38+) | We don't care; we'll use pubkey. |
| `AllowTcpForwarding` | `yes` | Doesn't matter for the shell case (only for nested forwarding). |
| `PermitTunnel` | `no` | Doesn't matter. |
| `AuthorizedKeysFile` | `.ssh/authorized_keys` | Standard. User must place our pubkey here. |
| `MaxAuthTries` | `6` | Plenty. |
| `LoginGraceTime` | `120s` | Plenty. |
| `KexAlgorithms` / `Ciphers` / `HostKeyAlgorithms` | Modern defaults on OpenSSH 8.x+ | russh 0.60 negotiates fine. |

**Edge cases that bite:**
- **AWS EC2 Amazon Linux 2023** ships sshd with **only** `KbdInteractiveAuthentication no` and `PasswordAuthentication no` — pubkey-only. Our default works.
- **Corporate hardened sshd** may have `AllowUsers user1 user2` or `AllowGroups developers`. If the user account isn't in the list, auth fails with a confusing "Permission denied (publickey)" error. **Mitigation:** clear error message that names the username being used.
- **No `~/.ssh/authorized_keys` and no `ssh-copy-id` workflow** — first-time connect fails with no diagnostic. **Mitigation:** Vector's first-connect flow should detect "publickey rejected" and offer to deploy the pubkey via a one-time `code tunnel`-RPC-spawned `cat >> ~/.ssh/authorized_keys` (using `handle_spawn` — its lack of PTY is fine for `cat`).
- **OpenSSH ed25519 key path on a stale CentOS 7 box** — ed25519 keys require OpenSSH ≥ 6.5. CentOS 7 ships 7.4. Should work. We're not targeting CentOS 6.

**Confidence: HIGH** — standard sshd defaults are friendly. The most common failure mode (no key in authorized_keys) is fixable via a setup helper that uses the same dev tunnel.

### Q12. Cost estimate (Path 2)

**Honest range, Rust engineer fluent in async:**

- SDK vendoring + russh patch chain (one-time): 0.5 week.
- Tunnel listing UI (REST + picker filter): 0.5 week — much of this is already in Phase 6's `CodespacesPickerModal`-shaped scaffolding.
- Path-2 specific glue (RelayTunnelClient → PortConnection → russh::client::connect_stream): 0.5 week. The hard part (russh client + PTY shell) is already in Phase 7.
- Host-key TOFU-with-prompt UX (native modal + known_hosts read/write): 1 week.
- First-connect publickey-deploy helper (handle_spawn over msgpack-RPC) — **only if we want self-bootstrap, can be deferred**: 1 week or skipped.
- Token refresh task: 0.5 week (12-hour timer + REST re-issue).
- Manual smoke matrix (EC2, Mac home box, behind-NAT laptop): 1 week.
- Reconnect plumbing for Phase 9: 1 week (mostly inherits from `Domain::reconnect()` design).

**Total minimum (without bootstrap helper): 4–5 dev-weeks.**
**Total with bootstrap helper + token-refresh polish: 5–6 dev-weeks.**

**~Half the cost of Path 1, dramatically lower long-tail maintenance burden.**

### Q13. Side-by-side risk comparison

| Risk | Path 1 (VS Code Server protocol) | Path 2 (SSH over forwarded port) |
|------|----------------------------------|----------------------------------|
| **Token expiry mid-session** | Tunnel JWT + VS Code Server `connectionToken` both expire. Both unrefreshable by SDK today. **High** | Tunnel JWT expires. SSH session itself doesn't expire. Only ONE token to refresh. **Medium** |
| **Protocol breakage on Microsoft release** | VS Code Server bumps protocol version monthly; we WILL get refused. **Critical** | russh-to-sshd is stable for years at a time; the tunnel relay protocol is also Microsoft-internal but the SDK insulates us. **Low** |
| **Wifi drop / NAT timeout** | Reconnect requires re-handshaking BOTH layers + reattaching to orphan PTYs via `$attachToProcess`. The replay buffer is a feature. **Medium-positive** (replay works) | russh session dies, must rebuild SSH from scratch. tmux on the remote provides session persistence (Phase 9 PERSIST-03). **Medium** |
| **sshd config refusing first connect** | N/A | Real risk. Mitigated by helper but UX-fragile. **Medium** |
| **Host-key trust** | Tunnel API attests relay host key — strong cryptographic chain. **Low** | We have to TOFU-prompt for sshd key; user can be tricked into accepting a wrong key. **Medium** |
| **`code tunnel` host RPC version drift** | Indirect — Path 1 calls `handle_serve` which is stable. **Low** | If we use Path 2a (`handle_forward` RPC), drift risk on that one RPC. If we use Path 2b (separate `devtunnel host`), zero `code tunnel` RPC involvement. **Low** |
| **Go-host interop bug** | N/A | If user runs `devtunnel host` (Go), `direct-tcpip` channel may be refused. **Low-medium** — but a separate Rust `vector-tunnel-agent` solves it. |
| **Long-term Microsoft strategy** | Microsoft has explicitly closed "Use `code tunnel` from a non-VS-Code client" feature requests (vscode-remote-release#... as out-of-scope). They have NO motivation to keep the protocol stable for us. **High** | Microsoft has no opinion on what we send through `direct-tcpip`. We're using documented transport, not undocumented protocol. **Low** |

### Q14. What does "the user already has it set up" look like?

**Path 1 (VS Code Server protocol client):**
- User installs `code` CLI on remote box (`curl -L https://aka.ms/code-tunnel-linux-x64 -o /usr/local/bin/code && chmod +x ...`).
- User runs `code tunnel` ONCE interactively to log in and approve the device.
- User runs `code tunnel service install` to make it persistent across reboots.
- User opens Vector on Mac, signs in with GitHub, picks the tunnel from the list. Done.

**Path 2 (SSH over forwarded port):**

*Variant 2a — user runs `code tunnel` and Vector RPCs `handle_forward(22)`:*
- Same setup as Path 1 (install `code`, run `code tunnel service install`).
- One additional ONE-TIME step: ensure sshd is running on the box and the user's `~/.ssh/authorized_keys` contains the Vector pubkey (or use Vector's first-connect bootstrap helper).
- Open Vector, pick tunnel. Vector RPCs `handle_forward(22)` then SSH-connects.

*Variant 2b — user runs a dedicated `devtunnel host -p 22` alongside `code tunnel`:*
- Install both `code` and `devtunnel` CLIs.
- Run `code tunnel service install` (their existing flow).
- Additionally: `devtunnel create vector-shell && devtunnel port create -p 22 && devtunnel host vector-shell` (under tmux/systemd).
- sshd must be running + authorized_keys configured.
- Vector picks the `vector-shell`-labeled tunnel from the picker.

*Variant 2c — Vector ships its own `vector-tunnel-agent`:*
- Install `vector-tunnel-agent` (single small binary).
- Run `vector-tunnel-agent install` (registers a systemd/launchd service, creates a tunnel labeled `vector-shell`, runs as host).
- Done. No sshd needed; agent handles PTY natively. **Best UX, highest engineering cost.**

**Best balance for v1: Variant 2a.** User does one thing they already do (`code tunnel service install`). Vector handles the rest. If `handle_forward` RPC proves too sharp to implement, fall back to Variant 2b (require user to also run `devtunnel host -p 22`).

### Q15. Strong recommendation

**Recommendation: Path 2, Variant 2a → fall back to 2b if 2a's msgpack-RPC implementation costs creep.**

**Confidence: HIGH.**

**Defense:**

1. **Cost ratio:** Path 2 is 4–6 dev-weeks vs Path 1's 7–10 dev-weeks. Path 2 also has ~zero long-tail maintenance vs Path 1's monthly upstream-tracking burden.
2. **Protocol stability:** russh ↔ sshd is the most stable protocol pair in the Unix world. vscode-remote breaks on a monthly cadence and Microsoft has zero incentive to stabilize it for third parties.
3. **Reuse of Phase 7:** `SshClient::connect_over(stream, ...)` was designed for exactly this seam. The integration is shallow. (`SshChannelTransport` works as-is.)
4. **User pushback was correct:** "Cursor and all other tools can do this." The mechanism they use (Path 1) is one of two paths — but it's the path that costs them a dedicated team. We don't have a dedicated team. Path 2 reaches the same UX with infrastructure that's already stable.
5. **The single biggest risk (Go-host interop for 2b) is testable in <1 day** — set up a tunnel, try `connect_to_port` against `devtunnel host`, see if it works. If it does, 2b is also viable.

**Path 1 is the wrong path for Vector v1.** It optimizes for an architecture (download-and-launch-VS-Code-Server-on-remote, speak vscode-remote) that makes sense if you're already shipping a full IDE that needs language servers, file watchers, extensions, debug protocol. We're a terminal. We don't need any of that — we need a PTY. SSH is built for PTYs.

### Invalidators (would change recommendation back to Path 1 or to defer)

- **`handle_forward` RPC turns out to require complex challenge-response auth that we can't implement in <400 LOC.** Re-evaluate to Variant 2b (separate `devtunnel host -p 22`) or Variant 2c (Vector-tunnel-agent).
- **Smoke test confirms Go-host interop is broken AND `handle_forward` is hard.** Fall to Variant 2c (ship our own agent).
- **A maintained Rust vscode-remote client crate appears on crates.io.** Re-evaluate Path 1 (the cost equation flips).
- **Microsoft publishes a stability commitment for the vscode-remote terminal channel.** Same — Path 1 becomes viable.
- **User refuses to install/run `code tunnel service install`.** Both paths fail; only Variant 2c works. Re-scope.

### Spike output for `.planning/research/spikes/dev-tunnels-decision.md`

```markdown
# Dev Tunnels Decision (Phase 8 Spike)

**Date:** 2026-05-20
**Decision:** (b) vendor `microsoft/dev-tunnels/rs/` — Path 2 Variant 2a.
**Reason:** The Dev Tunnels SDK gives us `RelayTunnelClient::connect_to_port(port) -> PortConnection`, where `PortConnection::into_rw()` returns an `AsyncRead+AsyncWrite` stream plug-compatible with `russh::client::connect_stream`. Combined with the `code tunnel` host's public `handle_forward(port, public)` RPC (port_forwarder.rs), Vector can ask the user's `code tunnel`-hosted machine to expose port 22, then SSH through it using the existing Phase 7 `SshClient`/`SshChannelTransport` scaffolding. This is the path the user's pushback ("Cursor and all other tools can do this") demanded, and it reaches that UX with ~half the engineering cost and a tenth of the long-tail maintenance burden of the vscode-remote protocol-reimplementation path.

**Path 1 (vscode-remote protocol client) is rejected** because:
- Greenfield (no Rust prior art exists, verified by crates.io + Zed's "we avoid this protocol on purpose" precedent).
- Protocol breaks monthly (vscode-remote-release issue tracker shows version-mismatch refusals as a recurring failure mode).
- Cost is 7–10 dev-weeks vs Path 2's 4–6, with ongoing per-VS-Code-release validation work.

**v1 plan:**
- Wave 0: vendor SDK, install russh patch chain, verify smoke build.
- Wave 1: tunnel listing + picker (reuses Phase 6 modal scaffolding).
- Wave 2: msgpack-RPC mini-client capable of one method (`handle_forward`); fall back to Variant 2b if costs creep.
- Wave 3: SSH wiring (SshClient::connect_over the SDK's PortConnectionRW); host-key TOFU prompt UI.
- Wave 4: Manual smoke matrix on real `code tunnel` host (EC2 + Mac home box).

**Trigger to revisit:**
- handle_forward RPC turns out to be too hard → drop to Variant 2b.
- Go-host interop tests for Variant 2b fail → drop to Variant 2c (Vector-tunnel-agent).
- A maintained Rust vscode-remote crate appears → reconsider Path 1.
```

