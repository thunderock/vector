---
phase: 08-vs-code-remote-tunnels-connect
plan: 03
subsystem: agent
tags: [tunnel-agent, dev-tunnels-sdk, pty, json-protocol, russh-0.37-activated, pitfall-14]

requires:
  - phase: 08-vs-code-remote-tunnels-connect
    plan: 01
    provides: tunnels SDK workspace dep declared; russh patch dormant; vector-tunnel-agent crate scaffolded; vector-tunnel-protocol::AgentMessage + PROTOCOL_VERSION

provides:
  - DT-01 (Vector Tunnel Agent binary, Linux user-space) operational end-to-end
  - DT-03 (JSON protocol loop with biased-select pump, PTY spawn, Exit propagation)
  - vector_tunnel_agent::session::run — per-channel session entry point taking AsyncRead+AsyncWrite
  - vector_tunnel_agent::host::run — RelayTunnelHost registration + accept loop
  - vector_tunnel_agent::token_cache — $XDG_CONFIG_HOME/vector/agent-token, mode 0600, atomic temp+rename
  - vector_tunnel_agent::auth — RFC 8628 device flow for GitHub + Microsoft providers
  - workspace deps: clap 4, dirs 5, hostname 0.4, nix 0.29 (added)
  - tunnels SDK activated in dep graph: russh 0.37 (patched via vscode-russh) coexists with vector-ssh's russh 0.60

affects: [08-04-mac-client-transport, 08-05-picker-ui-and-actor, 08-06-agent-distribution, 08-07-uat-smoke-matrix]

tech-stack:
  added:
    - "clap 4 (workspace dep) — derive subcommand parser for the agent CLI"
    - "dirs 5 (workspace dep) — $HOME / XDG_CONFIG_HOME resolution"
    - "hostname 0.4 (workspace dep) — for tunnel name `vector-{hostname}`"
    - "nix 0.29 (workspace dep, features=user,fs) — future POSIX helpers"
  patterns:
    - "Biased-select pump mirrors `vector-ssh/src/transport.rs` shape: priority 1 resize, 2 inbound wire, 3 outbound PTY, 4 child-exit poll"
    - "PTY blocking reader/writer bridged to tokio via spawn_blocking + mpsc::channel (Phase 2 vector-pty pattern)"
    - "Pitfall 3 retained: drop(pty.slave) immediately after spawn_command to prevent zombie shells"
    - "Pitfall 14 manual Debug applied to every token-bearing struct AND any struct/enum within 30 lines of a `device_code:` field (extends Phase-6 vector-codespaces precedent)"

key-files:
  created:
    - crates/vector-tunnel-agent/src/lib.rs
    - crates/vector-tunnel-agent/src/cli.rs
    - crates/vector-tunnel-agent/src/auth.rs
    - crates/vector-tunnel-agent/src/host.rs
    - crates/vector-tunnel-agent/src/session.rs
    - crates/vector-tunnel-agent/src/token_cache.rs
    - crates/vector-tunnel-agent/tests/auth_token_cache.rs
    - crates/vector-tunnel-agent/tests/session_lifecycle.rs
    - .planning/phases/08-vs-code-remote-tunnels-connect/08-03-SUMMARY.md
  modified:
    - Cargo.toml (workspace deps: +clap, +dirs, +hostname, +nix)
    - Cargo.lock (tunnels + russh 0.37 + dual-russh resolution)
    - crates/vector-tunnel-agent/Cargo.toml (lib target + deps + dev-deps)
    - crates/vector-tunnel-agent/src/main.rs (full async runtime + CLI dispatch + tracing init)
    - crates/vector-tunnel-agent/tests/protocol_codec.rs (Wave-0 stubs replaced with 4 real tests)

