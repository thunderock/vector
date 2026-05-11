# Vector

## What This Is

Vector is a native macOS terminal — written in Rust, GPU-accelerated — with first-class GitHub Codespaces and Dev Tunnels support baked in. It is meant to replace iTerm/ghostty as a daily-driver local terminal *and* let me (and a few Adobe teammates) sign in with GitHub, pick a Codespace, and drop into a remote dev shell without ever opening VS Code or a browser.

## Core Value

**Open the app, pick a Codespace, get a fast remote shell — no VS Code, no browser, no clunky `gh codespace ssh` plumbing.** Local-terminal niceties (tabs, splits, GPU rendering) are table-stakes; the differentiator is that a Codespaces/Dev-Tunnels session feels native, not bolted on.

## Requirements

### Validated

- [x] CI build pipeline that produces installable `.dmg` artifacts (Phase 1 — structural; first real CI run + first tagged release deferred to user's async push session)

### Active

- [ ] Native macOS app distributed as an unsigned `.dmg` (right-click → Open), Universal binary
- [ ] GPU-accelerated terminal rendering on Mac (Metal via wgpu) — performance comparable to Alacritty/WezTerm/ghostty
- [ ] xterm-compatible terminal core (parser + grid + scrollback) suitable as a daily-driver local shell
- [ ] Tabs and splits (horizontal/vertical), multiple sessions per window
- [ ] Session persistence + transparent reconnect — wifi drop should not lose Codespace state
- [ ] tmux pass-through that "just works" — no double-multiplex visual glitches when remote tmux is running
- [ ] GitHub OAuth sign-in flow (device-code or browser callback) with token caching
- [ ] List / pick / connect to existing GitHub Codespaces from the UI (no `gh` CLI required)
- [ ] Connect to a remote machine running `code tunnel` (Microsoft Dev Tunnels) using GitHub auth
- [ ] Saved profiles (`my-cs-frontend`, `my-corp-box`, etc.) for one-click reconnect
- [ ] Themes, fonts, ligatures (table-stakes terminal eye-candy)
- [x] CI build pipeline that produces installable `.dmg` artifacts on every tag (ghostty-style "tip" + tagged releases) — landed in Phase 1; awaiting first real push/tag

### Out of Scope

- **Apple Developer signing & notarization** — deferred. v1 ships unsigned with right-click-Open instructions; revisit only if right-click flow is too painful for teammates.
- **Linux and Windows builds** — Mac-only for v1. The user runs Mac and so do the teammates. Cross-platform doubles the surface area for no payoff today.
- **Codespaces lifecycle management (create/delete/rebuild)** — v1 is connect-only; lifecycle stays in `gh` CLI. Adding it later is straightforward; locking down connect first is more valuable.
- **Port-forwarding UI ("PORTS" panel)** — deferred to v2. Useful but not on the critical path; remote dev works without it for most flows.
- **File transfer (drag-drop / scp UI)** — deferred. `scp`/`rsync` in the shell suffice while we focus on terminal core.
- **Arbitrary SSH targets as first-class profiles** — deferred. v1 is Codespaces + Dev Tunnels. Plain SSH still works because the terminal launches whatever command you give it; there's just no special UI.
- **Browser-based / web companion (vscode.dev style)** — explicitly anti-goal. Native-only is a feature.
- **AI features beyond the optional Claude integration** — no command sharing, no analytics, no account system. Bloat is part of why we are not using Warp/Wave.
- **Fork of ghostty or VS Code** — we read them as reference but build fresh in Rust. Submodule references in this repo will be removed.

## Context

**Background — what triggered this:** I currently sign into GitHub from VS Code (or Claude Code) just to use their remote-tunnel feature, then do all my actual development inside their built-in terminal. I want that connection capability inside a real terminal so I can stop launching a heavyweight IDE just to get a shell.

**Reference implementations to read for design ideas (not vendored):**
- **ghostty** (Zig) — the gold standard for Mac-native terminal UX, AppKit integration, and DMG release pipeline. Reference for app shell, window/tab structure, packaging.
- **Alacritty** (Rust) — minimal GPU terminal. Reference for renderer architecture, escape-sequence parser, the `alacritty_terminal` crate (split out as a library).
- **WezTerm** (Rust) — closest existing Rust terminal to what we want; has SSH, multiplexing, tabs/splits, lua config. Reference for tab/split UX and SSH transport.
- **VS Code Remote Tunnels** — defines the Dev Tunnels client behavior we need to replicate. Microsoft Dev Tunnels has no public Rust SDK, so this is the riskiest piece.
- **`gh codespace ssh`** (Go, GitHub CLI) — defines the Codespaces SSH flow (auth → port allocation → SSH config). We will reimplement the relevant parts in Rust.

**Differentiators vs Warp / Wave / Tabby (which I tried):**
- They treat Codespaces as a second-class SSH target; we treat it as a headline UX (sign-in, picker, profile).
- They bundle cloud accounts, AI products, command sharing, analytics — we ship a terminal and a tunnel client, full stop.

**Why Rust:** I asked about Rust explicitly. We're not forking ghostty (Zig) or VS Code (TypeScript/Electron) — we're building fresh. Rust gives the right balance of performance, ecosystem (alacritty_terminal, vte, wgpu, tokio, octocrab/reqwest), and cross-platform potential when we eventually go beyond Mac.

**No GitHub approval needed:** GitHub Codespaces SSH and Dev Tunnels are public, documented, OAuth-authenticated APIs. Any GitHub user can call them. No special partner approval is required to ship a third-party client.

## Constraints

- **Tech stack:** Rust (workspace). GPU rendering via `wgpu` (Metal backend on macOS). Terminal core via `alacritty_terminal` crate or in-house VT parser using `vte`. Async runtime: `tokio`. GitHub API: `octocrab` or `reqwest`-based client. App shell: native AppKit via `objc2` / `cocoa-rs`, or a minimal cross-platform layer like `winit` + a Mac-native window scaffold.
- **Platform:** macOS only for v1. Apple Silicon + Intel via Universal binary. macOS 13 (Ventura) baseline.
- **Distribution:** Unsigned `.dmg` for v1. CI must produce a downloadable artifact per release. No Apple Developer subscription required initially.
- **Audience:** Personal use first; a handful of Adobe teammates as a soft second wave. No public open-source push for v1.
- **Workflow:** Commit each logical stage separately; **do not push** — the user reviews diffs and pushes asynchronously.
- **Scope discipline:** Resist scope creep. If a feature is not on the v1 list, default to deferring it.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Build in Rust from scratch (not fork ghostty/VS Code) | User explicitly asked about Rust; ghostty is Zig and VS Code is Electron, neither matches the desired stack. Rust ecosystem (alacritty_terminal, wgpu, tokio) is mature enough. | — Pending |
| Connect to BOTH Codespaces SSH and Dev Tunnels | User confirmed both flows matter. Codespaces covers the "use a managed dev VM" case; Dev Tunnels covers "sign into my own remote box and connect". | — Pending |
| Replace iTerm/ghostty as default local terminal (not remote-only launcher) | Halving the surface area to remote-only would shrink scope, but the user explicitly wants a daily-driver. Local terminal is "free" once we have the rendering core. | — Pending |
| Defer signing/notarization to v2 | Apple Developer cert costs $99/yr and adds CI complexity. Right-click-Open is acceptable for an internal tool. Revisit if Gatekeeper friction becomes painful for teammates. | — Pending |
| Defer port-forwarding UI and file-transfer to v2 | They're VS-Code-terminal niceties but not on the critical path; remote dev works without them. Keeps v1 scope finite. | — Pending |
| Remove the empty `ghostty` and `vscode` submodule references | The references are stale (160000 entries with no `.gitmodules`). We'll read those repos out-of-tree if we need them, not vendor them. | — Pending |
| Capture optional Claude-API integration (autocomplete/autosuggest) as a v2 ambition | User raised it as "if possible". It's a real differentiator vs Warp, but it must not gate the terminal-core work. | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-05-11 after Phase 1 (Foundation & CI/DMG Pipeline) completion. Workspace + 14 crate stubs + winit/tokio threading skeleton + AppKit window + Universal DMG build path + ci.yml + release.yml + ADRs 0001..0006 all in place; real-world CI/release telemetry pending user's async push.*
