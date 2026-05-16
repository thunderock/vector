# Phase 6: GitHub Auth + Codespaces Picker — Research

**Researched:** 2026-05-14
**Domain:** OAuth 2.0 Device Authorization Grant (RFC 8628), GitHub REST API (`/user/codespaces`), macOS Keychain via `keyring-core` + `apple-native-keyring-store`, AppKit modal overlays via `objc2-app-kit`.
**Confidence:** HIGH (stack + endpoints + Keychain surface), MEDIUM (octocrab Codespaces typed coverage — confirmed *missing*, must use `_get`/`_post` raw routes), HIGH (Pitfall 14 manual-Debug discipline — already enforced in `vector-secrets`).

## Summary

Phase 6 is a glue phase. Every load-bearing primitive is already on disk:

- `vector-secrets::Secrets::for_vector()` ships the `get`/`set`/`delete` Keychain triple over `keyring-core 1.0 + apple-native-keyring-store 1.0` with the manual-`Debug` discipline already in place. `GITHUB_OAUTH_ACCOUNT = "github_oauth_token"` is reserved for Phase 6's first writer.
- `vector-config::schema::ProfileBlock { kind, codespace_name, tint, ... }` is the *exact* shape D-87 saves into `~/.config/vector/config.toml`.
- AppKit menu bar install + `UserEvent` round-trip pattern (`EventLoopProxy::send_event` → main-thread `App::user_event` handler) are how every cross-thread signal travels today.
- `tokio` multi-thread runtime already runs on a background pool; OAuth poll + REST calls fit the existing actor topology.

The actual new code is small: an OAuth device-flow driver (`oauth2 5.0`), a thin Codespaces REST client (`octocrab 0.50` for auth+http, hand-rolled `_get`/`_post` for the codespaces routes because octocrab has no first-class Codespaces typed API), and two AppKit overlays (device-flow modal, picker modal). Plus a one-shot `vector-config::write_profile` helper that appends a `[profile.X]` block via `toml_edit` (already in workspace deps).

**Primary recommendation:** Build `vector-codespaces` as a transport-agnostic client: `GitHubAuth` (device flow + token cache + refresh), `CodespacesClient` (REST CRUD + state poll). Run `octocrab` calls in a tokio actor; route results to main thread via new `UserEvent` variants. Ship the picker as an `NSPanel` overlay (faster than wgpu-rendering a list); device-flow modal also `NSPanel`. Reuse Plan 05-09's `ForwardingListener` pattern: non-blocking channels, no shared state across `.await`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-84 — Sign-in trigger (AUTH-01):** `Vector → Sign in with GitHub` menu item is the primary entry point. A second trigger fires automatically when the user clicks a codespace profile in either the existing Cmd-Shift-P profile picker or the new Codespaces picker modal and no valid token is present. Both paths invoke the same OAuth device flow code path.
- **D-85 — Device flow presentation (AUTH-01):** OAuth device flow is presented as a modal window overlay (NSPanel-style). Modal:
  - Displays the 8-char user-code prominently (large monospace type).
  - Shows the URL `github.com/device` for manual entry.
  - Auto-copies the user-code to clipboard on open.
  - Single primary button: "Copy code and open github.com/device" (`NSWorkspace.openURL`).
  - "Cancel" action.
  - Stays on top of the Vector window until authentication completes or user cancels.
  - On success: dismisses itself and fires a toast ("Signed in as @username").
  - On cancel: dismisses without error (user can re-trigger via menu or profile click).
  - Pitfall 14 applies: the full token is **never** displayed in the modal or any tooltip.
- **D-86 — Codespaces picker access point (CS-01):** Dedicated Codespaces picker modal — separate from the existing Cmd-Shift-P profile picker (D-75). Triggered by:
  - Menu item: `Vector → Codespaces...`
  - Keyboard shortcut: `Cmd-Shift-G` (mnemonic: GitHub; avoids collision with existing Cmd-Shift-P, Cmd-Shift-D, Cmd-Shift-[], Cmd-Shift-C).
  - The existing Cmd-Shift-P picker continues to show all saved profiles (including `codespace` kind), but does not show the live-fetched Codespaces list.
  - **Modal columns:** state badge (color-coded: green = Available, yellow = Starting, gray = Shutdown), repo name, branch, last-used timestamp (relative: "2 hours ago").
  - **Actions per row:** "Connect" (shows toast if no transport yet, Phase 7), "Save as profile" (writes `[profile.X]` to config.toml), "Start" (visible only for Shutdown codespaces).
- **D-87 — Profile save (CS-03):** Saving a Codespace as a one-click profile writes a `[profile.X]` block directly to `~/.config/vector/config.toml` using D-74 schema exactly:
  ```toml
  [profile.octocat-hello-world]
  kind = "codespace"
  codespace_name = "octocat/hello-world-abc123"
  tint = "#7a3aaf"
  ```
  Profile name is derived from `codespace_name` by stripping owner prefix and randomized suffix (e.g. `octocat/hello-world-abc123` → `hello-world`), de-colliding with a numeric suffix. Default tint = `#7a3aaf` (distinct from local default which has no stripe).
- **D-88 — State refresh strategy (CS-01 / CS-02):** On-demand fetch + active transition poll.
  - Full list fetched from `GET /user/codespaces` each time picker opens (one call per open, spinner during fetch).
  - Manual "Refresh" icon in picker header triggers re-fetch.
  - While codespace in `starting` state: poll `GET /user/codespaces/{name}` at 1s interval, update row's state badge live, stop polling when state becomes `available` or `stopped` / on 2-min timeout.
  - No background polling when picker is closed.
  - On 401 during any API call: silently trigger token refresh (AUTH-03 path); re-run original request once; if still 401, show re-auth prompt.
- **D-89 — OAuth client registration:** Register a dedicated GitHub OAuth App for Vector (`vector-terminal`). Do NOT reuse `gh` CLI client ID (`178c6fc778ccc68e1d6a`). Scopes: `codespace read:user`. Client ID embedded as build constant (not a secret — device flow client IDs are public per spec). ROADMAP notes `gh` CLI client ID as reusable fallback if custom app isn't ready at planning time.
- **D-90 — Token storage shape:** Token storage via `vector-secrets::Secrets::for_vector()` with two accounts:
  - `GITHUB_OAUTH_ACCOUNT = "github_oauth_token"` (already defined) — access token.
  - Add `GITHUB_REFRESH_ACCOUNT = "github_refresh_token"` — refresh token (if GitHub provides one; device flow may return only an access token; in that case re-run device flow on expiry, which is AUTH-03).
  - Both fields use `zeroize::Zeroizing<String>` wrappers in memory per `vector-secrets`'s exported `use zeroize;`.
  - Manual `Debug` impl on every struct that holds a token — never derive (Pitfall 14).

### Claude's Discretion

