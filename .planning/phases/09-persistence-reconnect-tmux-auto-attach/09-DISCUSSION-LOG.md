# Phase 9: Persistence + Reconnect + tmux Auto-Attach - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in 09-CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-22
**Phase:** 09-persistence-reconnect-tmux-auto-attach
**Areas discussed:** Reconnect scope, tmux-on-host policy, tmux session sharing, Reconnect UX + give-up policy

---

## Reconnect scope

| Option | Description | Selected |
|--------|-------------|----------|
| Network blips only | TCP/SSH disconnect, wifi flap, lid-resume; Vector process stays alive | ✓ |
| Blips + sleep/wake survival across app suspend | Covers macOS App Nap / Vector backgrounding edge cases | |
| Blips + app-restart restore | Reconnect open remote panes after Vector quit/relaunch | |

**User's choice:** Network blips only.
**Notes:** Matches the ROADMAP goal phrasing ("closes laptop lid for a meeting, reopens it…"). App-restart restore deferred to a future phase (needs window/pane tree serialization).

---

| Option | Description | Selected |
|--------|-------------|----------|
| Stay live + lock input | Grid + scrollback frozen at last byte; cursor stops blinking; input dropped | ✓ |
| Dim the pane + drop input | 50% opacity overlay; soft visual feedback on dropped input | |
| Stay live + queue input (replay on reconnect) | Buffered input replayed after swap; predictive-echo territory | |

**User's choice:** Stay live + lock input.
**Notes:** Matches iTerm 'session offline' behavior. Replay rejected per Pitfall 22 (no predictive echo).

---

## tmux-on-host policy

| Option | Description | Selected |
|--------|-------------|----------|
| Connect raw, toast warning | Fall back to plain shell; toast: 'tmux not found — persistence unavailable' | ✓ (subsequently obviated) |
| Refuse to connect, show install hint | Hard error; user installs tmux before retry | |
| Detect and prompt once per profile | Modal on first connect: continue / install / cancel | |
| Auto-install via the agent | `sudo apt install -y tmux` on agent startup | |

**User's choice:** Connect raw, toast warning — **then superseded by the later "drop auto-attach entirely" decision below.** With Vector no longer managing tmux at all, this answer is moot.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Agent reports tmux capability at handshake | `tmux: { present: bool, version: '3.5' }` in JSON handshake | ✓ (subsequently obviated) |
| Client probes via a no-op shell command per attach | `tmux -V` over the open_pty pipe; re-probes each reconnect | |

**User's choice:** Agent reports tmux capability at handshake — **superseded by the later "drop auto-attach" decision.** Vector doesn't need tmux capability detection at all if it's not managing tmux.

---

## tmux session sharing

| Option | Description | Selected |
|--------|-------------|----------|
| Per-pane sessions | `vector-{profile-id}-{pane-uuid}`; isolated per pane | |
| Shared session per profile | All panes attach to `vector-{profile-id}`; tmux-native multi-client | ✓ (initially) |
| Shared by default, isolated via modifier | First pane creates shared; Cmd-Shift-N for fresh isolated | |

**User's choice:** Shared session per profile — **then user pivoted in next question.**

---

| Option | Description | Selected |
|--------|-------------|----------|
| Same view as pane 1 (single-window tmux) | `tmux new -A -s vector-{profile-id}`; second pane attaches to same window | |
| Auto-create a new tmux window per pane | `tmux new -A -s vector-{profile-id} \; new-window` | |
| **Free-text** | "we do not have to start with a tmux pane… I want to first log in to a bash session and then I am responsible to create and maintain the tmux session" | ✓ |

**User's choice:** Vector does NOT auto-attach to tmux. User connects to plain bash; the user manages tmux themselves.
**Notes:** This pivot invalidated the prior tmux-on-host and sharing-model answers and forced a roadmap revision (PERSIST-03, SC#3, SC#4 reframing).

---

### Roadmap conflict resolution

| Option | Description | Selected |
|--------|-------------|----------|
| Drop auto-attach entirely — revise PERSIST-03 | Vector never wraps with tmux; user owns it; SC#4 reframed | ✓ |
| Opt-in per profile (default off) | `tmux_autoattach = false` default; power users opt in | |
| Keep auto-attach as default, opt out per profile | Roadmap stays; opt out via `tmux_autoattach = false` | |

**User's choice:** Drop entirely.
**Notes:** This is the binding scope change of Phase 9 — captured in CONTEXT.md D-04..D-06 and reflected in commits to ROADMAP.md and REQUIREMENTS.md alongside this CONTEXT.

---

## Reconnect UX + give-up policy

| Option | Description | Selected |
|--------|-------------|----------|
| Inline status bar at pane top | ~22px bar with 'Reconnecting to {host}… (attempt N)' | ✓ |
| Centered modal-style card | Translucent card + spinner; obscures grid | |
| Tab badge only (no pane overlay) | `[reconnecting]` in tab title; pane content unchanged | |

**User's choice:** Inline status bar at pane top.
**Notes:** Grid remains visible; attempt counter grows so user can see Vector is still trying.

---

| Option | Description | Selected |
|--------|-------------|----------|
| 1s / 2s / 4s / 8s / 16s / 30s cap | Classic exponential; recovers fast from short blips | ✓ |
| Tighter: 0.5s / 1s / 2s / 5s / 10s cap | Sub-second recovery; risks hammering relay | |
| Looser: 2s / 4s / 8s / 16s / 30s / 60s cap | Gentler; worse for common short-blip case | |

**User's choice:** 1s / 2s / 4s / 8s / 16s / 30s cap.

---

| Option | Description | Selected |
|--------|-------------|----------|
| Never give up automatically | Retry forever at 30s cap; user closes pane manually | ✓ |
| Give up after 5 minutes | Flip to 'click to retry' after ~10 attempts at cap | |
| Give up after 15 minutes | ~30 attempts at cap | |

**User's choice:** Never give up automatically.
**Notes:** Pane stays useful as a 'scrollback archive' indefinitely; mental model is 'it's still trying.'

---

## Claude's Discretion

- Exact pixel/character dimensions of the status bar overlay
- Animation/transition between Active and Reconnecting states
- Whether to debounce the overlay for sub-250ms blips
- `tracing` event level (INFO vs WARN) for reconnect attempts
- SIGWINCH/resize behavior during a blip
- Whether to extend the agent JSON protocol with a resume-from-byte-offset op
- Visual cue for locked input during the blip (flash vs dim-cursor)
- Implementation approach for no-byte-loss verification (sequence numbers vs drain-and-swap vs checkpoint)

## Deferred Ideas

- App-restart restore (relaunch → reopen remote panes)
- Local pane shell-death recovery overlay
- Per-profile backoff configuration
- Per-profile give-up policy
- Reconnect attempt logging panel
- SIGWINCH coalescing during blips
- Opt-in `tmux_autoattach = true` profile field (future phase)