key-decisions:
  - "Plan called for `Tunnel.labels` as a `HashMap<String,String>` per the C# / TS SDK shape, but the Rust SDK port uses `Vec<String>`. Used the single string label `\"vector-agent\"` (constant `VECTOR_AGENT_LABEL`) — 08-04's filter test should call `tunnel.labels.iter().any(|l| l == \"vector-agent\")`, not a hashmap key/value lookup."
  - "Activated `tunnels = { workspace = true }` on the agent crate this wave, which lights up the `[patch.crates-io] russh -> vscode-russh` patch. Phase-7 vector-ssh on russh 0.60 SURVIVED — Cargo resolves dual versions in the graph (russh 0.37 patched for the tunnels SDK alongside russh 0.60.3 for vector-ssh). The dual-version path is exactly the binary-bloat trade documented in CLAUDE.md tech-stack §7 and 08-01 SUMMARY's risk section."
  - "Agent does NOT depend on `vector-codespaces` or `vector-tunnels`: device flow is reimplemented in ~280 LOC inside the agent to keep the binary small for Linux distribution (Plan 08-06 packages a single static-ish binary). The local impl mirrors Phase 6's `GitHubAuth` polling structure and Plan 08-02's Microsoft `common` authority endpoints — duplication is intentional per plan Step 3 (\"agent reuses this shape locally, NOT depending on vector-codespaces crate to keep agent binary small\")."
  - "Tests gated to multi-thread tokio (`#[tokio::test(flavor = \"multi_thread\", worker_threads = 2)]`) because `tokio::task::spawn_blocking` requires a multi-thread runtime — the PTY reader/writer bridges are blocking-thread tasks."
  - "Resize test asserts `42 120` substring in `stty size` output instead of `#[cfg(target_os=\"linux\")]`-gating the test off macOS — empirically the test passes on macOS too in practice (the plan permitted Linux-only gating but it wasn't necessary). All 3 session_lifecycle tests run on every host."

patterns-established:
  - "Agent-side OAuth device flow as a self-contained module (~280 LOC) instead of depending on the vector-tunnels/vector-codespaces crates — keeps the Linux binary small"
  - "Per-channel session as `pub async fn run<S>(stream: S) -> Result<()>` where `S: AsyncRead+AsyncWrite+Unpin+Send+'static` — tests use `tokio::io::duplex` to drive without a real relay"
  - "Forbid `#[derive(Debug)]` in any source file containing `device_code:`/`access_token:`/`refresh_token:`/`agent_token:`/`tunnel_access_token:` fields — Pitfall 14 arch-lint enforces with a 30-line window; we satisfy with manual `impl Debug for AgentTokenError { write!(f, \"{self}\") }` shorthand"

requirements-completed: [DT-01, DT-03]

metrics:
  duration: 14min
  completed: 2026-05-21
  tasks: 2
  files: 11
---

# Phase 8 Plan 03: Tunnel Agent Binary Summary

**Linux user-space `vector-tunnel-agent` binary ships end-to-end: clap-derive CLI (run/reauth/status/--version/--help) + condensed RFC-8628 device flow for both GitHub and Microsoft + atomic 0600 token cache at `~/.config/vector/agent-token` + RelayTunnelHost registration (label `vector-agent`, name `vector-{hostname}`) + JSON-framed protocol pump on each accepted relay channel with biased-select (resize > inbound > outbound > child-poll) + Pitfall-3 zombie-shell prevention. 12 tests green. tunnels SDK activated; vector-ssh survived dual-russh resolution.**

## Performance

- **Duration:** ~14 min
- **Started:** 2026-05-21T21:03:40Z
- **Completed:** 2026-05-21T21:17:29Z
- **Tasks:** 2 (TDD: RED + GREEN per task)
- **Files modified/created:** 11 (8 created, 3 modified)

## SDK API Surface Used (Verified Against microsoft/dev-tunnels@64048c1)

The plan's SDK pseudo-code did not match the actual Rust port — verified each call site against `~/.cargo/git/checkouts/dev-tunnels-aaeb61f56ce55f37/64048c1/rs/src/` at execution time. Final surface:

| API | Signature / Notes |
| --- | ----------------- |
| `tunnels::management::new_tunnel_management(&str)` | Returns `TunnelClientBuilder` |
| `TunnelClientBuilder::authorization(Authorization) -> &mut Self` | Mutable-builder; **no `.build()` method** — finalize via `.into() : TunnelManagementClient` |
| `Authorization::Github(String)` | GitHub access token (renders `github gho_...` header) |
| `Authorization::AAD(String)` | Microsoft / AAD access token (renders `aad <jwt>` header) |
| `Authorization::Bearer(String)` | Generic bearer — not used here |
| `TunnelManagementClient::create_tunnel(Tunnel, &TunnelRequestOptions)` | Returns `HttpResult<Tunnel>` with `access_tokens` populated |
| `Tunnel.labels: Vec<String>` | **Plan said HashMap, actually Vec.** Use `"vector-agent"` as the magic string. |
| `TunnelLocator::try_from(&Tunnel)` | Returns `TunnelLocator::ID { cluster, id }` if both fields populated |
| `RelayTunnelHost::new(TunnelLocator, TunnelManagementClient) -> Self` | Constructor |
| `RelayTunnelHost::connect(&mut self, &str) -> Result<RelayHandle, TunnelError>` | Pass `access_tokens["host"]` |
| `RelayTunnelHost::add_port_raw(&TunnelPort) -> mpsc::UnboundedReceiver<ForwardedPortConnection>` | One channel per port; `recv()` yields each incoming client |
| `RelayTunnelHost::unregister() -> Result<bool, TunnelError>` | Called on shutdown |
| `ForwardedPortConnection::into_rw() -> ForwardedPortRW` | `AsyncRead + AsyncWrite + Unpin` — handed straight to `session::run` |

## SDK API Deviations from 08-RESEARCH.md

| Plan / RESEARCH said | Reality (at SHA 64048c1) | Impact |
| -------------------- | ------------------------- | ------ |
| `Tunnel.labels: HashMap<String,String>` | `Vec<String>` | Used single string label `"vector-agent"`; 08-04 picker must match via `labels.iter().any(...)` not hashmap lookup |
| `TunnelClientBuilder` has `.build()` | Builder finalizes via `.into() : TunnelManagementClient` (`impl From<TunnelClientBuilder> for TunnelManagementClient`) | Cosmetic — corrected at exec time |
| `authorization_provider(impl AuthorizationProvider + 'static)` was the only auth API | `.authorization(Authorization)` exists as a sugar method that wraps in `StaticAuthorizationProvider` internally | Used the simpler API — no need to define our own provider trait impl |
| Inbound channels yielded by a generic `accept_next()` on the host | `add_port_raw(&port)` returns `mpsc::UnboundedReceiver<ForwardedPortConnection>` keyed per port | Used the per-port channel model |
| Authorization variant names guessed at plan time | Confirmed: `Authorization::Github` (capital G) for GitHub, `Authorization::AAD` for Microsoft/AAD | Plan's pseudocode `Authorization::Bearer` for Microsoft was wrong — `AAD` is the correct variant |

## Workspace Deps Pinned

Four workspace-level pins added to `Cargo.toml`:

| Crate | Version | Purpose |
| ----- | ------- | ------- |
| `clap` | `4` (features `derive`) | Agent CLI |
| `dirs` | `5` | `$HOME` / `XDG_CONFIG_HOME` resolution |
| `hostname` | `0.4` | Tunnel name `vector-{hostname}` |
| `nix` | `0.29` (features `user`, `fs`) | Reserved for future POSIX helpers |

## Tests

| File | Tests | Notes |
| ---- | ----- | ----- |
| `tests/auth_token_cache.rs` | 5 | XDG path, 0600 mode, round-trip, missing-file → Ok(None), corrupted → Err |
| `tests/protocol_codec.rs` | 4 | Round-trip, partial frame, two-frame, protocol_version mismatch → Error + close |
| `tests/session_lifecycle.rs` | 3 | PTY echo round-trip (printf HELLO_WORLD), Exit on shell exit, Resize → `stty size` reports `42 120` |
| **Total** | **12** | All cross-platform — no `#[cfg(target_os="linux")]` gating needed (works on macOS dev host too) |
| Linux-only tests | 0 | Plan permitted gating but it wasn't necessary in practice |
| `#[ignore]` tests | 0 | Wave-0 stubs from 08-01 fully replaced |

