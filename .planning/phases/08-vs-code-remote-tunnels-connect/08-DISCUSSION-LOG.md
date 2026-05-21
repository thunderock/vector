# Phase 8: VS Code Remote Tunnels Connect - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-20
**Phase:** 08-vs-code-remote-tunnels-connect
**Areas discussed:** Agent install UX (4 sub-questions), Microsoft account type, Agent protocol wire format, v1 session model, Agent auth, Tunnel naming

---

## Agent install UX — Linux distribution formats

| Option | Description | Selected |
|--------|-------------|----------|
| apt (Debian/Ubuntu .deb) | Hosted on a Vector apt repo. `apt install vector-tunnel-agent`. | ✓ |
| Static binary download | Plain binary on GitHub Releases, works on any Linux. | |
| yum/dnf (.rpm) | RHEL/CentOS/Fedora packaging. | |
| snap / flatpak / brew | Sandboxed / cross-distro alternatives. | |

**User's choice:** apt only for v1.
**Notes:** Adobe corporate boxes are Debian-family. Other distros deferred to v1.x.

---

## Agent install UX — Service install mode

| Option | Description | Selected |
|--------|-------------|----------|
| Manual run only in v1 | User runs interactively. Dies on shell close. | ✓ |
| systemd unit installed, opt-in `--service` | Package installs unit; user enables manually. | |
| Auto-enable systemd on apt install | Standard daemon UX. | |
| Binary + `service install` subcommand | Mirrors `code tunnel service install`. | |

**User's choice:** Manual run only in v1.
**Notes:** Matches user's framing 'install and run the agent manually'.

---

## Tunnel discovery — Scope of tunnels shown

| Option | Description | Selected |
|--------|-------------|----------|
| Only Vector-agent tunnels | Filter by `vector-agent: true` label set at registration. | ✓ |
| All tunnels, mark Vector-ready | Show everything, indicate clickability. | |
| All tunnels, try to connect | Show everything, fail on non-Vector. | |

**User's choice:** Only Vector-agent tunnels.

---

## Sign-in / auth provider (Mac client)

| Option | Description | Selected |
|--------|-------------|----------|
| GitHub OAuth only (already shipped) | Reuse Phase 6. Zero new auth code. | |
| Microsoft OAuth only | Add Microsoft Entra ID device flow. | |
| Both at sign-in | User picks GitHub or Microsoft. | ✓ |
| Defer Microsoft to v1.x | GitHub-only v1. | |

**User's choice:** Both at sign-in.
**Notes:** Adds ~1 dev-week to Wave 1 for Microsoft OAuth flow. Adobe corporate identity may require Microsoft.

---

## Microsoft account type

| Option | Description | Selected |
|--------|-------------|----------|
| Work / Entra ID only | Adobe corporate accounts only. | |
| Personal MSA only | outlook.com / hotmail.com only. | |
| Both (multi-tenant + consumers) | OAuth `common` authority. | ✓ |
| Defer Microsoft entirely to v1.x | GitHub-only v1. | |

**User's choice:** Both (`common` authority).

---

## Agent protocol — Wire format

| Option | Description | Selected |
|--------|-------------|----------|
| JSON over newline-delimited frames | Human-readable, base64 PTY bytes. | ✓ |
| MessagePack length-prefixed | Binary, no base64 needed, ~3x smaller. | |
| Protocol Buffers (prost) | Strict schema, compile-time validation. | |

**User's choice:** JSON over newline-delimited frames.

---

## v1 session model

| Option | Description | Selected |
|--------|-------------|----------|
| One shell per tunnel connection (v1) | Each pane opens its own connection. | ✓ |
| Multiplex shells in one connection | Session IDs in protocol; more efficient. | |
| One per tunnel + reconnect-existing | Server-side state; tmux territory. | |

**User's choice:** One shell per tunnel connection in v1.

---

## Agent auth on first run (remote side)

| Option | Description | Selected |
|--------|-------------|----------|
| Device Flow on agent (RFC 8628) | Prints device code; user completes in browser. | ✓ |
| Provisioning token from Mac client | Copy-paste from Mac to remote. | |
| Personal Access Token | User creates PAT in GitHub settings. | |

**User's choice:** Device Flow on the agent itself.

---

## Tunnel naming at registration

| Option | Description | Selected |
|--------|-------------|----------|
| Auto from `gethostname()` + prefix | `vector-corp-dev-box-42`. | ✓ |
| User picks at install | `--name` flag required. | |
| Hostname default + `--name` override | Best of both. | |

**User's choice:** Auto from `gethostname()` + `vector-` prefix.

---

## Claude's Discretion

User gave concrete answers on all questions. No questions deferred to Claude.

Planner discretion items captured in CONTEXT.md §"Claude's Discretion" (planner picks):
- Cargo.toml patch SHAs
- `vector-devtunnels` as new crate vs extension of `vector-codespaces`
- Picker UI per-row layout details
- Agent CLI subcommands beyond `run` and `--reauth`
- Agent tracing setup
- Error messages and toast copy
- Source file layout of `vector-tunnel-agent`
- Debian packaging tool choice (`cargo-deb` recommended)

## Deferred Ideas

Ideas mentioned during discussion that fall outside Phase 8 v1 scope. Captured in CONTEXT.md §Deferred so future phases can pick them up.

Key deferrals:
- Non-Debian Linux packaging (static binary, rpm, snap, flatpak, brew) — v1.x
- systemd auto-start — v1.x
- Multi-session multiplexing — v2
- Persistence / reconnect across wifi drops — Phase 9
- tmux auto-attach — Phase 9
- Agent self-update — v1.x
- Port forwarding / file transfer UI — out of scope per ROADMAP/PROJECT
- Agent on Windows/macOS — v2

## Rejected Architectures

The day-1 spike (08-RESEARCH.md) explored and rejected:
- **Path 1: vscode-remote protocol implementation** — 7-10 dev-weeks; greenfield; Microsoft breaks protocol monthly.
- **Path 2 Variant 2a: SSH over tunnel-forwarded port via `handle_forward(22)`** — would have worked but requires sshd on remote, which corporate box doesn't expose.
- **Path 2 Variant 2b: SSH over separate `devtunnel host -p 22`** — same sshd dependency.
- **Defer-to-v2 (initial recommendation)** — superseded once Vector-Tunnel-Agent path was identified.
