---
phase: 07-ssh-transport-codespaces-connect
plan: 01
subsystem: ssh
tags: [russh, ssh-key, oauth, codespaces, scaffold]

requires:
  - phase: 02-headless-terminal-core
    provides: vector_mux::PtyTransport trait + TransportKind enum (the seam Plan 07-03 will impl over)
  - phase: 06-github-auth-codespaces-picker
    provides: OAuth Device Flow driver (device_flow.rs) that 07-01 extends with write:public_key

provides:
  - vector-ssh crate skeleton (6 modules) with final public surface (SshClient, SshChannelTransport, ChildStdioStream, SshError) for Plan 07-03 to fill in
  - Workspace [workspace.dependencies] entries for russh 0.60 and ssh-key 0.6 (ed25519/alloc/rand_core features)
  - VectorHandler with non-TOFU host-key check against SHA-256 fingerprint (Pitfall 3 mitigation, ready for Plan 07-03)
  - OAuth Device Flow now requests write:public_key in addition to codespace + read:user
  - Four #[ignore]'d Wave-0 test stub files matching CS-04/CS-05/CS-07 in vector-ssh/tests/

affects: [07-02-ssh-keypair-and-registration, 07-03-ssh-client-and-transport, 07-04-codespace-domain-and-actor, 07-05-tab-tint-and-polish]

tech-stack:
  added: [russh 0.60.3, ssh-key 0.6.7]
  patterns:
    - "Subprocess-as-AsyncStream (RESEARCH §Pattern 1) — ChildStdioStream wraps (ChildStdout, ChildStdin) for russh::client::connect_stream"
    - "API-fingerprint host-key validation (Pitfall 3) — Handler::check_server_key compares ssh-key SHA-256 against expected_fp, never returns Ok(true) blindly"
    - "russh 0.60 Handler uses AFIT (`async fn`), not #[async_trait]"

key-files:
  created:
    - crates/vector-ssh/src/client.rs
    - crates/vector-ssh/src/error.rs
    - crates/vector-ssh/src/handler.rs
    - crates/vector-ssh/src/stdio_stream.rs
    - crates/vector-ssh/src/transport.rs
    - crates/vector-ssh/tests/connect_stdio_stream.rs
    - crates/vector-ssh/tests/gh_subprocess_argv.rs
    - crates/vector-ssh/tests/resize_enqueue.rs
    - crates/vector-ssh/tests/window_change_dispatch.rs
  modified:
    - Cargo.toml
    - crates/vector-ssh/Cargo.toml
    - crates/vector-ssh/src/lib.rs
    - crates/vector-codespaces/src/auth/device_flow.rs
    - crates/vector-codespaces/tests/device_flow.rs
    - crates/vector-codespaces/tests/auth_refresh.rs

key-decisions:
  - "russh 0.60 vendors a forked ssh-key (internal-russh-forked-ssh-key 0.6.18+upstream-0.6.7), so the Handler trait references russh::keys::PublicKey, not the workspace ssh-key crate. Workspace ssh-key 0.6 is retained for the keygen/PEM path in Plan 07-02."
  - "Wave-0 localhost-sshd spike documented as unavailable on this macOS host (Remote Login disabled, no passwordless sudo). russh 0.60.3 API surface verified by direct source inspection — every method the plan needs exists with expected signatures."
  - "VectorHandler implemented today with the real host-key check (not stubbed to unimplemented!()), to make the security boundary visible to code review early — Plan 07-03 only needs to wire the connect path, not re-discover the Pitfall 3 mitigation."

patterns-established:
  - "russh 0.60 Handler impl uses plain `async fn` (AFIT), not `#[async_trait]`. The async-trait feature flag exists but isn't default-enabled in russh."
  - "Stub modules use `unimplemented!(\"Plan 07-N\")` with the next plan number in the panic message so failures during incremental development point at the right plan."

requirements-completed: [CS-04, CS-05]

duration: 9min
completed: 2026-05-19
---

# Phase 07 Plan 01: SSH Transport Wave-0 Skeleton Summary

