---
phase: 06-github-auth-codespaces-picker
plan: 01
subsystem: auth
tags: [oauth2, octocrab, reqwest, chrono, tokio-util, keyring, github, codespaces, pitfall-14, arch-lint, scaffold]

requires:
  - phase: 05-polish-local-daily-driver
    provides: "vector-secrets::Secrets::for_vector() Keychain API + zeroize re-export + GITHUB_OAUTH_ACCOUNT; vector-config schema with Kind::Codespace + codespace_name + tint fields"
provides:
  - "Workspace pins for Phase-6 stack: oauth2 5.0, octocrab 0.50, reqwest 0.12 (rustls-tls), chrono 0.4, http 1, tokio-util 0.7, urlencoding 2, serde_json 1, zeroize 1 + wiremock 0.6 / tempfile 3 dev-deps"
  - "vector-codespaces crate surface: pub mod auth/{mod, error, device_flow, token_store} + pub mod client + pub mod model; GitHubAuth, TokenStore, DeviceCodeDisplay, CodespacesClient, ClientError, AuthError, Codespace, CodespaceState (with #[serde(other)] Unrecognized), RepositoryRef, GitStatus all stubbed"
  - "vector-secrets::Secrets::GITHUB_REFRESH_ACCOUNT const (per D-90)"
  - "vector-config::writer module with append_codespace_profile + derive_profile_name stubs + WriterError; toml_edit added as runtime dep"
  - "14 Wave-1/2 test stubs gated with #[ignore = \"Wave-0 stub — Plan 06-NN fills in\"] across 4 vector-codespaces test files + 5 in vector-config/tests/profile_writer.rs"
  - "Pitfall-14 arch-lint test (vector-arch-tests/tests/no_token_in_debug_or_log.rs): 2 passing greps blocking #[derive(...Debug...)] near token-bearing fields AND tracing! macros referencing token-named idents in vector-codespaces/src"
affects: [06-02-device-flow, 06-03-codespaces-rest, 06-04-profile-writer, 06-05-auth-modal, 06-06-codespaces-modal]

tech-stack:
  added:
    - "oauth2 5.0 (workspace) — RFC 8628 device-flow driver"
    - "octocrab 0.50 (workspace) — GitHub REST client; raw _get/_post for Codespaces routes (no typed coverage in 0.50)"
    - "reqwest 0.12 (workspace, rustls-tls) — HTTP transport used by oauth2 5.0 (which requires reqwest 0.12, NOT 0.13)"
    - "chrono 0.4 (workspace, clock+serde, default-features=false) — last_used_at parse"
    - "http 1 (workspace) — header types"
    - "tokio-util 0.7 (workspace, default features) — CancellationToken in default; tokio-util has no 'sync' feature (plan said 'sync', not real)"
    - "urlencoding 2 (workspace) — percent-encode codespace names in REST paths"
    - "zeroize 1 (workspace, derive) — explicit pin even though vector-secrets transitives it"
    - "serde_json 1 (workspace) — Codespace response parse"
    - "wiremock 0.6 (workspace dev-dep) — HTTP mocking for Plan 06-02/03"
    - "tempfile 3 (workspace dev-dep) — moved from per-crate to workspace for shared pin"
  patterns:
    - "Pitfall-14 manual Debug discipline: every token-bearing struct (GitHubAuth, TokenStore) hand-writes impl Debug; #[derive(Debug)] is forbidden anywhere near *_token / *_secret / device_code / user_code field names — enforced by arch-lint"
    - "Forward-compatible enum deser: CodespaceState uses #[serde(other)] Unrecognized so new GitHub states never crash deserialize (Pitfall 4)"
    - "Struct field flatten capture: Codespace has #[serde(flatten)] _rest: serde_json::Map to survive GitHub adding fields"
    - "Wave-0 stub seeding: every Wave-1/2 test file lands on disk in Wave-0 with #[ignore] gates so later plans flip them green without merging new test files"
    - "Module tree pre-locks crate surface so parallel Wave-1 plans (auth, REST, writer) compile against the same shape"

