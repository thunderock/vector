# Phase 8: VS Code Remote Tunnels Connect - Context

**Gathered:** 2026-05-20
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 8 delivers the end-to-end "pick a remote machine running our agent, get a remote shell" flow:

- **`vector-tunnel-agent`** — a Linux user-space binary (Debian/Ubuntu .deb for v1) that registers a Dev Tunnel under the user's GitHub or Microsoft identity, accepts client connections via Microsoft's Dev Tunnels relay, and spawns a PTY on demand.
- **`vector-devtunnels`** — a Mac-side client crate that lists Vector-agent tunnels via the Dev Tunnels Management API, opens connections via the SDK, speaks the agent JSON protocol, and returns `Box<dyn PtyTransport>` for the existing Phase 7 mux integration.

**OUT of scope (deferred):** sshd, vscode-remote protocol, port-forwarding panel, file transfer, agent on RHEL/Fedora/Windows, multi-session multiplexing, persistence/reconnect (Phase 9), tmux auto-attach (Phase 9), agent on macOS.

</domain>

<decisions>
## Implementation Decisions

### Architecture (locked pre-discussion in 08-RESEARCH.md)

- **D-A1:** Path 2 Variant 2c — Vector Tunnel Agent. Vector ships its own user-space agent binary; agent uses `microsoft/dev-tunnels rs/` as a Host; Mac client uses the same SDK as a Client. No sshd, no vscode-remote, no VPN.
- **D-A2:** Vendor `microsoft/dev-tunnels` `rs/` at a pinned SHA in workspace Cargo.toml + apply `[patch.crates-io] russh = { git = "https://github.com/microsoft/vscode-russh" }` at workspace level. Reason: SDK internally pins russh 0.37, our workspace uses 0.60. The patch unifies them.
- **D-A3:** Phase 7 transport scaffolding stays in tree (`vector-ssh`) — it's not used by Phase 8 but Phase 9 (Persistence + plain-SSH future) may reuse it.
- **D-A4:** Reuse Phase 7 mux integration: `TransportKind::DevTunnel`, `format_tab_title` `[remote]` badge, tint pipeline.

### Agent install UX

- **D-01:** Linux distribution format: **apt only for v1** (Debian/Ubuntu `.deb` package). Hosted on a Vector apt repo. Static binary fallback, rpm/yum, snap/flatpak/brew packagers all deferred to v1.x.
- **D-02:** Service mode: **manual run only in v1**. No systemd unit, no `service install` subcommand. User runs `vector-tunnel-agent` interactively in their shell; uses `tmux`/`screen`/`nohup` if they want persistence across SSH disconnect. Service-install ergonomics deferred to v1.x.

### Sign-in / auth (Mac client side)

- **D-03:** Sign-in providers: **BOTH GitHub OAuth AND Microsoft OAuth**, user picks at sign-in. Vector UI shows two buttons: "Sign in with GitHub" / "Sign in with Microsoft". Each provider gets its own modal mirroring `AuthDeviceFlowModal`.
- **D-04:** Microsoft OAuth authority: **`common`** (multi-tenant + consumers). Accepts both Adobe / corporate Entra ID accounts AND personal MSAs (outlook.com / hotmail.com).
- **D-05:** Microsoft token storage: separate Keychain entry from GitHub token. Same `vector-secrets` crate, new account constant `MICROSOFT_REFRESH_ACCOUNT`.
- **D-06:** Token-to-tunnel-API path: GitHub bearer flows directly to Dev Tunnels API (`Authorization: github <gho_token>`). Microsoft bearer flows directly too (`Authorization: Bearer <msft_token>`). No intermediate exchange step. Verified in 08-RESEARCH.md.

### Agent auth (remote side, first run)

