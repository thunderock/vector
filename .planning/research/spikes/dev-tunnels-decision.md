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