key-files:
  created:
    - "crates/vector-codespaces/src/lib.rs (rewritten from 9-line stub)"
    - "crates/vector-codespaces/src/auth/mod.rs"
    - "crates/vector-codespaces/src/auth/error.rs"
    - "crates/vector-codespaces/src/auth/device_flow.rs"
    - "crates/vector-codespaces/src/auth/token_store.rs"
    - "crates/vector-codespaces/src/client/mod.rs"
    - "crates/vector-codespaces/src/model.rs"
    - "crates/vector-codespaces/tests/device_flow.rs"
    - "crates/vector-codespaces/tests/codespaces_rest.rs"
    - "crates/vector-codespaces/tests/auth_refresh.rs"
    - "crates/vector-codespaces/tests/keychain_roundtrip.rs"
    - "crates/vector-codespaces/tests/fixtures/list_codespaces.json"
    - "crates/vector-config/src/writer.rs"
    - "crates/vector-config/tests/profile_writer.rs"
    - "crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs"
  modified:
    - "Cargo.toml (workspace.dependencies — pinned Phase-6 stack)"
    - "Cargo.lock"
    - "crates/vector-codespaces/Cargo.toml (rewrote [dependencies] + [dev-dependencies])"
    - "crates/vector-secrets/src/lib.rs (added GITHUB_REFRESH_ACCOUNT const)"
    - "crates/vector-config/src/lib.rs (pub mod writer + re-exports)"
    - "crates/vector-config/Cargo.toml (added toml_edit dep; tempfile → workspace)"
    - "crates/vector-arch-tests/Cargo.toml (added regex.workspace dev-dep)"

key-decisions:
  - "reqwest pinned to 0.12 (not 0.13): oauth2 5.0 requires reqwest ^0.12 — using 0.13 caused 'failed to select a version' resolve error. RESEARCH.md said 0.13 but the real dep constraint forces 0.12. reqwest 0.12 has the documented rustls-tls feature; 0.13.3 renamed it to 'rustls'."
  - "tokio-util pinned with default features (no 'sync' feature exists in tokio-util 0.7.x — plan was wrong about this). CancellationToken is unconditionally exported in tokio-util's lib root."
  - "cargo-platform pinned to 0.3.2 in Cargo.lock — 0.3.3 requires rustc 1.91 but workspace targets rust-version 1.88."
  - "Pitfall-14 grep regex looks for *_token, *_secret, device_code, user_code field names within 30 lines after a #[derive(Debug)] attribute — captures real derive proximity without flagging doc comments (verified clean against current crate)."

patterns-established:
  - "Module-tree-before-business-logic: Wave-0 scaffolds module structure with unimplemented!()-bodied methods + manual Debug impls before Wave-1 plans fill in"
  - "Test-stubs-before-tests: failing Plan-06-NN tests pre-land in Wave-0 as #[ignore] so Wave-1 plans flip them green rather than introducing test files"

requirements-completed: [AUTH-01, AUTH-02, AUTH-03, CS-01, CS-02, CS-03]

duration: 8min
completed: 2026-05-14
---

# Phase 6 Plan 01: Wave 0 — vector-codespaces scaffold + workspace deps + Wave-0 test stubs + arch-lint Summary

**Locked the Phase-6 crate surface: workspace deps pinned for oauth2/octocrab/reqwest/chrono/tokio-util/urlencoding/zeroize/wiremock, full vector-codespaces module tree with Pitfall-14 manual-Debug discipline, 14 Wave-1/2 test stubs gated for later plans, and a 2-test Pitfall-14 arch-lint blocking `#[derive(Debug)]` and `tracing!` calls near token-bearing identifiers.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-05-14T18:57:28Z
- **Completed:** 2026-05-14T19:05:34Z
- **Tasks:** 3
- **Files modified:** 15 (8 created, 7 modified)

