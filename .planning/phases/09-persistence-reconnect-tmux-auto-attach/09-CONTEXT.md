# Phase 9: Persistence + Reconnect + tmux Auto-Attach - Context

**Gathered:** 2026-05-22
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 9 delivers **transport-level reconnect for remote panes** when the network drops (TCP/SSH disconnect, wifi flap, laptop lid-close → resume). The `Pane` object stays alive across the blip, the local grid + scrollback remain visible, an inline status bar appears, and `Domain::reconnect()` re-establishes the underlying transport with exponential backoff and hot-swaps the new `PtyTransport` under the live `Pane` without losing in-flight bytes.

**SCOPE CHANGE FROM ORIGINAL ROADMAP:** The original Phase 9 included **Vector-managed tmux auto-attach** (`tmux new -A -s vector-{profile-id}`). That is **dropped**: Vector connects remote panes to the plain default shell (bash/zsh). If the user wants shell-state persistence across full disconnects, they run tmux themselves on the remote. PERSIST-03 and Phase 9 Success Criteria #3 (and #4's framing) are revised accordingly. The `tmux passthrough correctness` smoke test stays — but verifies Vector correctly passes through DCS-OSC-52, DECSCUSR, mouse modes, and `TERM` WHEN THE USER IS RUNNING TMUX ON THE REMOTE, not when Vector wraps the spawn.

**OUT of scope (deferred):**
- App-restart restore (relaunch Vector → reopen remote panes that were open at quit) — needs window/pane tree serialization, profile-id resurrection. Future phase.
- Predictive echo (mosh-style local-echo + diff-on-server) — Pitfall 22 explicit reject.
- mosh-style state-sync protocol — Pitfall 22 explicit reject.
- Vector-managed tmux session lifecycle (auto-attach, auto-spawn, auto-detect). User owns tmux.
- Local pane shell-death recovery overlay. Local panes still die silently on shell exit.
- Custom remote agent extensions for reconnect. The agent protocol (Phase 8 D-12..D-15) is sufficient — reconnect is purely a Mac-side concern.

</domain>

<decisions>
## Implementation Decisions

### Reconnect scope (D-01..D-03)

- **D-01:** **Scope = network blips only.** TCP/SSH disconnect, wifi flap, lid-close → resume. The Vector process stays alive throughout; the `Pane` and its `Window`/`Tab` parents stay in memory. App restart (quit Vector → relaunch) does NOT restore remote panes in this phase — user re-picks from the picker.
- **D-02:** **Grid + scrollback stay visible during the blip.** Last byte received remains on screen. Cursor stops blinking (visual signal of offline state). User can still scroll the scrollback while reconnecting.
- **D-03:** **Input is LOCKED during the blip, not queued.** Keystrokes typed while the transport is dead are dropped with a soft visual cue (no replay on reconnect — replay risks lands keystrokes on possibly-different shell state; predictive-echo territory is out-of-scope per Pitfall 22).

### tmux model (D-04..D-06) — DEPARTS FROM ORIGINAL ROADMAP

- **D-04:** **Vector does NOT wrap remote shells in tmux.** The spawn command sent to the agent is the plain default shell (or whatever Phase 8 D-13 `open_pty.shell` field resolves to). No `tmux new -A -s vector-{profile-id}` ceremony.
- **D-05:** **The user owns the tmux session lifecycle.** If they want shell-state persistence across full disconnects, they run `tmux new -s myname` themselves once they're in. Vector does not detect, create, attach, share, or name tmux sessions.
- **D-06:** **PERSIST-04 (passthrough smoke test) is kept, reframed.** The smoke test verifies: when the user IS running tmux on the remote, Vector's terminal correctly passes through DCS-wrapped OSC 52 (Pitfall 8), DECSCUSR cursor shapes, mouse modes 1000/1002/1003 + SGR 1006, and `TERM=xterm-256color` advertisement. This is the same correctness check Phase 5 D-71 made locally, verified end-to-end against real tmux on a real remote.

### Reconnect overlay UX (D-07..D-09)

- **D-07:** **Inline status bar at pane top.** Thin (~22px / one terminal row + padding) bar overlaid at the top of the affected pane. Text format: `Reconnecting to {profile-display-name}… (attempt N)`. Grid + scrollback remain fully visible underneath. Bar disappears on successful reconnect.
- **D-08:** **Backoff schedule: 1s / 2s / 4s / 8s / 16s / 30s cap.** Classic exponential. After the cap, keep retrying at 30s indefinitely. Per attempt, log the failure reason at `tracing::warn` (Phase 5 logging conventions).
- **D-09:** **Never give up automatically.** No timeout, no "Reconnect failed — click to retry" state. The pane keeps trying forever (at the 30s cap) until the user closes it with Cmd-W or the transport comes back. Status bar's attempt counter grows visibly.

### Transport hot-swap mechanics (D-10..D-12) — planner-detail-ish but locked here for clarity

