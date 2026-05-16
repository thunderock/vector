# Phase 6: GitHub Auth + Codespaces Picker - Context

**Gathered:** 2026-05-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 6 delivers: GitHub OAuth Device Flow sign-in, token storage in macOS Keychain, a dedicated Codespaces picker modal with live state/status, start-on-demand for Shutdown codespaces, and profile save to `config.toml`. **No SSH transport** — clicking "Connect" on a saved profile shows a placeholder toast (Phase 7 wires it).

In-scope capabilities:
- AUTH-01: GitHub OAuth Device Flow (RFC 8628) from inside the app — `Vector → Sign in with GitHub` menu item + auto-prompt when clicking a codespace profile with no valid token
- AUTH-02: Token storage in macOS Keychain via `keyring 4.0` — never written to disk in plaintext, never logged
- AUTH-03: Silent token refresh on 401; expired tokens trigger re-auth prompt rather than silent failure
- CS-01: Codespaces picker modal listing all codespaces with state, repo, branch, last-used time
- CS-02: Start a Shutdown codespace via `POST /user/codespaces/{name}/start`, swallow 409, poll 1s up to 2 min
- CS-03: Save a picked codespace as a `[profile.X]` block in `config.toml` (kind = "codespace" schema from D-74)

Explicit non-goals:
- No SSH/tunnel transport (Phase 7)
- No codespace lifecycle management (create/delete/rebuild) — out of scope per PROJECT.md
- No Dev Tunnels picker (Phase 8)
- No PORTS panel (v2)

</domain>

<decisions>
## Implementation Decisions

### Sign-in trigger (AUTH-01)

- **D-84:** **`Vector → Sign in with GitHub` menu item is the primary entry point.** A second trigger fires automatically when the user clicks a codespace profile in either the existing Cmd-Shift-P profile picker or the new Codespaces picker modal, and no valid token is present. Both paths invoke the same OAuth device flow code path.

### Device flow presentation (AUTH-01)

- **D-85:** **OAuth device flow is presented as a modal window overlay (NSPanel-style).** The modal:
  - Displays the 8-char user-code prominently (large monospace type)
  - Shows the URL `github.com/device` for manual entry
  - Auto-copies the user-code to clipboard on open
  - Has a single primary button: "Copy code and open github.com/device" (`NSWorkspace.openURL`)
  - Has a "Cancel" action
  - Stays on top of the Vector window until authentication completes or user cancels
  - On success: dismisses itself and fires a toast ("Signed in as @username")
  - On cancel: dismisses without error (user can re-trigger via menu or profile click)
  - Pitfall 14 applies: the full token is **never** displayed in the modal or any tooltip

### Codespaces picker access point (CS-01)

- **D-86:** **Dedicated Codespaces picker modal — separate from the existing Cmd-Shift-P profile picker (D-75).** Triggered by:
  - Menu item: `Vector → Codespaces...`
  - Keyboard shortcut: `Cmd-Shift-G` (mnemonic: GitHub; avoids collision with existing Cmd-Shift-P, Cmd-Shift-D, Cmd-Shift-[], Cmd-Shift-C)
  - The existing Cmd-Shift-P picker continues to show all saved profiles (including `codespace` kind), but does not show the live-fetched Codespaces list. The dedicated modal is the place for live state, start-on-demand, and save-as-profile.
  - **Modal columns:** state badge (color-coded: green = Available, yellow = Starting, gray = Shutdown), repo name, branch, last-used timestamp (relative: "2 hours ago")
  - **Actions per row:** "Connect" (shows toast if no transport yet, Phase 7), "Save as profile" (writes `[profile.X]` to config.toml), "Start" (visible only for Shutdown codespaces)

### Profile save (CS-03)