## Accomplishments

- Workspace `[workspace.dependencies]` table extended with 12 new Phase-6 pins (oauth2 5.0, octocrab 0.50, reqwest 0.12 [not 0.13], chrono 0.4, http 1, tokio-util 0.7, urlencoding 2, zeroize 1, serde_json 1 + wiremock 0.6 / tempfile 3 dev-pool)
- `vector-codespaces` went from a 9-line placeholder lib.rs to a 7-file module tree (auth/{mod, error, device_flow, token_store}, client/mod, model, lib) with manual `impl Debug` on every token-bearing struct
- `vector-secrets::Secrets::GITHUB_REFRESH_ACCOUNT` constant added per D-90; the second account name the Phase-6 OAuth code will write
- `vector-config::writer::{append_codespace_profile, derive_profile_name}` scaffolded; `toml_edit` lifted from a planned future addition to a current dep
- 14 Wave-1/2 test stubs landed across 4 test files + 1 fixture; later plans flip `#[ignore]` to make them real
- Pitfall-14 arch-lint shipped: 2 tests, both passing on the freshly-scaffolded crate, guarding against `#[derive(Debug)]` proximate to token fields and `tracing::*!` referencing token-named identifiers
- Full workspace `cargo build --workspace` exits 0; `cargo test -p vector-codespaces --tests` reports 0 passed / 0 failed / 14 ignored; arch-lint reports 2 passed

## Task Commits

Each task was committed atomically:

1. **Task 06-01-01: Workspace deps + vector-codespaces Cargo.toml + GITHUB_REFRESH_ACCOUNT** — `89a0539` (chore)
2. **Task 06-01-02: Scaffold vector-codespaces module tree with manual-Debug stubs** — `cd110ed` (feat)
3. **Task 06-01-03: Wave-0 test stubs + arch-lint + writer scaffold + fixture** — `f4e8917` (test)

**Plan metadata commit:** (to follow — SUMMARY/STATE/ROADMAP)

## Files Created/Modified