- **`oauth2 5.0` vs. raw `reqwest` for device flow** — Researcher evaluates; `oauth2 5.0` device flow is the recommended default per CLAUDE.md stack docs.
- **`octocrab 0.50` client setup** — Single shared `Arc<Octocrab>` with the bearer token injected via `OctocrabBuilder::personal_token`. Recreated on token refresh.
- **Picker modal rendering** — Whether to implement via AppKit `NSPanel` (`objc2-app-kit`) or render in-process via the existing wgpu compositor. Researcher's call; NSPanel is likely faster to ship.
- **`Cmd-Shift-G` conflict check** — Planner should verify no existing system-level shortcut conflicts on macOS (it's unused in standard AppKit).
- **Error display in picker modal** — Network errors during fetch show an inline error state in the list area ("Could not fetch codespaces — check your connection [Retry]"), not a separate dialog.
- **`GET /user/codespaces` pagination** — GitHub paginates at 30 per page; use `per_page=100` to reduce round trips. v1 cap at 100 codespaces (more than enough for personal use).

### Deferred Ideas (OUT OF SCOPE)

- GitHub status page integration — showing GitHub.com incident status in the picker when Codespaces are failing.
- Codespace creation from picker — out of scope per PROJECT.md (connect-only for v1).
- Multi-account GitHub support — single account only for v1; second GitHub account could be a v2 feature.
- Codespace rebuild / delete from picker — out of scope per PROJECT.md.

</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| AUTH-01 | GitHub OAuth Device Flow (RFC 8628) sign-in from inside the app — no browser plugin, no PAT pasting | Standard Stack §`oauth2 5.0` (device flow); Architecture §Device Flow State Machine; Code Examples §1 |
| AUTH-02 | OAuth tokens stored in macOS Keychain via `keyring 4.0`; never written to disk in plaintext, never logged | Existing `vector-secrets::Secrets::for_vector()` already wired; Pitfalls §Pitfall 14 (manual Debug); Code Examples §2 |
| AUTH-03 | Token refresh handled silently; expired tokens trigger re-auth prompt rather than silent failure | Architecture §401 → silent refresh → re-auth; D-88; Common Pitfalls §"Silent failures hide expired tokens" |
| CS-01 | Codespaces picker lists every codespace with state, repository name, branch, last-used time | Standard Stack §`octocrab 0.50` raw `_get`; Code Examples §3 (Codespace struct + `GET /user/codespaces`) |
| CS-02 | Selecting Shutdown codespace triggers `POST /start`, polls until Available (409 swallowed) | Code Examples §4 (start + 409 swallow + 1s poll loop with 2-min cap) |
| CS-03 | Picked codespace can be saved as a one-click profile that survives app restart | Existing `ProfileBlock` schema; Code Examples §5 (`toml_edit` append); Architecture §Profile Save Path |

</phase_requirements>

## Standard Stack

All four crates listed below are *already named* in CLAUDE.md as the canonical Phase 6 stack. The job of this section is to record verified current versions and pin them at workspace scope.

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `oauth2` | 5.0.0 (2025-01-21) | OAuth 2.0 Device Authorization Grant (RFC 8628) | Most-downloaded OAuth crate in Rust; first-class `DeviceAuthorizationUrl` + `device_authorization()` API; pure HTTP client (any `reqwest`-compatible transport). Avoids hand-rolling RFC 8628 polling + slow-down handling. |
| `octocrab` | 0.50.0 (latest as of 2026-05-05) | GitHub REST API client | Typed wrapper around `/repos/*`, `/user/*`, `/orgs/*`. Codespaces routes are NOT typed in 0.50 (confirmed via `docs.rs/octocrab`) but `Octocrab::_get`, `_post`, `_patch` are public for raw routes. We use the typed `OctocrabBuilder::personal_token` for auth header injection + shared HTTP client + retry plumbing; we hand-route the codespaces endpoints. |
| `keyring-core` + `apple-native-keyring-store` | `keyring-core 1.0` + `apple-native-keyring-store 1.0` (features = ["keychain"]) | macOS Keychain access (replaces top-level `keyring 4.0` library API per workspace decision) | Already wired in `crates/vector-secrets/src/lib.rs`. Service = `"vector"`, accounts = `"github_oauth_token"` (defined) + `"github_refresh_token"` (Phase 6 adds). Phase 6 is the first writer. |
| `zeroize` | 1.x via `vector-secrets`'s re-export (`use zeroize;`) | Memory hygiene for in-process token strings | Wrap raw token strings in `zeroize::Zeroizing<String>` at acquisition (oauth2 response → keychain set). Drops zero memory at scope exit. |
| `reqwest` | 0.13.x (workspace transitive) | HTTP transport for `oauth2` + `octocrab` | Both `oauth2 5.0` and `octocrab 0.50` accept any `reqwest::Client`-compatible HTTP layer. Pin once at workspace level so a single TLS stack (rustls preferred) ships. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `toml_edit` | 0.22 (already workspace dep) | Round-trip TOML editor for `~/.config/vector/config.toml` | Profile save (D-87): preserve user formatting, append `[profile.X]` table without reformatting unrelated blocks. Already used in `vector-config` for Plan 05-04's apply pipeline. |
| `serde` + `serde_json` | serde 1 (workspace), serde_json 1.x (add) | Deserialize Codespace REST response | Codespace JSON has ~30 fields; only the 6 we need (`name`, `state`, `repository.full_name`, `git_status.ref`, `last_used_at`, `display_name`) get typed; rest captured with `#[serde(flatten)] _rest: BTreeMap<String, serde_json::Value>` to survive future field additions. |
| `chrono` | 0.4 (new) | `last_used_at` ISO-8601 parse + relative "2 hours ago" rendering | GitHub returns timestamps in RFC 3339; chrono is the standard parser. Format helper does `now - parsed` → human string. |
| `tracing` | workspace | Structured logging for device-flow state transitions | Per Pitfall 14: log STATE NAMES only (`device_flow_initiated`, `device_flow_polling`, `device_flow_complete`, `token_refresh_attempted`, `token_refresh_failed`) — NEVER log token material, user code, refresh token, or even the device code. |
| `objc2-app-kit` | 0.3 (workspace) | AppKit modal panel (`NSPanel`), `NSWorkspace::openURL`, `NSPasteboard` for code copy | Existing dep. `NSPanel { styleMask: .titled \| .closable }`, `setLevel(NSFloatingWindowLevel)` for stay-on-top. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `oauth2 5.0` | `gh-device-flow` crate (jakewilkins) | Tiny library specifically for GitHub device flow. Pro: zero ceremony. Con: re-implements oauth2 internals; less audited; not what CLAUDE.md prescribes. Reject. |
| `oauth2 5.0` | Hand-rolled `reqwest::Client::post` of the two device-flow endpoints | ~120 lines saved at the cost of writing the slow-down / expires_in / interval state machine ourselves. RFC 8628 §3.5 ("slow_down" response code, polling interval bump) is the part that gets gotchas. Reject. |
| `octocrab 0.50` raw `_get/_post` | Direct `reqwest::Client::get("https://api.github.com/user/codespaces")` | Saves one crate. Loses: typed `Octocrab` builder, automatic `Authorization` header injection, `accept: application/vnd.github+json` header consistency, retry helpers, paginator (octocrab has `OctocrabBuilder::add_preview_header` etc.). Reject; keep octocrab. |
| `serde_json` flatten + `BTreeMap<String, Value>` | `#[serde(deny_unknown_fields)]` (project default per D-68) | Codespace response is a moving target on GitHub's side. `deny_unknown_fields` will break Vector the day GitHub adds a field. Document the exception explicitly in module-level rustdoc; this is one of the rare places where it's the wrong default. |
| AppKit `NSPanel` modal | wgpu-rendered overlay in the existing compositor | wgpu route forces us to re-implement text input + button hit-testing inside the renderer for two screens that exist for ~10 seconds each. NSPanel ships in ~50 LOC each. Reject wgpu route. |
| `chrono 0.4` | `time 0.3` | `time` is the modern alternative with no deprecated TZ APIs. Either works for ISO-8601 parse. `chrono`'s `humantime`-style formatting is more straightforward; if `time` is already a transitive elsewhere, use it. Either choice is HIGH confidence. |

**Installation:**
```toml
# Workspace Cargo.toml additions:
oauth2 = { version = "5.0", default-features = false, features = ["reqwest", "rustls-tls"] }
octocrab = "0.50"
serde_json = "1"
chrono = { version = "0.4", default-features = false, features = ["clock", "serde"] }
# reqwest already pinned via octocrab's transitive; add explicit workspace pin if not already:
reqwest = { version = "0.13", default-features = false, features = ["rustls-tls", "json"] }
```

**Version verification:** Versions in CLAUDE.md (oauth2 5.0.0, octocrab 0.50.0, keyring 4.0.0) date to 2025-2026; `keyring 4.0` library API was split into `keyring-core 1.0 + apple-native-keyring-store 1.0` and the workspace has already migrated. Re-verify the day Phase 6 starts with `cargo search oauth2 octocrab chrono` against crates.io — if any has shipped a 6.x / 0.51 / 0.5, walk the changelog.

## Architecture Patterns

### Recommended Crate Wiring

```
crates/
├── vector-codespaces/          # NEW: full Phase-6 implementation
│   └── src/
│       ├── lib.rs              # Public surface: GitHubAuth + CodespacesClient + types
│       ├── auth.rs             # Device flow driver + token cache + refresh on 401
│       ├── client.rs           # CodespacesClient over Arc<Octocrab>; raw _get / _post
│       ├── model.rs            # Codespace, CodespaceState, RepositoryRef, GitStatus
│       └── error.rs            # CodespacesError (oauth2, octocrab, http, decode)
├── vector-config/              # ADD: write_profile API for D-87
│   └── src/
│       └── writer.rs           # NEW: append_profile(path, name, ProfileBlock) -> Result<()>
└── vector-app/                 # WIRE: menu items, UserEvents, NSPanel modals
    └── src/
        ├── menu.rs             # Vector → Sign in / Codespaces... menu items
        ├── auth_modal.rs       # NEW: device-flow NSPanel
        ├── codespaces_modal.rs # NEW: picker NSPanel
        └── app.rs              # NEW UserEvent variants + handlers
```

### Pattern 1: OAuth Device-Flow State Machine

The 4-state machine the device-flow modal drives (`oauth2 5.0` already encodes the protocol — we only wire UI ↔ tokio state):

```
[Idle]
  ↓ user clicks "Sign in with GitHub"
[RequestingCode]
  POST https://github.com/login/device/code
  ↓ response { device_code, user_code, verification_uri, expires_in, interval }
[AwaitingUser]
  show user_code in NSPanel; copy to NSPasteboard; openURL(verification_uri)
  ↓ poll every interval seconds:
  POST https://github.com/login/oauth/access_token
    grant_type=urn:ietf:params:oauth:grant-type:device_code
  responses:
    authorization_pending → loop
    slow_down            → bump interval by 5s per RFC 8628 §3.5; loop
    expired_token        → fail; dismiss modal; toast "Code expired, try again"
    access_denied        → fail; dismiss; toast "Sign-in cancelled"
    success              → goto Storing
[Storing]
  zeroize::Zeroizing<String> wrap of access_token (+ refresh_token if present)
  Secrets::for_vector().set("github_oauth_token", &access_token)
  Secrets::for_vector().set("github_refresh_token", &refresh_token)?  # only if present
  fetch GET /user → @username
  ↓
[Idle] (dismiss modal; toast "Signed in as @username")
```

**Threading note:** Steps `[RequestingCode]`/`[AwaitingUser]`/`[Storing]` all run in a tokio task spawned via the existing background runtime. UI events go back through `EventLoopProxy::send_event(UserEvent::AuthStateChanged(_))`. The modal observes `UserEvent::AuthDisplayCode { code, url, expires_at }` and `UserEvent::AuthCompleted { user_login }` and `UserEvent::AuthFailed { reason }`.

### Pattern 2: 401 → Silent Refresh → Re-Auth Prompt

```
CodespacesClient::get_codespaces() -> Result<Vec<Codespace>>:
  attempt 1: octocrab._get("/user/codespaces?per_page=100")
  if 401:
    if let Some(refresh_token) = Secrets::get("github_refresh_token").ok():
      // GitHub may rotate the refresh token; both new access and new refresh must be stored.
      new_tokens = oauth2.exchange_refresh_token(&refresh_token).request_async().await?
      rebuild Arc<Octocrab> with new bearer
      attempt 2: octocrab._get(...)
      if 401: send UserEvent::AuthRequired (show device-flow modal)
    else:
      // No refresh token — device flow only returned access token
      send UserEvent::AuthRequired
  if 2xx: parse + return
```

Decision: GitHub's OAuth App device flow can issue a refresh token only when the app opts in (Settings → OAuth Apps → "Enable Device Flow" + the Apps API: refresh tokens are app-scoped, see [docs](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/refreshing-user-access-tokens)). If `vector-terminal` (D-89) is registered as a plain OAuth App, **no refresh token is issued** — access tokens last 8 hours (per GitHub's OAuth App default) and 401 → re-run full device flow. AUTH-03's "silent refresh" is then defined as "no extra UI; just re-run the device flow modal," which still meets the requirement.