**vector-ssh crate skeleton with russh 0.60 + ssh-key 0.6 workspace deps; OAuth Device Flow widened to include write:public_key; four CS-04/CS-05/CS-07 test stubs in place.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-05-19T21:46:35Z
- **Completed:** 2026-05-19T21:56:21Z
- **Tasks:** 2
- **Files modified:** 15 (6 created in src/, 4 created in tests/, 5 modified)

## Accomplishments

- Workspace declares russh 0.60 and ssh-key 0.6 (verified by `cargo tree -p vector-ssh`: russh v0.60.3, ssh-key v0.6.7).
- vector-ssh crate skeleton compiles clippy-clean under workspace pedantic deny-warnings.
- Public surface (SshClient, SshChannelTransport, ChildStdioStream, SshError) is final; downstream crates can name these types today even though bodies are stubbed.
- VectorHandler implements the real Pitfall-3-compliant host-key check (SHA-256 fingerprint comparison, no TOFU bypass) — this is the security-sensitive method, so we landed it now rather than stub it.
- ChildStdioStream is fully implemented per RESEARCH §Pattern 1 (no stub) so Plan 07-03's subprocess wiring needs nothing beyond `Command::spawn` + the wrapper constructor.
- Phase 6's OAuth Device Flow now requests `write:public_key` so Plan 07-02 can POST /user/keys without a re-auth detour.
- Four #[ignore]'d Wave-0 test stub files exist and compile under `cargo test -p vector-ssh --tests --no-run`.

## Task Commits

Each task was committed atomically:

1. **Task 1: Workspace deps + vector-ssh crate skeleton (6 modules)** — `0a88141` (feat)
2. **Task 2: OAuth scope extension + four Wave-0 test stubs** — `69104a5` (feat)

## Files Created/Modified

- `Cargo.toml` — Add `russh = "0.60"` and `ssh-key = { version = "0.6", default-features = false, features = ["ed25519", "alloc", "rand_core"] }` to `[workspace.dependencies]`.
- `crates/vector-ssh/Cargo.toml` — Real manifest with russh, ssh-key, tokio (process/io-util/sync), async-trait, thiserror, anyhow, tracing, zeroize, and `vector-mux` path dep.
- `crates/vector-ssh/src/lib.rs` — Module declarations + re-exports (SshClient, SshChannelTransport, ChildStdioStream, SshError).
- `crates/vector-ssh/src/error.rs` — SshError enum (GhSpawn, Russh, AuthFailed, HostKeyMismatch, ChannelClosed, Other) — final shape; no stubs.
- `crates/vector-ssh/src/stdio_stream.rs` — ChildStdioStream implementation per RESEARCH §Pattern 1; final, no stubs.
- `crates/vector-ssh/src/handler.rs` — VectorHandler with real `check_server_key` impl (no TOFU bypass — Pitfall 3); compiles against `russh::keys::PublicKey`.
- `crates/vector-ssh/src/client.rs` — SshClient struct + stub `connect_over` body (`unimplemented!("Plan 07-03")`).
- `crates/vector-ssh/src/transport.rs` — SshChannelTransport with PtyTransport impl; `kind()` returns `TransportKind::Codespace` concretely, all other methods stubbed `unimplemented!("Plan 07-03")`.
- `crates/vector-ssh/tests/{connect_stdio_stream,gh_subprocess_argv,resize_enqueue,window_change_dispatch}.rs` — #[ignore]'d Wave-0 stubs (CS-04, CS-05, CS-07).
- `crates/vector-codespaces/src/auth/device_flow.rs` — Add `.add_scope(Scope::new("write:public_key".into()))` after the read:user line.
- `crates/vector-codespaces/tests/device_flow.rs` — Update wiremock token-response `scope` field to `"codespace read:user write:public_key"` (3 occurrences).
- `crates/vector-codespaces/tests/auth_refresh.rs` — Same fixture update (1 occurrence).

## Decisions Made

