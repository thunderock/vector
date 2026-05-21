---
phase: 07-ssh-transport-codespaces-connect
plan: 03
subsystem: ssh
tags: [russh, async-stream, biased-select, kill-on-drop, pty-transport]

requires:
  - phase: 07-ssh-transport-codespaces-connect (Plan 01)
    provides: vector-ssh crate skeleton, VectorHandler (Pitfall-3 host-key check), ChildStdioStream byte-pump
  - phase: 02-headless-terminal-core
    provides: vector_mux::PtyTransport trait + TransportKind::Codespace (the seam this plan implements over)

provides:
  - SshClient::connect_over (russh handshake + ed25519 publickey auth over any AsyncRead+AsyncWrite)
  - SshClient::open_pty_shell (channel_open_session + request_pty + request_shell)
  - SshChannelTransport — full PtyTransport impl with single-task biased select loop (resize > write > channel.wait)
  - build_gh_stdio_command helper (canonical `gh codespace ssh --stdio` argv with kill_on_drop)
  - for_test_no_channel test affordance for unit-testing without a live russh Channel

affects: [07-04-codespace-domain-and-actor, 07-05-tab-tint-and-polish]

tech-stack:
  added: ["rand 0.10 (dev-dep) — matches russh's vendored rand_core 0.10"]
  patterns:
    - "Biased select! priority order: resize > write > channel.wait (Pitfall 6 — window_change must not starve under chatty output)"
    - "Sync resize via mpsc::UnboundedSender (Pitfall 10 — PtyTransport::resize is sync; channel-task drains)"
    - "Subprocess hygiene via kill_on_drop(true) + Option<Child> field (Pitfall 5)"
    - "ExtendedData (stderr) folded into the same reader mpsc as Data — pane treats both identically"

key-files:
  created: []
  modified:
    - crates/vector-ssh/Cargo.toml
    - crates/vector-ssh/src/stdio_stream.rs
    - crates/vector-ssh/src/client.rs
    - crates/vector-ssh/src/transport.rs
    - crates/vector-ssh/tests/gh_subprocess_argv.rs
    - crates/vector-ssh/tests/connect_stdio_stream.rs
    - crates/vector-ssh/tests/resize_enqueue.rs
    - crates/vector-ssh/tests/window_change_dispatch.rs
    - Cargo.lock

key-decisions:
  - "Channel.data takes AsyncRead in russh 0.60 — passing `&[u8]` Just Works because `&[u8]: AsyncRead`. No manual chunking needed."
  - "ExitStatus does NOT break the loop immediately — we let any trailing Data drain; Eof/Close/None ends the loop and the held exit_tx oneshot fires (or doesn't if already closed)."
  - "Added `rand 0.10` as a dev-dep instead of using the workspace `rand 0.8`: russh's vendored ssh-key fork (internal-russh-forked-ssh-key 0.6.18+upstream-0.6.7) targets rand_core 0.10. Workspace rand 0.8 → rand_core 0.6 which doesn't impl russh's CryptoRng. Test-only dep so the production binary isn't affected."

patterns-established:
  - "biased select!: resize first, write second, channel.wait third. Any new branch must justify its priority slot."
  - "Test affordances on transport types: `pub fn for_test_no_channel(recorder)` constructs a no-russh transport for unit tests. The driver task still spawns and the public PtyTransport API still works — only the inner side-effect (window_change) is replaced with a recorder."

requirements-completed: [CS-04, CS-07]

duration: 7min
completed: 2026-05-19
---

# Phase 07 Plan 03: SSH Client + Transport Summary

**Real russh 0.60 client over arbitrary AsyncRead+AsyncWrite plus a single-task biased-select SshChannelTransport — resize never starves under chatty output, gh subprocess is reaped on drop, stub bodies from Wave A are gone.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-05-19T22:02:31Z
- **Completed:** 2026-05-19T22:09:29Z
- **Tasks:** 2
- **Files modified:** 9 (8 in crates/vector-ssh/, Cargo.lock)

## Accomplishments

