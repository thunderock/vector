# Phase 6: GitHub Auth + Codespaces Picker - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-14
**Phase:** 06-github-auth-codespaces-picker
**Areas discussed:** Sign-in trigger + device flow UX, Codespaces picker access point, State refresh strategy

---

## Sign-in trigger

| Option | Description | Selected |
|--------|-------------|----------|
| Menu item | `Vector → Sign in with GitHub` menu item + auto-prompt when clicking a codespace profile with no valid token | ✓ |
| Picker-initiated only | No menu item; sign-in only triggers from picker interaction | |
| Both menu + prominent button | Menu item AND a visible "Sign in" call-to-action in the Codespaces picker area | |

**User's choice:** Menu item (Recommended)
**Notes:** Menu item is primary entry point; auto-prompt when clicking a codespace profile without a token is a secondary trigger sharing the same code path.

---

## Device flow presentation

| Option | Description | Selected |
|--------|-------------|----------|
| Modal window overlay | NSPanel-style: 8-char code prominently, auto-copy, "Copy code and open browser" button, Cancel, stays on top until auth completes | ✓ |
| Toast + clipboard auto-copy | Extend D-69 toast surface: blocking-style toast with code + URL, auto-copies, fades only after auth | |
| Dedicated pane / new tab | New Vector pane displaying device-flow instructions as rendered terminal text | |

**User's choice:** Modal window overlay (Recommended)
**Notes:** Stays on top until authentication completes or user cancels. Critical auth moment warrants a dedicated UI surface rather than a transient banner.

---

## Codespaces picker location

| Option | Description | Selected |
|--------|-------------|----------|
| Separate dedicated modal | `Vector → Codespaces...` or `Cmd-Shift-G` — separate from existing Cmd-Shift-P profile picker | ✓ |
| Extend Cmd-Shift-P picker | Integrate live Codespaces as a section in the existing profile picker | |
| New Tab / home screen | Opening a new tab shows a Codespaces landing page | |

**User's choice:** Separate dedicated modal (Recommended)
**Notes:** Profile picker (D-75) is for saved profiles / quick switching. Codespaces modal is for live API data, discovery, and start-on-demand. Different affordances warrant separation.

---

## Profile save destination

| Option | Description | Selected |
|--------|-------------|----------|
| Write to config.toml | Adds `[profile.X]` block using existing D-74 schema | ✓ |
| Separate internal store | Saves to `~/.config/vector/codespaces.toml` separate from user config | |

**User's choice:** Write to config.toml (Recommended)
**Notes:** Uses the exact D-74 schema already designed for this purpose. User can inspect and edit saved profiles directly in their config file.

---

## State refresh strategy

| Option | Description | Selected |
|--------|-------------|----------|
| On-demand + active transition poll | Fetch on picker open; poll 1s while Starting; no background poll | ✓ |
| Background poll always-on | Poll every 30s regardless of picker visibility | |
| Manual refresh only | Refresh button; no automatic refresh | |

**User's choice:** On-demand + active transition poll (Recommended)
**Notes:** Conserves GitHub REST API quota. Background polling at 30s would consume ~1200 API calls/hour across active instances. On-demand keeps well within the 5000 req/hour limit.

---

## Claude's Discretion

- OAuth2 crate choice (`oauth2 5.0` vs raw `reqwest`)
- `octocrab` client setup and sharing pattern
- Picker modal rendering (NSPanel vs wgpu compositor)
- `Cmd-Shift-G` conflict verification
- Error display strategy in picker
- API pagination strategy

## Deferred Ideas

- GitHub status page integration in picker
- Codespace creation, rebuild, delete
- Multi-account GitHub support
