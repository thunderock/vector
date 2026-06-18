# Design: GitHub-auth Dev Tunnels (app-side auth swap)

**Date:** 2026-05-28
**Status:** Approved (design) — pending implementation planning
**Trigger:** Phase 9 UAT Test 3 hit `AADSTS500011` — the Dev Tunnels resource principal
`46da2f7e-b5ef-422a-9a4e-fb5e1cb7da14` is not consented in the Adobe Azure tenant, so a
corporate `@adobe.com` Microsoft sign-in cannot complete. Microsoft Entra auth is a dead
end for the project's primary audience (the author + Adobe teammates).

## Problem

The Mac app authenticates to the Dev Tunnels service using a **Microsoft Entra** device-code
flow against `login.microsoftonline.com/common`. With an `@adobe.com` account, Azure routes the
token request to Adobe's tenant, which has not consented to the Dev Tunnels app → `AADSTS500011`.
The tunnel relay, transport, and remote agent are all working; only the **identity provider** is
the blocker.

## Key finding: Dev Tunnels natively accepts GitHub identities

Verified in the vendored `microsoft/dev-tunnels` Rust SDK (rev `64048c1`) and the local codebase:

- SDK `Authorization` enum has a `Github(String)` variant → emits header `github <token>`
  (`rs/src/management/authorization.rs:10-32`), alongside the `AAD` variant we use today.
- SDK ships **Dev Tunnels GitHub App** client IDs per environment
  (`rs/src/contracts/tunnel_service_properties.rs`): `PROD_GITHUB_APP_CLIENT_ID = "Iv1.e7b89e013f801f03"`.
  This is the app `devtunnel user login -g` uses.
- SDK constants `TUNNEL_AUTHENTICATION_SCHEMES_GITHUB = "github"` and `PROVIDERS_GITHUB = "github"`.
- Vector's own `AuthProvider` enum **already** has `GitHub(String)` →
  `format!("github {t}")` (`crates/vector-tunnels/src/model.rs:69-92`), and the Management API
  client already sends `auth.format_header()` (`crates/vector-tunnels/src/api.rs:90-143`).
  **The API layer is already provider-agnostic.**
- The remote agent (`crates/vector-tunnel-agent`) **already** has a working GitHub device-flow
  (`auth.rs:97-117`), already prompts GitHub-or-Microsoft on first run, and **defaults to GitHub**
  (`auth.rs:84-95`). It dispatches `Authorization::Github` vs `Authorization::AAD` from the cached
  provider (`host.rs:123-133`).

GitHub auth bypasses the Adobe Azure tenant entirely — no Entra consent required.

## Scope

**App-side only.** Relay, transport, Management API client, and agent are untouched.

### What changes (`crates/vector-tunnels` + `crates/vector-app`)

1. **New** `crates/vector-tunnels/src/auth/device_flow_github.rs` — mirror of the existing
   `device_flow_microsoft.rs`, driving GitHub's RFC 8628 device flow against the **Dev Tunnels
   GitHub App** client ID (`Iv1.e7b89e013f801f03`), not the `gh` CLI client ID.
2. **Token store** — persist/read the `github_refresh_token` keychain slot
   (`vector-secrets::GITHUB_REFRESH_ACCOUNT` already exists). Mirror `MicrosoftTokenStore`.
3. **Remove** the app-side Microsoft device-flow driver + `MicrosoftTokenStore` usage. GitHub
   becomes the **only** app sign-in path.
4. **UI swap** — `devtunnels_modal` sign-in button, the three menu items
   (`install_microsoft_menu_items` → GitHub equivalent), and footer copy flip from
   "Microsoft" back to "GitHub." (Reverts the Phase 9.1 cosmetic direction.)
5. **Agent** — no change *expected*, but see contingency below. Stays dual-provider,
   GitHub-default. (A non-Adobe Microsoft account could still host if explicitly chosen.)

   **Contingency:** The agent authenticates its own host registration with the **`gh` CLI**
   client ID (`auth.rs:13`, scope `read:user`). If the spike (below) shows the Dev Tunnels relay
   rejects `gh`-CLI-issued GitHub tokens and requires its own GitHub App, then the **agent's**
   GitHub client ID must move to `Iv1.e7b89e013f801f03` as well — otherwise GitHub host
   registration was silently relying on never being tested against the real service. This makes
   the spike doubly important: it validates the token path for **both** ends. Treat the agent
   client-ID bump as a conditional task gated on the spike result.