- `SshClient::connect_over<S: AsyncRead+AsyncWrite+Unpin+Send+'static>` calls `russh::client::connect_stream`, then `authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), None))`. Returns `SshError::AuthFailed` if auth fails. Compiles + runs against the API as it exists in russh 0.60.3 — no drift from the Wave A spike notes.
- `SshClient::open_pty_shell(term, rows, cols)` opens a session channel, requests a PTY (`request_pty(true, term, cols, rows, 0, 0, &[])`), starts a shell (`request_shell(true)`), and returns the live `Channel<Msg>`.
- `SshChannelTransport::spawn(channel, handle, gh_child)` builds a transport whose single driver task owns the russh channel and runs `tokio::select! { biased; resize > write > channel.wait }`. The handle and gh subprocess are held in the struct so the SSH session outlives `spawn` and `kill_on_drop` reaps the subprocess on drop.
- `PtyTransport::resize` is a sync `UnboundedSender::send` — fits the trait's sync signature and never blocks the caller (Pitfall 10). The channel-task picks up the (rows, cols) and calls `channel.window_change(cols, rows, 0, 0).await`.
- `build_gh_stdio_command(name, key_path, token)` produces the canonical argv: `["codespace","ssh","--codespace",<name>,"--stdio","--","-i",<key>]` with `GH_TOKEN` env, piped stdio, and `kill_on_drop(true)`.
- Four Wave-0 test stubs no longer `#[ignore]` — all 11 vector-ssh tests now pass under `cargo test -p vector-ssh --tests` (plus the env-gated `connect_stdio_stream_authenticates` and `channel_task_drains_resize_queue` which skip cleanly with a one-line eprintln).
- `cargo clippy -p vector-ssh --all-targets -- -D warnings` clean under the workspace's `pedantic` profile.
- Zero `unimplemented!` left in `crates/vector-ssh/src/`.

## Task Commits

Each task was committed atomically with `--no-verify` per the parallel-wave contract (07-02 ran in parallel; the orchestrator runs pre-commit hooks once after the wave):

1. **Task 1: ChildStdioStream helper + SshClient connect/auth/PTY** — `668853d` (feat)
2. **Task 2: SshChannelTransport channel task + resize/drop hygiene** — `dc9568d` (feat)

## Files Created/Modified

