---
phase: 08-vs-code-remote-tunnels-connect
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - crates/vector-tunnels/Cargo.toml
  - crates/vector-tunnels/src/lib.rs
  - crates/vector-tunnels/tests/list_tunnels.rs
  - crates/vector-tunnels/tests/fixtures/dev_tunnels_list.json
  - crates/vector-tunnel-agent/Cargo.toml
  - crates/vector-tunnel-agent/src/main.rs
  - crates/vector-tunnel-agent/tests/protocol_codec.rs
  - crates/vector-tunnel-protocol/Cargo.toml
  - crates/vector-tunnel-protocol/src/lib.rs
  - crates/vector-tunnel-protocol/tests/messages.rs
  - crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs
  - crates/vector-secrets/src/lib.rs
  - crates/vector-secrets/tests/microsoft_account.rs
  - .planning/research/spikes/dev-tunnels-decision.md
autonomous: true
requirements:
  - DT-01
  - DT-02
  - DT-03
  - DT-04
user_setup: []
must_haves:
  truths:
    - "DT-01 spike decision document is committed at `.planning/research/spikes/dev-tunnels-decision.md` BEFORE any integration code lands (ROADMAP §Phase 8 SC#1)"
    - "Workspace builds with new vendored microsoft/dev-tunnels rs/ git dep + russh patch applied"
    - "Three new crates exist as workspace members: vector-tunnels (fill-out), vector-tunnel-agent (new binary), vector-tunnel-protocol (new shared lib)"
    - "vector-secrets exposes MICROSOFT_REFRESH_ACCOUNT constant"
    - "Pitfall-14 arch-lint scans all three new crates"
    - "Wave-0 #[ignore] test stubs exist for every Phase 8 requirement"
  artifacts:
    - path: ".planning/research/spikes/dev-tunnels-decision.md"
      provides: "DT-01 spike outcome — codification of Path 2 Variant 2c decision (locked in 08-CONTEXT.md D-A1 and 08-RESEARCH.md)"
      min_lines: 30
    - path: "Cargo.toml"
      provides: "workspace deps: tunnels (git pin), tokio-tungstenite, base64, msgpack-rpc (optional), [patch.crates-io] russh = vscode-russh"
      contains: "vector-tunnel-agent"
    - path: "crates/vector-tunnel-protocol/src/lib.rs"
      provides: "AgentMessage enum + serde JSON codec + PROTOCOL_VERSION const"
      min_lines: 30
    - path: "crates/vector-tunnel-agent/src/main.rs"
      provides: "binary entry point stub (CLI parsing + run/--reauth dispatch placeholder)"
    - path: "crates/vector-tunnels/src/lib.rs"
      provides: "module surface: api, auth, transport, domain"
    - path: "crates/vector-secrets/src/lib.rs"
      provides: "MICROSOFT_REFRESH_ACCOUNT constant"
      contains: "MICROSOFT_REFRESH_ACCOUNT"
    - path: "crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs"
      provides: "extended scan of vector-tunnels + vector-tunnel-agent + vector-tunnel-protocol"
  key_links:
    - from: "Cargo.toml [workspace.dependencies]"
      to: "microsoft/dev-tunnels rs/ at pinned SHA"
      via: "tunnels = { git = ..., rev = ... }"
      pattern: "tunnels.*git.*microsoft/dev-tunnels"
    - from: "Cargo.toml [patch.crates-io]"
      to: "microsoft/vscode-russh"
      via: "[patch.crates-io] russh = { git = ... }"
      pattern: "patch\\.crates-io"
    - from: "crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs"
      to: "crates/vector-tunnel-agent/src + crates/vector-tunnels/src + crates/vector-tunnel-protocol/src"
      via: "scan path list"
      pattern: "vector-tunnel-agent|vector-tunnel-protocol"
---

<objective>
Wave 0 foundations: (Step 0) commit DT-01 spike decision document so SC#1 is satisfied BEFORE any integration code lands; then vendor microsoft/dev-tunnels SDK, apply russh patch, scaffold the three new crates (`vector-tunnels` fill-out, `vector-tunnel-agent` binary, `vector-tunnel-protocol` shared lib), extend `vector-secrets` with `MICROSOFT_REFRESH_ACCOUNT`, extend Pitfall-14 arch-lint to cover the new crates, land all Wave-0 #[ignore] test stubs that downstream waves flip green.