### Pattern 3: Picker Open → Fetch → Poll Transitions

```
Cmd-Shift-G OR menu → UserEvent::OpenCodespacesPicker
  NSPanel modal opens with spinner
  spawn tokio task:
    GET /user/codespaces?per_page=100 (sync inside the task)
    → UserEvent::CodespacesLoaded(Vec<Codespace>) OR CodespacesLoadFailed(err)
  modal renders rows from received list

  For each row in `starting` state at load:
    spawn poll task:
      loop every 1s for up to 120s:
        GET /user/codespaces/{name}
        → UserEvent::CodespaceStateChanged { name, state }
        break when state ∈ {available, stopped, failed}

  User clicks "Start" on Shutdown row:
    POST /user/codespaces/{name}/start
      treat 409 as success (already starting; another client also started)
    → spawn the same poll task as above

  User clicks "Save as profile":
    sync: vector_config::writer::append_profile(...)
    → UserEvent::ToastInfo("Profile saved as {name}")

  Modal closes → cancel all in-flight poll tasks (one-shot cancellation tokens; see Pattern 5)
```

### Pattern 4: Octocrab + Codespaces (no first-class typed support)

`octocrab 0.50` does NOT have `octocrab.codespaces()`. Raw routes via `_get` / `_post` are public:

```rust
// model.rs
#[derive(serde::Deserialize, Debug, Clone)]
pub struct Codespace {
    pub name: String,
    pub state: CodespaceState,
    pub repository: RepositoryRef,
    pub git_status: GitStatus,
    pub last_used_at: chrono::DateTime<chrono::Utc>,
    pub display_name: Option<String>,
    // Survive GitHub adding fields:
    #[serde(flatten)]
    _rest: serde_json::Map<String, serde_json::Value>,
}

#[derive(serde::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]   // GitHub returns "Available", "Starting", ...
pub enum CodespaceState {
    Available,
    Starting,
    ShuttingDown,
    Shutdown,
    Archived,
    Failed,
    Provisioning,
    Queued,
    Updating,
    Rebuilding,
    Unknown,
    Created,
    #[serde(other)]
    Unrecognized,   // future-proof for new states
}
```