- **D-07:** **OAuth Device Flow (RFC 8628) on the agent itself.** First `vector-tunnel-agent` run prints the device code + verification URL on stdout (e.g. `Go to github.com/login/device and enter ABCD-1234`). User completes the flow from any browser. Token persisted to `~/.config/vector/agent-token` (mode 0600) on the remote box.
- **D-08:** Agent uses whichever provider (GitHub or Microsoft) the user signs in with on first run. Single-provider per agent install. Switching providers requires `vector-tunnel-agent --reauth` (exact CLI surface = planner's discretion).

### Tunnel registration + discovery

- **D-09:** **Tunnel naming at registration: auto from `gethostname()` prefixed `vector-`.** Example: `vector-corp-dev-box-42`. Picker display strips the prefix → `corp-dev-box-42`. No user-override flag in v1.
- **D-10:** **Tunnel discovery filter: show ONLY Vector-agent tunnels** in the picker. Filter by tunnel label `vector-agent: true` set at agent registration. `code tunnel` tunnels (without our label) are invisible to Vector's picker.
- **D-11:** Picker UI: mirror Phase 6 `CodespacesPickerModal` shape (NSPanel, list view, search-as-you-type). `Cmd-Shift-T` keybind to open (distinct from Phase 6's `Cmd-Shift-G`).

### Agent protocol (client ↔ agent over relay channel)

- **D-12:** **Wire format: JSON over newline-delimited frames.** Each message is a single line ending in `\n`. PTY bytes encoded as base64 in `data` payloads.
- **D-13:** Message types (minimum viable v1):
  - Client → agent: `{"op":"open_pty","protocol_version":1,"rows":N,"cols":N,"shell":null|"path"}`
  - Agent → client: `{"op":"opened","protocol_version":1,"session":"..."}` or `{"op":"error","reason":"..."}`
  - Both directions: `{"op":"data","session":"...","bytes":"<base64>"}`
  - Client → agent: `{"op":"resize","session":"...","rows":N,"cols":N}`
  - Agent → client: `{"op":"exit","session":"...","code":N}`
- **D-14:** **Session model: ONE shell per tunnel connection** in v1. Each Vector tab/pane = one `connect_to_port` + one `open_pty`. No multiplexing. (Multi-session deferred to v2.)
- **D-15:** Protocol versioning: include a `protocol_version: 1` field in `open_pty` and `opened`. Mismatch → agent rejects with `{"op":"error","reason":"protocol_version_mismatch"}`.

### Visual & UX (reuse from Phase 7)

- **D-16:** `TransportKind::DevTunnel` returned by the new transport; `[remote]` badge appears via existing `format_tab_title`.
- **D-17:** Tab tint color: **Microsoft blue `#0078d4`** (Dev Tunnels brand color). Distinguishes from Phase 7's GitHub-purple `#7a3aaf` (legacy from Codespaces work, still present in tint pipeline code paths).

### Claude's Discretion (planner picks)

- Exact Cargo.toml patch SHA for `microsoft/vscode-russh` and `microsoft/dev-tunnels`
- Whether `vector-devtunnels` is a new crate or extends `vector-codespaces` (planner picks based on dep graph)
- Picker UI per-row layout (icon, host name, last-seen formatting) — defer to UI-phase if frontend complexity warrants it
- Agent CLI subcommands beyond `run` and `--reauth` (`status`? `unregister`? `version`?)
- Agent logging / tracing setup (workspace `tracing` conventions apply)
- Error messages and toast copy (UI-phase if invoked; otherwise planner)
- File layout of `vector-tunnel-agent` source (single bin or multi-module)
- Deb packaging tooling (`cargo-deb` recommended given it's already idiomatic, but planner decides)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase-local
- `.planning/phases/08-vs-code-remote-tunnels-connect/08-RESEARCH.md` — full research with locked recommendation (Path 2c), Wave structure, cost estimates, invalidators, code examples
- `.planning/REQUIREMENTS.md` — DT-01..04 (v1 requirements)
- `.planning/ROADMAP.md` §Phase 8 — goal, depends-on, success criteria

### Project-level
- `.planning/PROJECT.md` — Vector pivot to VS Code Remote Tunnels (2026-05-19) + Out of Scope list
- `.planning/STATE.md` — current phase status + Phase 7 descope record
- `CLAUDE.md` — coding style + lint discovery + workflow constraints (do not push, commit per stage)

### Carry-over from prior phases (must understand before extending)
- `crates/vector-codespaces/src/auth/device_flow.rs` — RFC 8628 Device Flow reference. Phase 8 GitHub auth reuses verbatim; Microsoft Device Flow mirrors its shape.
- `crates/vector-codespaces/src/client/mod.rs` — REST client + token refresh chain. `vector-devtunnels` mirrors this.
- `crates/vector-codespaces/src/auth/token_store.rs` (or whichever holds it) — Keychain TokenStore. Add `MICROSOFT_REFRESH_ACCOUNT` constant.
- `crates/vector-secrets/src/lib.rs` — Keychain wrapper + `GITHUB_REFRESH_ACCOUNT` const. Extended for Microsoft.
- `crates/vector-mux/src/mux.rs::create_tab_async_with_transport` — install seam for `Box<dyn PtyTransport>`.
- `crates/vector-mux/src/transport.rs::TransportKind` — currently `Local | DevTunnel`. Phase 8 implementations return `DevTunnel`.
- `crates/vector-mux/src/pane.rs::format_tab_title` — already appends `[remote]` for DevTunnel.
- `crates/vector-ssh/src/transport.rs::SshChannelTransport` — reference for the async select-loop pattern (resize > write > read biased select). Phase 8 transport mirrors this shape over the JSON protocol.
- `crates/vector-app/src/codespaces_actor.rs` — Phase 6 actor pattern: tokio task + `EventLoopProxy<UserEvent>` for cross-thread signaling. `devtunnels_actor` mirrors.
- `crates/vector-app/src/codespaces_modal.rs` — Phase 6 picker NSPanel. `DevTunnelsPickerModal` mirrors.
- `crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` — Pitfall 14 arch-lint. Applies to all token-bearing types in `vector-devtunnels` and `vector-tunnel-agent`.

### External
- `microsoft/dev-tunnels` repo `rs/` folder — Rust SDK source. Pin a recent SHA in workspace Cargo.toml. `rs/src/connections/relay_tunnel_host.rs` for the Host API; `rs/src/connections/relay_tunnel_client.rs` for the Client API.
- `microsoft/vscode-russh` repo — required `[patch.crates-io] russh = { git = ... }`.
- Dev Tunnels Management API: `GET /api/v1/tunnels` (listing). Documented at `https://learn.microsoft.com/en-us/azure/developer/dev-tunnels/api`.
- RFC 8628 OAuth Device Authorization Grant — for both the Mac client and the agent first-run.
- Microsoft identity platform `common` authority: device code endpoint `https://login.microsoftonline.com/common/oauth2/v2.0/devicecode`, token endpoint `https://login.microsoftonline.com/common/oauth2/v2.0/token`. Scope for Dev Tunnels: `46da2f7e-b5ef-422a-9a4e-fb5e1cb7da14/.default` (verify before plan).
- `cargo-deb` crate documentation — Debian packaging.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`vector-codespaces::GitHubAuth`** (`auth/device_flow.rs`): RFC 8628 driver. Reused as-is for GitHub auth. `MicrosoftAuth` is a parallel struct against MS endpoints, same shape.
- **`vector-codespaces::TokenStore`**: token persistence via Keychain. Extended to support Microsoft tokens (separate account name `MICROSOFT_REFRESH_ACCOUNT`).
- **`vector-mux::Mux::create_tab_async_with_transport(window_id, transport, rows, cols)`**: install seam for `Box<dyn PtyTransport>`. `vector-devtunnels` returns its tunnel-backed transport through this.
- **`vector-mux::TransportKind::DevTunnel`**: tag returned by the new transport.
- **`vector-mux::format_tab_title`**: already appends `[remote]` for non-Local panes.
- **`vector-render::TintStripePipeline`**: tab tint application. Phase 8 calls it with `#0078d4` Microsoft blue when active pane is DevTunnel.
- **`portable-pty 0.9` (workspace dep)**: used by `vector-pty`. Agent uses this directly to spawn `$SHELL` with a PTY.
- **`vector-codespaces::CodespacesPickerModal`**: Phase 6 NSPanel pattern. `DevTunnelsPickerModal` is a structural mirror.
- **`vector-app::codespaces_actor::spawn_*`**: Phase 6 actor pattern. `devtunnels_actor` mirrors.

### Established Patterns
- **Manual `impl Debug` on token-bearing structs (Pitfall 14):** every token-bearing type has a hand-written Debug impl that omits secrets. Phase 8 follows the same rule for `MicrosoftAuth`, `MicrosoftTokens`, agent-side token-bearing types, the `vector-tunnel-agent` token cache. The arch-lint `vector-arch-tests::no_token_in_debug_or_log` enforces this automatically.
- **EventLoopProxy actor pattern:** tokio actors send `UserEvent` variants to the main thread. `devtunnels_actor` adds variants like `DevTunnelsLoaded`, `DevTunnelsLoadFailed`, `DevTunnelConnectStarted`, `DevTunnelPaneReady`.
- **`vector-secrets::Secrets` Keychain wrapper:** all token writes go through here. Add `MICROSOFT_REFRESH_ACCOUNT` constant.
- **`#[deny(clippy::await_holding_lock)]` (D-11):** transports never hold a sync lock across `.await`. Same applies to the agent's PTY reader/writer.

### Integration Points
- **Menu:** "Vector" menu gets new items — "Sign in with Microsoft" / "Dev Tunnels…". Mirror Phase 6's wiring.
- **Keymap:** `Cmd-Shift-T` to open Dev Tunnels picker (in `vector-input::keymap`). Distinct from Phase 6's `Cmd-Shift-G` (codespaces).
- **Config:** profiles already support `kind = "dev_tunnel"` (Phase 5 POLISH-07 D-79). Reuse the profile-save flow from Phase 6's `vector-config::writer::append_codespace_profile` (or generalize).
- **Phase 7's `vector-ssh` crate:** untouched. Phase 9 (Persistence) may reuse for plain-SSH future, but Phase 8 doesn't.

</code_context>

<specifics>
## Specific Ideas

- User's primary use case: connect to Adobe corporate Linux box (no VPN dependency, no sshd accessible from non-VPN networks). Phase 8 design specifically targets this constraint.
- Tunnel display in picker: strip the `vector-` registration prefix so users see `corp-dev-box-42` instead of `vector-corp-dev-box-42`.
- First-time agent install path is the most critical UX moment — getting `apt install vector-tunnel-agent && vector-tunnel-agent` to "just work" through to the device code prompt is the user-facing v1 win.
- Tunnel tint color: Microsoft blue `#0078d4` to match Dev Tunnels brand identity. Phase 6's tint default for `kind = "dev_tunnel"` profiles can be updated to match (planner detail).

</specifics>

<deferred>
## Deferred Ideas

These ideas surfaced during discussion but are out of scope for Phase 8 v1. Tracked here so future phases can pick them up.

- **Static binary download fallback for non-Debian Linux** (RHEL, Arch, NixOS, Alpine) — defer to v1.x. v1 is apt-only.
- **rpm packaging for RHEL/Fedora** — defer to v1.x.
- **snap / flatpak / brew packaging** — defer past v1.x.
- **`vector-tunnel-agent service install` (systemd auto-start)** — defer to v1.x. v1 is manual-run only.
- **Multi-session multiplexing per tunnel connection** — defer to v2. v1 opens a separate connection per pane.
- **Tunnel reconnect / session persistence across wifi drops** — Phase 9 (Persistence + Reconnect) owns this. Phase 8 panes die on disconnect; user re-clicks.
- **tmux auto-attach on connect** — Phase 9 (PERSIST-03).
- **`vector-tunnel-agent` self-update** — v1.x. v1 relies on `apt upgrade`.
- **Port forwarding / file transfer UI** — explicitly out-of-scope per ROADMAP.md and PROJECT.md.
- **Multi-user remote box concerns** — corner case. Agent runs as the invoking user; sessions are per-user-token-scoped. Defer if it ever bites.
- **Per-tunnel custom tint color** — v2 ergonomic. v1 uses single Microsoft blue across all DevTunnel panes.
- **Agent on Windows or macOS as a target host** — v2. v1 Linux-only.
- **`code tunnel`-only tunnels visible in picker** — explicitly rejected: Vector picker is Vector-agent-only (D-10).
- **vscode-remote protocol implementation (Path 1)** — rejected in 08-RESEARCH.md.
- **SSH-over-tunnel-forwarded-port (Path 2a/2b)** — rejected because user doesn't want sshd / VPN dependency.

</deferred>

---

*Phase: 08-vs-code-remote-tunnels-connect*
*Context gathered: 2026-05-20*