Purpose: lock spike decision + crate surface + dep graph + arch-lint scope before any feature code lands so Waves 1+ can run in parallel against a stable foundation.
Output: spike doc committed; workspace compiles with `cargo build --workspace`; arch-lint passes; 12+ #[ignore] test stubs land that map to DT-01..04.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/08-vs-code-remote-tunnels-connect/08-CONTEXT.md
@.planning/phases/08-vs-code-remote-tunnels-connect/08-RESEARCH.md
@.planning/phases/08-vs-code-remote-tunnels-connect/08-VALIDATION.md
@Cargo.toml
@crates/vector-tunnels/Cargo.toml
@crates/vector-tunnels/src/lib.rs
@crates/vector-secrets/src/lib.rs
@crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs

<interfaces>
From crates/vector-secrets/src/lib.rs (existing pattern to mirror):
```rust
impl Secrets {
    pub const VECTOR_SERVICE: &str = "vector";
    pub const GITHUB_OAUTH_ACCOUNT: &str = "github_oauth_token";
    pub const GITHUB_REFRESH_ACCOUNT: &str = "github_refresh_token";
}
```
This plan adds: `pub const MICROSOFT_REFRESH_ACCOUNT: &str = "microsoft_refresh_token";`

From crates/vector-mux/src/transport.rs (already shipped Phase 7):
```rust
pub enum TransportKind { Local, DevTunnel }
pub trait PtyTransport: Send + 'static { ... }
```
DevTunnel variant already present; vector-tunnels DevTunnelTransport in Wave 2 returns `TransportKind::DevTunnel`.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Spike decision doc + workspace deps + russh patch + scaffold three crates</name>
  <files>.planning/research/spikes/dev-tunnels-decision.md, Cargo.toml, crates/vector-tunnels/Cargo.toml, crates/vector-tunnels/src/lib.rs, crates/vector-tunnel-agent/Cargo.toml, crates/vector-tunnel-agent/src/main.rs, crates/vector-tunnel-protocol/Cargo.toml, crates/vector-tunnel-protocol/src/lib.rs, crates/vector-secrets/src/lib.rs</files>
  <read_first>
    - .planning/phases/08-vs-code-remote-tunnels-connect/08-RESEARCH.md (Day-1 Spike Decision Recommendation — locked 2026-05-20)
    - .planning/phases/08-vs-code-remote-tunnels-connect/08-CONTEXT.md (D-A1 — Path 2c locked architecture)
    - Cargo.toml (workspace root — must understand current `[workspace.dependencies]` shape before extending)
    - crates/vector-tunnels/Cargo.toml + crates/vector-tunnels/src/lib.rs (existing stub — preserve license/description fields)
    - crates/vector-secrets/src/lib.rs (existing constants pattern to mirror)
    - crates/vector-codespaces/src/lib.rs + crates/vector-codespaces/Cargo.toml (reference for module layout + dep style)
  </read_first>
  <behavior>
    - Test 1 (vector-tunnel-protocol/tests/messages.rs): `AgentMessage::OpenPty { protocol_version: 1, rows: 24, cols: 80, shell: None }` round-trips through serde_json::to_string + from_str byte-identical
    - Test 2 (vector-tunnel-protocol/tests/messages.rs): `AgentMessage::Data { session: "s1".into(), bytes: vec![0xc3, 0xa9] }` serializes the `bytes` field as base64 string (e.g. `"w6k="`) NOT as `[195,169]` array
    - Test 3 (vector-tunnel-protocol/tests/messages.rs): unknown `op` field deserializes to `AgentMessage::Unknown` variant (forward-compat) — assert no panic on `{"op":"future_thing"}`
    - Test 4 (vector-tunnel-protocol/tests/messages.rs): `PROTOCOL_VERSION` const is exactly 1 (u32)
    - Test 5 (vector-secrets/tests/microsoft_account.rs): `Secrets::MICROSOFT_REFRESH_ACCOUNT == "microsoft_refresh_token"`
    - Test 6 (vector-tunnels/tests/list_tunnels.rs): file exists and contains 3+ `#[ignore]` stubs named `list_tunnels_filters_to_vector_agent_label`, `list_tunnels_handles_401`, `list_tunnels_strips_vector_prefix`
    - Test 7 (vector-tunnel-agent/tests/protocol_codec.rs): file exists and contains 2+ `#[ignore]` stubs named `agent_echoes_data_frames`, `agent_emits_exit_on_shell_exit`
  </behavior>
  <action>
    Step 0 — DT-01 spike decision document (ROADMAP §Phase 8 SC#1: must commit BEFORE any integration code).

    Create `.planning/research/spikes/dev-tunnels-decision.md` with the following content. This is a codification of the locked decision from 08-RESEARCH.md (per user decision D-A1, 08-CONTEXT.md); no new analysis is performed — it's a one-shot, derived deliverable. After committing, the rest of this task (deps + scaffolds) proceeds.

    ```markdown
    # Dev Tunnels Decision (Phase 8 Spike — DT-01)

    **Decision date:** 2026-05-20
    **Decision:** (b) Path 2 Variant 2c — Vector Tunnel Agent.
    **Status:** LOCKED by user (08-CONTEXT.md D-A1).

    ## Decision

    Vector ships its own user-space Linux binary (`vector-tunnel-agent`) that the
    user installs on each remote box alongside (or instead of) `code tunnel`.
    The agent uses `microsoft/dev-tunnels` rs/ as a Host; the Mac client uses
    the same SDK as a Client. Both sides speak a small Vector-controlled
    framed JSON protocol on the relay channel.

    No sshd dependency. No vscode-remote protocol. No VPN.

    ## Why (a) and Path 1 were rejected

    - **(a) subprocess `code tunnel client`:** `code tunnel client` does not
      exist as a CLI subcommand. The `devtunnel connect` standalone CLI
      forwards ports but does not give a shell. Path eliminated.
    - **Path 1 — vscode-remote protocol client:** would require reimplementing
      a Microsoft-internal IPC protocol (terminal channel + IPCRPCProtocol
      framing) with no maintained Rust prior art. Monthly upstream breakage.
      Cost: 7–10 dev-weeks + ongoing maintenance treadmill. Path eliminated.
    - **Path 2 Variant 2a (SSH over `handle_forward(22)` RPC):** would require
      sshd on the remote box AND a partial msgpack-RPC client. The user's
      target machines (Adobe corporate box + personal) do NOT expose sshd
      outside their VPN, and the user does not want a VPN dependency. Path
      eliminated.
    - **Path 2 Variant 2b (`devtunnel host -p 22` separate process):** same
      sshd-not-available constraint as 2a. Plus `devtunnel host` is Go and
      may not interop with the Rust SDK's `direct-tcpip` channels. Path
      eliminated.

    ## Why (b) Path 2 Variant 2c was chosen

    1. **No sshd required.** Agent handles PTY natively via `portable-pty 0.9`.
       Compatible with corporate machines that block inbound SSH.
    2. **No VPN required.** Dev Tunnels relay is outbound-only from both client
       and host.
    3. **Microsoft-stable transport.** The SDK's `RelayTunnelHost` + WebSocket +
       russh layer is Microsoft's own VS Code CLI transport. Stable for years.
    4. **Vector owns the protocol above the SDK.** Small, simple, versioned
       (D-15 `protocol_version: 1`). No third-party protocol-drift risk.
    5. **Reuses Phase 7 patterns.** Biased select for resize > write > read,
       `Box<dyn PtyTransport>` via `Mux::create_tab_async_with_transport`,
       `TransportKind::DevTunnel` + `[remote]` badge already wired.
    6. **Cost: ~3-4 calendar weeks** (5-7 dev-weeks). Within Phase 8 budget.

    ## Carry-over from Phase 7

    - `crates/vector-ssh/` — Phase 7 scaffolding remains in tree but is NOT used
      by Phase 8. Phase 9 (persistence + reconnect + plain-SSH future) may reuse.
    - `Mux::create_tab_async_with_transport` — install seam (unchanged).
    - `TransportKind::DevTunnel`, `format_tab_title` `[remote]` badge — wired.

    ## v1 commitment

    - New crate: `vector-tunnel-protocol` (shared message types, JSON+base64 codec).
    - New crate: `vector-tunnel-agent` (Linux binary, installable as Debian `.deb`).
    - Filled-out crate: `vector-tunnels` (Mac client + Dev Tunnels REST + DevTunnelTransport + DevTunnelDomain).
    - Vendor `microsoft/dev-tunnels rs/` at pinned SHA `64048c1409ff56cb958b879de7ea069ec71edc8b`.
    - Workspace `[patch.crates-io] russh = vscode-russh`.
    - Two auth providers: GitHub OAuth (existing) + Microsoft OAuth `common` authority (new).
    - CI distribution: Linux x86_64 + aarch64 `.deb` attached to GitHub Releases.

    ## Invalidators (would re-open the decision)

    - **SDK regression in `RelayTunnelHost`:** if `microsoft/dev-tunnels rs/`
      archives or breaks the Host API → re-evaluate (likely fall to plain SSH
      + VPN tolerance, or fork the SDK).
    - **Adobe IT blocks `vector-tunnel-agent`:** if the user's company IT
      blocks arbitrary user-space binaries → fall back reluctantly to Path 1
      (vscode-remote + accept the cost) or wrap `code tunnel` as a subprocess
      if it ever gains a shell endpoint.
    - **`code tunnel` ships a shell endpoint upstream:** if a `pty: true` field
      lands on `SpawnParams` in `vscode/cli/src/tunnels/protocol.rs` → Path 1
      collapses to a thin RPC wrapper and becomes preferable.

    ## Plan references

    - Plan 08-01: Wave 0 foundations (vendor SDK + russh patch + scaffolds) — this plan also commits this spike doc.
    - Plan 08-02: Microsoft OAuth Device Flow + Keychain storage.
    - Plan 08-03: vector-tunnel-agent binary (RelayTunnelHost + PTY + protocol loop).
    - Plan 08-04: Mac client (vector-tunnels) — REST + DevTunnelTransport + DevTunnelDomain.
    - Plan 08-05: Picker UI + Microsoft sign-in modal + actor + keymap + tint.
    - Plan 08-06: Linux .deb packaging + GitHub Actions release workflow.
    - Plan 08-07: manual UAT smoke matrix (verifies this spike doc exists in Item 1).
    ```

    Step 1 — Workspace Cargo.toml:
    1.1  Add to `[workspace.members]`: `"crates/vector-tunnel-agent"`, `"crates/vector-tunnel-protocol"`.
    1.2  Add to `[workspace.dependencies]`:
         - `tunnels = { git = "https://github.com/microsoft/dev-tunnels", rev = "64048c1409ff56cb958b879de7ea069ec71edc8b", features = ["connections"] }`
         - `tokio-tungstenite = "0.29"` (transitive but pin for stability)
         - `vector-tunnel-protocol = { path = "crates/vector-tunnel-protocol" }`
         - `vector-tunnels = { path = "crates/vector-tunnels" }`
    1.3  Add `[patch.crates-io]` section (workspace root, NOT inside `[workspace]`):
         ```toml
         [patch.crates-io]
         russh = { git = "https://github.com/microsoft/vscode-russh", branch = "main" }
         russh-keys = { git = "https://github.com/microsoft/vscode-russh", branch = "main" }
         russh-cryptovec = { git = "https://github.com/microsoft/vscode-russh", branch = "main" }
         ```
         Note: this downgrades workspace russh from 0.60 to 0.37 surface — Phase 7's `vector-ssh` already compiled before; rebuild verifies it still does. If `vector-ssh` breaks on 0.37 API, Task 1 stops and a follow-up patch task lands BEFORE proceeding (escalate to user — do not blindly modify Phase 7 code).
    1.4  Use `base64 = "0.22"` (already workspace dep) for protocol byte encoding.

    Step 2 — `crates/vector-tunnel-protocol/Cargo.toml` (NEW):
    ```toml
    [package]
    name = "vector-tunnel-protocol"
    version.workspace = true
    edition.workspace = true
    rust-version.workspace = true
    license.workspace = true
    description = "Vector Tunnel Agent wire protocol — JSON frames between Mac client and agent."

    [lints]
    workspace = true

    [dependencies]
    serde = { workspace = true }
    serde_json = { workspace = true }
    base64 = { workspace = true }
    thiserror = { workspace = true }
    ```

    Step 3 — `crates/vector-tunnel-protocol/src/lib.rs` (NEW): per D-12/D-13/D-15 wire format
    ```rust
    //! Vector Tunnel Agent wire protocol. JSON, newline-delimited per D-12.
    //! See 08-CONTEXT.md §<decisions> D-12 through D-15.

    use serde::{Deserialize, Serialize};

    pub const PROTOCOL_VERSION: u32 = 1;

    /// Wire messages. `op` is the JSON tag; `bytes` fields are base64-encoded
    /// in the on-wire form via the serde_bytes_b64 module.
    #[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
    #[serde(tag = "op", rename_all = "snake_case")]
    pub enum AgentMessage {
        OpenPty { protocol_version: u32, rows: u16, cols: u16, shell: Option<String> },
        Opened  { protocol_version: u32, session: String },
        Data    { session: String, #[serde(with = "serde_bytes_b64")] bytes: Vec<u8> },
        Resize  { session: String, rows: u16, cols: u16 },
        Exit    { session: String, code: i32 },
        Error   { reason: String },
        #[serde(other)]
        Unknown,
    }

    // Manual Debug — never include `bytes` content (could contain shell output / secrets).
    impl std::fmt::Debug for AgentMessage {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::OpenPty { rows, cols, shell, .. } =>
                    f.debug_struct("OpenPty").field("rows", rows).field("cols", cols).field("shell", shell).finish(),
                Self::Opened { session, .. } => f.debug_struct("Opened").field("session", session).finish(),
                Self::Data { session, bytes } =>
                    f.debug_struct("Data").field("session", session).field("bytes_len", &bytes.len()).finish(),
                Self::Resize { session, rows, cols } =>
                    f.debug_struct("Resize").field("session", session).field("rows", rows).field("cols", cols).finish(),
                Self::Exit { session, code } => f.debug_struct("Exit").field("session", session).field("code", code).finish(),
                Self::Error { reason } => f.debug_struct("Error").field("reason", reason).finish(),
                Self::Unknown => f.write_str("Unknown"),
            }
        }
    }

    mod serde_bytes_b64 {
        use base64::{engine::general_purpose::STANDARD, Engine};
        use serde::{Deserialize, Deserializer, Serializer};
        pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
            s.serialize_str(&STANDARD.encode(v))
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
            let s = String::deserialize(d)?;
            STANDARD.decode(s).map_err(serde::de::Error::custom)
        }
    }
    ```

    Step 4 — `crates/vector-tunnel-protocol/tests/messages.rs` (NEW): land Tests 1–4 above. Set status to running (not `#[ignore]`).

    Step 5 — `crates/vector-tunnel-agent/Cargo.toml` (NEW):
    ```toml
    [package]
    name = "vector-tunnel-agent"
    version.workspace = true
    edition.workspace = true
    rust-version.workspace = true
    license.workspace = true
    description = "Vector Tunnel Agent — Linux user-space binary that hosts a Dev Tunnel and serves PTY shells (Phase 8 D-A1 Path 2c)."

    [lints]
    workspace = true

    [[bin]]
    name = "vector-tunnel-agent"
    path = "src/main.rs"

    [dependencies]
    anyhow = { workspace = true }
    tokio = { workspace = true, features = ["rt-multi-thread", "macros", "time", "sync", "io-util", "process"] }
    tracing = { workspace = true }
    tracing-subscriber = { workspace = true }
    serde = { workspace = true }
    serde_json = { workspace = true }
    portable-pty = { workspace = true }
    vector-tunnel-protocol = { workspace = true }
    base64 = { workspace = true }
    oauth2 = { workspace = true }
    keyring-core = { workspace = true }
    reqwest = { workspace = true }
    thiserror = { workspace = true }
    # tunnels = { workspace = true }  # Wave 1 (Plan 04) wires; keep out of dep graph this wave to keep Wave 0 minimal-surface
    ```

    Step 6 — `crates/vector-tunnel-agent/src/main.rs` (NEW, stub):
    ```rust
    //! vector-tunnel-agent — Linux user-space binary. Phase 8 Wave 0 = stub.
    //! Wave 1 (Plan 08-04) fills in RelayTunnelHost + PTY spawn + protocol loop.

    fn main() -> anyhow::Result<()> {
        let args: Vec<String> = std::env::args().collect();
        let cmd = args.get(1).map(String::as_str);
        match cmd {
            Some("--reauth") => {
                eprintln!("vector-tunnel-agent: --reauth not yet implemented (Phase 8 Wave 1)");
                std::process::exit(2);
            }
            Some("--version") => {
                println!("vector-tunnel-agent {}", env!("CARGO_PKG_VERSION"));
                Ok(())
            }
            _ => {
                eprintln!("vector-tunnel-agent: stub. Phase 8 Wave 1 (Plan 08-04) wires the run loop.");
                std::process::exit(2);
            }
        }
    }
    ```

    Step 7 — `crates/vector-tunnel-agent/tests/protocol_codec.rs` (NEW): land Test 7's two `#[ignore]` stubs. Bodies are `unimplemented!("Wave 1 Plan 08-04")`.

    Step 8 — `crates/vector-tunnels/Cargo.toml` (FILL OUT existing stub):
    ```toml
    [package]
    name = "vector-tunnels"
    version.workspace = true
    edition.workspace = true
    rust-version.workspace = true
    license.workspace = true
    description = "Microsoft Dev Tunnels client (Mac side) — Phase 8 Vector Tunnel Agent integration."

    [lints]
    workspace = true

    [dependencies]
    anyhow = { workspace = true }
    async-trait = { workspace = true }
    base64 = { workspace = true }
    oauth2 = { workspace = true }
    reqwest = { workspace = true }
    serde = { workspace = true }
    serde_json = { workspace = true }
    thiserror = { workspace = true }
    tokio = { workspace = true, features = ["rt-multi-thread", "macros", "time", "sync", "io-util"] }
    tokio-util = { workspace = true }
    tracing = { workspace = true }
    vector-mux = { path = "../vector-mux" }
    vector-secrets = { path = "../vector-secrets" }
    vector-tunnel-protocol = { workspace = true }
    chrono = { workspace = true }
    keyring-core = { workspace = true }
    # tunnels = { workspace = true }  # Wave 2 Plan 05 wires
    zeroize = { workspace = true }

    [dev-dependencies]
    wiremock = { workspace = true }
    tokio = { workspace = true, features = ["rt-multi-thread", "macros", "time", "sync", "io-util", "test-util"] }
    ```

    Step 9 — `crates/vector-tunnels/src/lib.rs` (REPLACE existing one-line stub):
    ```rust
    //! Microsoft Dev Tunnels client (Mac side) for Phase 8 Vector Tunnel Agent.
    //! Module surface locked Wave 0; bodies filled by Wave 2 Plan 08-05.

    pub mod api;
    pub mod auth;
    pub mod domain;
    pub mod model;
    pub mod transport;
    ```
    Create the empty/stub module files: `api.rs`, `auth/mod.rs`, `auth/device_flow_microsoft.rs`, `auth/token_store.rs`, `domain.rs`, `model.rs`, `transport.rs`. Each file body = a top-of-file comment "Wave 2 Plan 08-05" plus minimal `pub fn _wave_0_placeholder() {}` to keep cargo silent. Module surface only; types live in Wave 2.

    Step 10 — `crates/vector-tunnels/tests/list_tunnels.rs` (NEW): three `#[ignore]` stubs from Test 6 with bodies `unimplemented!("Wave 2 Plan 08-05")`.

    Step 11 — Create `crates/vector-tunnels/tests/fixtures/dev_tunnels_list.json` — a 5-tunnel sample: 2 with `labels: ["vector-agent: true", ...]` and 3 without. Each record has fields `tunnelId`, `name`, `labels`, `endpoints[0].hostId`, `endpoints[0].clientRelayUri`, `endpoints[0].hostPublicKeys[0]`, `lastUpdatedAt` (ISO 8601).

    Step 12 — `crates/vector-secrets/src/lib.rs` (extend existing `impl Secrets`):
    Add ONE constant line inside the `impl Secrets` block, immediately after `GITHUB_REFRESH_ACCOUNT`:
    ```rust
    pub const MICROSOFT_REFRESH_ACCOUNT: &str = "microsoft_refresh_token";
    ```

    Step 13 — `crates/vector-secrets/tests/microsoft_account.rs` (NEW):
    ```rust
    use vector_secrets::Secrets;
    #[test]
    fn microsoft_account_constant_value() {
        assert_eq!(Secrets::MICROSOFT_REFRESH_ACCOUNT, "microsoft_refresh_token");
    }
    ```

    Verify build: run `cargo build --workspace --all-targets`. Must exit 0. If the russh-0.37 patch breaks Phase 7 `vector-ssh`, STOP and escalate (do not modify vector-ssh in this plan).
  </action>
  <verify>
    <automated>test -f .planning/research/spikes/dev-tunnels-decision.md &amp;&amp; grep -q "Path 2 Variant 2c" .planning/research/spikes/dev-tunnels-decision.md &amp;&amp; grep -q "LOCKED" .planning/research/spikes/dev-tunnels-decision.md &amp;&amp; cargo build --workspace --all-targets 2>&amp;1 | tail -5 &amp;&amp; cargo test -p vector-tunnel-protocol --tests &amp;&amp; cargo test -p vector-secrets microsoft_account_constant_value &amp;&amp; grep -q "MICROSOFT_REFRESH_ACCOUNT" crates/vector-secrets/src/lib.rs &amp;&amp; grep -q "patch.crates-io" Cargo.toml &amp;&amp; grep -q "microsoft/vscode-russh" Cargo.toml &amp;&amp; grep -q "microsoft/dev-tunnels" Cargo.toml &amp;&amp; test -f crates/vector-tunnel-protocol/src/lib.rs &amp;&amp; test -f crates/vector-tunnel-agent/src/main.rs &amp;&amp; test -f crates/vector-tunnels/tests/list_tunnels.rs &amp;&amp; test -f crates/vector-tunnels/tests/fixtures/dev_tunnels_list.json</automated>
  </verify>
  <acceptance_criteria>
    - `test -f .planning/research/spikes/dev-tunnels-decision.md` exit 0 (DT-01 SC#1)
    - `grep -c "Path 2 Variant 2c" .planning/research/spikes/dev-tunnels-decision.md` >= 1
    - `grep -c "LOCKED" .planning/research/spikes/dev-tunnels-decision.md` >= 1
    - `grep -c "64048c1409ff56cb958b879de7ea069ec71edc8b" .planning/research/spikes/dev-tunnels-decision.md` >= 1
    - `wc -l .planning/research/spikes/dev-tunnels-decision.md` >= 30
    - `cargo build --workspace --all-targets` exit 0
    - `cargo test -p vector-tunnel-protocol --tests` reports 4 passed / 0 failed
    - `cargo test -p vector-secrets microsoft_account_constant_value` 1 passed
    - `grep -c "MICROSOFT_REFRESH_ACCOUNT" crates/vector-secrets/src/lib.rs` >= 1
    - `grep -c "vector-tunnel-agent" Cargo.toml` >= 1 (member listed)
    - `grep -c "vector-tunnel-protocol" Cargo.toml` >= 1 (member listed)
    - `grep -E "rev\s*=\s*\"64048c1409ff56cb958b879de7ea069ec71edc8b\"" Cargo.toml` exit 0
    - `grep -E "russh\s*=\s*\{\s*git\s*=\s*\"https://github.com/microsoft/vscode-russh\"" Cargo.toml` exit 0
    - `test -f crates/vector-tunnels/tests/fixtures/dev_tunnels_list.json` exit 0
    - `jq '.value | length' crates/vector-tunnels/tests/fixtures/dev_tunnels_list.json` >= 5
    - `jq '[.value[] | select(.labels | index("vector-agent: true"))] | length' crates/vector-tunnels/tests/fixtures/dev_tunnels_list.json` == 2
    - `grep -c "#\\[ignore\\]" crates/vector-tunnels/tests/list_tunnels.rs` >= 3
    - `grep -c "#\\[ignore\\]" crates/vector-tunnel-agent/tests/protocol_codec.rs` >= 2
  </acceptance_criteria>
  <done>DT-01 spike doc committed; workspace compiles with three new crates wired and russh patch applied; Wave-0 test stubs land for downstream waves; vector-secrets exposes MICROSOFT_REFRESH_ACCOUNT.</done>
</task>

<task type="auto">
  <name>Task 2: Extend Pitfall-14 arch-lint to cover three new crates</name>
  <files>crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs</files>
  <read_first>
    - crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs (existing file — read all of it; the test currently scans `crates/vector-codespaces/src` only; extend the scan list)
    - crates/vector-tunnel-protocol/src/lib.rs (Task 1 output — confirm manual Debug impl is in place; arch-lint must accept this file)
    - crates/vector-tunnel-agent/src/main.rs (Task 1 stub — confirm no derived Debug near token fields)
  </read_first>
  <action>
    Extend the existing `no_token_in_debug_or_log.rs` arch-lint test to scan:
    - `crates/vector-tunnels/src`
    - `crates/vector-tunnel-agent/src`
    - `crates/vector-tunnel-protocol/src`

    Concrete steps:
    1. Read the existing file. Identify the constant or function holding the scan-path list (e.g., a `const SCAN_PATHS: &[&str] = &[...]` or a `Vec<PathBuf>` constructed in the test body).
    2. Add the three new paths to that list. Preserve alphabetic ordering if existing list is alphabetic.
    3. Confirm the regex patterns that flag `#[derive(...Debug...)]` within 30 lines of a `*_token` / `*_secret` / `device_code` / `user_code` / `refresh_token` field STILL fire on the new crates. The Phase 8 token-bearing identifiers introduced are:
       - `device_code`, `user_code` (Microsoft device flow — same names as GitHub)
       - `refresh_token`, `access_token` (Microsoft tokens)
       - `agent_token` (vector-tunnel-agent persisted token)
       - `tunnel_access_token` (Dev Tunnels JWT)
       Add these LAST TWO (`agent_token`, `tunnel_access_token`) to the token-substring regex if not already covered. The first four are already in the list per Phase 6.
    4. The two passing tests already in the file (the test that asserts arch-lint FIRES on a known-bad fixture + the test that asserts it does NOT fire on the codespaces crate) MUST continue passing.
    5. Add ONE new test case at the bottom: `scan_paths_include_new_phase_8_crates` — asserts the scan-path list contains all three new paths (compile-time assertion via a `const` check or runtime `assert!(paths.iter().any(|p| p.contains("vector-tunnel-agent")))`).
  </action>
  <verify>
    <automated>cargo test -p vector-arch-tests --tests 2>&amp;1 | tail -5 &amp;&amp; grep -c "vector-tunnel-agent" crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs &amp;&amp; grep -c "vector-tunnels/src" crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs &amp;&amp; grep -c "vector-tunnel-protocol" crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test -p vector-arch-tests --tests` reports all existing tests still pass + new `scan_paths_include_new_phase_8_crates` passes; 0 failed
    - `grep -c "vector-tunnel-agent" crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` >= 1
    - `grep -c "vector-tunnels/src" crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` >= 1
    - `grep -c "vector-tunnel-protocol" crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` >= 1
    - `grep -cE "agent_token|tunnel_access_token" crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` >= 1 (Phase 8 identifiers added)
  </acceptance_criteria>
  <done>Arch-lint scans all three new Phase 8 crates and flags any future `#[derive(Debug)]` placed near token-bearing fields including the Phase 8-specific identifiers.</done>
</task>

</tasks>

<scope_note>
Wave-0 plan intentionally bundles spike-doc + deps + crate scaffolds + arch-lint extension into one plan (13 files / 2 tasks). Rationale: Wave-0 must complete atomically before Wave 1+ can run in parallel; splitting it would introduce ordering hazards (e.g. arch-lint scan paths can only land after the crates they scan exist, and the spike doc must precede any integration code per ROADMAP §Phase 8 SC#1). Executor context budget is ~50% — within the 2-3 task target.
</scope_note>

<verification>
- `make lint` exit 0
- `make test` exit 0 (workspace tests including the new Wave 0 stubs)
- `cargo build --workspace --all-targets` exit 0
- `.planning/research/spikes/dev-tunnels-decision.md` exists per ROADMAP §Phase 8 SC#1
</verification>

<success_criteria>
- DT-01 spike decision document committed at the canonical path (gates SC#1)
- Workspace builds with vendored microsoft/dev-tunnels + russh patch
- Three new crates exist: vector-tunnels (filled out), vector-tunnel-agent (stub binary), vector-tunnel-protocol (typed messages + tests)
- vector-secrets exposes MICROSOFT_REFRESH_ACCOUNT
- Pitfall-14 arch-lint covers the three new crates and the two new token identifiers
- Wave-0 #[ignore] test stubs land for Waves 1+ to flip green
</success_criteria>

<output>
After completion, create `.planning/phases/08-vs-code-remote-tunnels-connect/08-01-SUMMARY.md` documenting:
- confirmation that spike doc landed at `.planning/research/spikes/dev-tunnels-decision.md` (path + commit ref)
- exact pinned SHAs for `tunnels` + `vscode-russh`
- list of #[ignore] test stubs created and which Plan flips each green
- whether Phase 7 `vector-ssh` survived the russh 0.37 downgrade (and any forced reverts)
</output>