```rust
// client.rs
use octocrab::Octocrab;
use serde::Deserialize;

#[derive(Deserialize)]
struct CodespacesPage {
    total_count: u32,
    codespaces: Vec<Codespace>,
}

pub async fn list(octo: &Octocrab) -> Result<Vec<Codespace>, CodespacesError> {
    let page: CodespacesPage = octo._get("/user/codespaces?per_page=100").await?.json().await?;
    Ok(page.codespaces)
}

pub async fn get(octo: &Octocrab, name: &str) -> Result<Codespace, CodespacesError> {
    let path = format!("/user/codespaces/{}", urlencoding::encode(name));
    Ok(octo._get(path).await?.json().await?)
}

pub async fn start(octo: &Octocrab, name: &str) -> Result<(), CodespacesError> {
    let path = format!("/user/codespaces/{}/start", urlencoding::encode(name));
    let res = octo._post(path, None::<&()>).await?;
    match res.status().as_u16() {
        200 | 202 => Ok(()),
        409 => Ok(()),  // already starting (concurrent start), per D-88
        s => Err(CodespacesError::StartFailed(s)),
    }
}
```

### Pattern 5: Cancellation via `tokio_util::sync::CancellationToken`

The picker spawns N poll tasks (one per `starting` row). When the modal closes, every task must stop. Use one `CancellationToken` per modal session; clone it into every spawned task; `select!` between `token.cancelled()` and the next `sleep(1s)`. `tokio_util` is already in workspace transitives via `tonic`/`reqwest`.

### Anti-Patterns to Avoid

- **Derive `Debug` on any struct holding a token, refresh token, device code, or even user code** — Pitfall 14 fires hard. Every new struct in `vector-codespaces::auth` gets a manual `Debug` impl that omits secret-bearing fields. Model: `Secrets`'s impl in `vector-secrets/src/lib.rs`.
- **Log token material in `tracing::info!`** — even at `tracing::debug!`. Log state-machine *names* only (`device_flow_polling`, `token_refresh_attempted`).
- **Block the AppKit main thread on an HTTP call** — every REST call routes through a tokio task; results arrive via `EventLoopProxy::send_event`. (Pitfall: holding a `parking_lot::Mutex<Octocrab>` across an `await` will hit `clippy::await_holding_lock = "deny"` per workspace lint.)
- **Reuse the existing Cmd-Shift-P picker** for the live-fetched codespaces — D-86 explicitly forbids it. Codespaces modal is its own NSPanel.
- **Hand-rolled OAuth polling math** — let `oauth2 5.0`'s `exchange_device_access_token` handle the slow_down / interval / expires_in logic.
- **Reading the access token to put in a `String` field that gets `Debug`-logged anywhere** — Token only ever travels in `zeroize::Zeroizing<String>`. To attach to an HTTP request: clone into a fresh `Zeroizing<String>` immediately consumed by `OctocrabBuilder::personal_token`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| RFC 8628 polling state machine | Hand-rolled timer + `match response.error` table | `oauth2 5.0`'s `exchange_device_access_token().request_async(&http_client)` | RFC §3.5 "slow_down" + interval/expires_in handling is gotcha-prone. |
| Keychain access | Direct `Security.framework` FFI via `objc2` | `vector-secrets::Secrets::for_vector()` | Already implemented; Pitfall 14 audit trail already passes review. |
| TOML preserving-edit | Re-serialize whole `ConfigFile` with `toml::to_string` | `toml_edit::DocumentMut::insert` on the existing parsed doc | `toml::to_string` reformats the user's hand-edited file; `toml_edit` preserves comments + ordering. Already used by `vector-config`. |
| HTTP retry / 401-detect / pagination | Per-call match arm | `octocrab::OctocrabBuilder::personal_token` + `_get` returning `reqwest::Response` (status detectable) | Reusing octocrab's plumbing means consistent `Accept: application/vnd.github+json`, `User-Agent: Vector/0.1` headers. |
| Relative time strings ("2 hours ago") | Hand-rolled `Duration` match arms | `chrono` + a tiny `fn humanize(d: Duration)` (~25 LOC) | Reasonable to ship the 25 lines; full crate (`chrono-humanize`) adds 200 KB for one function. Keep `chrono` for parsing; write the formatter inline. |
| OAuth client ID secret-storage | Embed obfuscated in code, fetch from env, etc. | Embed as `const CLIENT_ID: &str = "Iv1.<...>"` | Device flow client IDs are PUBLIC per RFC 8628 §3.1. No "client secret" exists for device flow. CLAUDE.md confirms `gh` CLI does this exact thing. |

**Key insight:** Phase 6 has near-zero hand-rolled new logic. The work is connecting four well-trodden libraries (`oauth2`, `octocrab`, `vector-secrets`, `objc2-app-kit`) with two AppKit panels and one TOML-edit helper. Pitfall 14 manual-Debug discipline is the one place where care matters; every other line of code is glue.

## Runtime State Inventory

> Phase 6 is greenfield (not a rename/refactor). Section retained for the few touchpoints that *do* persist state outside git.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | macOS Keychain entries with `service = "vector"`, accounts = `"github_oauth_token"` (already reserved) + `"github_refresh_token"` (new in Phase 6). | Phase 6 writes them; no migration of existing data because no Phase ≤ 5 writes secrets. `Secrets::delete(account)` exists for the sign-out path. |
| Live service config | GitHub OAuth App registration (`vector-terminal`) on the github.com side — exists *outside* the repo. D-89 requires it. | Action: register the OAuth App and copy the client ID into a build constant before Phase 6 starts. If not ready in time, fall back to `gh` CLI client ID `178c6fc778ccc68e1d6a` per D-89 / ROADMAP. |
| OS-registered state | None. AppKit menu items and NSPanels are created at runtime; they don't persist across launches. | None. |
| Secrets/env vars | `vector-secrets` consumes nothing from env. The build constant for OAuth client ID is checked into source. | None. |
| Build artifacts | New crate `vector-codespaces` joins the workspace; existing skeleton's `src/lib.rs` is replaced (no breaking change to consumers because nothing depends on it yet). | None — `cargo build --workspace` will pick it up automatically. |

**Verified nothing-found:** No persisted on-disk caches outside macOS Keychain; no env-var injection; no Tailscale/Datadog/n8n style external services tied to Vector.

## Common Pitfalls

### Pitfall 1: Token leaks into `Debug` / logs

**What goes wrong:** Someone adds `#[derive(Debug)]` on `struct GitHubAuth { access_token: String, ... }`. A `tracing::debug!("auth = {auth:?}")` lands in a CI log. Now the OAuth token is in a log retention bucket forever.
**Why it happens:** Manual `Debug` is fiddly; the project's `#[derive(Debug, Clone)]` muscle memory wins.
**How to avoid:**
- Use the linter-enforceable pattern: any type holding `Zeroizing<String>` or any field named `*_token` or `*_secret` gets a hand-written `impl Debug` printing `Type { service, account }` and a `<redacted>` placeholder.
- Add an arch-lint test in `vector-arch-tests` that greps `crates/vector-codespaces/src/auth.rs` for `#[derive(.*Debug` near `token` or `secret` and fails the build.
**Warning signs:** Any line in `auth.rs` with both `derive` and `token` in the same string. Any `tracing::*!` arg with a struct that holds a token.

### Pitfall 2: octocrab `Octocrab` cloned with stale token after refresh

