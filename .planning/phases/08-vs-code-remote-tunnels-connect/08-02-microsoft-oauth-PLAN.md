---
phase: 08-vs-code-remote-tunnels-connect
plan: 02
type: execute
wave: 2
depends_on: [01]
files_modified:
  - crates/vector-tunnels/src/auth/mod.rs
  - crates/vector-tunnels/src/auth/device_flow_microsoft.rs
  - crates/vector-tunnels/src/auth/token_store.rs
  - crates/vector-tunnels/src/auth/error.rs
  - crates/vector-tunnels/tests/microsoft_device_flow.rs
  - crates/vector-tunnels/tests/microsoft_token_store.rs
autonomous: true
requirements:
  - DT-02
user_setup:
  - service: microsoft-entra-multi-tenant
    why: "Microsoft OAuth Device Flow against `common` authority (D-04) — accepts Adobe Entra + personal MSAs"
    env_vars: []
    dashboard_config:
      - task: "Verify the Microsoft application/client ID hard-coded in the device flow driver is the public-multi-tenant `common`-eligible ID for Dev Tunnels (no Vector-specific app registration needed for v1 since we piggyback on a Microsoft public client; if Microsoft requires a registered app, user creates an Azure App Registration with Public-client/native + redirect URI omitted + `Mobile and desktop applications` platform + reply to dev/null + add scope `46da2f7e-b5ef-422a-9a4e-fb5e1cb7da14/.default`)"
        location: "https://portal.azure.com/#view/Microsoft_AAD_RegisteredApps/ApplicationsListBlade"
must_haves:
  truths:
    - "Microsoft OAuth Device Flow drives against `https://login.microsoftonline.com/common/oauth2/v2.0/devicecode` (D-04 multi-tenant)"
    - "On success, refresh + access tokens write to Keychain under account `microsoft_refresh_token` / `microsoft_oauth_token` via vector-secrets"
    - "Silent refresh on 401 fires via stored refresh_token; expired refresh_token raises a typed error for the picker to surface re-auth"
    - "All token-bearing structs have manual `impl Debug` (Pitfall 14 arch-lint passes)"
  artifacts:
    - path: "crates/vector-tunnels/src/auth/device_flow_microsoft.rs"
      provides: "MicrosoftAuth driver: start_device_flow(), poll_until_authorized(), refresh()"
      min_lines: 80
    - path: "crates/vector-tunnels/src/auth/token_store.rs"
      provides: "MicrosoftTokenStore: load(), save(MicrosoftTokens), clear()"
      min_lines: 40
    - path: "crates/vector-tunnels/src/auth/error.rs"
      provides: "MicrosoftAuthError enum (TimedOut, DeviceCodeExpired, AccessDenied, NetworkError, RefreshExpired)"
  key_links:
    - from: "MicrosoftTokenStore::save"
      to: "vector_secrets::Secrets::set(..., MICROSOFT_REFRESH_ACCOUNT, refresh)"
      via: "Keychain write"
      pattern: "Secrets::MICROSOFT_REFRESH_ACCOUNT"
    - from: "MicrosoftAuth::poll_until_authorized"
      to: "https://login.microsoftonline.com/common/oauth2/v2.0/token"
      via: "reqwest POST + interval polling"
      pattern: "login\\.microsoftonline\\.com/common/oauth2/v2\\.0/token"
---

<objective>
Stand up the Microsoft OAuth Device Flow driver, token cache, and Keychain persistence. Mirror Phase 6's `vector-codespaces::GitHubAuth` shape one-to-one but against Microsoft `common` endpoints (D-03/D-04/D-05). Token-bearing types use manual `impl Debug` so Pitfall-14 arch-lint passes.

Purpose: gate-2 for picker UI (Plan 08-06 / 08-07) and tunnel listing (Plan 08-05) — both consume a Microsoft bearer token.
Output: typed `MicrosoftAuth` + `MicrosoftTokenStore` + `MicrosoftAuthError`; 6+ unit tests; one wiremock integration test against a mocked Microsoft token endpoint.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/phases/08-vs-code-remote-tunnels-connect/08-CONTEXT.md
@.planning/phases/08-vs-code-remote-tunnels-connect/08-RESEARCH.md
@crates/vector-codespaces/src/auth/device_flow.rs
@crates/vector-codespaces/src/auth/token_store.rs
@crates/vector-codespaces/src/auth/error.rs
@crates/vector-secrets/src/lib.rs