- **D-87:** **Saving a Codespace as a one-click profile writes a `[profile.X]` block directly to `~/.config/vector/config.toml`.** Uses the existing D-74 schema exactly:
  ```toml
  [profile.octocat-hello-world]
  kind = "codespace"
  codespace_name = "octocat/hello-world-abc123"
  tint = "#7a3aaf"
  ```
  Profile name is derived from `codespace_name` by stripping the owner prefix and randomized suffix (e.g. `octocat/hello-world-abc123` → `hello-world`), de-colliding with a numeric suffix if needed. User can rename it manually in the TOML after save. The `tint` value defaults to a fixed purple (#7a3aaf, distinct from local default which has no stripe) unless the user has a per-profile preference from a prior save.

### State refresh strategy (CS-01 / CS-02)

- **D-88:** **On-demand fetch + active transition poll.** Behavior:
  - Full list fetched from `GET /user/codespaces` each time the picker modal is opened (one network call per open, shows a spinner during fetch)
  - A manual "Refresh" icon button in the picker header triggers a re-fetch on demand
  - While a codespace is in `starting` state (user clicked Start OR codespace was already Starting at open time): poll `GET /user/codespaces/{name}` at 1s interval, update the row's state badge live, stop polling when state becomes `available` or `stopped` / on timeout (2 min per ROADMAP)
  - No background polling when the picker is closed — conserves API quota and avoids rate-limiting issues with the GitHub REST API
  - On 401 during any API call: silently trigger token refresh (AUTH-03 path); re-run the original request once; if still 401, show re-auth prompt

### OAuth client registration

- **D-89:** **Register a dedicated GitHub OAuth App for Vector (`vector-terminal`).** Do NOT reuse the `gh` CLI client ID (`178c6fc778ccc68e1d6a`). Reasons: separate rate-limit quota, correct app name shown in the user's GitHub OAuth apps list, branding. Scopes: `codespace read:user`. Redirect URI is not used for device flow. The client ID is embedded as a build constant (not a secret — device flow client IDs are public by spec). ROADMAP notes `178c6fc778ccc68e1d6a` as reusable as a fallback if the custom app registration isn't ready at planning time.

### Token storage shape

- **D-90:** **Token storage via `vector-secrets::Secrets::for_vector()` with two accounts:**
  - `GITHUB_OAUTH_ACCOUNT` (already defined as `"github_oauth_token"`) — stores the access token
  - Add `GITHUB_REFRESH_ACCOUNT = "github_refresh_token"` — stores the refresh token (if GitHub provides one; device flow may return only an access token with no refresh; in that case re-run device flow on expiry, which is AUTH-03)
  - Both fields use `zeroize::Zeroizing<String>` wrappers in memory per `vector-secrets`'s exported `use zeroize;`
  - Manual `Debug` impl on every struct that holds a token — never derive (Pitfall 14)

### Claude's Discretion

The following are downstream-agent calls — researcher/planner pick the best approach without re-asking the user:

- **`oauth2 5.0` vs. raw `reqwest` for device flow** — Researcher evaluates; `oauth2 5.0` device flow is the recommended default per CLAUDE.md stack docs.
- **`octocrab 0.50` client setup** — Single shared `Arc<Octocrab>` with the bearer token injected via `OctocrabBuilder::personal_token`. Recreated on token refresh.
- **Picker modal rendering** — Whether to implement via AppKit `NSPanel` (`objc2-app-kit`) or render in-process via the existing wgpu compositor. Researcher's call; NSPanel is likely faster to ship.
- **`Cmd-Shift-G` conflict check** — Planner should verify no existing system-level shortcut conflicts on macOS (it's unused in standard AppKit).
- **Error display in picker modal** — Network errors during fetch show an inline error state in the list area ("Could not fetch codespaces — check your connection [Retry]"), not a separate dialog.
- **`GET /user/codespaces` pagination** — GitHub paginates at 30 per page; use `per_page=100` to reduce round trips. v1 cap at 100 codespaces (more than enough for personal use).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Roadmap + requirements
- `.planning/ROADMAP.md` §"Phase 6: GitHub Auth + Codespaces Picker" — goal, success criteria, stack additions, risks/notes, out-of-scope re-check
- `.planning/REQUIREMENTS.md` AUTH-01, AUTH-02, AUTH-03, CS-01, CS-02, CS-03 — checkbox requirements + traceability

### Prior phase decisions (load-bearing)
- `.planning/phases/05-polish-local-daily-driver/05-CONTEXT.md` D-74 — Profile schema (`ProfileBlock`, `Kind::Codespace`, `codespace_name` field) — Phase 6 writes `kind = "codespace"` profiles using this exact schema
- `.planning/phases/05-polish-local-daily-driver/05-CONTEXT.md` D-75 — Cmd-Shift-P profile picker exists; Phase 6 does NOT replace it — the dedicated Codespaces modal (D-86) is additive
- `.planning/phases/05-polish-local-daily-driver/05-CONTEXT.md` D-69 — Toast surface (top-of-window banner) — reused for "Signed in as @username" and "Connecting..." feedback
- `.planning/phases/01-foundation-ci-dmg-pipeline/01-CONTEXT.md` D-33 (ADR practice) — new OAuth client decision should be recorded as an ADR
- `.planning/phases/02-headless-terminal-core/02-CONTEXT.md` D-38 (`PtyTransport`/`Domain` trait) — Phase 6's `Connect` button stub must return a `not yet implemented` toast without crashing the domain abstraction

### Research / pitfalls
- `.planning/research/PITFALLS.md` §Pitfall 14 — Manual `Debug` on every token-bearing struct; never derive; applies to all new auth types in this phase
- `.planning/research/PITFALLS.md` §Security (OAuth scope row) — classic scopes only (`codespace`, `read:user`); fine-grained PATs are broken with Codespaces (cli/cli#7819 — do not use)

### Existing crate API surfaces
- `crates/vector-secrets/src/lib.rs` — `Secrets::for_vector()`, `get`, `set`, `delete`, `GITHUB_OAUTH_ACCOUNT` constant, `zeroize` re-export — Phase 6 is the first writer
- `crates/vector-config/src/schema.rs` — `ProfileBlock`, `Kind::Codespace`, `codespace_name: Option<String>` — D-87 writes profiles using these types
- `crates/vector-codespaces/src/lib.rs` — skeleton; Phase 6 fills in the Codespaces REST client and OAuth flow
- `crates/vector-ui/src/lib.rs` — skeleton; Phase 6 adds the Codespaces picker modal and OAuth device-flow overlay
- `crates/vector-config/src/loader.rs` — how `config.toml` is loaded and written; D-87 needs a `write_profile` API here or adjacent

### External specifications
- GitHub OAuth Device Flow: RFC 8628 (IETF) — canonical spec for the device authorization grant
- `oauth2 5.0` crate docs (docs.rs/oauth2) — device flow implementation reference
- `octocrab 0.50` crate docs — `GET /user/codespaces`, `POST /user/codespaces/{name}/start`, `GET /user/codespaces/{name}` endpoints
- GitHub REST API: `/user/codespaces` — list, state values (`available`, `starting`, `stopped`, `shutdown`, etc.), `POST /start`, 409 Conflict behavior
- `keyring 4.0` crate docs — already wired in `vector-secrets`; no new integration needed

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`vector-secrets::Secrets::for_vector()`** (`crates/vector-secrets/src/lib.rs`) — complete Keychain API; Phase 6 is the first caller of `set`. `GITHUB_OAUTH_ACCOUNT = "github_oauth_token"` constant already defined. `zeroize` re-exported.
- **`vector-config::schema::ProfileBlock`** with `kind: Option<Kind>`, `codespace_name: Option<String>`, `tint: Option<String>` — D-87's profile-save path writes directly to this schema.
- **Toast surface** (D-69, Phase 5) — existing in-window non-modal banner; reuse for sign-in success ("Signed in as @username"), connect stub toast, and error states.
- **Cmd-Shift-P profile picker** (D-75, Phase 5, `crates/vector-ui`) — NOT replaced; the Codespaces modal is a separate entry point. Both coexist.
- **AppKit menu bar** (D-15 + Phase 5) — `Vector` menu already wired; D-84's sign-in item and D-86's "Codespaces..." item extend it.
- **`tracing` infrastructure** (D-32) — use for OAuth flow logging (never log token material per Pitfall 14; log flow state transitions instead: `device_flow_initiated`, `device_flow_polling`, `device_flow_complete`).

### Established Patterns
- **Pitfall 14 manual Debug** — every new struct holding an OAuth token, refresh token, or API key gets a hand-written `Debug` impl that shows only `service` and `account` fields (model: existing `Secrets` impl in `vector-secrets/src/lib.rs`)
- **`zeroize::Zeroizing<String>` for in-memory tokens** — wrap all raw token strings in this type at acquisition point; drops and zeroes on scope exit
- **Threading** — `octocrab` calls are async tokio; all UI updates route back to AppKit main thread via `EventLoopProxy::send_event` (existing D-09 pattern)
- **`deny_unknown_fields`** on all serde types (D-68) — new `Codespace` API response structs should use `#[serde(deny_unknown_fields)]` or at minimum document what fields are captured vs. ignored

### Integration Points
- **`crates/vector-codespaces`** — fills in Phase 6 with: `GitHubAuth` (device flow + token refresh), `CodespacesClient` (wraps `octocrab`), `CodespaceList` / `Codespace` response types
- **`crates/vector-ui`** — fills in Phase 6 with: `DeviceFlowModal` (NSPanel overlay), `CodespacesPickerModal` (dedicated list modal)
- **`crates/vector-config/src/loader.rs`** — needs a `write_profile(name, ProfileBlock)` API for D-87's profile save; planner must add this
- **AppKit menu bar** — add "Sign in with GitHub" and "Codespaces..." items to `Vector` menu

</code_context>

<specifics>
## Specific Ideas

- **Auth modal stays on top** (D-85) — User explicitly chose NSPanel-style modal that doesn't dismiss until auth is complete or explicitly cancelled. This is the right UX because the OAuth flow requires the user to switch to a browser and come back — a transient toast would be confusing.
- **Separate picker from profile switcher** (D-86) — User chose a dedicated "Codespaces" modal rather than extending Cmd-Shift-P. This is deliberate: the profile picker is for quick switching between saved profiles; the Codespaces modal is a discovery/management surface for live API data. Different affordances.
- **On-demand fetch, not background poll** (D-88) — Deliberately conservative. GitHub REST API rate limits at 5000 req/hour for authenticated users; background polling at 30s across all open app instances would eat into this. On-demand keeps the app well within limits.
- **Register a fresh OAuth App** (D-89) — Named `vector-terminal`; separate rate limit bucket from `gh` CLI.
- **Classic OAuth scopes only** — fine-grained PATs explicitly broken with Codespaces per cli/cli#7819.

</specifics>

<deferred>
## Deferred Ideas

- **GitHub status page integration** — showing GitHub.com incident status in the picker when Codespaces are failing — noted, not Phase 6
- **Codespace creation from picker** — out of scope per PROJECT.md (connect-only for v1)
- **Multi-account GitHub support** — single account only for v1; second GitHub account could be a v2 feature
- **Codespace rebuild / delete from picker** — out of scope per PROJECT.md

### Reviewed Todos (not folded)

None — no pending todos matched Phase 6 scope.

</deferred>

---

*Phase: 06-github-auth-codespaces-picker*
*Context gathered: 2026-05-14*