**What goes wrong:** App holds `Arc<Octocrab>` for the session. On 401, refresh tokens, but every Codespaces task already cloned the old `Arc` — they keep failing with 401 forever.
**Why it happens:** `Arc<Octocrab>`'s bearer token is immutable; rebuilding the client returns a *new* `Octocrab` that the existing clones never see.
**How to avoid:** Hold `parking_lot::RwLock<Arc<Octocrab>>` at the app level (or `arc_swap::ArcSwap<Octocrab>` if we add the dep). Every call site does `let octo = state.octocrab.read().clone();` immediately before the await, so a refresh that swaps the inner `Arc` is picked up on the *next* call without breaking in-flight ones.
**Warning signs:** Tests that simulate a 401 followed by a refresh — second call still 401.

### Pitfall 3: NSPanel modal blocks event loop / steals key forever

**What goes wrong:** `NSPanel` with `styleMask: .nonactivatingPanel` and `setLevel(.modalPanel)` sometimes refuses to give up key-window status after dismiss; cmd-Q does nothing.
**Why it happens:** `becomesKeyOnlyIfNeeded` + `.modalPanel` interactions on macOS 13/14 differ.
**How to avoid:** Use `NSPanel { styleMask: .titled \| .closable }` + `setLevel(.floatingWindow)`. Don't set `.modalPanel` level. Add an explicit `panel.orderOut(nil)` + `mainWindow.makeKeyAndOrderFront(nil)` on dismiss.
**Warning signs:** Cmd-Q nonfunctional after closing the modal; titlebar focus ring stuck on the panel.

### Pitfall 4: GitHub adds a new `CodespaceState` value → deserialization fails

**What goes wrong:** GitHub introduces `Hibernated`. Vector's `serde::Deserialize` on `enum CodespaceState` returns an error; the entire list fails to load.
**Why it happens:** Default serde behavior for un-matched enum variants is hard error.
**How to avoid:** Add `#[serde(other)] Unrecognized` as the last variant. Existing rows render with a neutral state badge ("Unknown") and the user can still hit "Refresh" — Vector never crashes.
**Warning signs:** A future `GET /user/codespaces` returning an HTTP 200 with valid JSON, but Vector showing "Could not load."

### Pitfall 5: 409 from POST /start treated as failure

**What goes wrong:** Two Vector instances (or Vector + `gh` CLI) start the same Codespace simultaneously. GitHub returns 409 on the second call. Vector pops a toast "Failed to start" even though the codespace is in fact starting.
**Why it happens:** Naive HTTP status check.
**How to avoid:** Per D-88 / ROADMAP, treat 409 as a no-op success: codespace is already (or now) starting; subsequent state polls will see `starting → available`. Match `200 | 202 | 409` as success.
**Warning signs:** Spurious "Start failed" toasts during testing with multiple GitHub clients open.

### Pitfall 6: Silent failures hide expired tokens (AUTH-03)

**What goes wrong:** Token expires; `octocrab._get` returns 401. We silently fail and show an empty codespaces list. User thinks they have no codespaces.
**Why it happens:** Catching errors at the wrong layer.
**How to avoid:** All `CodespacesError` variants flow up to the modal handler. `CodespacesError::Unauthenticated` (synthesized when the 401-refresh-401 chain completes) triggers `UserEvent::AuthRequired`, which opens the device-flow modal. Empty list ≠ unauthenticated. Always distinguish.
**Warning signs:** Picker showing "0 codespaces" when the user has 5; or an error toast with no explanation.

### Pitfall 7: User-code in clipboard leaked to subsequent paste

**What goes wrong:** Modal copies the 8-char user-code to NSPasteboard on open. User authenticates, dismisses, switches to another app, hits Cmd-V — pastes their user code into a chat window.
**Why it happens:** Pasteboard persists until overwritten.
**How to avoid:** On modal dismiss (success OR cancel), restore the previous clipboard content. Capture `NSPasteboard.generalPasteboard().stringForType(NSPasteboardTypeString)` on open, restore on close. (Apple recommends not stomping the clipboard at all — but D-85 explicitly chose auto-copy; this is the mitigation.)
**Warning signs:** Test: copy "hello", open auth modal, dismiss, paste — expect "hello," not the user code.

### Pitfall 8: TOML profile write race vs. file watcher

**What goes wrong:** `vector-config::writer::append_profile` writes to `config.toml`. The Plan 05-04 watcher fires `ConfigReloaded` immediately, which clones the in-memory state and the picker modal — which was holding `Vec<Codespace>` — re-renders with stale data.
**Why it happens:** Hot-reload is wired to every config touch.
**How to avoid:** Profile-save sets a 200ms `suppress_next_reload` flag in `App`; the next `ConfigReloaded` arm checks it and skips the full-state-rebuild path (still applies the new profile to keybinds, etc.). Alternative: write through `toml_edit` then synchronously invoke the in-process apply path bypassing the watcher (cleanest; matches how Plan 05-04 documents the apply pipeline).
**Warning signs:** UI flicker after "Save as profile"; toast appears twice.

### Pitfall 9: Token in `tracing::Span` recorded as field

**What goes wrong:** `tracing::span!("github_request", token = %token).in_scope(...)` — the token value is now in every log line emitted inside the span.
**Why it happens:** Convenience macro.
**How to avoid:** No `tracing` span ever takes a token-bearing variable. Lint via arch-test grep: `tracing::span!.*token` is a build failure.

## Code Examples

Verified patterns from official sources + project conventions. All examples assume the current `vector-secrets` API and the `oauth2 5.0` / `octocrab 0.50` shape.

### Example 1: Device Flow Driver

```rust
// crates/vector-codespaces/src/auth.rs
// Source: https://docs.rs/oauth2/latest/oauth2/ (DeviceAuthorization API)
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, DeviceAuthorizationUrl, Scope, TokenUrl,
};
use zeroize::Zeroizing;

const CLIENT_ID: &str = "Iv1.vector_terminal_app";  // public per RFC 8628; D-89 actual value TBD
const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

pub struct GitHubAuth {
    oauth_client: BasicClient,
    http: reqwest::Client,
}

// Pitfall 14: manual Debug — NEVER derive
impl std::fmt::Debug for GitHubAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitHubAuth").finish_non_exhaustive()
    }
}

pub struct DeviceCodeDisplay {
    pub user_code: String,           // 8 chars, e.g. "WDJB-MJHT" — safe to display
    pub verification_uri: String,    // "https://github.com/login/device"
    pub expires_at: std::time::Instant,
}

impl GitHubAuth {
    pub fn new() -> Result<Self, AuthError> {
        let oauth_client = BasicClient::new(ClientId::new(CLIENT_ID.into()))
            .set_auth_url(AuthUrl::new(DEVICE_CODE_URL.into())?)
            .set_token_url(TokenUrl::new(TOKEN_URL.into())?)
            .set_device_authorization_url(DeviceAuthorizationUrl::new(DEVICE_CODE_URL.into())?);
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())  // RFC 8628: do not follow redirects
            .build()?;
        Ok(Self { oauth_client, http })
    }

    /// Begin device flow. Returns user_code + verification_uri to display in the modal.
    pub async fn request_device_code(&self)
        -> Result<(DeviceCodeDisplay, oauth2::DeviceAuthorizationResponse), AuthError>
    {
        let details = self.oauth_client
            .exchange_device_code()
            .add_scope(Scope::new("codespace".into()))
            .add_scope(Scope::new("read:user".into()))
            .request_async(&self.http)
            .await?;

        let display = DeviceCodeDisplay {
            user_code: details.user_code().secret().clone(),
            verification_uri: details.verification_uri().to_string(),
            expires_at: std::time::Instant::now()
                + std::time::Duration::from_secs(details.expires_in().as_secs()),
        };
        Ok((display, details))
    }

    /// Poll token endpoint until success/failure/timeout.
    /// oauth2 5.0 handles slow_down + interval + authorization_pending internally.
    pub async fn poll_for_token(&self, details: oauth2::DeviceAuthorizationResponse)
        -> Result<Zeroizing<String>, AuthError>
    {
        let token_response = self.oauth_client
            .exchange_device_access_token(&details)
            .request_async(&self.http, tokio::time::sleep, None)
            .await?;
        // wrap immediately — never let the raw String escape
        Ok(Zeroizing::new(token_response.access_token().secret().clone()))
    }
}
```