`cargo test -p vector-tunnel-agent`: **12 passed / 0 failed / 0 ignored** (with multi-thread tokio runtime; required for the spawn_blocking PTY bridge tasks).

## Phase 7 vector-ssh Status: SURVIVED (dual-russh)

The 08-01 SUMMARY warned that activating the workspace russh patch by depending on the tunnels SDK may break Phase-7 `vector-ssh` (russh 0.60). Result: **no escalation needed.** Cargo resolves dual versions in the dep graph:

- `russh 0.60.3` (crates.io) → consumed by `vector-ssh` ← unchanged
- `russh 0.37.1` (patched via `microsoft/vscode-russh`) → consumed by the `tunnels` SDK

`cargo build -p vector-ssh` exit 0 with zero source changes; vector-ssh's russh-0.60 API surface (`russh::client::Handle`, `russh::keys::PrivateKey`, etc.) is intact. The trade is binary bloat (~3 MB extra) — acceptable for v1. Plan 08-04 will face the same question when the Mac client picks up `vector-tunnels`; this plan's result establishes the dual-russh resolution as viable.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Critical Functionality] Added `[lib]` target to vector-tunnel-agent so integration tests can import modules**

- **Found during:** Task 1 RED cargo test
- **Issue:** Plan 08-01 scaffolded the crate as `[[bin]]`-only, but Task 1's `tests/auth_token_cache.rs` (and Task 2's session_lifecycle.rs) need to `use vector_tunnel_agent::token_cache::*` and `use vector_tunnel_agent::session::run`. Without a library target the symbols aren't reachable.
- **Fix:** Added `[lib] name = "vector_tunnel_agent" path = "src/lib.rs"` to the crate's Cargo.toml plus a `src/lib.rs` that re-exports every module (`pub mod auth; pub mod cli; pub mod host; pub mod session; pub mod token_cache;`). `main.rs` now imports from `vector_tunnel_agent::{cli, host, token_cache}` instead of declaring them as inline `mod`s.
- **Files modified:** crates/vector-tunnel-agent/Cargo.toml, crates/vector-tunnel-agent/src/lib.rs (new), crates/vector-tunnel-agent/src/main.rs
- **Committed in:** 636afd5 (Task 1 commit)

**2. [Rule 1 - Pitfall-14 arch-lint violation] `AgentAuthError` was within 30 lines of `device_code:` field**

- **Found during:** Task 1 GREEN — first `cargo test -p vector-arch-tests` run
- **Issue:** The arch-lint regex bans `#[derive(...Debug...)]` within 30 lines of any field named `device_code | access_token | refresh_token | user_code | client_secret | agent_token | tunnel_access_token`. My `auth.rs` had `#[derive(thiserror::Error, Debug)] pub enum AgentAuthError` immediately followed (within 30 lines) by `struct DeviceCodeReply { device_code: String, ... }`. The error enum has no token-bearing variants but the lint's window catches it.
- **Fix:** Dropped `Debug` from the derive list on `AgentAuthError` and added `impl std::fmt::Debug for AgentAuthError { fn fmt(...) { write!(f, "{self}") } }`. Same shorthand applied to `Provider` and `AgentTokenError` in `token_cache.rs` to keep the literal acceptance grep (`! grep -E "#\[derive\([^)]*Debug" ...token_cache.rs"`) green even though those types have no token fields at all.
- **Files modified:** crates/vector-tunnel-agent/src/auth.rs, crates/vector-tunnel-agent/src/token_cache.rs
- **Verification:** `cargo test -p vector-arch-tests --tests` → 7 passed / 0 failed
- **Committed in:** 636afd5 (Task 1 commit)