### Created
- `crates/vector-codespaces/src/auth/mod.rs` — GitHubAuth struct + manual Debug + re-exports
- `crates/vector-codespaces/src/auth/error.rs` — AuthError thiserror enum (OAuth/Http/Secrets/Url/Cancelled/Expired/NoRefreshToken)
- `crates/vector-codespaces/src/auth/device_flow.rs` — DeviceCodeDisplay struct + manual Debug (omits user_code from Debug to be safe even though RFC 8628 §3.1 says it's public)
- `crates/vector-codespaces/src/auth/token_store.rs` — TokenStore over Secrets, manual Debug, unimplemented!() bodies
- `crates/vector-codespaces/src/client/mod.rs` — CodespacesClient over Arc<Octocrab> + ClientError + manual Debug + unimplemented!() bodies
- `crates/vector-codespaces/src/model.rs` — Codespace + CodespaceState (#[serde(other)] Unrecognized) + RepositoryRef + GitStatus, all serde::Deserialize
- `crates/vector-codespaces/tests/device_flow.rs` — 4 AUTH-01 stubs
- `crates/vector-codespaces/tests/codespaces_rest.rs` — 7 CS-01/CS-02 stubs
- `crates/vector-codespaces/tests/auth_refresh.rs` — 2 AUTH-03 stubs
- `crates/vector-codespaces/tests/keychain_roundtrip.rs` — 1 manual-UAT stub
- `crates/vector-codespaces/tests/fixtures/list_codespaces.json` — 1-row CS-01 fixture
- `crates/vector-config/src/writer.rs` — WriterError + append_codespace_profile + derive_profile_name stubs
- `crates/vector-config/tests/profile_writer.rs` — 5 CS-03 stubs
- `crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` — 2 Pitfall-14 lints

### Modified
- `Cargo.toml` — 12 new `[workspace.dependencies]` entries
- `Cargo.lock` — pinned cargo-platform 0.3.2 (avoids rustc 1.91 requirement)
- `crates/vector-codespaces/Cargo.toml` — replaced 3-dep [dependencies] with 14-dep + 3-dev-dep block
- `crates/vector-codespaces/src/lib.rs` — rewritten from 9-line stub to module-tree root
- `crates/vector-secrets/src/lib.rs` — added GITHUB_REFRESH_ACCOUNT const
- `crates/vector-config/src/lib.rs` — `pub mod writer;` + re-exports
- `crates/vector-config/Cargo.toml` — added toml_edit.workspace, swapped tempfile to .workspace
- `crates/vector-arch-tests/Cargo.toml` — added regex.workspace dev-dep

## Decisions Made

- **reqwest 0.12, not 0.13 (deviation from RESEARCH.md guidance):** oauth2 5.0's `Cargo.toml` requires `reqwest = "^0.12"` (verified via crates.io API). Attempting `reqwest = "0.13"` produced `error: failed to select a version for reqwest`. reqwest 0.12 has the documented `rustls-tls` feature; in 0.13 the rustls feature was renamed to `rustls`. Workspace pin is `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "http2"] }`.
- **tokio-util has no `sync` feature (deviation from PLAN.md):** Plan specified `tokio-util = { version = "0.7", features = ["sync"] }` but tokio-util 0.7.x has no `sync` feature — CancellationToken is in the default re-exports. Switched to `tokio-util = "0.7"`.
- **cargo-platform pinned to 0.3.2:** Transitive dep (via octocrab→tower-http→…) pulled in cargo-platform 0.3.3 which requires rustc 1.91; we target 1.88. `cargo update cargo-platform --precise 0.3.2` resolves cleanly.
- **DeviceCodeDisplay's Debug omits user_code:** RFC 8628 §3.1 says user_code is public-by-design, but the conservative path is to also keep it out of Debug to avoid accidentally logging it during local dev. Plan 06-02 can override if visibility helps debugging the modal.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] reqwest feature name `rustls-tls` not present in reqwest 0.13**
- **Found during:** Task 06-01-01 (workspace dep wire-up + cargo check)
- **Issue:** Plan said `reqwest = { version = "0.13", ... features = ["rustls-tls", "json"] }`. reqwest 0.13.3 renamed `rustls-tls` to `rustls`. `cargo check` failed with `reqwest does not have that feature`.
- **Investigation:** Tried `features = ["rustls", "json", "http2"]` on reqwest 0.13 — then hit `oauth2 5.0 requires reqwest ^0.12`. Verified via crates.io dependency API.
- **Fix:** Pinned workspace `reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json", "http2"] }`.
- **Verification:** `cargo check -p vector-codespaces` exits 0.
- **Committed in:** `89a0539`

**2. [Rule 3 — Blocking] tokio-util "sync" feature does not exist**
- **Found during:** Task 06-01-01
- **Issue:** Plan said `tokio-util = { version = "0.7", features = ["sync"] }`. cargo errored: `tokio-util does not have that feature`.
- **Fix:** Switched to `tokio-util = "0.7"` (default features). CancellationToken is at the root of the crate.
- **Committed in:** `89a0539`

**3. [Rule 3 — Blocking] cargo-platform 0.3.3 requires rustc 1.91; workspace pins 1.88**
- **Found during:** Task 06-01-01
- **Issue:** First post-fix `cargo check` failed with `cargo-platform@0.3.3 requires rustc 1.91`.
- **Fix:** `cargo update cargo-platform --precise 0.3.2` — downgrade lockfile entry, leaves everything else at latest.
- **Verification:** `cargo check -p vector-codespaces` succeeds.
- **Committed in:** `89a0539` (Cargo.lock change bundled)

**4. [Rule 3 — Blocking] vector-config needed toml_edit for writer.rs WriterError**
- **Found during:** Task 06-01-03 (vector-config writer scaffold)
- **Issue:** `writer.rs` imports `toml_edit::TomlError`. vector-config/Cargo.toml didn't have toml_edit.
- **Fix:** Added `toml_edit.workspace = true` to vector-config/Cargo.toml `[dependencies]`. (Already a workspace-pinned dep used by Plan 05-04's apply pipeline; just not wired into this crate.)
- **Committed in:** `f4e8917`

**5. [Rule 2 — Missing Critical] Default impl for TokenStore**
- **Found during:** Task 06-01-02
- **Issue:** clippy::pedantic (workspace lint) warns on structs with `new()` and no `Default`. TokenStore::new() takes no args.
- **Fix:** Added `impl Default for TokenStore`.
- **Committed in:** `cd110ed`

---

**Total deviations:** 5 auto-fixed (4 Rule-3 blocking, 1 Rule-2 missing critical)
**Impact on plan:** All five were dep-resolution / lint hygiene fixes that the plan couldn't have predicted without running cargo. No scope creep — the crate surface, test stubs, and arch-lint behaviors are exactly as specified.

## Issues Encountered

The five issues above were the only friction. All resolved in-line with deviation-rule fixes; no architectural changes required.

## User Setup Required

None — no external service configuration required at Wave 0. D-89's GitHub OAuth App registration is a Plan 06-02 prerequisite (or use the `gh` CLI client ID fallback `178c6fc778ccc68e1d6a`).

## Next Phase Readiness

- **Plan 06-02 (Wave 1 — device flow)** can flip `tests/device_flow.rs` `#[ignore]` to active and fill in `GitHubAuth::request_device_code` + `poll_for_token` + `TokenStore::{save_access, save_refresh, load_*, clear}` against the locked crate surface.
- **Plan 06-03 (Wave 1 — REST + 401-refresh chain)** can fill in `CodespacesClient::{list, get, start}` + the poll loop; tests are pre-staged in `codespaces_rest.rs` + `auth_refresh.rs`.
- **Plan 06-04 (Wave 1 — profile writer)** can fill in `vector-config::writer::{append_codespace_profile, derive_profile_name}`; tests are pre-staged in `tests/profile_writer.rs`.
- **Plans 06-05 / 06-06 (Wave 2 — AppKit modals)** can build the AuthModal / CodespacesPickerModal NSPanels against the populated client APIs.
- **Pitfall-14 arch-lint is enforcing from this commit onward** — any new struct in `vector-codespaces/src` that derives Debug near a token-bearing field will fail `cargo test -p vector-arch-tests`. Plans 06-02/03 should pattern-match TokenStore's manual-Debug impl.

## Self-Check: PASSED

Verified each created file + commit on disk:

- `crates/vector-codespaces/src/lib.rs` — FOUND
- `crates/vector-codespaces/src/auth/mod.rs` — FOUND
- `crates/vector-codespaces/src/auth/error.rs` — FOUND
- `crates/vector-codespaces/src/auth/device_flow.rs` — FOUND
- `crates/vector-codespaces/src/auth/token_store.rs` — FOUND
- `crates/vector-codespaces/src/client/mod.rs` — FOUND
- `crates/vector-codespaces/src/model.rs` — FOUND
- `crates/vector-codespaces/tests/device_flow.rs` — FOUND
- `crates/vector-codespaces/tests/codespaces_rest.rs` — FOUND
- `crates/vector-codespaces/tests/auth_refresh.rs` — FOUND
- `crates/vector-codespaces/tests/keychain_roundtrip.rs` — FOUND
- `crates/vector-codespaces/tests/fixtures/list_codespaces.json` — FOUND
- `crates/vector-config/src/writer.rs` — FOUND
- `crates/vector-config/tests/profile_writer.rs` — FOUND
- `crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` — FOUND
- Commit `89a0539` — FOUND
- Commit `cd110ed` — FOUND
- Commit `f4e8917` — FOUND

---
*Phase: 06-github-auth-codespaces-picker*
*Completed: 2026-05-14*