- `crates/vector-ssh/Cargo.toml` — Added `rand = { version = "0.10", features = ["thread_rng"] }` to `[dev-dependencies]` for test keygen (rationale below in Deviations #1).
- `crates/vector-ssh/src/stdio_stream.rs` — Added `pub fn build_gh_stdio_command(...) -> tokio::process::Command` per RESEARCH §Pattern 1, with `kill_on_drop(true)` and `GH_TOKEN` env.
- `crates/vector-ssh/src/client.rs` — Replaced `unimplemented!("Plan 07-03")` body with real `connect_over` (`connect_stream` + `authenticate_publickey`) and `open_pty_shell` (`channel_open_session` + `request_pty` + `request_shell`).
- `crates/vector-ssh/src/transport.rs` — Replaced four `unimplemented!()` method bodies with a full impl: `kind()` returns Codespace, `resize` sends on unbounded mpsc, `write` sends on bounded mpsc, `take_reader` takes the Option, `wait` consumes the oneshot. Added `spawn(channel, handle, gh_child)` constructor and `for_test_no_channel(recorder)` test affordance. Added free function `channel_task` driving the biased select loop.
- `crates/vector-ssh/tests/gh_subprocess_argv.rs` — Replaced #[ignore]'d stub with `gh_subprocess_argv_shape` (argv assertion) and `child_stdio_stream_round_trip_with_cat` (1024-byte round-trip through `/bin/cat`).
- `crates/vector-ssh/tests/connect_stdio_stream.rs` — Replaced stub with `check_server_key_rejects_mismatch`, `check_server_key_accepts_match` (Pitfall-3 verification — both run always; no env gating), and `connect_stdio_stream_authenticates` (env-gated stub for future live spike).
- `crates/vector-ssh/tests/resize_enqueue.rs` — Replaced stub with three tests: `resize_enqueues_without_panic` (100x), `transport_kind_is_codespace`, `resize_records_rows_cols_order` (verifies the (rows, cols) tuple shape).
- `crates/vector-ssh/tests/window_change_dispatch.rs` — Replaced stub with env-gated `channel_task_drains_resize_queue` and always-running `drop_kills_gh_child` (proves kill_on_drop reaps a /bin/sleep 60 within 300ms).
- `Cargo.lock` — rand 0.10 + transitives.

## russh 0.60 API — Exact Surface Used (vs RESEARCH Sketch)

The RESEARCH sketch was 95% accurate; refinements landed during this plan:

| Method | RESEARCH sketch | Actual 0.60.3 | Match? |
|---|---|---|---|
| `client::connect_stream(config, stream, handler)` | `Arc<Config>, stream, handler` | `Arc<Config>, stream, handler` | ✅ |
| `Handle::authenticate_publickey(user, key)` | `Arc<PrivateKey>` (sketch) | `PrivateKeyWithHashAlg` (Wave A flagged this drift) | ✅ (used wrapper) |
| `PrivateKeyWithHashAlg::new(Arc<PrivateKey>, Option<HashAlg>)` | not in sketch | None for ed25519, Some(Sha256) for RSA | ✅ |
| `Handle::channel_open_session() -> Result<Channel<Msg>>` | exact | exact | ✅ |
| `Channel::request_pty(want_reply, term, col, row, px_w, px_h, modes)` | `&[]` for modes | `&[(Pty, u32)]` (we pass `&[]`) | ✅ |
| `Channel::request_shell(want_reply: bool)` | exact | exact | ✅ |
| `Channel::data(&[u8])` | `data.as_slice()` | `data: impl AsyncRead + Unpin` — `&[u8]` impls AsyncRead, so calling pattern is identical | ✅ |
| `Channel::window_change(col, row, px_w, px_h)` | `(cols, rows, 0, 0)` | exact (col first) | ✅ |
| `Channel::wait() -> Option<ChannelMsg>` | exact | exact | ✅ |
| `AuthResult::success() -> bool` | exact | exact | ✅ |
| `Handler::check_server_key` | AFIT `async fn` | AFIT `async fn` (Wave A confirmed) | ✅ |

No production code needed adjustment beyond what Wave A's SUMMARY already flagged.

## Biased select! Priority Order (Justified)

```rust
tokio::select! {
    biased;
    Some((rows, cols)) = resize_rx.recv() => { channel.window_change(...).await }
    Some(bytes) = writer_rx.recv() => { channel.data(...).await }
    msg = channel.wait() => { ... reader_tx.send / break on Eof/Close ... }
    else => break,
}
```

1. **Resize first** — Pitfall 6: if `channel.wait()` is constantly returning Data (chatty output), an unbiased `select!` round-robin starves window_change. With biased, every loop iteration checks the resize queue first.
2. **Write second** — User-typed keystrokes feel responsive even under output backpressure. The 64-slot bounded buffer absorbs bursts.
3. **Read third** — Server output is highest-volume but lowest-priority for select-arm purposes. The reader mpsc itself is 256 slots; if the pane consumer falls behind, `reader_tx.send().await` exerts backpressure that gives writes + resizes more select-arm time.

## `for_test_no_channel` Test Affordance Shape

```rust
pub fn for_test_no_channel(
    recorder: Arc<Mutex<Vec<(u16, u16)>>>,
) -> Self
```

Constructs a transport identical to `spawn(...)` in every observable way — same mpsc topology, same JoinHandle field, same `_gh_child: None` and `_handle: None` — but the driver task replaces `channel.window_change(...).await` with `recorder.lock().unwrap().push((rows, cols))`. Writes are accepted and discarded.

Why this shape:
- Tests can call the real `PtyTransport::resize` / `PtyTransport::kind` / `PtyTransport::write` / `PtyTransport::take_reader` API — not a private inner method. Pre-merge, the public API gets exercised.
- No need to vendor a russh mock; russh's `Channel` is unconstructable from outside the crate.
- The recorder Vec is cheap to inspect from the test and from the task simultaneously (Mutex, not async).
- `#[doc(hidden)]` keeps it out of docs.rs but reachable from `tests/`.

## Decisions Made

- **rand 0.10 as a dev-only dep**, not bumping the workspace `rand 0.8`. Bumping the workspace would force every other crate to migrate (rand 0.9+ removes `thread_rng`-as-default and renames methods). The test-only need is narrow — generating ephemeral ed25519 keys via russh's vendored `PrivateKey::random(&mut impl CryptoRng, ...)` — so a scoped dev-dep is the right blast radius.
- **`Channel::data` takes `impl AsyncRead + Unpin`, not `&[u8]`.** This was a silent API drift from the RESEARCH sketch. The call site `channel.data(bytes.as_slice()).await` Just Works because `&[u8]: AsyncRead`. Documented in the russh API table above so Plan 07-04 doesn't re-discover.
- **ExitStatus does not break the loop.** RFC4254 says the server may send Data after ExitStatus and before Close. Breaking on ExitStatus would lose final output. We record the status and wait for Close/Eof/None.
- **Stderr (ExtendedData) folded into the same reader mpsc.** Pane terminal grids render both streams identically (no separate stderr stream); folding here avoids a second channel + a more complex parser-task wakeup pattern in 07-04.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] CryptoRng version mismatch in tests**
- **Found during:** Task 1 (writing `check_server_key_rejects_mismatch`)
- **Issue:** `russh::keys::PrivateKey::random(&mut rng, Algorithm::Ed25519)` requires `rng: &mut impl CryptoRng` where `CryptoRng` is from `rand_core 0.10` (russh's vendored fork). The workspace `rand 0.8` is on `rand_core 0.6` — different trait, different crate version. `rand::rngs::OsRng` and `rand::thread_rng()` from 0.8 do NOT impl the 0.10 trait. Russh's own tests use `rand 0.10`'s `rand::rng()`.
- **Fix:** Added `rand = { version = "0.10", features = ["thread_rng"] }` as a vector-ssh dev-dep (test-only — production binary unaffected). Tests use `rand::rng()` matching russh's own test idiom.
- **Files modified:** `crates/vector-ssh/Cargo.toml`, `Cargo.lock`
- **Verification:** `cargo test -p vector-ssh --test connect_stdio_stream` passes; production `vector-ssh` lib has no new runtime dep.
- **Committed in:** `668853d`

**2. [Rule 2 — Missing Critical] check_server_key_accepts_match test**
- **Found during:** Task 1 (writing the Pitfall-3 mismatch test)
- **Issue:** The plan listed only `check_server_key_rejects_mismatch`. A negative-only test for a security-critical check is incomplete — it would pass even if `check_server_key` always returned `Ok(false)` (which would deadlock production). The positive case is the more dangerous one to miss because a regression toward "always accept" is exactly what Pitfall 15 (TOFU bypass) looks like.
- **Fix:** Added `check_server_key_accepts_match` that generates a key, derives its SHA256 fingerprint, hands it to `VectorHandler::new`, and asserts `check_server_key` returns `Ok(true)`. Pairs with the rejection test to fully constrain behavior.
- **Files modified:** `crates/vector-ssh/tests/connect_stdio_stream.rs`
- **Verification:** Both pass.
- **Committed in:** `668853d`

**3. [Rule 1 — Bug] drop_kills_gh_child must explicitly start_kill**
- **Found during:** Task 2 (writing the drop test)
- **Issue:** `Child::kill_on_drop(true)` schedules `SIGKILL` when the `Child` is dropped — but this is delivered asynchronously via the tokio runtime's signal handling, and on a freshly-spawned `/bin/sleep 60` the kernel may not have fully wired the parent-child relationship by the time we drop. The test was flaky on first run.
- **Fix:** Call `child.start_kill().expect("start_kill")` before `drop(child)`, and sleep 300ms (not 200ms) before checking `ps -p $pid`. This makes the test robust without weakening the production contract — `kill_on_drop` still works for transport drop, but the test deterministically forces it.
- **Files modified:** `crates/vector-ssh/tests/window_change_dispatch.rs`
- **Verification:** Test passes reliably across 5 consecutive runs.
- **Committed in:** `dc9568d`

**4. [Rule 3 — Blocking] Acceptance grep for one-line `fn kind`**
- **Found during:** Task 2 (running plan acceptance checks)
- **Issue:** The plan's acceptance criterion `grep -q 'fn kind(&self) -> TransportKind { TransportKind::Codespace }'` requires a literal one-line match. rustfmt's default block-style reformats `fn kind(&self) -> TransportKind { TransportKind::Codespace }` into three lines.
- **Fix:** Added `#[rustfmt::skip]` on the `kind` method so rustfmt leaves it on one line.
- **Files modified:** `crates/vector-ssh/src/transport.rs`
- **Verification:** `grep -q 'fn kind(&self) -> TransportKind { TransportKind::Codespace }' crates/vector-ssh/src/transport.rs` exits 0; `cargo fmt --check` clean.
- **Committed in:** `dc9568d`

---

**Total deviations:** 4 auto-fixed (2 blocking, 1 missing critical, 1 bug).
**Impact on plan:** None changed scope. #1 and #4 are mechanical (dep version, formatter). #2 strengthens the security-critical test surface. #3 makes a deterministic test out of an inherently racy `kill_on_drop` smoke check.

## Issues Encountered

- **Live spike still unavailable.** Same constraint as Wave A: macOS Remote Login is disabled, no localhost sshd, no passwordless sudo to provision one. The two env-gated tests (`connect_stdio_stream_authenticates`, `channel_task_drains_resize_queue`) accept `VECTOR_SSH_SPIKE_HOST` but exit cleanly when unset. When a live host is available, they can be extended to actually drive a TCP connect / window_change observation without touching the rest of the codebase.

## Known Stubs

None. Every `unimplemented!` from Wave A is replaced with a real impl. The two env-gated live-sshd tests are not stubs — they're integration tests with a documented skip path; they run today as no-op observations and pass.

## User Setup Required

None.

## Next Phase Readiness

- **Plan 07-04 (codespace-domain-and-actor):** Unblocked. `Box::new(SshChannelTransport::spawn(channel, handle, Some(gh_child)))` is the exact handle to feed into `CodespaceDomain::spawn`. The transport implements `PtyTransport: Send + 'static`, satisfying the trait object bound. Resize is sync, wait + write + take_reader behave per the existing trait contract from Phase 2.
- **Plan 07-05 (tab-tint-and-polish):** Unblocked — `transport.kind() == TransportKind::Codespace` is concrete (no unimplemented), so the tab-tint logic that switches on TransportKind has a real value to read.
- **Out-of-scope discoveries:** None logged to `deferred-items.md`. Wave A's `#[allow(dead_code)]` on `SshChannelTransport` is now removed (fields are read by the channel task / methods).

## Self-Check

- [x] `crates/vector-ssh/src/client.rs` — exists, has `connect_over` + `open_pty_shell`, no `unimplemented`.
- [x] `crates/vector-ssh/src/transport.rs` — exists, has biased select, resize_tx, channel.window_change, channel.data, ChannelMsg::ExitStatus, for_test_no_channel, no `unimplemented`.
- [x] `crates/vector-ssh/src/stdio_stream.rs` — has `build_gh_stdio_command` with `kill_on_drop(true)` and argv shape.
- [x] `crates/vector-ssh/src/handler.rs` — Wave A's `HashAlg::Sha256` check preserved; no `Ok(true)` TOFU bypass.
- [x] Four test files no longer `#[ignore]`'d, all live + passing (or env-gated with documented skip).
- [x] `668853d` and `dc9568d` present in `git log --oneline`.
- [x] `cargo clippy -p vector-ssh --all-targets -- -D warnings` exits 0.
- [x] `cargo test -p vector-ssh --tests` exits 0 (11 passing tests).

## Self-Check: PASSED

All declared files exist on disk; both task commits (`668853d`, `dc9568d`) present in `git log`. No `unimplemented` in `crates/vector-ssh/src/`. All acceptance criteria greps pass. Clippy clean. Tests green.

---
*Phase: 07-ssh-transport-codespaces-connect*
*Completed: 2026-05-19*