### Example 2: Keychain Persistence with Manual Debug

```rust
// crates/vector-codespaces/src/auth.rs (continued)
// Source: vector-secrets/src/lib.rs — Pitfall 14 model
use vector_secrets::{Secrets, zeroize::Zeroizing};

pub struct TokenStore {
    secrets: Secrets,
}

impl TokenStore {
    pub fn new() -> Self {
        Self { secrets: Secrets::for_vector() }
    }

    pub fn save_access(&self, token: &Zeroizing<String>) -> Result<(), AuthError> {
        self.secrets.set(Secrets::GITHUB_OAUTH_ACCOUNT, token)?;
        Ok(())
    }

    pub fn load_access(&self) -> Option<Zeroizing<String>> {
        self.secrets.get(Secrets::GITHUB_OAUTH_ACCOUNT)
            .ok()
            .map(Zeroizing::new)
    }

    pub fn clear(&self) -> Result<(), AuthError> {
        let _ = self.secrets.delete(Secrets::GITHUB_OAUTH_ACCOUNT);
        let _ = self.secrets.delete("github_refresh_token");
        Ok(())
    }
}

impl std::fmt::Debug for TokenStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenStore").field("service", &self.secrets.service()).finish_non_exhaustive()
    }
}
```

### Example 3: Octocrab + Bearer + Raw Codespaces GET

```rust
// crates/vector-codespaces/src/client.rs
// Source: https://docs.rs/octocrab/latest/octocrab/struct.OctocrabBuilder.html
use octocrab::Octocrab;
use std::sync::Arc;
use zeroize::Zeroizing;

pub fn build_octocrab(access_token: &Zeroizing<String>) -> Result<Arc<Octocrab>, ClientError> {
    let octo = Octocrab::builder()
        .personal_token(access_token.to_string())  // clones into octocrab's bearer header
        .add_header(http::header::ACCEPT, "application/vnd.github+json".into())
        .add_header(http::header::USER_AGENT, "Vector/0.1".into())
        .build()?;
    Ok(Arc::new(octo))
}

#[derive(serde::Deserialize, Debug)]
pub struct CodespacesPage {
    pub total_count: u32,
    pub codespaces: Vec<crate::model::Codespace>,
}

pub async fn list_codespaces(octo: &Octocrab) -> Result<Vec<crate::model::Codespace>, ClientError> {
    let page: CodespacesPage = octo
        ._get("/user/codespaces?per_page=100")
        .await?
        .json::<CodespacesPage>()
        .await?;
    Ok(page.codespaces)
}
```

### Example 4: Start + 409 Swallow + Poll Loop

```rust
// crates/vector-codespaces/src/client.rs (continued)
use std::time::{Duration, Instant};

pub async fn start_codespace(octo: &Octocrab, name: &str) -> Result<(), ClientError> {
    let path = format!("/user/codespaces/{}/start", urlencoding::encode(name));
    let res: reqwest::Response = octo._post(path, None::<&()>).await?;
    match res.status().as_u16() {
        200 | 202 | 409 => Ok(()),   // 409: already starting; treat as success per D-88
        s => Err(ClientError::StartFailed { status: s }),
    }
}

pub async fn poll_until_available(
    octo: &Octocrab,
    name: &str,
    cancel: tokio_util::sync::CancellationToken,
    on_state: impl Fn(crate::model::CodespaceState),
) -> Result<crate::model::CodespaceState, ClientError> {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => return Err(ClientError::Cancelled),
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
        if Instant::now() >= deadline { return Err(ClientError::PollTimeout); }
        let cs = crate::client::get_codespace(octo, name).await?;
        on_state(cs.state);
        match cs.state {
            crate::model::CodespaceState::Available
            | crate::model::CodespaceState::Failed
            | crate::model::CodespaceState::Shutdown => return Ok(cs.state),
            _ => continue,
        }
    }
}
```

### Example 5: TOML Profile Append via `toml_edit`

```rust
// crates/vector-config/src/writer.rs
// Source: https://docs.rs/toml_edit/latest/toml_edit/ — DocumentMut.insert
use toml_edit::{DocumentMut, table, value};

pub fn append_codespace_profile(
    config_path: &std::path::Path,
    profile_name: &str,
    codespace_name: &str,
    tint: &str,
) -> Result<(), std::io::Error> {
    let source = std::fs::read_to_string(config_path)?;
    let mut doc: DocumentMut = source.parse().expect("config already validated at load time");

    let key = format!("profile.{profile_name}");
    if doc.get(&key).is_some() {
        // de-collide with numeric suffix per D-87
        // ... (left to implementation: try profile_name-2, -3, ...)
    }

    let mut t = table();
    t["kind"] = value("codespace");
    t["codespace_name"] = value(codespace_name);
    t["tint"] = value(tint);

    // toml_edit's dotted-key insert: doc["profile"][profile_name] = t
    let profiles = doc.entry("profile").or_insert(toml_edit::Item::Table(toml_edit::Table::new()));
    if let toml_edit::Item::Table(profiles_tbl) = profiles {
        profiles_tbl.insert(profile_name, t);
    }

    // Atomic write: write to tempfile + rename (Pitfall 1 — vim atomic-rename equivalent)
    let tmp = config_path.with_extension("toml.tmp");
    std::fs::write(&tmp, doc.to_string())?;
    std::fs::rename(&tmp, config_path)?;
    Ok(())
}
```

### Example 6: NSPanel Modal Skeleton (objc2-app-kit)