**3. [Rule 3 - Blocking dep contract] SDK API surface differed from plan's pseudocode**

- **Found during:** Task 2 GREEN — first build of `host.rs`
- **Issue:** Plan's pseudocode used `TunnelClientBuilder::authorization_provider(Arc::new(Box::new(provider)))` + `.build()`. Reality: `.authorization_provider(impl AuthorizationProvider + 'static)` takes a value (not `Arc<Box<_>>`), and there's no `.build()` — the builder finalizes via `.into() : TunnelManagementClient`.
- **Fix:** Switched to the simpler `.authorization(Authorization)` sugar method (which the SDK provides for static auth) and used `Ok(builder.into())`. Documented the surface used at the top of `host.rs`.
- **Files modified:** crates/vector-tunnel-agent/src/host.rs
- **Committed in:** fd5178d (Task 2 commit)

**4. [Rule 1 - clippy pedantic] Multiple clippy fixups**

- **Found during:** Task 2 clippy gate
- **Issue:** `unused_async` on placeholder stubs, `match`→`if let`, `unnested_or_patterns`, `redundant_closure_for_method_calls`, `manual_let_else`, `too_many_lines` on the biased-select pump (177/100).
- **Fix:**
  - Stub functions in `host.rs` / `session.rs` annotated `#[allow(clippy::unused_async)]` (replaced with real bodies in Task 2; the `status()` stub keeps the allow for forward-compat with a future Management API tunnel-state query).
  - `match` patterns refactored to `if let` / `let ... else` where applicable.
  - Or-pattern `Some('m') | Some('M')` collapsed to `Some('m' | 'M')`.
  - The `session::run` pump function got `#[allow(clippy::too_many_lines)]` — it's intentionally one function to keep the biased-select arms colocated (mirrors `vector-ssh/src/transport.rs::channel_task` precedent).
- **Files modified:** crates/vector-tunnel-agent/src/{auth,host,session}.rs, crates/vector-tunnel-agent/tests/{auth_token_cache,session_lifecycle}.rs
- **Verification:** `cargo clippy -p vector-tunnel-agent --all-targets -- -D warnings` exit 0
- **Committed in:** fd5178d (Task 2 commit)

---

**Total deviations:** 4 (1 critical missing functionality auto-added; 3 mechanical clippy/arch-lint fixups).
**Impact on plan:** No semantic change. The `[lib]` target addition is the only structural change beyond what the plan called out; it was clearly required by the integration-test contract and is invisible to consumers (the binary still ships as `target/{debug,release}/vector-tunnel-agent`).

## Known Stubs

`auth::run_first_run_device_flow` blocks on synchronous stdin (`std::io::stdin().lock().read_line`) to choose the provider. This is correct for a daemon's first interactive run, but on subsequent runs the token is loaded from disk (`ensure_token`) and stdin is never touched. No data flows to UI here — agent is a Linux daemon.

`host::status` currently prints local token info only (provider + `expires_at_unix`); it does NOT query the live Management API for tunnel state. The `#[allow(clippy::unused_async)]` documents this is intentional — a future enhancement may await a live state query. Out of scope for v1.

## Issues Encountered

- **SDK has no published `Authorization::Microsoft` variant.** The Rust port uses `Authorization::AAD(String)` for Microsoft / Entra access tokens (renders `aad <jwt>` header). The plan's pseudocode `Authorization::Bearer` was incorrect — Bearer is for tunnel-scoped tokens, not user-OAuth bearers. Fixed in `build_mgmt_client`.
- **`Tunnel.labels` schema diverges between Rust port and the spec docs.** C# / TS SDKs use a `Dictionary<String,String>` for labels; the Rust port uses `Vec<String>`. Plan 08-04's filter test needs to know this — documented in `key-decisions` above.

## Self-Check: PASSED

**Files verified to exist:**