<interfaces>
From `crates/vector-codespaces/src/auth/device_flow.rs` — pattern to mirror verbatim.
Key API shape to replicate (substitute Microsoft endpoints + types):
- `GitHubAuth::new(client_id)` → `MicrosoftAuth::new(client_id)`
- `GitHubAuth::start_device_flow() -> Result<DeviceFlowStart>` (returns `device_code`, `user_code`, `verification_uri`, `interval`, `expires_in`)
- `GitHubAuth::poll_until_authorized(device_code, interval, expires_in, cancel: CancellationToken) -> Result<Tokens>`
- `GitHubAuth::refresh(refresh_token) -> Result<Tokens>`

Microsoft endpoints (D-04):
- Device code endpoint: `https://login.microsoftonline.com/common/oauth2/v2.0/devicecode`
- Token endpoint:       `https://login.microsoftonline.com/common/oauth2/v2.0/token`
- Scope for Dev Tunnels: `46da2f7e-b5ef-422a-9a4e-fb5e1cb7da14/.default`
- Grant type for poll:   `urn:ietf:params:oauth:grant-type:device_code`
- Grant type for refresh: `refresh_token`

From `crates/vector-secrets/src/lib.rs` (post Plan 08-01):
- `Secrets::MICROSOFT_REFRESH_ACCOUNT: &str = "microsoft_refresh_token"`
- Also add (this plan, Task 1): `Secrets::MICROSOFT_OAUTH_ACCOUNT: &str = "microsoft_oauth_token"` for access tokens.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Microsoft device flow driver + error types (TDD)</name>
  <files>crates/vector-tunnels/src/auth/mod.rs, crates/vector-tunnels/src/auth/device_flow_microsoft.rs, crates/vector-tunnels/src/auth/error.rs, crates/vector-tunnels/tests/microsoft_device_flow.rs, crates/vector-secrets/src/lib.rs</files>
  <read_first>
    - crates/vector-codespaces/src/auth/device_flow.rs (entire file — this plan mirrors its shape exactly)
    - crates/vector-codespaces/src/auth/error.rs (error variants to mirror)
    - crates/vector-tunnels/src/auth/mod.rs (Wave 0 stub from Plan 08-01 — replace contents)
    - crates/vector-secrets/src/lib.rs (add MICROSOFT_OAUTH_ACCOUNT constant alongside existing MICROSOFT_REFRESH_ACCOUNT)
  </read_first>
  <behavior>
    - Test 1 (device flow start): when Microsoft device endpoint returns `{ "device_code":"DC","user_code":"ABCD-1234","verification_uri":"https://microsoft.com/devicelogin","expires_in":900,"interval":5 }`, `MicrosoftAuth::start_device_flow()` returns a `DeviceFlowStart { device_code, user_code, verification_uri, interval: Duration::from_secs(5), expires_in: Duration::from_secs(900) }` — exact field shape, no extras.
    - Test 2 (polling success): when token endpoint returns 200 `{ "access_token":"at","refresh_token":"rt","expires_in":3600,"token_type":"Bearer","scope":"..." }`, `poll_until_authorized` returns `MicrosoftTokens { access_token, refresh_token: Some, expires_at: SystemTime::now + 3600s }`.
    - Test 3 (polling slow_down): when token endpoint returns 400 with `{"error":"slow_down"}`, the driver doubles the polling interval (Phase 6 precedent — same logic) up to a cap of 60s.
    - Test 4 (polling authorization_pending): 400 `{"error":"authorization_pending"}` keeps polling (no error raised) until expiry.
    - Test 5 (device code expired): after `expires_in` elapses without success, `poll_until_authorized` returns `MicrosoftAuthError::DeviceCodeExpired`.
    - Test 6 (cancel mid-poll): when the `CancellationToken` is cancelled, `poll_until_authorized` returns `MicrosoftAuthError::Cancelled` within 1 polling interval.
    - Test 7 (refresh success): `refresh(rt)` POSTs `grant_type=refresh_token` and returns new `MicrosoftTokens`.
    - Test 8 (refresh expired): 400 `{"error":"invalid_grant"}` from refresh returns `MicrosoftAuthError::RefreshExpired`.
    - Test 9 (Debug never leaks tokens): `format!("{:?}", tokens)` on a `MicrosoftTokens { access_token: "at_secret".into(), refresh_token: Some("rt_secret".into()), ... }` never contains `"at_secret"` or `"rt_secret"`.
  </behavior>
  <action>
    Step 1 — Add constant in `crates/vector-secrets/src/lib.rs`:
    Inside the existing `impl Secrets`, alongside `MICROSOFT_REFRESH_ACCOUNT`, add:
    ```rust
    pub const MICROSOFT_OAUTH_ACCOUNT: &str = "microsoft_oauth_token";
    ```

    Step 2 — `crates/vector-tunnels/src/auth/error.rs` (NEW):
    ```rust
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum MicrosoftAuthError {
        #[error("HTTP error: {0}")]
        Http(#[from] reqwest::Error),
        #[error("device code expired before user completed sign-in")]
        DeviceCodeExpired,
        #[error("user denied authorization")]
        AccessDenied,
        #[error("refresh token expired or revoked — re-authentication required")]
        RefreshExpired,
        #[error("sign-in cancelled")]
        Cancelled,
        #[error("Microsoft returned unexpected response: {0}")]
        Unexpected(String),
        #[error("token persistence error: {0}")]
        Storage(String),
    }
    ```

    Step 3 — `crates/vector-tunnels/src/auth/device_flow_microsoft.rs` (NEW):
    Follow `vector-codespaces/src/auth/device_flow.rs` structure EXACTLY (same fn signatures, same polling loop, same `CancellationToken` integration). Substitute:
    - Hard-code `pub const MICROSOFT_DEVICE_CODE_ENDPOINT: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/devicecode";`
    - Hard-code `pub const MICROSOFT_TOKEN_ENDPOINT: &str = "https://login.microsoftonline.com/common/oauth2/v2.0/token";`
    - Hard-code `pub const MICROSOFT_TUNNELS_SCOPE: &str = "46da2f7e-b5ef-422a-9a4e-fb5e1cb7da14/.default";`
    - Make endpoints configurable for tests via an `endpoints_override: Option<EndpointsOverride>` field on `MicrosoftAuth` so wiremock can swap in `http://127.0.0.1:PORT` URLs.

    Types (manual Debug REQUIRED — Pitfall 14):
    ```rust
    pub struct MicrosoftAuth { /* http: reqwest::Client, client_id: String, endpoints: ... */ }
    impl std::fmt::Debug for MicrosoftAuth { /* prints client_id only, NEVER http internals */ }

    pub struct DeviceFlowStart {
        pub device_code: String,    // SECRET — never derive Debug
        pub user_code: String,      // user-facing but treat as device-bound
        pub verification_uri: String,
        pub interval: std::time::Duration,
        pub expires_in: std::time::Duration,
    }
    impl std::fmt::Debug for DeviceFlowStart {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("DeviceFlowStart")
                .field("user_code", &self.user_code)
                .field("verification_uri", &self.verification_uri)
                .field("interval", &self.interval)
                .field("expires_in", &self.expires_in)
                // NOTE: device_code intentionally omitted (Pitfall 14)
                .finish()
        }
    }

    pub struct MicrosoftTokens {
        pub access_token: String,
        pub refresh_token: Option<String>,
        pub expires_at: std::time::SystemTime,
    }
    impl std::fmt::Debug for MicrosoftTokens {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MicrosoftTokens")
                .field("access_token_len", &self.access_token.len())
                .field("has_refresh", &self.refresh_token.is_some())
                .field("expires_at", &self.expires_at)
                .finish()
        }
    }
    ```

    Methods (mirror GitHubAuth):
    - `pub fn new(client_id: impl Into<String>) -> Self`
    - `pub fn with_endpoints(client_id, device_endpoint, token_endpoint, scope) -> Self` (test seam)
    - `pub async fn start_device_flow(&self) -> Result<DeviceFlowStart, MicrosoftAuthError>` — POSTs form-encoded `client_id=...&scope=...` to device endpoint; parses JSON response.
    - `pub async fn poll_until_authorized(&self, dc: &str, interval: Duration, expires_in: Duration, cancel: CancellationToken) -> Result<MicrosoftTokens, MicrosoftAuthError>` — interval polling loop; handles `authorization_pending` (continue), `slow_down` (double interval, cap 60s), `expired_token` → `DeviceCodeExpired`, `access_denied` → `AccessDenied`. Polls token endpoint with form-encoded `grant_type=urn:ietf:params:oauth:grant-type:device_code&device_code=...&client_id=...`.
    - `pub async fn refresh(&self, refresh_token: &str) -> Result<MicrosoftTokens, MicrosoftAuthError>` — POSTs `grant_type=refresh_token&refresh_token=...&client_id=...&scope=...`; maps `invalid_grant` to `RefreshExpired`.

    Use `serde_json::Value` for response parsing (mirrors GitHubAuth). Use `chrono::Utc::now()` or `std::time::SystemTime::now() + Duration::from_secs(expires_in)` for `expires_at`.

    Step 4 — `crates/vector-tunnels/src/auth/mod.rs`: replace placeholder with module exports:
    ```rust
    pub mod device_flow_microsoft;
    pub mod error;
    pub mod token_store;
    pub use device_flow_microsoft::{DeviceFlowStart, MicrosoftAuth, MicrosoftTokens};
    pub use error::MicrosoftAuthError;
    pub use token_store::MicrosoftTokenStore;
    ```

    Step 5 — `crates/vector-tunnels/tests/microsoft_device_flow.rs` (NEW): use `wiremock::MockServer` (Phase 6 pattern). Mount mock responses for device endpoint + token endpoint. Land Tests 1–9 above. Construct `MicrosoftAuth::with_endpoints(client_id, server.uri() + "/devicecode", server.uri() + "/token", "scope/.default")`.

    Pitfall 14 check: `MicrosoftTokens` MUST NOT derive Debug. Manual impl required — arch-lint will fire if violated.
  </action>
  <verify>
    <automated>cargo test -p vector-tunnels --test microsoft_device_flow &amp;&amp; cargo test -p vector-arch-tests --tests &amp;&amp; cargo clippy -p vector-tunnels --all-targets -- -D warnings &amp;&amp; ! grep -q "derive.*Debug" crates/vector-tunnels/src/auth/device_flow_microsoft.rs</automated>
  </verify>
  <acceptance_criteria>
    - `cargo test -p vector-tunnels --test microsoft_device_flow` reports >= 9 passed / 0 failed
    - `cargo test -p vector-arch-tests --tests` 0 failed (Pitfall 14 stays green)
    - `grep -c "impl std::fmt::Debug for MicrosoftAuth" crates/vector-tunnels/src/auth/device_flow_microsoft.rs` >= 1
    - `grep -c "impl std::fmt::Debug for MicrosoftTokens" crates/vector-tunnels/src/auth/device_flow_microsoft.rs` >= 1
    - `grep -c "impl std::fmt::Debug for DeviceFlowStart" crates/vector-tunnels/src/auth/device_flow_microsoft.rs` >= 1
    - `grep -q "https://login.microsoftonline.com/common/oauth2/v2.0/devicecode" crates/vector-tunnels/src/auth/device_flow_microsoft.rs` exit 0
    - `grep -q "https://login.microsoftonline.com/common/oauth2/v2.0/token" crates/vector-tunnels/src/auth/device_flow_microsoft.rs` exit 0
    - `grep -q "46da2f7e-b5ef-422a-9a4e-fb5e1cb7da14/.default" crates/vector-tunnels/src/auth/device_flow_microsoft.rs` exit 0
    - `! grep -E "#\\[derive\\([^)]*Debug" crates/vector-tunnels/src/auth/device_flow_microsoft.rs` exit 0 (no derived Debug)
    - `grep -q "MICROSOFT_OAUTH_ACCOUNT" crates/vector-secrets/src/lib.rs` exit 0
  </acceptance_criteria>
  <done>Microsoft Device Flow driver passes 9 unit tests including slow_down throttling, cancellation, and Debug-never-leaks-tokens. Arch-lint passes.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: MicrosoftTokenStore Keychain persistence (TDD)</name>
  <files>crates/vector-tunnels/src/auth/token_store.rs, crates/vector-tunnels/tests/microsoft_token_store.rs</files>
  <read_first>
    - crates/vector-codespaces/src/auth/token_store.rs (mirror this verbatim against Microsoft accounts)
    - crates/vector-secrets/src/lib.rs (Secrets::set / Secrets::get / Secrets::delete API — Plan 08-01 added MICROSOFT_REFRESH_ACCOUNT; Task 1 of this plan added MICROSOFT_OAUTH_ACCOUNT)
    - crates/vector-tunnels/src/auth/device_flow_microsoft.rs (just-created MicrosoftTokens type)
  </read_first>
  <behavior>
    - Test 1 (in-memory mock backend): `save(tokens)` followed by `load()` returns `Some(tokens)` with identical `access_token`/`refresh_token`/`expires_at` fields.
    - Test 2 (clear): `clear()` after `save` yields `load() == Ok(None)`.
    - Test 3 (load when never saved): `load()` on a fresh store returns `Ok(None)` not `Err`.
    - Test 4 (Debug doesn't leak): `format!("{:?}", MicrosoftTokenStore::new(secrets))` does not contain any token bytes.
  </behavior>
  <action>
    Step 1 — `crates/vector-tunnels/src/auth/token_store.rs`:
    ```rust
    use crate::auth::device_flow_microsoft::MicrosoftTokens;
    use crate::auth::error::MicrosoftAuthError;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use vector_secrets::Secrets;

    /// Persists Microsoft OAuth tokens (refresh + access + expiry) to macOS Keychain.
    /// Mirrors Phase 6 TokenStore one-to-one; uses MICROSOFT_REFRESH_ACCOUNT +
    /// MICROSOFT_OAUTH_ACCOUNT constants.
    pub struct MicrosoftTokenStore {
        secrets: Secrets,
    }

    impl std::fmt::Debug for MicrosoftTokenStore {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("MicrosoftTokenStore")
                .field("service", &self.secrets.service())
                .finish()
        }
    }

    impl MicrosoftTokenStore {
        pub fn new(secrets: Secrets) -> Self { Self { secrets } }

        /// Save access + refresh + expiry. Packs the three values into a single
        /// JSON blob stored under `MICROSOFT_REFRESH_ACCOUNT` (mirrors Phase 6).
        pub fn save(&self, t: &MicrosoftTokens) -> Result<(), MicrosoftAuthError> {
            let expires_at_secs = t.expires_at.duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs()).unwrap_or(0);
            let blob = serde_json::json!({
                "access_token": t.access_token,
                "refresh_token": t.refresh_token,
                "expires_at_unix": expires_at_secs,
            }).to_string();
            self.secrets.set(Secrets::MICROSOFT_REFRESH_ACCOUNT, &blob)
                .map_err(|e| MicrosoftAuthError::Storage(e.to_string()))
        }

        /// Load. Returns Ok(None) if not present.
        pub fn load(&self) -> Result<Option<MicrosoftTokens>, MicrosoftAuthError> {
            match self.secrets.get(Secrets::MICROSOFT_REFRESH_ACCOUNT) {
                Ok(blob) => {
                    let v: serde_json::Value = serde_json::from_str(&blob)
                        .map_err(|e| MicrosoftAuthError::Storage(format!("invalid blob: {e}")))?;
                    let access = v["access_token"].as_str().unwrap_or("").to_string();
                    let refresh = v["refresh_token"].as_str().map(String::from);
                    let exp_unix = v["expires_at_unix"].as_u64().unwrap_or(0);
                    Ok(Some(MicrosoftTokens {
                        access_token: access,
                        refresh_token: refresh,
                        expires_at: UNIX_EPOCH + Duration::from_secs(exp_unix),
                    }))
                }
                Err(_) => Ok(None),  // not-present is the common path; treat any error as "not stored"
            }
        }

        pub fn clear(&self) -> Result<(), MicrosoftAuthError> {
            // Best-effort delete; ignore "not found"
            let _ = self.secrets.delete(Secrets::MICROSOFT_REFRESH_ACCOUNT);
            Ok(())
        }
    }
    ```

    Step 2 — `crates/vector-tunnels/tests/microsoft_token_store.rs` (NEW):
    Test against a real `Secrets::for_vector()` with a UNIQUE service-namespace per test so concurrent runs don't collide. Use `Secrets::new(format!("vector-test-msft-{}", uuid::Uuid::new_v4()))` — add `uuid = "1"` to `[dev-dependencies]` if not already present, or use a process-pid + nano-time string. Cleanup at end of each test via `clear()`.

    Land Tests 1–4 from `<behavior>`.

    If Phase 6's `TokenStore` test discipline is "manual UAT only because Keychain prompts on macOS," follow the same pattern: gate this test file behind `#[ignore]` and document in the file's top comment that manual run is required. Read `crates/vector-codespaces/src/auth/token_store.rs` test stub (`crates/vector-codespaces/tests/keychain_roundtrip.rs` per STATE.md) to confirm the pattern, then mirror it.

    Step 3 — Verify Plan 08-01 already added `crates/vector-secrets/src/lib.rs::Secrets::delete` method. If it does NOT exist (vector-secrets currently only ships `get` + `set`), STOP and add it minimally:
    ```rust
    pub fn delete(&self, account: &str) -> Result<(), SecretsError> {
        ensure_default_store()?;
        let entry = Entry::new(&self.service, account)?;
        let _ = entry.delete_credential();  // ignore not-found
        Ok(())
    }
    ```
    Place it inside `impl Secrets`. Run `cargo test -p vector-secrets` to confirm zero regression.
  </action>
  <verify>
    <automated>cargo build -p vector-tunnels --tests &amp;&amp; cargo test -p vector-tunnels --test microsoft_token_store -- --include-ignored 2>&amp;1 | tail -20 || echo "manual UAT pattern (Phase 6 precedent) — test scaffold ships" &amp;&amp; grep -q "MICROSOFT_REFRESH_ACCOUNT" crates/vector-tunnels/src/auth/token_store.rs &amp;&amp; grep -q "impl std::fmt::Debug for MicrosoftTokenStore" crates/vector-tunnels/src/auth/token_store.rs</automated>
  </verify>
  <acceptance_criteria>
    - `cargo build -p vector-tunnels --tests` exit 0
    - `grep -c "Secrets::MICROSOFT_REFRESH_ACCOUNT" crates/vector-tunnels/src/auth/token_store.rs` >= 1
    - `grep -c "impl std::fmt::Debug for MicrosoftTokenStore" crates/vector-tunnels/src/auth/token_store.rs` >= 1
    - `! grep -E "#\\[derive\\([^)]*Debug" crates/vector-tunnels/src/auth/token_store.rs` exit 0 (no derived Debug)
    - `grep -c "fn save\\|fn load\\|fn clear" crates/vector-tunnels/src/auth/token_store.rs` >= 3
    - `cargo test -p vector-arch-tests --tests` 0 failed
    - `cargo clippy -p vector-tunnels --all-targets -- -D warnings` exit 0
    - Test file `crates/vector-tunnels/tests/microsoft_token_store.rs` exists with >= 4 test functions
  </acceptance_criteria>
  <done>Microsoft Keychain token persistence ships matching Phase 6 GitHub TokenStore shape. All token-bearing types have manual Debug.</done>
</task>

</tasks>

<verification>
- `make lint` exit 0
- `make test` exit 0 (workspace tests including 9+ device-flow tests + token_store stubs)
- `cargo test -p vector-arch-tests --tests` 0 failed (Pitfall 14 holds)
</verification>

<success_criteria>
- Microsoft OAuth Device Flow + Keychain TokenStore land matching Phase 6 GitHub auth shape
- Manual Debug on every token-bearing struct (no derives)
- Wiremock-backed tests cover happy path + slow_down + expired_token + access_denied + cancellation + refresh + invalid_grant
- vector-secrets exposes MICROSOFT_OAUTH_ACCOUNT in addition to MICROSOFT_REFRESH_ACCOUNT
</success_criteria>

<output>
After completion, create `.planning/phases/08-vs-code-remote-tunnels-connect/08-02-SUMMARY.md` documenting:
- the verified Microsoft Dev Tunnels scope GUID used (note: 46da2f7e-... — confirm this is still current at execution time by checking 08-RESEARCH.md; if Microsoft changed it, document the new GUID)
- the public-multi-tenant client ID used (and whether a Vector-specific Azure App Registration was needed)
- any deviation from Phase 6 GitHubAuth shape (should be zero — flag if any)
</output>
