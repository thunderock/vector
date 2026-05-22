# Phase 9 — Deferred Items

Logged during Plan 09-06 execution. These are pre-existing issues unrelated to
this plan's scope; they should be addressed in a separate cleanup pass.

## clippy::redundant_pattern_matching in `vector-tunnels`

- **File:** `crates/vector-tunnels/tests/open_pty_no_shell_override.rs:53`
- **Issue:** `assert!(matches!(shell, None))` should be `assert!(shell.is_none())`.
- **Discovered:** During Plan 09-06 Task 1 (`cargo clippy -p vector-tunnels --tests -- -D warnings`).
- **Why deferred:** Pre-existing issue in a test file untouched by 09-06; the
  Phase-9 plan modifies `live_devtunnel_smoke.rs` only.
- **Suggested follow-up:** A workspace clippy-clean PR (or a quick fix in the
  Phase-9 polish/release plan).