```rust
// crates/vector-app/src/auth_modal.rs (sketch)
// Source: https://docs.rs/objc2-app-kit/latest/objc2_app_kit/struct.NSPanel.html
use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_app_kit::{NSPanel, NSWindowStyleMask};
use objc2_foundation::NSString;

pub struct AuthModal {
    panel: Retained<NSPanel>,
}

impl AuthModal {
    /// # Safety
    /// Must be called on main thread.
    pub unsafe fn show(mtm: MainThreadMarker, user_code: &str, verification_url: &str) -> Self {
        let style = NSWindowStyleMask::Titled | NSWindowStyleMask::Closable;
        let panel = unsafe {
            // NSPanel::initWithContentRect:styleMask:backing:defer:
            // (build content view: NSTextField with user_code, NSButton "Copy & Open")
            NSPanel::alloc(mtm)  // pseudo — actual init omitted for brevity
        };
        let _ = panel;
        let _ = user_code;
        let _ = verification_url;
        todo!("Plan-stage; see Pattern 1")
    }
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `keyring 4.0` library crate API (`keyring::Entry::new(...)`) | `keyring-core 1.0` library + `apple-native-keyring-store 1.0` backend (current workspace deps); `keyring 4.x` is now the CLI binary, not the library | 2026 split | Already done in `vector-secrets`. CLAUDE.md still says "keyring 4.0" for the library — outdated; trust `Cargo.toml`'s `keyring-core` pin. |
| `octocrab.codespaces()` (typed) | Hand-routed `_get`/`_post` for Codespaces endpoints | Always — octocrab never typed Codespaces (verified docs.rs 2026-05-14) | Plan accordingly; raw routes are easy. |
| GitHub OAuth tokens were long-lived (no expiry) | Tokens from OAuth Apps expire after 8 hours by default (configurable per-app) | Mid-2022 GitHub rollout | AUTH-03 is non-negotiable; cannot ship without re-auth path. |
| Fine-grained PATs for Codespaces | **Broken with Codespaces** — known issue `cli/cli#7819` | Persistent | D-89 mandates classic OAuth scopes only: `codespace`, `read:user`. |

**Deprecated/outdated:**
- `keyring` (top-level v4 library API) — replaced by `keyring-core` + per-platform stores.
- `gh-device-flow` crate — works fine, but Vector's needs are met by `oauth2 5.0` with broader coverage.
- Older `octocrab 0.30s` — pre-`Octocrab::_get` raw-route support.

## Open Questions

1. **Does GitHub's OAuth App for Vector (`vector-terminal`) issue refresh tokens?**
   - What we know: GitHub OAuth Apps registered with "Device Flow enabled" issue access tokens; refresh tokens are issued only if the app is configured for them at registration.
   - What's unclear: Whether the `vector-terminal` registration in D-89 will be configured for refresh tokens, and whether GitHub returns them via the device-flow `access_token` endpoint or only via the standard authorization-code flow.
   - Recommendation: Implement both branches. If a refresh token is present in the device-flow response, store it under `GITHUB_REFRESH_ACCOUNT` and use it on 401 (Pattern 2). If absent, 401 fast-path is "re-run device flow" (Pattern 2 fallback). AUTH-03 passes either way; no UI difference for the user beyond modal cadence.

2. **`Cmd-Shift-G` system-shortcut collision check.**
   - What we know: macOS Find → Find Next is `Cmd-G`. Find Previous is `Cmd-Shift-G` in many apps that implement the Find menu. Plan 05-13 wired `Cmd-F` (`Action::OpenSearch`) but not its `Cmd-Shift-G` "find previous" partner.
   - What's unclear: Whether Vector's search overlay uses `Cmd-Shift-G` for "find previous" today. (Plan 05-13 added F1/N/P/Shift-P/Shift-R; quick check shows search-prev is unwired.)
   - Recommendation: Planner verifies via grep `crates/vector-input/src/keymap.rs` for `"g"`/`"G"`. If unwired, use `Cmd-Shift-G` for `OpenCodespacesPicker` per D-86. If wired, choose an alternative (suggestion: `Cmd-Shift-K` for "Codespaces", or `Cmd-Opt-G`).

3. **Octocrab `_post` body type for empty-body POSTs.**
   - What we know: `Octocrab::_post(path, body)` takes `Option<&impl Serialize>`. `None::<&()>` should serialize to no body.
   - What's unclear: Whether GitHub's `/user/codespaces/{name}/start` returns 200/202 with an empty body OK or requires `Content-Length: 0` explicitly.
   - Recommendation: Smoke-test against a real Codespace early in Phase 6; if octocrab's `_post` omits the body header, fall back to `octocrab.execute(http::Request::post(...).body("").unwrap())` or a direct `reqwest::Client` call inside `client.rs`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| macOS Keychain (`Security.framework`) | `vector-secrets` Keychain writes | Always present on macOS 13+ | system | None needed |
| Internet access to `api.github.com` + `github.com/login/...` | All Phase 6 work | Required at runtime; unavailable in CI without auth | n/a | Tests against octocrab use `wiremock` (add as dev-dep); device-flow tests stub the `reqwest::Client` |
| A real GitHub account with Codespaces | Manual UAT smoke matrix | User has this | n/a | None — Phase 6 cannot ship without it |
| Registered GitHub OAuth App `vector-terminal` | D-89 | **Not yet registered** as of 2026-05-14 | n/a | Use `gh` CLI client ID `178c6fc778ccc68e1d6a` per D-89 fallback; flip to `vector-terminal` client ID when ready |
| `urlencoding` crate (for `/user/codespaces/{name}` path-encoding when names contain slashes — `octocat/hello-world-abc123`) | `vector-codespaces::client` | Not yet a workspace dep | latest | Hand-roll percent-encoding inline (~10 LOC); avoid for clarity, add the dep |
| `tokio_util` (CancellationToken) | Pattern 5 | Workspace transitive via `tonic`/`reqwest` | latest | Already present; add explicit `tokio_util = { version = "0.7", features = ["sync"] }` workspace dep if not surfaced |

**Missing dependencies with no fallback:** None blocking. Registration of `vector-terminal` OAuth App is the only "outside the repo" item and it has a documented fallback (D-89).