### Out of scope

- Agent code changes — *except* the conditional client-ID bump described above if the spike requires it.
- Relay/transport/Management API changes.
- Re-introducing GitHub Codespaces or the deleted `vector-codespaces` crate.
- Refresh-token rotation differences beyond what GitHub's device flow returns (handle in driver).

## The one real risk — which GitHub client ID

The agent today uses the **`gh` CLI** client ID (`178c6fc778ccc68e1d6a`, scope `read:user`).
Dev Tunnels may only accept GitHub tokens **issued by its own registered GitHub App**
(`Iv1.e7b89e013f801f03`). If so, a `gh`-CLI token would authenticate to GitHub but be **rejected
by the Dev Tunnels relay**. This end-to-end path has likely never been exercised (the project was
on the Microsoft path during Phase 7–9).

**Mitigation — spike first.** Before full implementation, prove that a device flow against the
Dev Tunnels GitHub App client ID (a) completes (the app has device flow enabled), and (b) yields a
token that the Management API accepts (`list_tunnels` returns 200, not 401). This de-risks the
entire phase. If the Dev Tunnels GitHub App does **not** support device flow, fall back options
(evaluate in the spike): GitHub App user-to-server token via a minimal local loopback OAuth
redirect, or escalate.

## Data flow (after change)

```
User clicks "Sign in with GitHub" (modal button or menu item)
  → device_flow_github: POST github.com/login/device/code (client_id = Iv1.e7b89e013f801f03)
  → user enters code at github.com/login/device
  → poll github.com/login/oauth/access_token → access_token (+ refresh if provided)
  → GitHubTokenStore::save → Keychain "github_refresh_token"
  → AuthProvider::GitHub(access_token)
  → api.list_tunnels(auth)         [Authorization: github <token>]
  → api.get_access_token(auth, id) [Authorization: github <token>] → tunnel-scoped token
  → transport.connect(tunnel, tunnel_token) [Authorization: tunnel <token>]  (unchanged)
```

## Testing

- Unit tests for `device_flow_github` mirroring the Microsoft driver's tests (request shaping,
  poll/backoff, expiry math).
- Token-store round-trip test for the `github_refresh_token` slot.
- Arch-lint (Pitfall-14): the new driver must keep manual `Debug` on token-bearing structs and
  no `tracing` of token material. Extend SCAN_PATHS to the new file.
- **Live smoke (the real gate):** GitHub sign-in → picker lists tunnels → connect to an
  agent-hosted tunnel → type in the remote pane. This doubles as resuming Phase 9 UAT Test 3+.

## GSD placement

Slot as **Phase 9.2** (decimal, before Phase 10). Phase 10's held `10-04` release tag depends on
a working sign-in path, and Phase 9 UAT (Tests 3–9) cannot complete without it. Suggested plan
shape:
- Plan 09.2-01 — **Spike:** device flow against Dev Tunnels GitHub App ID; prove relay acceptance.
- Plan 09.2-02 — GitHub device-flow driver + token store (TDD).
- Plan 09.2-03 — Remove app-side Microsoft auth; wire GitHub into modal button + menu + copy.
- Plan 09.2-04 — Live UAT smoke (or fold into resumed Phase 9 UAT).

## Decisions

- **D-1:** GitHub is the **only** app sign-in path; app-side Microsoft auth is removed. (User: "remove it.")
- **D-2:** Use the **Dev Tunnels GitHub App** client ID, not the `gh` CLI ID.
- **D-3:** Spike the client-ID acceptance question before building the full flow.
- **D-4:** Agent unchanged by default (dual-provider, GitHub-default) — *unless* the spike shows
  the relay requires the Dev Tunnels GitHub App token, in which case the agent's GitHub client ID
  also moves to `Iv1.e7b89e013f801f03`.
- **D-5:** No return to Codespaces; keep the Phase 7/8 relay + agent architecture intact.