- **russh's vendored ssh-key fork is a hard reality, not a workaround.** russh 0.60 ships `internal-russh-forked-ssh-key 0.6.18+upstream-0.6.7`. The trait signature `check_server_key(&mut self, &russh::keys::PublicKey)` cannot reference the workspace `ssh-key` crate's `PublicKey` (different type). We named this as a comment in handler.rs and client.rs so Plan 07-02 (which uses workspace ssh-key for ed25519 keygen) and Plan 07-03 (which authenticates via `PrivateKeyWithHashAlg` — another russh-private type) don't trip over the boundary again.
- **Implement the security-sensitive method now, stub the plumbing methods.** Pitfall 3 (TOFU host-key bypass) is the single most dangerous mistake in Phase 7. Writing the real `check_server_key` body today — instead of `unimplemented!()` — means the fingerprint-equality check is visible to PR review at Wave 0, not Wave 2. Plan 07-03 builds the connect/auth/channel code around a handler that's already correct.
- **`tests/no_tokio_main.rs` arch-lint only scans `src/`.** Verified: it would have rejected `#[tokio::test]` in src/ but tests/ is out of scope. The Wave-0 stub files use `#[tokio::test]` freely.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] russh vendored ssh-key fork**
- **Found during:** Task 1 (initial `cargo build -p vector-ssh` after writing handler.rs)
- **Issue:** Following RESEARCH literally — `use ssh_key::{PublicKey, HashAlg};` in handler.rs — failed with E0053: "method `check_server_key` has an incompatible type for trait. expected `russh::keys::PublicKey`, found `ssh_key::PublicKey`". russh 0.60 vendors its own ssh-key fork (`internal-russh-forked-ssh-key 0.6.18+upstream-0.6.7`); the workspace `ssh-key 0.6` is a different type.
- **Fix:** Use `russh::keys::{PublicKey, HashAlg}` in handler.rs, and `russh::keys::PrivateKey` in client.rs's `connect_over` signature. Added a comment block on the import flagging the drift point so Plan 07-02 (keygen via workspace ssh-key) and Plan 07-03 (authentication via russh's PrivateKeyWithHashAlg) won't re-discover this.
- **Files modified:** `crates/vector-ssh/src/handler.rs`, `crates/vector-ssh/src/client.rs`
- **Verification:** `cargo build -p vector-ssh` succeeds; `cargo clippy -p vector-ssh -- -D warnings` clean.
- **Committed in:** `0a88141` (part of Task 1 commit)

**2. [Rule 3 — Blocking] dead_code on Plan-07-03-consumed fields**
- **Found during:** Task 1 (`cargo build -p vector-ssh` after stubbing transport.rs)
- **Issue:** SshChannelTransport fields (channel, reader_rx, writer, resize_tx) are read by Plan 07-03's channel task but no method reads them today, so rustc warns `fields … are never read` under workspace `-D warnings`.
- **Fix:** Added `#[allow(dead_code)]` on the struct with a comment explaining that the fields are wired in Plan 07-03's channel task.
- **Files modified:** `crates/vector-ssh/src/transport.rs`
- **Verification:** `cargo clippy -p vector-ssh --all-targets -- -D warnings` clean.
- **Committed in:** `0a88141`