**Missing dependencies with fallback:** OAuth App registration → `gh` CLI client ID.

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` + workspace-standard `tokio::test` for async |
| Config file | none (rust-standard); `Cargo.toml` `[dev-dependencies]` per crate |
| Quick run command | `cargo test -p vector-codespaces` |
| Full suite command | `cargo test --workspace --tests` (current baseline ~290/0/0 ignored) |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| AUTH-01 | Device flow obtains user_code + verification_uri | unit (mock HTTP via `wiremock`) | `cargo test -p vector-codespaces device_flow_request_code -x` | ❌ Wave 0 |
| AUTH-01 | Polling handles `authorization_pending` → `success` | unit (wiremock with scripted responses) | `cargo test -p vector-codespaces device_flow_poll_success -x` | ❌ Wave 0 |
| AUTH-01 | Polling handles `slow_down` by bumping interval | unit | `cargo test -p vector-codespaces device_flow_slow_down -x` | ❌ Wave 0 |
| AUTH-01 | `grep -r 'gho_' .` returns zero hits across artifacts + logs | manual smoke | shell script `scripts/audit_token_leak.sh` (Plan-time deliverable) | ❌ Wave 0 |
| AUTH-02 | Token round-trips through Keychain | integration (real Keychain — manual UAT only; CI runner has no keychain) | `cargo test -p vector-codespaces --test keychain_roundtrip -- --ignored` | ❌ Wave 0 |
| AUTH-02 | `security find-generic-password -s vector -a github_oauth_token` returns the entry | manual smoke | shell verification (Plan 06 manual UAT) | ❌ Wave 0 |
| AUTH-02 | Tokens never `Debug`-printed (Pitfall 14 arch-lint) | arch-lint | `cargo test -p vector-arch-tests no_derive_debug_on_token_bearers` | ❌ Wave 0 |
| AUTH-03 | 401 → silent refresh → retry → succeeds | unit (wiremock) | `cargo test -p vector-codespaces auth_401_refresh_retry -x` | ❌ Wave 0 |
| AUTH-03 | 401 → refresh fails → AuthRequired event fired | unit | `cargo test -p vector-codespaces auth_refresh_fail_emits_event -x` | ❌ Wave 0 |
| CS-01 | `list_codespaces` parses real GitHub JSON fixture | unit (fixture from `gh api /user/codespaces > tests/fixtures/list.json`) | `cargo test -p vector-codespaces list_codespaces_fixture -x` | ❌ Wave 0 |
| CS-01 | New `CodespaceState` variant deserializes as `Unrecognized` | unit | `cargo test -p vector-codespaces state_other_variant -x` | ❌ Wave 0 |
| CS-01 | Picker UI shows state + repo + branch + relative time | manual smoke | UAT step in Plan 06 SUMMARY | ❌ Wave 0 |
| CS-02 | `start_codespace` treats 200/202/409 as success | unit (wiremock) | `cargo test -p vector-codespaces start_swallows_409 -x` | ❌ Wave 0 |
| CS-02 | `poll_until_available` stops on `available` | unit (wiremock scripted) | `cargo test -p vector-codespaces poll_terminates_on_available -x` | ❌ Wave 0 |
| CS-02 | `poll_until_available` times out at 120s | unit (tokio::time::pause) | `cargo test -p vector-codespaces poll_times_out -x` | ❌ Wave 0 |
| CS-02 | Manual: starting a real Codespace from picker reaches Available | manual smoke | UAT step | ❌ Wave 0 |
| CS-03 | `append_codespace_profile` writes correct `[profile.X]` block | unit (tempfile) | `cargo test -p vector-config append_codespace_profile -x` | ❌ Wave 0 |
| CS-03 | Profile-name de-collision works | unit | `cargo test -p vector-config profile_name_decollide -x` | ❌ Wave 0 |
| CS-03 | Saved profile survives app restart | manual smoke | UAT step | ❌ Wave 0 |

### Sampling Rate

- **Per task commit:** `cargo test -p vector-codespaces` (target: < 5s with mocked HTTP)
- **Per wave merge:** `cargo test --workspace --tests` (target: < 60s)
- **Phase gate:** Full suite green + manual UAT matrix sign-off (real GitHub account, real Codespaces) before `/gsd:verify-work`

### Wave 0 Gaps

- [ ] `crates/vector-codespaces/Cargo.toml` — add `oauth2 5.0`, `octocrab 0.50`, `serde_json 1`, `chrono 0.4`, `urlencoding 2`, `tokio_util 0.7 sync`, `vector-secrets` path-dep, `wiremock` dev-dep, `tempfile` dev-dep
- [ ] `crates/vector-codespaces/src/{auth,client,model,error}.rs` — new files (currently single `lib.rs` skeleton)
- [ ] `crates/vector-codespaces/tests/{device_flow.rs, codespaces_rest.rs, auth_refresh.rs, keychain_roundtrip.rs}` — Wave-0 stubs `#[ignore]`-gated, un-ignored as plans land
- [ ] `crates/vector-codespaces/tests/fixtures/list.json` — captured `gh api /user/codespaces` payload (sanitized: replace real codespace_name + machine fields with placeholders)
- [ ] `crates/vector-config/src/writer.rs` — new file
- [ ] `crates/vector-config/tests/profile_writer.rs` — Wave-0 stubs
- [ ] `crates/vector-app/src/{auth_modal.rs, codespaces_modal.rs}` — new files
- [ ] `crates/vector-app/src/app.rs` — extend `UserEvent` enum with: `OpenCodespacesPicker`, `AuthRequired`, `AuthDisplayCode { code, url, expires_at }`, `AuthCompleted { user_login }`, `AuthFailed { reason }`, `CodespacesLoaded(Vec<Codespace>)`, `CodespacesLoadFailed(String)`, `CodespaceStateChanged { name, state }`, `SignOut`
- [ ] `crates/vector-arch-tests/tests/no_token_in_debug_or_log.rs` — grep arch-lint for `derive.*Debug` near `token`, `tracing::*.*token`
- [ ] Optional: `scripts/audit_token_leak.sh` — grep `gho_` across `target/`, `~/Library/Logs`, recent terminal scrollback

## Sources

### Primary (HIGH confidence)

- [oauth2 crate docs (latest, 5.0)](https://docs.rs/oauth2/latest/oauth2/) — DeviceAuthorization API surface, RFC 8628 implementation
- [GitHub REST: Codespaces endpoints (apiVersion 2022-11-28)](https://docs.github.com/en/rest/codespaces/codespaces?apiVersion=2022-11-28) — GET /user/codespaces, POST /user/codespaces/{name}/start, GET /user/codespaces/{name}, scopes, state values
- [octocrab docs (latest)](https://docs.rs/octocrab/latest/octocrab/) — confirmed Codespaces is NOT in typed surface; `_get`/`_post` raw routes are public
- [octocrab repo](https://github.com/XAMPPRocky/octocrab) — confirms typed coverage gaps; raw-route usage pattern documented
- `crates/vector-secrets/src/lib.rs` (Vector workspace) — existing Keychain API + Pitfall 14 model
- `crates/vector-config/src/schema.rs` (Vector workspace) — `ProfileBlock` + `Kind::Codespace` + `codespace_name` are pre-existing fields
- `.planning/research/PITFALLS.md` §Pitfall 14 — manual `Debug` discipline canonized at the workspace level
- `.planning/phases/06-github-auth-codespaces-picker/06-CONTEXT.md` — D-84 through D-90 lock the implementation
- CLAUDE.md §"Technology Stack" — versions and rationale pre-verified at project init
- [RFC 8628 (IETF)](https://www.rfc-editor.org/rfc/rfc8628) — OAuth 2.0 Device Authorization Grant

### Secondary (MEDIUM confidence)

- [GitHub OAuth Apps — Device Flow](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps#device-flow) — GitHub-side device flow specifics; refresh-token behavior depends on app config
- [keyring-core 1.0 / apple-native-keyring-store 1.0](https://crates.io/crates/keyring-core) — workspace-pinned library API (replaces top-level `keyring 4.0` library calls)
- [toml_edit](https://docs.rs/toml_edit/latest/toml_edit/) — round-trip TOML editing
- [cli/cli#7819](https://github.com/cli/cli/issues/7819) — fine-grained PATs broken with Codespaces; classic scopes only

### Tertiary (LOW confidence, flagged for validation)

- [knowledgelib.io OAuth2 Device Flow reference](https://knowledgelib.io/software/patterns/oauth2-device-flow/2026) — secondary explainer of RFC 8628; verify any claim against the RFC directly
- Octocrab `_post` empty-body behavior — **untested in this research**; smoke-test against a real Codespace start endpoint during Plan 06 Wave 0

## Metadata

**Confidence breakdown:**
- Standard stack (versions, choices): HIGH — every crate is named in CLAUDE.md with current-version verification; `vector-secrets` already proves the keyring side.
- Architecture (state machines, threading): HIGH — direct application of patterns established in Plans 02-05 / 03-04 / 05-04 / 05-09; nothing novel.
- Pitfalls: HIGH for #1, #2, #4, #5, #6 (well-documented in OAuth / GitHub literature); MEDIUM for #3 (NSPanel modal quirks — depends on macOS version, mitigation is conservative); HIGH for #7, #8, #9 (project-specific patterns from prior phases).
- Open Questions: MEDIUM — three open items, all with clear "decide at implementation time" fallbacks. None are blockers for planning.

**Research date:** 2026-05-14
**Valid until:** 2026-06-14 (30 days — OAuth and GitHub REST are stable; re-verify crate versions if Phase 6 slips past mid-June).
