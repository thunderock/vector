# Phase 9: Persistence + Reconnect — Research

**Researched:** 2026-05-22
**Domain:** Transport-level reconnect for remote panes (tokio actor model, wgpu overlay UI, Dev Tunnels relay)
**Confidence:** HIGH (all critical claims grounded in existing source files; one MEDIUM area called out below)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Reconnect scope (D-01..D-03):**
- **D-01:** Scope = network blips only. TCP/SSH disconnect, wifi flap, lid-close → resume. The Vector process stays alive throughout; the `Pane` and its `Window`/`Tab` parents stay in memory. App restart does NOT restore remote panes (user re-picks).
- **D-02:** Grid + scrollback stay visible during the blip. Last byte received remains on screen. Cursor stops blinking. User can scroll the scrollback while reconnecting.
- **D-03:** Input is LOCKED during the blip, not queued. Keystrokes typed while the transport is dead are dropped with a soft visual cue (no replay on reconnect — replay risks lands keystrokes on possibly-different shell state).

**tmux model (D-04..D-06) — DEPARTS FROM ORIGINAL ROADMAP:**
- **D-04:** Vector does NOT wrap remote shells in tmux. Spawn command sent to agent is the plain default shell. No `tmux new -A -s vector-{profile-id}` ceremony.
- **D-05:** The user owns the tmux session lifecycle. If they want shell-state persistence across full disconnects, they run `tmux new -s myname` themselves. Vector does not detect, create, attach, share, or name tmux sessions.
- **D-06:** PERSIST-04 (passthrough smoke test) is kept, reframed. Smoke test verifies: when the user IS running tmux on the remote, Vector's terminal correctly passes through DCS-wrapped OSC 52 (Pitfall 8), DECSCUSR cursor shapes, mouse modes 1000/1002/1003 + SGR 1006, and `TERM=xterm-256color` advertisement.

**Reconnect overlay UX (D-07..D-09):**
- **D-07:** Inline status bar at pane top. Thin (~22px / one row + padding). Text: `Reconnecting to {profile-display-name}… (attempt N)`. Grid + scrollback fully visible underneath. Bar disappears on success.
- **D-08:** Backoff schedule 1s / 2s / 4s / 8s / 16s / 30s cap. Classic exponential. After cap, keep retrying at 30s indefinitely. Log failure at `tracing::warn` per attempt.
- **D-09:** Never give up automatically. No timeout. No "click to retry". Pane keeps trying forever until user closes with Cmd-W or transport comes back.

**Transport hot-swap mechanics (D-10..D-12):**
- **D-10:** `Domain::reconnect()` is the integration point. `LocalDomain::reconnect` stays a no-op. `DevTunnelDomain::reconnect` implements the backoff loop and SDK call. Trait already declares `async fn reconnect(&self) -> Result<()>`; planner may extend the signature if needed, but the seam stays in `crates/vector-mux/src/domain.rs`.
- **D-11:** No-byte-loss verification approach is planner's discretion. The test is locked by SC#2: disconnect mid-`cat /dev/urandom`, reconnect, assert no byte loss. Implementation choices (sequence numbers in the agent protocol, drain-old-transport-before-swap, MD5 checkpoint, etc.) are not user-visible.
- **D-12:** State machine `Active → Reconnecting → Swapping → Active`. Tab badge updates `[remote]` → `[reconnecting]` → `[remote]`. Inline status bar mirrors the state.

### Claude's Discretion