- FOUND: crates/vector-tunnel-agent/src/lib.rs
- FOUND: crates/vector-tunnel-agent/src/cli.rs
- FOUND: crates/vector-tunnel-agent/src/auth.rs (351 lines)
- FOUND: crates/vector-tunnel-agent/src/host.rs (186 lines)
- FOUND: crates/vector-tunnel-agent/src/session.rs (244 lines)
- FOUND: crates/vector-tunnel-agent/src/token_cache.rs (107 lines)
- FOUND: crates/vector-tunnel-agent/tests/auth_token_cache.rs
- FOUND: crates/vector-tunnel-agent/tests/protocol_codec.rs
- FOUND: crates/vector-tunnel-agent/tests/session_lifecycle.rs

**Commits verified in git log:**

- FOUND: 4d333c2 (test 08-03 RED — auth_token_cache)
- FOUND: 636afd5 (feat 08-03 Task 1 GREEN — CLI + device flow + token cache + lib.rs)
- FOUND: 3d7913a (test 08-03 RED — protocol_codec + session_lifecycle)
- FOUND: fd5178d (feat 08-03 Task 2 GREEN — RelayTunnelHost + session pump)

**Acceptance gates verified:**

- `cargo build -p vector-tunnel-agent` exit 0
- `./target/debug/vector-tunnel-agent --version` prints `vector-tunnel-agent 2026.5.10`
- `./target/debug/vector-tunnel-agent --help` lists `run`, `reauth`, `status`
- `cargo test -p vector-tunnel-agent`: 12 passed / 0 failed
- `cargo clippy -p vector-tunnel-agent --all-targets -- -D warnings` exit 0
- `cargo test -p vector-arch-tests --tests`: 7 passed / 0 failed
- `grep -c "0o600" .../token_cache.rs` = 1
- `grep -c "0o700" .../token_cache.rs` = 1
- `grep -c "impl std::fmt::Debug for CachedToken" .../token_cache.rs` = 1
- `grep -E "#\[derive\([^)]*Debug" .../token_cache.rs` returns nothing (literal acceptance grep clean)
- `grep -E "#\[derive\([^)]*Debug" .../auth.rs` returns nothing
- `grep -c "drop(pty.slave)" .../session.rs` = 1 (Pitfall 3)
- `grep -c "biased" .../session.rs` = 3 (biased select for resize priority)
- `grep -c "\"vector-agent\"" .../host.rs` = 2 (D-10 label key — doc + const)
- `grep -c "vector-" .../host.rs` = 9 (D-09 name prefix + comments)
- `grep -c "TERM" .../session.rs` = 1 (xterm-256color advertised)
- `cargo build -p vector-ssh` exit 0 (russh 0.60 unaffected by dual-version resolution)

## Next Phase / Plan Readiness

- **Plan 08-04 (Mac client + transport):** Inherits the wire-protocol contract proven by `tests/protocol_codec.rs` + `tests/session_lifecycle.rs`. The label filter must use `Vec<String>::iter().any(|l| l == "vector-agent")` against the Rust SDK's `Tunnel.labels`, NOT a hashmap lookup. Mac client connects to `VECTOR_PTY_PORT = 16632` on each tunnel. Russh patch is now live in the dep graph; vector-ssh on russh 0.60 remains intact in dual-version form.
- **Plan 08-05 (Picker UI + actor):** Inherits the agent registration contract — picker lists tunnels filtered by the `vector-agent` label and strips the `vector-` prefix from the name for display.
- **Plan 08-06 (Agent .deb / install script):** Inherits the working `vector-tunnel-agent` binary, the `~/.config/vector/agent-token` file convention (mode 0600, atomic temp+rename), and the systemd-compatible signal handling (SIGTERM/SIGINT → unregister + exit 0).
- **Plan 08-07 (UAT smoke matrix):** Inherits a functioning end-to-end loop: agent on Linux registers tunnel → Mac picker sees it → connects → JSON-framed PTY shell. The 12 unit/integration tests in this plan establish the protocol contract before manual UAT.

---
*Phase: 08-vs-code-remote-tunnels-connect*
*Completed: 2026-05-21*