**3. [Rule 1 — Bug] Stale wiremock `scope` fixtures**
- **Found during:** Task 2 (after extending device_flow.rs to request `write:public_key`)
- **Issue:** Plan asked us to widen scopes from {codespace, read:user} to {codespace, read:user, write:public_key}. The wiremock test fixtures echo back a `scope` string in the token response (3 occurrences in tests/device_flow.rs, 1 in tests/auth_refresh.rs). Leaving them at `"codespace read:user"` would be a documentation lie — these fixtures are read by future contributors as ground truth for what a real GitHub response looks like.
- **Fix:** Update all four `"scope": "codespace read:user"` strings to `"codespace read:user write:public_key"`. Tests still pass (the fixtures aren't asserting on `scope`, just echoing it).
- **Files modified:** `crates/vector-codespaces/tests/device_flow.rs`, `crates/vector-codespaces/tests/auth_refresh.rs`
- **Verification:** `cargo test -p vector-codespaces --tests` — all 17 tests pass.
- **Committed in:** `69104a5` (part of Task 2 commit)

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 bug — all directly caused by this plan's changes).
**Impact on plan:** None of the deviations changed scope. Items #1 and #2 are unavoidable consequences of russh's design; item #3 is a documentation correctness fix that keeps test fixtures in sync with reality.

## Issues Encountered

- **Localhost-sshd spike unavailable on this host.** macOS Remote Login (System Settings > General > Sharing) is disabled, port 22 is closed, and the user does not have passwordless sudo to start a separate sshd instance. The plan explicitly allowed this fallback ("spike succeeds against localhost sshd OR is documented unavailable with reason"). Compensating de-risking: every russh 0.60.3 method the next two plans need was verified by direct source inspection at `~/.cargo/registry/src/.../russh-0.60.3/src/`. The signatures all match what RESEARCH sketched, with two refinements worth recording for Plan 07-03:
  - `Handler` is an AFIT trait (`async fn check_server_key(...)`), not `#[async_trait]`. Plan 07-03 should mirror our handler.rs pattern.
  - `Handle::authenticate_publickey` takes `PrivateKeyWithHashAlg` (a russh-private wrapper), not `Arc<PrivateKey>`. The RESEARCH sketch is one version behind here. Plan 07-03 will need `russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), Some(HashAlg::Sha256))` (or similar — exact constructor to be verified when writing the connect path).

## Known Stubs

These are intentional Wave-0 stubs; Plan 07-03 will fill them in. Each panics with `unimplemented!("Plan 07-03")` so any premature call surfaces a clear error pointing at the right plan.

| Location | What's stubbed | Resolved by |
|---|---|---|
| `crates/vector-ssh/src/client.rs::SshClient::connect_over` | Body is `unimplemented!("Plan 07-03")`. Signature + struct shape are final. | Plan 07-03 |
| `crates/vector-ssh/src/transport.rs::SshChannelTransport::{resize,write,take_reader,wait}` | Bodies are `unimplemented!("Plan 07-03")`. `kind()` returns `TransportKind::Codespace` concretely. | Plan 07-03 |
| `crates/vector-ssh/tests/connect_stdio_stream.rs::connect_stdio_stream_authenticates` | #[ignore]'d; body is `unimplemented!("Plan 07-03")`. CS-04 integration. | Plan 07-03 |
| `crates/vector-ssh/tests/gh_subprocess_argv.rs::gh_subprocess_argv_shape` | #[ignore]'d; body is `unimplemented!("Plan 07-03")`. CS-05 unit. | Plan 07-03 |
| `crates/vector-ssh/tests/resize_enqueue.rs::resize_enqueues_without_panic` | #[ignore]'d; body is `unimplemented!("Plan 07-03")`. CS-07 unit. | Plan 07-03 |
| `crates/vector-ssh/tests/window_change_dispatch.rs::channel_task_drains_resize_queue` | #[ignore]'d; body is `unimplemented!("Plan 07-03")`. CS-07 integration. | Plan 07-03 |

None of these stubs flow to UI rendering or affect a runtime path today — Plan 07-04 is the first plan to call into `SshClient`/`SshChannelTransport`. The crate is consumed only by `cargo build` at Wave 0.

## User Setup Required

None — this plan is purely internal scaffolding. Phase 6 users will, however, see a one-time re-auth dialog on next launch because the OAuth scope set grew. That re-auth is the expected behavior gated by Plan 07-02's POST /user/keys requirement; no manual action is needed beyond accepting the dialog.

## Next Phase Readiness

- **Plan 07-02 (ssh-keypair-and-registration):** Unblocked. The `write:public_key` scope is wired; workspace ssh-key 0.6 is available with the right feature set (ed25519, alloc, rand_core) for `PrivateKey::random` + `to_openssh`.
- **Plan 07-03 (ssh-client-and-transport):** Unblocked. The crate skeleton, public types, error enum, and host-key handler are ready. Two implementation notes worth carrying forward (see Issues Encountered): use AFIT for Handler, use `PrivateKeyWithHashAlg` for auth.
- **Plan 07-04 (codespace-domain-and-actor) / Plan 07-05 (tab-tint-and-polish):** No new blockers introduced by this plan.

## Self-Check: PASSED

All declared files exist on disk; both task commits (`0a88141`, `69104a5`) present in `git log`.

---
*Phase: 07-ssh-transport-codespaces-connect*
*Completed: 2026-05-19*