- Exact pixel/character dimensions of the status bar overlay (~22px rule of thumb; planner picks exact metric in renderer).
- Animation/transition between Active and Reconnecting (fade vs instant; planner picks if any).
- Whether to debounce the overlay (e.g. don't show it for blips < 250ms).
- Whether `tracing` events on reconnect emit at `INFO` or `WARN` — Phase 5 conventions apply.
- SIGWINCH/resize behavior during a blip (likely "send size on reconnect" since the agent re-reads window size on `open_pty`).
- Whether to extend the agent JSON protocol with a resume-from-byte-offset op (planner may decide we DON'T need it if drain-and-swap is correct).
- Whether the soft visual cue for locked input is a brief flash or a dim-cursor.

### Deferred Ideas (OUT OF SCOPE)

- Vector-managed tmux session lifecycle (was the original PERSIST-03 plan; dropped per D-04..D-06). If a future phase wants an opt-in `tmux_autoattach = true` profile field, that's a separate phase (likely v1.1).
- Mosh-style state-sync protocol — Pitfall 22 explicit reject.
- Predictive local echo — Pitfall 22 explicit reject.
- App-restart restore (relaunch Vector → reopen remote panes). Needs window/pane tree serialization. Likely v1.1.
- Local pane shell-death recovery overlay.
- SIGWINCH coalescing during blips.
- Reconnect attempt logging panel.
- Per-profile backoff configuration.
- Custom give-up policy per profile.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **PERSIST-01** | On TCP/SSH disconnect, the affected pane enters a `Reconnecting` state, the local grid + scrollback are kept in memory, and a reconnect overlay is shown. | State machine lives in the pane I/O actor (§Architecture P1). Overlay = `ToastPass`-style wgpu pipeline added to `ChromePipelines` (§Architecture P3). Grid stays alive because the `Pane.term` `Arc<Mutex<Term>>` is independent of the transport — only `Pane.transport` is hot-swapped (§Standard Stack). |
| **PERSIST-02** | `Domain::reconnect()` re-establishes the transport with exponential backoff and hot-swaps the `PtyTransport` under the live `Pane` without dropping bytes already in flight — verified by a test that disconnects + reconnects with `cat /dev/urandom` and asserts no byte loss. | Reconnect call site at `crates/vector-mux/src/devtunnel_domain.rs:41-43` (existing `unimplemented!` stub). Hot-swap approach = drain old reader into `CoalesceBuffer` to EOF, install new transport, resume read loop (§Pattern 2). Backoff = hand-rolled `tokio::time::sleep` (§Pattern 3 — no `backoff` crate). |
| **PERSIST-03 (revised)** | Vector does NOT auto-attach to tmux. Remote panes drop into the user's default shell. | No code change required — Phase 8 agent `open_pty` already sends `shell: None` (see `crates/vector-tunnels/src/transport.rs:84-89`). Documentation-only requirement; ensure no plan accidentally adds a `tmux new -A` wrap. |
| **PERSIST-04 (revised)** | End-to-end smoke test against a live Dev Tunnels agent on a remote box running tmux 3.4+ verifies DCS-wrapped OSC 52, DECSCUSR, mouse 1000/1002/1003 + SGR 1006, `TERM=xterm-256color` — when the user is running tmux themselves. | Test pattern reuses the existing `osc52_tmux.rs` shape (`crates/vector-term/tests/osc52_tmux.rs`) extended over a real Dev Tunnels relay. Test infrastructure: ignored `#[ignore]` test gated on `VECTOR_E2E_TUNNEL_ID` env var (§Validation Architecture). |

</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **Rust 1.88+** pinned via `rust-toolchain.toml`. Edition 2021.
- **macOS 13+ baseline.** Universal binary (arm64 + x86_64 via `lipo`).
- **Workspace dep pins** are LOCKED — no version bumps in Phase 9: `tokio 1.52`, `russh 0.60` (patched), `wgpu 29`, `winit 0.30`, `alacritty_terminal 0.26`, `portable-pty 0.9`, `objc2-app-kit 0.3`, `tracing` + `tracing-subscriber`.
- **Comments:** succinct, single short line, only when WHY is non-obvious. No multi-paragraph docstrings.
- **Linting:** use the project's existing `make` / cargo commands. Don't switch toolchains. The repo's lint command is whatever CI runs (see `.github/workflows/ci.yml` — `lint` and `test` jobs gate the branch).
- **Scope discipline:** if not on the v1 list, default to deferring.
- **No `tokio::main`** — workspace-wide arch-lint (the `no_tokio_main.rs` tests in every crate enforce this). The reconnect loop runs on the existing tokio runtime via `Mux::get_handle()` or similar.

## Summary

Phase 9 fills in the `DevTunnelDomain::reconnect` stub (`crates/vector-mux/src/devtunnel_domain.rs:41-43`) and adds a `Reconnecting` state machine to the per-pane I/O actor in `crates/vector-app/src/pty_actor.rs`. The architecture is already in place: the `Domain` trait already declares `async fn reconnect(&self) -> Result<()>` (`crates/vector-mux/src/domain.rs:44`), the `Pane.transport` field is already a `Mutex<Option<Box<dyn PtyTransport>>>` (`crates/vector-mux/src/pane.rs:87`) that supports atomic hot-swap, and `DevTunnelTransport::connect()` is the existing call site (`crates/vector-tunnels/src/transport.rs:199-245`) that the reconnect loop will re-invoke.

Five new concerns: (1) detecting transport death without coupling to russh internals — observe EOF on `reader.recv()` returning `None` and `transport.wait()` resolving in the existing biased select loop; (2) hot-swap discipline — never hold a lock across `await` (the Anti-Pattern 5 idiom established in `mux.rs:397`'s `create_tab_async`); (3) the inline status bar overlay — a new `ReconnectPass` wgpu pipeline added to `ChromePipelines` (`crates/vector-app/src/chrome.rs:13-29`) using `ToastPass` as the structural template; (4) input gating — the existing `EncodedKey::App` dispatch in `app.rs` adds a `pane_state.is_reconnecting(pane_id)` early-return that drops keystrokes; (5) the live tmux smoke test — extends the existing `crates/vector-term/tests/osc52_tmux.rs` pattern with a `VECTOR_E2E_TUNNEL_ID` env-gated `#[ignore]`d e2e variant.

**Primary recommendation:** Implement reconnect entirely inside the per-pane I/O actor (`pane_io_loop`) by replacing the current "exit on EOF" path with a `reconnect_loop` that drains, sleeps with backoff, calls `Domain::reconnect()` to obtain a fresh transport, hot-swaps it under `Pane.transport`, and resumes. The `DevTunnelDomain` stub gets a `connect_handle` field populated at construction so `reconnect()` can re-run the same `connect_tunnel(...)` helper the picker actor uses.

## Standard Stack

### Core (already present in workspace)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `tokio` | 1.52.3 | async runtime; `time::sleep` for backoff; `select! biased` for actor loop | Project-wide pin. The per-pane actor at `crates/vector-app/src/pty_actor.rs:114-156` already uses `biased` select; reconnect re-uses the same pattern. |
| `tokio-util` | 0.7 (via workspace) | `CancellationToken` for aborting backoff on pane close | Already used by `devtunnels_actor` (`vector-app/src/devtunnels_actor.rs:266`). Standard tokio cooperative-cancel pattern. |
| `tracing` | workspace | structured warn/info logs per attempt | Phase 8 pattern (`crates/vector-tunnels/src/transport.rs:167-173`). Match the `?pane_id, ?err` field convention from `pty_actor.rs:73`. |
| `wgpu` | 29.0.3 | new `ReconnectPass` pipeline for inline status bar | Existing pipelines (`ToastPass`, `SearchBarPass`, `TintStripePipeline`) provide template — see `crates/vector-render/src/` and `crates/vector-app/src/chrome.rs:11-28`. |
| `parking_lot::Mutex` | workspace | `Pane.transport: Mutex<Option<Box<dyn PtyTransport>>>` swap | Already in place (`pane.rs:87`). Sync mutex inside Arc — held briefly, released before await. |
| `async_trait` | workspace | `Domain::reconnect` is `async fn` in trait | Same pattern as `Domain::spawn` (`domain.rs:33`). No churn. |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tokio::sync::mpsc` | tokio | actor command channel (new `PaneCommand::Reconnect` variant) | Already used in `pty_actor.rs:26-27` for write/resize. Reconnect-trigger may not need a separate channel — see Pattern 1 (transport-death-detected in actor loop body). |
| `anyhow::Result` | workspace | bubble errors back to the `Domain::reconnect` trait method | Trait return type already `Result<()>`. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-rolled `tokio::time::sleep` loop | `backoff 0.4` crate | `backoff` brings `tokio` integration via a feature but is ~20 lines of value for a new dep. CLAUDE.md scope discipline + Phase 5 §"Risks & notes" idiom (no bespoke crate where 20 lines of tokio do it) → reject. |
| New `ReconnectPass` wgpu pipeline | Reuse `ToastPass` with a custom mode | Toast is the wrong abstraction — it auto-dismisses, lives at window-bottom, and is window-scoped (one toast per window). Reconnect overlay is pane-scoped, persists until success, and lives at pane-top. Sharing a pipeline forces too many "is this a toast or reconnect" branches. Build a new pipeline that copies ~80% of `ToastPass` source. |
| Sequence numbers in agent protocol | Drain-and-resync only | Sequence numbers buy "we can replay" — but D-03 explicitly drops input on disconnect, and the byte-loss test (SC#2) is about output direction. Drain-old-reader-to-EOF before swap is sufficient on the OUTPUT path: agent's TCP socket FIN tells us the relay end-of-stream; everything in `reader.recv()` queue still arrives. **Verification:** Phase 8's `pump` task at `crates/vector-tunnels/src/transport.rs:132-185` flushes `read_tx` until either EOF or error — so the existing `mpsc::Receiver<Vec<u8>>` already buffers in-flight bytes. |
| Re-issue `clear; tput reset` on reconnect | Don't | Phase 5 already advertises `TERM=xterm-256color` and the shell's `$PROMPT_COMMAND` redraws naturally. PERSIST-03's "user owns tmux" means we don't even pretend to track shell state — we just hand them a fresh bash/zsh. |

**No installation needed** — every crate is already in `Cargo.toml`.

**Version verification:** No new deps added in Phase 9. All listed crates already pinned in `Cargo.toml` workspace section.

## Architecture Patterns

### Recommended Project Structure (no new crates)

```
crates/vector-mux/src/
├── domain.rs              # Domain trait — reconnect() already declared at line 44
├── devtunnel_domain.rs    # FILL IN reconnect stub at line 41-43
├── pane.rs                # No changes to Pane shape; transport is already swappable
└── local_domain.rs        # reconnect() stays no-op (line 107-109)

crates/vector-tunnels/src/
├── transport.rs           # DevTunnelTransport — existing connect() at line 199-245 is reused
└── domain.rs              # connect_tunnel() helper — reused by both picker actor + reconnect

crates/vector-app/src/
├── pty_actor.rs           # ADD: pane_state enum (Active/Reconnecting/Swapping); MODIFY pane_io_loop
├── chrome.rs              # ADD: reconnect: ReconnectPass field to ChromePipelines (line 13-18)
├── app.rs                 # ADD: input gating in EncodedKey dispatch; render the reconnect overlay
└── lib.rs                 # ADD: UserEvent::PaneReconnecting { pane_id, attempt }
                            #      UserEvent::PaneReconnected { pane_id }

crates/vector-render/src/
└── reconnect_pass.rs      # NEW FILE: clone of toast_pass.rs structure; pane-top overlay rect
```

### Pattern 1: Per-pane reconnect loop (state machine lives in `pane_io_loop`)

**What:** The `Active → Reconnecting → Swapping → Active` state machine is implemented as the structure of the actor's main loop. No separate `enum State` field is needed; the state IS where in the loop body the task is currently parked.

**When to use:** This is the canonical path for D-10. The actor already owns the transport (the `mut transport: Box<dyn PtyTransport>` parameter at `pty_actor.rs:116`), so it can swap it without any external coordination.

**Example structure (pseudocode following the existing `pane_io_loop` shape):**

```rust
// Source: extending crates/vector-app/src/pty_actor.rs:114-156
async fn pane_io_loop(
    pane_id: PaneId,
    mut transport: Box<dyn PtyTransport>,
    domain: Arc<dyn Domain>,           // NEW — held to call reconnect()
    profile_label: String,              // NEW — for the status bar text
    proxy: EventLoopProxy<UserEvent>,
    coalesce: Arc<CoalesceBuffer>,
    mut write_rx: mpsc::Receiver<Vec<u8>>,
    mut resize_rx: mpsc::Receiver<(u16, u16)>,
    cancel: CancellationToken,          // NEW — abort backoff on pane close
) {
    'outer: loop {
        // State: Active. Run the existing biased select until reader EOF.
        let exit_reason = run_active(&mut transport, &coalesce, &mut write_rx, &mut resize_rx, &proxy, pane_id).await;
        match exit_reason {
            ExitReason::Closed => break 'outer,               // user Cmd-W
            ExitReason::TransportDead => {}                   // fall through to reconnect
        }

        // State: Reconnecting. Drain old reader queue into coalesce so no bytes lost.
        drain_to_end(&mut transport, &coalesce).await;

        // Run backoff loop. domain.reconnect() called on each iteration.
        let new_transport = match reconnect_with_backoff(&domain, &profile_label, &proxy, pane_id, &cancel).await {
            Some(t) => t,
            None => break 'outer,                              // cancelled
        };

        // State: Swapping. Replace transport, emit UserEvent::PaneReconnected.
        transport = new_transport;
        let _ = proxy.send_event(UserEvent::PaneReconnected { pane_id });
        // SIGWINCH on reconnect — the agent re-reads rows/cols on OpenPty, but
        // the user may have resized the window during the blip. Push current dims.
        // (Discretion D-08-discretion: planner picks "send size on reconnect".)
    }
    let _ = proxy.send_event(UserEvent::PaneExited(pane_id));
}
```

**Key contract:** `Domain::reconnect()` returns `Result<Box<dyn PtyTransport>>` — the planner extends the trait signature. The CONTEXT.md D-10 explicitly permits this: *"the trait already declares `async fn reconnect(&self) -> Result<()>` — planner may extend the signature if needed."* Without the new transport in the return value, the actor has no way to obtain the swapped instance. Trait signature change is small (one file: `crates/vector-mux/src/domain.rs:44`).

### Pattern 2: Hot-swap with zero byte loss (drain-old-then-swap)

**What:** Before constructing a new transport, drain the OLD transport's reader queue until `recv()` returns `None`. This flushes any bytes already buffered in the `mpsc::Receiver<Vec<u8>>` between the agent's TCP socket and the actor's read end. THEN install the new transport.

**Why this works (HIGH confidence — verified against `transport.rs:132-185`):**
- The Phase 8 pump task pushes every successfully-decoded `AgentMessage::Data { bytes }` into `read_tx` (line 158). The pump only breaks on EOF or decode error AFTER having sent the bytes (line 158 → 165).
- When TCP socket FINs, `buf_reader.read_line` returns `Ok(0)` and the loop breaks (line 152). Any data already enqueued in `read_tx` survives — the channel has capacity 64 (line 128).
- The reader the actor holds (`reader: mpsc::Receiver<Vec<u8>>`) drains independently. As long as the actor calls `reader.recv()` until it returns `None` BEFORE dropping the transport, all in-flight bytes reach the coalesce buffer → frame_tick → grid.

**The discipline:** Implement `drain_to_end(transport, coalesce)` as:

```rust
// Source: new function in crates/vector-app/src/pty_actor.rs
async fn drain_to_end(transport: &mut Box<dyn PtyTransport>, coalesce: &Arc<CoalesceBuffer>) {
    // The reader is owned by the actor (was take_reader'd at startup). Drain
    // any remaining frames until the pump task drops its sender.
    // (Implementation detail: actor holds reader as a local, not on transport;
    // this fn signature is sketch only — planner picks the exact shape.)
    // CRITICAL: do NOT hold any Pane lock here. Anti-Pattern 5.
}
```

**Anti-Pattern 5 enforcement:** The Pane's `transport: Mutex<Option<Box<dyn PtyTransport>>>` lock is `parking_lot::Mutex` (sync). To swap:

```rust
// LOCK → MUTATE → DROP, then AWAIT.
{
    let mut slot = pane.transport.lock();
    *slot = Some(new_transport);
}  // <-- guard dropped here
let _ = proxy.send_event(...);  // await/send AFTER the lock is released
```

This matches the idiom at `crates/vector-mux/src/pane.rs:137-139` (`take_transport`) and `crates/vector-mux/src/mux.rs:152-164` (`install_tab`'s write lock scope).

**Important caveat:** In the current actor model, the actor already OWNS the transport directly (it was moved via `transport: Box<dyn PtyTransport>` parameter at `pty_actor.rs:116`). The `Pane.transport` mutex is `None` after `take_transport()` is called by the actor at startup. So the "swap" is at the actor's local variable, not the mutex slot. Decision for the planner: either (a) leave `Pane.transport` as `None` after takeover and store the live transport solely in the actor's local — simplest; or (b) on swap, also write the new transport back into the mutex slot so external code (e.g. a future "force reconnect" UI command) can observe transport identity. Recommended: (a). Keep it simple; reconnect-trigger UI is deferred (CONTEXT.md "Deferred Ideas").

### Pattern 3: Exponential backoff (hand-rolled, no crate)

**What:** A loop with `tokio::time::sleep`, doubling delay until 30s cap, retries forever or until cancel.

**Why hand-roll:** The `backoff` crate adds a workspace dep for ~20 lines. CLAUDE.md scope discipline; Phase 5 already uses inline tokio sleep idioms (e.g. `notify-debouncer-full` consumer). The `Domain::reconnect()` trait method shape lets us keep the schedule local to `DevTunnelDomain::reconnect` if we want — or in the actor's loop. Recommendation: **schedule lives in the actor loop**, not in `DevTunnelDomain::reconnect`, because (1) the actor knows the pane_id and can emit `UserEvent::PaneReconnecting { pane_id, attempt }`, (2) `DevTunnelDomain::reconnect` becomes a single-shot "try to connect" — simpler shape, easier to mock in unit tests.

**Example (canonical pattern):**

```rust
// Source: new in crates/vector-app/src/pty_actor.rs
async fn reconnect_with_backoff(
    domain: &Arc<dyn Domain>,
    profile_label: &str,
    proxy: &EventLoopProxy<UserEvent>,
    pane_id: PaneId,
    cancel: &CancellationToken,
) -> Option<Box<dyn PtyTransport>> {
    const SCHEDULE: &[u64] = &[1, 2, 4, 8, 16, 30]; // seconds (D-08)
    let mut attempt: u32 = 1;
    loop {
        let _ = proxy.send_event(UserEvent::PaneReconnecting {
            pane_id,
            attempt,
            profile_label: profile_label.to_string(),
        });
        match domain.reconnect_one_shot().await {       // see trait change below
            Ok(t) => return Some(t),
            Err(err) => {
                tracing::warn!(?pane_id, attempt, ?err, "reconnect attempt failed");
            }
        }
        let delay_idx = ((attempt as usize) - 1).min(SCHEDULE.len() - 1);
        let delay = std::time::Duration::from_secs(SCHEDULE[delay_idx]);
        tokio::select! {
            biased;
            () = cancel.cancelled() => return None,
            () = tokio::time::sleep(delay) => {}
        }
        attempt = attempt.saturating_add(1);
    }
}
```

**Cancellation:** Tied to pane close — `App::handle_close_pane` cancels the token; the sleep wakes immediately; reconnect_with_backoff returns `None`; outer loop exits cleanly.

### Pattern 4: Inline status bar overlay — `ReconnectPass` (new wgpu pipeline)

**What:** A new pipeline in `crates/vector-render/src/reconnect_pass.rs` that draws a single thin colored rect + glyph row at the top of a per-pane viewport. Structure cloned from `crates/vector-render/src/` (look at `toast_pass.rs` shape — also referenced in `chrome.rs:25`).

**Why a new pipeline, not Toast:** Toast is window-scoped at the bottom, auto-dismissed, info/action variants. Reconnect is pane-scoped at the top, persistent until success, single variant. Sharing the pipeline would force a "kind" branch through `ToastPass` that distorts both surfaces.

**Composition order (extending the existing chrome pass at `app.rs:1061+`):**

```
1. Per-pane Compositor render_into_view (existing — terminal grid + selection + cursor + border)
2. ReconnectPass.render (NEW — per-pane top bar, ONE per reconnecting pane)
3. TintStripePipeline.render (existing — bottom of window)
4. SearchBarPass.render (existing)
5. ToastPass.render (existing)
6. PickerPass.render (existing)
```

**Where to hook in:** `crates/vector-app/src/app.rs` line ~1055 (immediately after the per-pane compositor loop, before the existing chrome snapshot block at line 1061). For each pane in the layout, if `pane_state.is_reconnecting(pane_id)`, push the reconnect rect onto a draw list keyed by pane rect.

**Pane state ownership:** Add a per-pane state map on `App` (not on `Pane` — `vector-mux` should not know about UI state). Sketch:

```rust
// crates/vector-app/src/app.rs (new field)
struct App {
    // ...existing fields...
    reconnecting_panes: HashMap<PaneId, ReconnectingState>,
}
struct ReconnectingState {
    profile_label: String,
    attempt: u32,
    started_at: Instant,  // for D-08-discretion debounce
}
```

Updated on `UserEvent::PaneReconnecting` (insert/overwrite) and `UserEvent::PaneReconnected` (remove).

### Pattern 5: Input gating during Reconnecting state

**What:** Early-return on keystrokes when `reconnecting_panes.contains_key(active_pane_id)`. Drop bytes; do NOT enqueue.

**Where:** `crates/vector-app/src/app.rs` — the keystroke dispatch handler (search for `EncodedKey::Raw` or `EncodedKey::Bytes` write paths feeding into `router.send_write`). Add the guard right before `router.send_write(pane_id, bytes)`:

```rust
// Source: extending existing dispatch in app.rs
if self.reconnecting_panes.contains_key(&active_pane_id) {
    // D-03: input locked during reconnect; drop with soft visual cue.
    // (planner picks: brief flash via existing toast? dim cursor? both?)
    return;
}
router.send_write(active_pane_id, bytes);
```

**Soft visual cue (D-03 + Discretion):** Recommendation — reuse `ToastBanner::info(...)` for a one-shot 5s message "Input ignored — reconnecting" on the FIRST dropped keystroke during a given Reconnecting span. Don't toast on every keystroke (flood). Reset the "first" flag on `PaneReconnected`.

### Anti-Patterns to Avoid

- **Holding `Pane.transport` lock across `await`.** Existing `parking_lot::Mutex` is sync — DROP the guard before any `.await`. Pattern enforced everywhere in `mux.rs`; the `create_tab_async` comment at line 396 calls this out explicitly: *"the `.await` happens BEFORE any RwLock write — no held lock across await points."* Same discipline for reconnect.
- **Storing reconnect state on `vector-mux::Pane`.** Pane is the transport-model seam; UI state (overlay, attempt counter) lives in `vector-app`. The CONTEXT.md "Architecture Anti-Pattern 1" reference (D-38 byte-identical seam) means `vector-term` and `vector-mux` stay transport-aware-only; UI overlay is `vector-app` + `vector-render`.
- **Centralized reconnect coordinator task.** Each pane runs its own reconnect loop. Phase 4 D-67 ("per-pane actor model") is the established pattern; centralizing would re-introduce the round-robin pump Phase 4 explicitly avoided (`pty_actor.rs:6-9`).
- **`Domain::reconnect()` returning `Result<()>` and the actor reaching back into the domain for the new transport via a separate getter.** Two-call APIs invite races (what if a second reconnect lands between the two calls?). Make `reconnect` return the new transport directly.
- **Input replay queue.** D-03 explicit reject. Replay is predictive-echo territory (Pitfall 22).
- **Adding sequence numbers to the agent protocol.** Discretion D-10-discretion permits it, but the protocol is intentionally simple (memory `project-phase8-tunnel-agent` says *"Do not bolt on protocol features... until v2."*). Drain-and-swap is sufficient for the byte-loss test.
- **Re-using `ToastPass` for the reconnect overlay.** See Pattern 4.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Async sleep with cancellation | Custom `Future` + waker | `tokio::time::sleep` + `tokio::select!` with `CancellationToken::cancelled()` | Standard tokio idiom; already used elsewhere. |
| Connection backoff | `backoff 0.4` or `tokio-retry` crate | Hand-rolled `for delay in SCHEDULE` loop | 20 LOC; CLAUDE.md scope discipline. |
| State-sync protocol | Mosh-style replay buffer | Drain-old-then-swap; user runs tmux for shell state | Pitfall 22 explicit reject. |
| Re-implementing `DevTunnelTransport::connect()` for reconnect | Inline a second copy of the SDK call | Call `connect_tunnel(&api, &auth, &tunnel_record, rows, cols)` from `crates/vector-tunnels/src/domain.rs:12-22` | Existing helper already constructs `Box<dyn PtyTransport>`. Reconnect needs the same inputs — the `DevTunnelDomain` should be constructed with these handles. |
| Custom JSON resume protocol | Add `Resume { offset }` to `AgentMessage` | Treat reconnect as a new session: `open_pty` again. The old session on the agent dies when its socket FINs. | Memory `project-phase8-tunnel-agent`: protocol stays simple. |
| Watchdog ping/pong to detect dead transport | App-level heartbeat | Observe EOF on the existing reader + `transport.wait()` resolving. The relay TCP layer + russh keepalive already detect dead sockets. | The pump task at `transport.rs:152` already breaks on `Ok(0)` (EOF) or `Err(_)` (io error). No new heartbeat needed. |

**Key insight:** Phase 9 is almost entirely a *wiring* phase. Every primitive — the trait seam, the transport constructor, the actor, the chrome pipeline shape — already exists. The new code is glue: a state machine in the actor, a `reconnect_one_shot()` body in `DevTunnelDomain`, a `ReconnectPass` pipeline, two `UserEvent` variants, and an input guard.

## Runtime State Inventory

> Phase 9 is a feature-addition phase, not a rename/refactor. This section is included for completeness because the live tmux smoke test (PERSIST-04) touches external runtime state.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — no databases, no on-disk session state. Vector v1 doesn't persist pane state across app restarts (CONTEXT.md D-01 out-of-scope). | None. |
| Live service config | Dev Tunnels are user-managed (the user creates them via `code tunnel` or via Vector's existing picker). No Vector-side service config to update. | None. |
| OS-registered state | `vector-tunnel-agent` is a Linux user-space binary the user installs themselves on the remote (memory `project-phase8-tunnel-agent`). Phase 9 doesn't change agent installation, just exercises it. | None — Phase 8 already documented agent setup. |
| Secrets/env vars | Microsoft + GitHub OAuth tokens in Keychain — unchanged. New env var introduced by Phase 9: `VECTOR_E2E_TUNNEL_ID` (test-only, gates the live e2e smoke test). | Document `VECTOR_E2E_TUNNEL_ID` in the smoke-test README; no Keychain churn. |
| Build artifacts | None new. No package renames. | None. |

## Common Pitfalls

### Pitfall 1: Holding the `Pane.term` lock across `await`

**What goes wrong:** Code that does `let term = pane.term.lock(); ...; pane.transport.write(bytes).await` deadlocks because the `parking_lot::Mutex` guard isn't released. With wgpu render frames blocked on the same lock, the UI freezes during reconnect.

**Why it happens:** `parking_lot::Mutex` returns a guard that's `!Send`; holding it across an await point either won't compile (good) or will compile in an `async {}` block that doesn't capture across the suspension point (subtler). The CONTEXT.md callout (Architecture Anti-Pattern 5) is *the* reason this is named in capital letters.

**How to avoid:**
1. Never lock `Pane.term` in the reconnect loop body. The terminal model is read by the renderer and written by the parser thread — reconnect doesn't touch term state.
2. If the planner adds any code that touches `pane.term`, drop the guard before any `.await`:
   ```rust
   let view_state = { let t = pane.term.lock(); t.cursor_pos() }; // guard dropped at };
   some_other_await(view_state).await;
   ```

**Warning signs:** UI freezes for the duration of a reconnect attempt; `cargo clippy` warns about lock guards in async blocks (clippy lint: `await_holding_lock`).

### Pitfall 2: Reconnecting the picker actor instead of per-pane

**What goes wrong:** The temptation is to add a "reconnect" command to `devtunnels_actor` (since it already owns the API + auth). But the actor's `Command::Connect` (`devtunnels_actor.rs:60-65`) creates a NEW pane via `Mux::create_tab_async_with_transport`. If reconnect goes through the picker actor, it'd either install a new tab (wrong — the user wants the same pane back) or need a parallel "install transport under existing pane_id" path.

**How to avoid:** Reconnect lives on the pane's I/O actor (`pty_actor.rs`). The `DevTunnelDomain` is constructed at connect time and stored somewhere the pane actor can reach — either: (a) `Pane` gets a new `domain: Arc<dyn Domain>` field, or (b) the per-pane actor takes `Arc<dyn Domain>` in its spawn signature. Option (b) is cleaner — no `vector-mux` API change beyond extending the existing `spawn_pane` call site in `pty_actor_router`.

**Where to wire:** The `devtunnels_actor::handle_connect` (`devtunnels_actor.rs:192-254`) already has the auth + api handles. After `connect_tunnel(...)` returns the transport, construct an `Arc<dyn Domain>` that captures (api_arc, auth_arc, tunnel_record, rows, cols) in its closure, and hand it to the new pane actor.

### Pitfall 3: Backoff schedule starts the clock from "transport dead detected" but the user perceives it from "screen froze"

**What goes wrong:** A wifi blip can leave TCP hanging for 30+ seconds before the kernel decides the socket is dead. The user sees the pane unresponsive for 30s, THEN sees "Reconnecting (attempt 1)…", THEN waits another 1s, then 2s, etc.

**How to avoid (planner discretion D-08-discretion notes the debounce angle):**
- Either: (a) accept this is reality — TCP detection is what it is — and let the user know the reconnect is *starting* the 1s wait, not "we've been trying for 30s already";
- Or: (b) introduce an application-layer keepalive (agent sends `{"type":"ping"}` every 5s; client expects pong; trigger reconnect on 3 missed pongs). This is OUT of scope per CONTEXT.md "No application-level heartbeat" implicit constraint (memory `project-phase8-tunnel-agent` says protocol stays simple).
- Recommendation: (a) for v1. Document the TCP-dead-timeout behavior in the smoke test as expected.

**Warning signs:** User-perceived latency between "wifi died" and "Reconnecting…" overlay > 30s without action. If this is unacceptable, scope a separate phase for transport-level keepalive.

### Pitfall 4: `Domain::reconnect()` signature breaks `LocalDomain`

**What goes wrong:** Extending the trait to return `Result<Box<dyn PtyTransport>>` forces `LocalDomain::reconnect` (`local_domain.rs:107-109`) to return SOMETHING. But `LocalDomain::reconnect` is documented as a no-op — local PTY death is permanent (shell exited).

**How to avoid:** Make the trait `async fn reconnect(&self) -> Result<Option<Box<dyn PtyTransport>>>`. `LocalDomain` returns `Ok(None)`. The actor treats `None` as "domain doesn't support reconnect — exit cleanly" — which is the local-PTY-shell-died path that already exists today. `DevTunnelDomain` always returns `Some(transport)` on success.

Alternative: separate trait `Reconnectable` that only `DevTunnelDomain` implements. Cleaner semantically but adds a downcast / `Option<Arc<dyn Reconnectable>>` somewhere. Recommendation: `Result<Option<...>>` for minimal API churn.

### Pitfall 5: Resize during the blip is lost

**What goes wrong:** User resizes window while pane is reconnecting. The actor's `resize_rx` channel buffers the new size — but the resize is delivered to the OLD (dead) transport, which fails silently. After reconnect, the new transport has the WRONG initial dims.

**How to avoid:** Track the latest `(rows, cols)` in the actor's local state. On every `resize_rx.recv()` while in the Active state, store the latest dims. On reconnect success, call `transport.resize(rows, cols, 0, 0)` immediately with the stored dims. Also pass the stored dims to `domain.reconnect_one_shot(rows, cols)` so the agent's `OpenPty` handshake sends the right initial size.

**Trait shape change:** `reconnect_one_shot(rows: u16, cols: u16) -> Result<Option<Box<dyn PtyTransport>>>`. Same as `Domain::spawn` taking a `SpawnCommand`. Symmetric.

### Pitfall 6: Tab badge stays `[remote]` instead of flipping to `[reconnecting]`

**What goes wrong:** `format_tab_title` at `crates/vector-mux/src/pane.rs:205-218` keys on `TransportKind` (cached from the original transport). The cached value never changes — so even when the pane is reconnecting, the badge says `[remote]`.

**How to avoid:** Add a second input to `format_tab_title` (or a sibling function) that takes a `PaneUiState` enum (`Active | Reconnecting`). The badge formatter is called by the App / TabWindow code (`crates/vector-app/src/tab_window.rs`) — find the call site, plumb the pane state from `App.reconnecting_panes` to the title formatter.

**Trade-off:** Modifying `format_tab_title`'s signature ripples through tests. Lighter alternative: leave `format_tab_title` alone; have the App overlay the `[reconnecting]` suffix at the AppKit/NSWindow tab-title set call. Recommendation: planner picks based on how `format_tab_title` is currently called — if it's the single source of truth for the tab string, modify it; if the App already post-processes, do the swap at the App layer.

### Pitfall 7: DCS OSC 52 chunking regresses through the relay

**What goes wrong:** Phase 5 chunked outbound OSC 52 at 58 bytes (D-71) to survive tmux passthrough's ~60-char truncation bug. The chunking is done by `vector-input::clipboard::osc52_outbound` — locally tested. When the path is Vector → Dev Tunnels relay → agent → user-started tmux on the remote, the relay layer might re-fragment the byte stream (Phase 5 didn't test this). If chunks arrive in different reads, tmux's DCS parser sees them as separate sequences and the base64 payload is corrupted.

**How to avoid:** The PERSIST-04 smoke test (§Validation Architecture) specifically verifies this end-to-end. If it fails, the fix is one of: (a) the agent buffers outbound and writes only on `\e\\` boundaries (sequence-aware framing — out of scope per memory `project-phase8-tunnel-agent`); (b) reduce chunk size below 58 bytes; (c) accept that very-large clipboard payloads through tmux-on-the-remote are broken and document it.

**Warning signs:** OSC 52 works locally (Phase 5 smoke green), fails through real tmux on the remote with payloads > ~60 bytes.

### Pitfall 8: Live smoke test depends on test rig that doesn't exist in CI

**What goes wrong:** Writing the PERSIST-04 e2e test as a normal `cargo test` makes it fail on every CI run because no Dev Tunnel agent is reachable from `macos-14` GitHub runners.

**How to avoid:** Pattern from `osc52_tmux.rs` (`crates/vector-term/tests/osc52_tmux.rs:7`): `#[ignore = "..."]` annotation + `cargo test ... -- --ignored` only on a dedicated CI job. Phase 9 mirrors this: the live test runs with `VECTOR_E2E_TUNNEL_ID=<id>` env var. The CI job is `continue-on-error: true` (matching the existing `tmux-smoke` job at `.github/workflows/ci.yml:100-113`). Local dev runs `--ignored` manually when they have a tunnel.

## Code Examples

### Trait extension (`crates/vector-mux/src/domain.rs`)

```rust
// Source: extending crates/vector-mux/src/domain.rs:30-45
#[async_trait::async_trait]
pub trait Domain: Send + Sync {
    async fn spawn(&self, cmd: SpawnCommand) -> Result<Box<dyn PtyTransport>>;
    fn label(&self) -> String;
    fn is_alive(&self) -> bool;

    /// LocalDomain returns Ok(None). DevTunnelDomain re-runs connect_tunnel.
    /// `rows` / `cols` are the latest known terminal dims (so the agent OpenPty
    /// handshake matches the user's current window size).
    async fn reconnect_one_shot(
        &self,
        rows: u16,
        cols: u16,
    ) -> Result<Option<Box<dyn PtyTransport>>>;
}
```

### `DevTunnelDomain` shape change (`crates/vector-mux/src/devtunnel_domain.rs`)

Currently a zero-field unit struct; the new shape needs to hold whatever `connect_tunnel` requires. But per WIN-04 (`crates/vector-mux/src/devtunnel_domain.rs:3-7`), `vector-mux` must stay free of `vector-tunnels` dep. Resolution: the concrete `DevTunnelDomain` impl that holds api+auth+tunnel handles **lives in `vector-tunnels`, not `vector-mux`**. The stub `DevTunnelDomain` in `vector-mux/devtunnel_domain.rs` stays as-is (or is deleted entirely). A new type `vector_tunnels::domain::ReconnectableDevTunnelDomain` implements `vector_mux::Domain` and is constructed in `devtunnels_actor::handle_connect`.

```rust
// Source: new in crates/vector-tunnels/src/domain.rs (extending current contents)
pub struct ReconnectableDevTunnelDomain {
    api: Arc<DevTunnelsApi>,
    token_store: Arc<MicrosoftTokenStore>,
    tunnel: Arc<TunnelRecord>,
    label: String,
}

#[async_trait::async_trait]
impl vector_mux::Domain for ReconnectableDevTunnelDomain {
    async fn spawn(&self, _cmd: vector_mux::SpawnCommand)
        -> anyhow::Result<Box<dyn vector_mux::PtyTransport>>
    {
        // Same shape as picker — unused via Domain seam, but keep symmetric.
        anyhow::bail!("use reconnect_one_shot — picker actor handles initial connect")
    }
    fn label(&self) -> String { self.label.clone() }
    fn is_alive(&self) -> bool { true /* opt-in for refinement later */ }
    async fn reconnect_one_shot(&self, rows: u16, cols: u16)
        -> anyhow::Result<Option<Box<dyn vector_mux::PtyTransport>>>
    {
        let auth = AuthProvider::Microsoft(/* load from token_store */);
        let t = connect_tunnel(&self.api, &auth, &self.tunnel, rows, cols).await?;
        Ok(Some(t))
    }
}
```

### `UserEvent` extensions (`crates/vector-app/src/lib.rs`)

```rust
// Source: appending to crates/vector-app/src/lib.rs:49 (UserEvent enum)
//   — appended; never reorder per established convention (see lib.rs:85 "appended; never reorder")
PaneReconnecting {
    pane_id: PaneId,
    attempt: u32,
    profile_label: String,
},
PaneReconnected {
    pane_id: PaneId,
},
```

### Reading the existing inline-overlay precedent

For the `ReconnectPass` wgpu pipeline structure, the closest template is `crates/vector-render/src/` — start from `chrome_quad.rs` (rect + solid color) and `tint_stripe.rs` (positioned overlay). The `ToastPass` (referenced at `vector-render` re-export in `chrome.rs:11`) handles text within a banner — copy the glyph-path code path. Per CONTEXT.md "Discretion": planner picks exact pixel dimensions.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Original Phase 9 roadmap: Vector auto-attaches via `tmux new -A -s vector-{profile-id}` | User-led tmux. Vector does not detect, spawn, attach, or name tmux. | 2026-05-22 (this CONTEXT) | Drops PERSIST-03 work; PERSIST-04 reframed as a passthrough-correctness check that runs only when the USER is in tmux. Removes ~1 plan's worth of complexity. |
| Re-implement reconnect via picker actor (parallel `Command::Reconnect` route) | Per-pane I/O actor owns the reconnect loop | Implicit since Phase 4 D-67 | Reconnect is local to the actor that observes the EOF; no cross-actor coordination. |
| Toast-style overlay for "Reconnecting…" | Dedicated pane-top `ReconnectPass` pipeline | Phase 9 planning | Pane-scoped UI is incompatible with window-scoped Toast. New pipeline, cloned from `ToastPass`. |
| Sequence numbers / resume offsets in agent protocol | Drain-old-then-swap; new session on reconnect | Phase 8 protocol freeze (memory `project-phase8-tunnel-agent`) | Simpler protocol. User-perceived behavior is identical (no shell state preserved either way; user runs tmux for that). |

**Deprecated/outdated:**
- The `tmux new -A` ceremony from the original ROADMAP §Phase 9 — explicitly dropped 2026-05-22.

## Open Questions

1. **Should the `DevTunnelDomain` stub in `crates/vector-mux/src/devtunnel_domain.rs` be deleted?**
   - What we know: it's currently a stub with no callers (per its own doc-comment: *"the picker actor (Plan 08-06) NEVER routes through DevTunnelDomain::spawn"*).
   - What's unclear: whether deleting it now is a Phase 9 task or a separate cleanup.
   - Recommendation: leave it as-is; making the trait extension a no-op for that stub keeps the diff smaller. Mark for v1.x cleanup.

2. **Where should `ReconnectableDevTunnelDomain` actually live — `vector-tunnels::domain` or a new `vector-tunnels::reconnect_domain` module?**
   - What we know: the existing `crates/vector-tunnels/src/domain.rs` is a tiny 22-line file with just `connect_tunnel`. Adding the type there fits.
   - What's unclear: future expansion (Codespace `ReconnectableDomain`?). For v1, single file is fine.
   - Recommendation: same file, single module. Planner can refactor later.

3. **Does the `LocalDomain::reconnect_one_shot` body need any change?**
   - What we know: local PTY death is permanent (shell exited).
   - What's unclear: nothing — returning `Ok(None)` is correct.
   - Recommendation: implement as `async fn reconnect_one_shot(&self, _rows: u16, _cols: u16) -> Result<Option<Box<dyn PtyTransport>>> { Ok(None) }`.

4. **Should the reconnect loop emit `tracing::warn` or `tracing::info` for the FIRST attempt?**
   - What we know: D-08 says `warn` per attempt.
   - What's unclear: whether the very first "transport died" event should be `warn` or `error`.
   - Recommendation: `warn` for the first failure (just lost the connection), `warn` for each subsequent attempt. `info` for `PaneReconnected` success. Reserve `error` for cases where the trait reports `reconnect_one_shot` returned an unrecoverable error (e.g. auth permanently invalid — though CONTEXT.md D-09 says we don't give up; so this case may not exist).

5. **Live e2e test for SC#2 byte-loss verification: how do we trigger a disconnect mid-`cat /dev/urandom`?**
   - What we know: easiest in-process: instantiate a fake `Domain` that returns a fake transport, write urandom bytes, drop the transport at offset N, expose a "new transport with bytes from offset N+1 onward" via test fixture, assert checksum matches.
   - What's unclear: whether the live e2e test should ALSO do this against a real tunnel (would require an agent-side "drop me" command — out of scope).
   - Recommendation: byte-loss verification is a UNIT/INTEGRATION test against a `tokio::io::duplex`-based fake transport pair (the same pattern as `transport_protocol.rs:21-73`). The live e2e test (PERSIST-04) is for passthrough correctness, not byte-loss.

## Environment Availability

Phase 9 development environment was probed on 2026-05-22:

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| `cargo` | Workspace build | ✓ | 1.88.0 (873a06493 2025-05-10) | — |
| `rustc` | Workspace build | ✓ | 1.88.0 (6b00bc388 2025-06-23) | — |
| `tmux` | PERSIST-04 smoke test (local rehearsal) | ✓ | 3.6a | — |
| Linux remote box w/ `vector-tunnel-agent` installed | PERSIST-04 live e2e | ✗ on dev box; user-provided | — | Test marked `#[ignore]`; gated on `VECTOR_E2E_TUNNEL_ID` env var. Skipped in CI; user runs locally when they have a tunnel. |
| `pbpaste` | PERSIST-04 OSC 52 verification | ✓ (macOS) | system | — |
| Dev Tunnels relay | PERSIST-04 live e2e | n/a (Microsoft-hosted) | — | — |

**Missing dependencies with no fallback:** None — the e2e test is opt-in.

**Missing dependencies with fallback:** Remote agent host — gated behind `#[ignore]` + env var. Same pattern as `osc52_tmux.rs`.

**Note on tmux 3.4+ requirement:** Local tmux 3.6a meets the floor. The user MUST install tmux ≥ 3.4 on their remote box themselves (PERSIST-03 explicit). Phase 9 test setup should document this in the test header.

## Validation Architecture

> Nyquist validation is enabled per `.planning/config.json` (`workflow.nyquist_validation: true`).

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (workspace built-in; no external test runner) |
| Config file | None — uses Cargo defaults |
| Quick run command | `cargo test -p vector-mux -p vector-tunnels -p vector-app --lib` (skips integration tests; ~10s) |
| Full suite command | `cargo test --workspace --all-targets` (includes `tests/` and `#[ignore]`d tests selected manually) |
| Live e2e command | `VECTOR_E2E_TUNNEL_ID=<id> cargo test -p vector-app --test persist_e2e -- --ignored --nocapture` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PERSIST-01 | Pane stays alive on transport EOF; `reconnecting_panes` map gets entry; grid/scrollback unchanged | unit | `cargo test -p vector-app pty_actor_reconnect_state` | ❌ Wave 0 |
| PERSIST-01 | `UserEvent::PaneReconnecting { pane_id, attempt, profile_label }` is emitted on first attempt | unit | `cargo test -p vector-app reconnect_emits_pane_reconnecting_event` | ❌ Wave 0 |
| PERSIST-01 | Input is dropped (not queued) while pane in Reconnecting state | unit | `cargo test -p vector-app input_locked_during_reconnect` | ❌ Wave 0 |
| PERSIST-01 | `ReconnectPass` renders a top-of-pane banner with text matching `Reconnecting to {label}… (attempt N)` | unit (snapshot-style on glyph layout, not pixel) | `cargo test -p vector-app reconnect_overlay_text_format` | ❌ Wave 0 |
| PERSIST-02 | `Domain::reconnect_one_shot` returns `Some(new_transport)` after one or more backoff cycles | unit | `cargo test -p vector-tunnels reconnect_one_shot_returns_transport` | ❌ Wave 0 |
| PERSIST-02 | Backoff schedule matches 1/2/4/8/16/30/30/30… (use `tokio::time::pause()` + advance) | unit | `cargo test -p vector-app reconnect_backoff_schedule` | ❌ Wave 0 |
| PERSIST-02 | Cancellation via `CancellationToken::cancel()` aborts the sleep within ms | unit | `cargo test -p vector-app reconnect_cancellable` | ❌ Wave 0 |
| PERSIST-02 | **No byte loss across hot-swap.** Two-transport fake: write N bytes via transport A, drop A mid-stream, replace with transport B that continues from byte N+1; assert receiver sees concatenation == intended N+M bytes. Uses `tokio::io::duplex` pattern from `transport_protocol.rs`. | integration | `cargo test -p vector-app no_byte_loss_under_transport_swap` | ❌ Wave 0 |
| PERSIST-02 | Tab badge transitions `[remote]` → `[reconnecting]` → `[remote]` on the corresponding UserEvents | unit | `cargo test -p vector-app tab_badge_during_reconnect` | ❌ Wave 0 |
| PERSIST-03 | `DevTunnelTransport::connect` sends `OpenPty { shell: None }` (no tmux wrap) — regression test | unit | `cargo test -p vector-tunnels open_pty_sends_no_shell_override` | ✅ (existing transport_protocol.rs:46-58 covers the assertion shape; add a dedicated test) |
| PERSIST-04 | Local rehearsal — same as existing `osc52_tmux` against a LOCAL tmux | integration | `cargo test -p vector-term --test osc52_tmux -- --ignored` | ✅ (`crates/vector-term/tests/osc52_tmux.rs`) |
| PERSIST-04 | Live e2e — connect to real Dev Tunnel agent, user-started tmux 3.4+, run OSC 52 roundtrip + DECSCUSR + mouse + TERM check | integration `#[ignore]` | `VECTOR_E2E_TUNNEL_ID=<id> cargo test -p vector-app --test persist_e2e -- --ignored` | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p vector-mux -p vector-tunnels -p vector-app --lib` (quick — affected crates only)
- **Per wave merge:** `cargo test --workspace --all-targets` (full automated suite; excludes `#[ignore]`d tests)
- **Phase gate:** Full suite green + manual run of `VECTOR_E2E_TUNNEL_ID=<id> cargo test ... -- --ignored` against a real tunnel before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `crates/vector-app/tests/pty_actor_reconnect.rs` — covers PERSIST-01 (state-machine entry/exit, event emission, input lock)
- [ ] `crates/vector-app/tests/reconnect_backoff.rs` — covers PERSIST-02 backoff schedule + cancellation (uses `tokio::time::pause()`)
- [ ] `crates/vector-app/tests/byte_loss_under_swap.rs` — covers PERSIST-02 no-byte-loss (the SC#2 invariant) via fake-Domain that hands out two `tokio::io::duplex`-backed transports in sequence
- [ ] `crates/vector-app/tests/reconnect_overlay.rs` — covers PERSIST-01 `ReconnectPass` text format + per-pane positioning
- [ ] `crates/vector-app/tests/tab_badge_reconnect.rs` — covers PERSIST-01 badge state transitions
- [ ] `crates/vector-tunnels/tests/reconnect_one_shot.rs` — covers PERSIST-02 `ReconnectableDevTunnelDomain::reconnect_one_shot` returns Some(transport)
- [ ] `crates/vector-tunnels/tests/open_pty_no_shell_override.rs` — covers PERSIST-03 regression assertion (agent never receives a `shell: Some(...)` from us)
- [ ] `crates/vector-app/tests/persist_e2e.rs` — new file; `#[ignore]`d live e2e for PERSIST-04
- [ ] CI matrix update: add a new job `persist-e2e` modeled after `tmux-smoke` (`.github/workflows/ci.yml:100-113`), `continue-on-error: true`, runs `cargo test ... -- --ignored` only when `VECTOR_E2E_TUNNEL_ID` repo secret is present. Optional for v1 — manual local runs are acceptable.

**Framework install:** None — `cargo test` is built-in.

## Sources

### Primary (HIGH confidence — existing source files in the repo)

- `crates/vector-mux/src/domain.rs:30-45` — `Domain` trait shape; reconnect() declaration at line 44
- `crates/vector-mux/src/devtunnel_domain.rs:41-43` — stub to fill
- `crates/vector-mux/src/local_domain.rs:107-109` — no-op precedent
- `crates/vector-mux/src/pane.rs:80-152` — `Pane` shape including `transport: Mutex<Option<Box<dyn PtyTransport>>>` at line 87, `take_transport` at line 137, `format_tab_title` at line 205
- `crates/vector-mux/src/mux.rs:393-447` — `create_tab_async` + `create_tab_async_with_transport` (lock-discipline reference, comment at line 396)
- `crates/vector-mux/src/transport.rs:1-35` — `PtyTransport` trait + `TransportKind`
- `crates/vector-tunnels/src/transport.rs:52-246` — `DevTunnelTransport` + `connect()` flow at 199-245, pump task at 132-185 (EOF + drain semantics)
- `crates/vector-tunnels/src/domain.rs:12-22` — `connect_tunnel` helper
- `crates/vector-app/src/pty_actor.rs:1-213` — per-pane actor model (`pane_io_loop` at 114-156, `JoinSet<PaneId>` shape, biased select)
- `crates/vector-app/src/devtunnels_actor.rs:192-254` — picker `handle_connect`; reference for where `ReconnectableDevTunnelDomain` gets constructed
- `crates/vector-app/src/lib.rs:49-162` — `UserEvent` enum (append-only convention)
- `crates/vector-app/src/chrome.rs:13-28` — `ChromePipelines` shape where `ReconnectPass` is added
- `crates/vector-app/src/app.rs:980-1090` — per-pane render loop + chrome pass composition order
- `crates/vector-app/src/toast.rs:1-82` — `ToastBanner` + `ToastStack` pattern (structural template for state)
- `crates/vector-term/tests/osc52_tmux.rs:1-40` — `#[ignore]` smoke-test pattern for PERSIST-04
- `crates/vector-tunnels/tests/transport_protocol.rs:1-80` — `tokio::io::duplex`-based fake-transport test pattern for byte-loss verification
- `.github/workflows/ci.yml:97-113` — `tmux-smoke` CI job pattern for live e2e gating
- `.planning/phases/09-persistence-reconnect-tmux-auto-attach/09-CONTEXT.md` — locked decisions
- `.planning/REQUIREMENTS.md:72-75` — PERSIST-01..04 (revised)
- `.planning/ROADMAP.md:230-246` — Phase 9 description (revised 2026-05-22)
- `.planning/research/PITFALLS.md:190-213` — Pitfall 8 (tmux DCS passthrough)
- `.planning/research/PITFALLS.md:481-493` — Pitfall 22 (no mosh)

### Secondary (MEDIUM confidence — memory + project context)

- Memory `project-phase8-tunnel-agent` (2026-05-20) — agent protocol stays simple; do not extend with file transfer / port forwarding / resume opcodes in this phase
- Memory `project-vector-pivot-to-tunnels` (2026-05-19) — Phase 7→8 pivot context (no codespaces work to consider)
- `CLAUDE.md` Tech-stack pins — Rust 1.88+, workspace dep versions LOCKED through Phase 9

### Tertiary (LOW confidence — none flagged)

No external WebSearch findings. All sources are repo-local or workspace metadata. This is a wiring-phase research: the externals (wgpu, tokio, dev-tunnels SDK) were validated in Phases 3, 4, 8.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — every named crate is already in `Cargo.toml` and exercised by prior phases
- Architecture: HIGH — the seams (Domain, Pane.transport mutex, per-pane actor, ChromePipelines) are all in place; Phase 9 is wiring
- Pitfalls: HIGH for items 1-6 (verified against source files); MEDIUM for item 7 (relay re-fragmentation of OSC 52 — needs the PERSIST-04 smoke test to verify in practice); HIGH for item 8 (CI pattern is established)
- Validation architecture: HIGH — test patterns mirror existing files (`transport_protocol.rs`, `osc52_tmux.rs`, `.github/workflows/ci.yml`)

**Research date:** 2026-05-22
**Valid until:** 2026-06-21 (30 days; this is a stable architecture domain — extend if Dev Tunnels SDK or russh patch changes)