- **D-10:** **`Domain::reconnect()` is the integration point.** `LocalDomain::reconnect` stays a no-op. `DevTunnelDomain::reconnect` implements the backoff loop and SDK call, returns `Result<Box<dyn PtyTransport>>` (the trait already declares `async fn reconnect(&self) -> Result<()>` — planner may extend the signature if needed, but the seam stays in `vector-mux/src/domain.rs`).
- **D-11:** **No-byte-loss verification approach is planner's discretion.** The test is locked by SC#2: disconnect mid-`cat /dev/urandom`, reconnect, assert no byte loss. Implementation choices (sequence numbers in the agent protocol, drain-old-transport-before-swap, MD5 checkpoint, etc.) are not user-visible — let the planner pick the cleanest.
- **D-12:** **State machine: Active → Reconnecting → Swapping → Active.** As per ROADMAP. Tab badge updates: `[remote]` → `[reconnecting]` → `[remote]`. Inline status bar mirrors the state.

### Claude's Discretion

- Exact pixel/character dimensions of the status bar overlay (~22px is a rule of thumb; planner picks the exact metric in the renderer).
- Animation/transition between Active and Reconnecting (fade vs instant; planner picks if any).
- Whether to debounce the overlay (e.g. don't show it for blips < 250ms because they look like a flicker).
- Whether `tracing` events on reconnect emit at `INFO` or `WARN` — Phase 5 conventions apply.
- SIGWINCH/resize behavior during a blip (queue or drop? — likely "send size on reconnect" since the agent re-reads window size on `open_pty` anyway).
- Whether to extend the agent JSON protocol with a resume-from-byte-offset op (planner may decide we DON'T need it if drain-and-swap is correct).
- Whether the soft visual cue for locked input is a brief flash or a dim-cursor.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap + Requirements (revised in this commit)

- `.planning/ROADMAP.md` §"Phase 9: Persistence + Reconnect + tmux Auto-Attach" — phase goal, dependencies, success criteria. Note: SC#3 (tmux auto-attach) is **dropped** and SC#4 (tmux passthrough smoke) is **reframed** in this commit to align with D-04..D-06.
- `.planning/REQUIREMENTS.md` lines 72-75 (PERSIST-01..04) — acceptance criteria. PERSIST-03 is revised in this commit to remove the auto-attach mandate and replace it with "user owns tmux".

### Prior phase decisions that bind Phase 9

- `.planning/phases/04-mux-tabs-splits/04-CONTEXT.md` — D-38 byte-identical seam invariant (no transport-aware code in `vector-term`); `Domain::reconnect()` trait declaration in `crates/vector-mux/src/domain.rs:44`.
- `.planning/phases/05-polish-local-daily-driver/05-CONTEXT.md` §"Clipboard + tmux passthrough" — D-71: accept both raw OSC 52 and DCS-wrapped, chunk outbound payloads at 58 bytes for tmux passthrough; reference for what PERSIST-04 (revised) must verify works against a REAL remote tmux.
- `.planning/phases/08-vs-code-remote-tunnels-connect/08-CONTEXT.md` — D-12..D-15 agent JSON protocol (the wire format reconnect re-runs `open_pty` on); D-14 single-shell-per-tunnel-connection (preserved; Phase 9 does not change the session model).

### Pitfalls + Anti-Patterns

- `.planning/research/PITFALLS.md` §Pitfall 8 — Tmux DCS passthrough, ~60-char truncation, `allow-passthrough on`. Phase 9 reverifies this end-to-end against real tmux on a real remote.
- `.planning/research/PITFALLS.md` §Pitfall 22 — No mosh-style state-sync protocol. Locks D-03 (no input replay) and the entire "tmux on the remote, not in Vector" decision direction.
- `.planning/research/PITFALLS.md` §Architecture Anti-Pattern 1 (D-38) — No transport-aware code in `vector-term`. Reconnect logic lives in `vector-mux` and the per-Domain impls, never inside the terminal model.
- `.planning/research/PITFALLS.md` §Architecture Anti-Pattern 5 — Never hold the terminal lock across `await`. Critical for the swap: lock → mutate the `Pane.transport` field → drop → await.

### Code-level seams

- `crates/vector-mux/src/domain.rs:30-45` — `Domain` trait. `reconnect()` declaration is the integration point.
- `crates/vector-mux/src/transport.rs:17` — `PtyTransport` trait. The thing that gets hot-swapped.
- `crates/vector-mux/src/local_domain.rs:107` — `LocalDomain::reconnect` no-op reference.
- `crates/vector-mux/src/devtunnel_domain.rs:41-43` — `DevTunnelDomain::reconnect` `unimplemented!("Phase 9: Persistence + reconnect")` — this is what gets filled in.
- `crates/vector-tunnels/src/transport.rs` — `DevTunnelTransport` (Phase 8). The SDK call site for relay-channel + agent-protocol reuse on reconnect.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- **`Domain::reconnect()` trait method** is already declared in `crates/vector-mux/src/domain.rs:44`. `LocalDomain::reconnect` is a no-op. `DevTunnelDomain::reconnect` is an `unimplemented!("Phase 9: Persistence + reconnect")` stub. The trait seam is final — Phase 9 just fills it in.
- **`DevTunnelTransport::connect()`** (Phase 8 followup commit `66d95e0`) opens a relay channel + speaks the agent JSON protocol + returns a `PtyTransport`. Reconnect re-runs this exact call site under the existing `Pane`.
- **Status-bar overlay component** does not exist yet. Phase 5's inline overlays (e.g. the OSC 52 toast in D-70) used `vector-app/src/toast.rs` — pattern is reusable but a new persistent (not auto-dismissing) overlay is needed for reconnect status.
- **Tab badge string formatting** (`[remote]` in Phase 7/8) lives in `vector-app::format_tab_title` (or equivalent — search for `[remote]`). Adding `[reconnecting]` is a trivial extension.
- **Exponential backoff helper:** workspace already uses `tokio::time::sleep`. No bespoke crate needed (`backoff` crate is overkill).

### Established Patterns

- **Per-pane actor model** from Phase 4: each pane has its own `tokio` task driving its transport. The actor is the natural owner of the reconnect loop — it observes EOF on its `PtyTransport`, transitions to `Reconnecting`, calls `Domain::reconnect()`, on success swaps the transport and resumes the read loop.
- **`tokio::sync::Mutex` and `Arc<Mutex<Term>>`** is the lock pattern for the terminal model. Architecture Anti-Pattern 5 lock-mutate-drop-await applies tightly here: the swap of `Pane.transport` must NOT hold the term lock across the new transport's await.
- **`tracing::warn` for transient failures** with structured fields (`profile=`, `attempt=`, `error=`). Phase 8 uses this pattern already in the agent error paths.

### Integration Points

- **Pane actor → Mux::resize_window** path stays unchanged. SIGWINCH delivery during a blip queues at the actor; the actor sends a `resize` op to the new transport once the swap completes.
- **Renderer → status-bar overlay** is a new draw-layer concern. The renderer already composites per-pane; adding a thin top-of-pane overlay is a Compositor extension (see `crates/vector-render` Phase 4 changes).
- **Agent JSON protocol** stays as-is. Phase 9 does NOT extend it. Reconnect = new `connect_to_port` + new `open_pty`. (Sequence numbers / resume-from-offset are explicit non-goals; tmux on the remote is the answer for shell-state persistence — except now the user owns that, not Vector.)

</code_context>

<specifics>
## Specific Ideas

- **User-led tmux is the headline departure from the original roadmap.** The user explicitly stated: "we do not have to start with a tmux pane… I want to first log in to a bash session and then I am responsible to create and maintain the tmux session." This is the binding constraint for D-04..D-06 and drives the ROADMAP/REQUIREMENTS edits committed alongside this CONTEXT.
- **Status bar text is user-readable, not jargon.** `Reconnecting to corp-dev-box-42… (attempt 3)`, not `transport=DevTunnel state=Reconnecting attempt=3`.
- **Mental model:** A remote pane is a long-lived "channel to a remote box" — the box may briefly drop the call, we redial, conversation resumes. Like a VOIP app handling a wifi-to-cellular handoff. NOT like SSH where a disconnect means "kill the pane and start over."
- **Pitfall 8 verification is the trust gate for v1 shipping.** Phase 5 verified DCS-OSC-52 locally. Phase 9's smoke test verifies the same correctness through the Dev Tunnels relay and a real remote tmux. If this regresses, clipboard sharing through tmux silently breaks for every Vector user.

</specifics>

<deferred>
## Deferred Ideas

These ideas surfaced during discussion but are out of scope for Phase 9 v1.

### Carried over from earlier phase deferrals (still apply)
- **Vector-managed tmux session lifecycle** — was the original PERSIST-03 plan; dropped in this phase per user direction (see D-04..D-06). If a future phase wants to add an opt-in `tmux_autoattach = true` profile field, that's a separate phase (likely milestone v1.1).
- **Mosh-style state-sync protocol** — Pitfall 22 explicit reject.
- **Predictive local echo** — Pitfall 22 explicit reject.

### Surfaced during discussion
- **App-restart restore (relaunch Vector → reopen remote panes)** — Big additional surface: needs `Mux`/`Window`/`Tab` tree serialization, profile-id resurrection at startup, possibly split-tree restoration. Likely Milestone v1.1 phase.
- **Local pane shell-death recovery overlay** — "shell exited — press Enter to restart" treatment. Currently local panes die silently. Worth doing for parity, but not in Phase 9 scope. Add to backlog.
- **SIGWINCH coalescing during blips** — micro-optimization for users who resize the window many times during a blip. Default behavior (send size on reconnect, drop intermediate) is fine for v1; revisit if it ever bites.
- **Reconnect attempt logging panel** — surfacing the `tracing::warn` events for the current pane in a UI inspector. Diagnostic feature, defer to a v1.x debug-tools phase.
- **Per-profile backoff configuration** — letting power users override the 1/2/4/8/16/30 schedule per profile (e.g. tighter for known-good corporate links). v1.x ergonomics.
- **Custom give-up policy per profile** — if a user wants Vector to stop retrying after N minutes for specific profiles, defer to v1.x.

</deferred>

---

*Phase: 09-persistence-reconnect-tmux-auto-attach*
*Context gathered: 2026-05-22*
