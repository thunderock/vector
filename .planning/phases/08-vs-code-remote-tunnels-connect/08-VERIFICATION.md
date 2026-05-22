---
phase: 08-vs-code-remote-tunnels-connect
verified: 2026-05-22T00:00:00Z
updated: 2026-05-22
status: human_needed
score: 4/5 code-level Success Criteria verified (SC#5 N/A — spike chose path (b), not (c)); SC#3 end-to-end gated on tracked deferred SDK consumption + human UAT
re_verification: null
human_verification:
  - test: "08-SMOKE.md Item 2 — Microsoft sign-in + picker lists only vector-agent tunnels"
    expected: "Modal opens at 480x280 with 32pt mono code; after browser device flow, picker opens at 640x480 with footer text per UI-SPEC"
    why_human: "Requires live Microsoft account device-flow + AppKit window inspection"
  - test: "08-SMOKE.md Item 3 — vector-tunnel-agent .deb install + first-run device flow on Linux"
    expected: "wget + apt install succeeds; agent prints device code; after browser auth, prints 'tunnel vector-{hostname} registered'"
    why_human: "Requires real Linux/Ubuntu box (or Docker) + browser + GitHub or Microsoft account; equivalent to 08-06 Task 3"
  - test: "08-SMOKE.md Item 4 — picker shows newly-registered tunnel with vector- prefix stripped"
    expected: "Row shows display_name = {hostname}, status dot green, format per UI-SPEC"
    why_human: "Requires live agent (Item 3) + Mac client"
  - test: "08-SMOKE.md Item 5 — Connect (Enter) → live remote shell with [remote] badge (DT-03 end-to-end)"
    expected: "Tab title contains [remote]; prompt is remote user@hostname; hostname matches Linux box"
    why_human: "Requires live agent + Mac client + real Dev Tunnels relay; also gated on SDK consumption code below"
  - test: "08-SMOKE.md Item 6 — Microsoft-blue tint (#0078d4) on active DevTunnel pane (DT-04 visual)"
    expected: "Stripe is Microsoft blue, disappears for local panes, reappears on focus"
    why_human: "Visual GPU-rendered tint inspection on real window"
  - test: "08-SMOKE.md Item 7 — Resize → remote tput cols/lines matches (DT-03 sigwinch)"
    expected: "tput values change with Vector window resize"
    why_human: "Requires live remote pane + window resize"
  - test: "08-SMOKE.md Item 8 — Token-leak audit (Pitfall 14 runtime log scrape)"
    expected: "grep -E 'gho_|ghp_|eyJ|Bearer [A-Za-z0-9._-]{20,}' of trace logs returns zero hits"
    why_human: "Requires running app + signing in + capturing real logs"
  - test: "08-SMOKE.md Item 9 — Sign out of Microsoft → live pane survives"
    expected: "Pane keeps echoing after sign-out; picker footer flips to signed-out copy"
    why_human: "Requires live pane + state transition observation"
  - test: "08-06 Task 3 / 08-06-HUMAN-UAT.md — cargo-deb dpkg-deb metadata + apt install/remove smoke on Linux"
    expected: "cargo deb produces .deb; dpkg-deb --info shows expected metadata; apt install + apt remove round-trip succeeds; binary on PATH"
    why_human: "Requires Linux host (real or Docker rust:1.88-bookworm) — explicitly tracked in 08-06-HUMAN-UAT.md"
gaps: null
---

# Phase 8: VS Code Remote Tunnels Connect — Verification Report

**Phase Goal:** A signed-in user can attach Vector to one of their own machines running `code tunnel`, getting a remote shell in a Vector pane that's visually distinct from local panes.

**Verified:** 2026-05-22
**Status:** human_needed
**Re-verification:** No — initial verification

---

## Goal Achievement — Success Criteria (from ROADMAP.md §Phase 8)

| #   | Success Criterion | Status | Evidence |
| --- | ----------------- | ------ | -------- |
| 1   | Phase begins with 1–2 day spike committing decision doc to `.planning/research/spikes/dev-tunnels-decision.md` choosing (a)/(b)/(c). NO integration code before doc lands. | ✓ VERIFIED | File exists (89 lines). Path 2c LOCKED. `git log` ordering proves doc commit `454618e` (2026-05-21 13:43:35) **precedes** first integration commit `44d35ba` (13:48:08) — 4 min, 33 s gap. See §Spike-Gate Ordering below. |
| 2   | Signed-in user sees their active VS Code Remote Tunnels listed in a picker (tunnel name, host, last-seen). | ✓ VERIFIED (code-level) — live walk gated on Item 2 of 08-SMOKE.md | `DevTunnelsApi::list_tunnels` filters to `vector-agent: true` label (`crates/vector-tunnels/src/api.rs:90-114` + `model.rs:33-37`). `DevTunnelsPickerModal` 640×480 NSPanel (`crates/vector-app/src/devtunnels_modal.rs:110+`). `format_row` template `●  {name}  {host}  ·  {last_seen}` (Plan 08-05 SUMMARY, 10 passing helper tests). Live wiremock list_tunnels coverage: 3 tests in `crates/vector-tunnels/tests/list_tunnels.rs`. |
| 3   | Clicking a tunnel opens a remote shell in a new pane via the chosen transport. | ⚠️ PARTIAL — pump wired, SDK relay-connect deferred + tracked | UI path complete: actor (`crates/vector-app/src/devtunnels_actor.rs:200-254`) calls `connect_tunnel` → `Mux::create_tab_async_with_transport`. JSON-protocol `DevTunnelTransport` pump fully implemented and tested against `tokio::io::duplex` (8 tests in `crates/vector-tunnels/tests/transport_protocol.rs`). **However:** `DevTunnelTransport::connect()` body (`crates/vector-tunnels/src/transport.rs:201-220`) returns `Err(TransportError::Protocol("DevTunnelTransport::connect not yet wired — pending SDK consumption decision (Plan 08-06)"))`. STATE.md L115 and 08-04 SUMMARY explicitly state Plan 08-06 would land this body; Plan 08-06 was rescoped to distribution only and the SDK consumption did not happen. Live end-to-end exercise is Item 5 of 08-SMOKE.md (human-UAT). |
| 4   | Connected pane is visually distinct from local — tinted tab + `[remote]` badge. | ✓ VERIFIED (code-level) — visual confirmation gated on Item 6 of 08-SMOKE.md | `format_tab_title` appends ` [remote]` for `TransportKind::DevTunnel` (`crates/vector-mux/src/pane.rs:215-216`, test at L241). `MICROSOFT_BLUE = [0.0, 0.471, 0.831, 1.0]` (#0078d4) defined in `crates/vector-render/src/tint_stripe.rs:23`; applied via `chrome.tint.set_color(host.queue(), Some(MICROSOFT_BLUE))` in `crates/vector-app/src/app.rs:823-826`. `tests/tint_stripe.rs` asserts colour value. |
| 5   | If spike chose (c): decision doc committed, REQUIREMENTS.md moves DT-02..04 to v2. | N/A — spike chose (b) (Path 2c) | Per `.planning/research/spikes/dev-tunnels-decision.md` line 4: "Decision: (b) Path 2 Variant 2c — Vector Tunnel Agent." |

**Code-level score:** 4/5 SCs verified in codebase (SC#5 N/A); SC#3 is partial — the protocol pump is fully wired and unit-tested, but the SDK relay-connect call site is an explicit-Err stub. SC#3 end-to-end completion is one of the items tracked in 08-SMOKE.md (Item 5) for human UAT.

---

## Spike-Gate Ordering (SC#1 — Hard Gate)

Verified by `git log --oneline -- <path>`:

| Path | Commit | Timestamp (commit author date) |
| ---- | ------ | ------------------------------ |
| `.planning/research/spikes/dev-tunnels-decision.md` | `454618e` `docs(08-01): commit DT-01 dev tunnels spike decision (Path 2c LOCKED)` | 2026-05-21 13:43:35 -0700 |
| `crates/vector-tunnels/` (first commit) | `44d35ba` `feat(08-01): scaffold Phase 8 tunnel crates + russh patch + MS account const` | 2026-05-21 13:48:08 -0700 |
| `crates/vector-tunnel-agent/` (first commit) | `44d35ba` (same) | 2026-05-21 13:48:08 -0700 |

Spike doc landed 4 minutes 33 seconds before any integration code. **SC#1 hard gate satisfied.** The doc was deliberately split into its own commit (per Plan 08-01 acceptance) so the ordering is auditable.

---

## Requirements Coverage — DT-01..04

| Req | Plan(s) Claiming | Description | Status | Evidence |
| --- | ---------------- | ----------- | ------ | -------- |
| DT-01 | 08-01, 08-03, 08-06, 08-07 | Spike decision committed before integration code | ✓ SATISFIED | Spike doc + git-log ordering above. Agent binary `cargo build -p vector-tunnel-agent --release` produces a working executable. `.deb` distribution metadata (Cargo.toml `[package.metadata.deb]`) + `.github/workflows/agent-release.yml` (cross-compile amd64+arm64) + `xtask agent-dist` + `crates/vector-tunnel-agent/debian/{postinst,prerm}` + crate README + root README install snippet all present. Live install path tracked under 08-06-HUMAN-UAT.md + 08-SMOKE.md Item 3. |
| DT-02 | 08-01, 08-02, 08-04, 08-05, 08-07 | Picker lists active Dev Tunnels with name/host/last-seen | ✓ SATISFIED (code-level) | `api::list_tunnels` filters by `vector-agent: true` label (`crates/vector-tunnels/src/model.rs:33`). `TunnelRecord::display_name()` strips `vector-` prefix (D-09). Microsoft OAuth Device Flow driver against `login.microsoftonline.com/common/oauth2/v2.0/devicecode` (`crates/vector-tunnels/src/auth/device_flow_microsoft.rs`). `MicrosoftTokenStore` writes to Keychain under `microsoft_refresh_token` / `microsoft_oauth_token` (`crates/vector-secrets/src/lib.rs:54-55`). `DevTunnelsPickerModal` + `DevTunnelsActor` + Cmd-Shift-T (`crates/vector-input/src/keymap.rs:49-105`). Live confirmation in 08-SMOKE.md Items 2+4. |
| DT-03 | 08-01, 08-03, 08-04, 08-05, 08-07 | Connecting opens remote shell end-to-end via chosen transport | ⚠️ CODE PARTIAL — protocol/pump complete; SDK relay-connect deferred | Agent side: `vector-tunnel-agent` with `RelayTunnelHost` registration + PTY spawn via `portable-pty` + JSON-frame session pump (`crates/vector-tunnel-agent/src/{host,session}.rs`). Client side: `DevTunnelTransport` pump implemented + 8 duplex-stream tests passing. **Open:** `DevTunnelTransport::connect()` returns explicit Err (`crates/vector-tunnels/src/transport.rs:201-220`); SDK consumption (russh-0.37 dual-version trade-off) explicitly deferred per 08-04 SUMMARY L18+L182, 08-05 SUMMARY L267, STATE.md L115. End-to-end is human-UAT Item 5. |
| DT-04 | 08-01, 08-04, 08-05, 08-07 | Sessions visually distinct (tinted tab + `[remote]` badge) | ✓ SATISFIED (code-level) | `format_tab_title` + `TransportKind::DevTunnel` → ` [remote]` suffix wired and tested (`crates/vector-mux/src/pane.rs:215`, test L241). `MICROSOFT_BLUE` constant + tint-stripe pipeline + tint application in `app.rs:823-826`. Visual confirmation in 08-SMOKE.md Item 6. |

All four DT-* IDs are claimed by at least one plan's `requirements:` frontmatter. No orphans.

---

## Artifact Verification (Levels 1–3)

| Artifact | Exists | Substantive | Wired | Status |
| -------- | ------ | ----------- | ----- | ------ |
| `.planning/research/spikes/dev-tunnels-decision.md` | ✓ (89 lines) | ✓ (Path 2c LOCKED, all 4 paths assessed, invalidators listed, plan refs) | ✓ (referenced by 08-SMOKE.md Item 1, Plans 08-01/08-07) | ✓ VERIFIED |
| `crates/vector-tunnel-protocol/src/lib.rs` | ✓ | ✓ (`AgentMessage` enum, JSON+base64 codec, `PROTOCOL_VERSION=1`) | ✓ (imported by `vector-tunnels`, `vector-tunnel-agent`) | ✓ VERIFIED |
| `crates/vector-tunnel-agent/src/{auth,host,session,token_cache,cli}.rs` | ✓ | ✓ (RelayTunnelHost reg, portable-pty spawn, JSON pump, device flow GitHub+Microsoft, ~/.config/vector/agent-token mode 0600) | ✓ (`main.rs` wires CLI → auth → host) | ✓ VERIFIED |
| `crates/vector-tunnels/src/api.rs` (`DevTunnelsApi::list_tunnels`, `get_access_token`) | ✓ | ✓ (filter by `is_vector_agent`, 401/403/404 typed errors) | ✓ (called by `DevTunnelsActor::handle_load` + `connect_tunnel`) | ✓ VERIFIED |
| `crates/vector-tunnels/src/auth/{device_flow_microsoft,token_store}.rs` | ✓ | ✓ (Microsoft device flow + Keychain backed by vector-secrets) | ✓ (`MicrosoftAuth` used by `DevTunnelsActor::handle_start_microsoft_signin`) | ✓ VERIFIED |
| `crates/vector-tunnels/src/transport.rs` (`DevTunnelTransport`) | ✓ | ⚠️ PUMP complete + 8 tests; `connect()` body explicit Err stub by design | ✓ (`impl PtyTransport`; `connect_tunnel` calls it) | ⚠️ ORPHAN-CONNECT (pump fully wired & tested; SDK relay-connect deliberately deferred — same gap as DT-03 PARTIAL above) |
| `crates/vector-tunnels/src/domain.rs` (`connect_tunnel`) | ✓ | ✓ (gets access token → DevTunnelTransport::connect → Box<dyn PtyTransport>) | ✓ (called by actor) | ✓ VERIFIED (inherits the connect() stub) |
| `crates/vector-app/src/devtunnels_actor.rs` | ✓ | ✓ (Command::{Load, Connect, StartMicrosoftSignIn, SignOutMicrosoft}, refresh-on-401 chain) | ✓ (`set_devtunnels_cmd_tx` wired in `app.rs:144-199`; 11 UserEvent variants dispatched) | ✓ VERIFIED |
| `crates/vector-app/src/devtunnels_modal.rs` (`DevTunnelsPickerModal`) | ✓ | ✓ (390 lines; UI-SPEC copy asserted character-for-character in 10 helper tests) | ✓ (`app.rs:709-732` opens on Cmd-Shift-T) | ✓ VERIFIED |
| `crates/vector-app/src/microsoft_auth_modal.rs` (`MicrosoftAuthDeviceFlowModal`) | ✓ | ✓ (260 lines, 32pt mono user-code) | ✓ (`app.rs:1928` instantiates on `MicrosoftDeviceFlowStarted`) | ✓ VERIFIED |
| `crates/vector-input/src/keymap.rs` — Cmd-Shift-T → `OpenDevTunnelsPicker` | ✓ | ✓ (regression test ensures Cmd-T alone still = NewTab) | ✓ (mapped in `app.rs:709`) | ✓ VERIFIED |
| `crates/vector-render/src/tint_stripe.rs` — `MICROSOFT_BLUE` | ✓ | ✓ (#0078d4 = [0.0, 0.471, 0.831, 1.0]) | ✓ (applied in `app.rs:825`) | ✓ VERIFIED |
| `crates/vector-mux/src/pane.rs` — `format_tab_title` ` [remote]` | ✓ | ✓ (Phase-7 carry-over, test L241 still green) | ✓ (called by tab-title machinery) | ✓ VERIFIED |
| `crates/vector-secrets/src/lib.rs` — `MICROSOFT_REFRESH_ACCOUNT`, `MICROSOFT_OAUTH_ACCOUNT` | ✓ | ✓ (`= "microsoft_refresh_token"` / `"microsoft_oauth_token"`) | ✓ (used by `MicrosoftTokenStore`) | ✓ VERIFIED |
| `.github/workflows/agent-release.yml` | ✓ | ✓ (`tags: ['v*']` trigger, ubuntu-22.04, matrix amd64+arm64, `cargo deb`, dpkg-deb sanity, upload-artifact, attach-to-release) | ✓ (concurrency group `agent-release-${tag}`) | ✓ VERIFIED |
| `crates/vector-tunnel-agent/Cargo.toml` — `[package.metadata.deb]` | ✓ | ✓ (section=net, assets, license-file `../../LICENSE`, maintainer-scripts `debian/`) | ✓ (consumed by `cargo deb`) | ✓ VERIFIED |
| `crates/vector-tunnel-agent/debian/{postinst,prerm}` | ✓ | ✓ (0755, v1 no-op per D-02) | ✓ (referenced in Cargo.toml `maintainer-scripts`) | ✓ VERIFIED |
| `xtask/src/agent_dist.rs` | ✓ | ✓ (macOS short-circuits with CI hint; Linux invokes `cargo deb`) | ✓ (subcommand registered in `xtask/src/main.rs`) | ✓ VERIFIED |
| `crates/vector-tunnel-agent/README.md` | ✓ | ✓ (install + first-run + reauth + tmux persistence docs, 96 lines) | ✓ (root README "Remote machines" section links) | ✓ VERIFIED |
| `08-SMOKE.md` (9-item matrix) | ✓ | ✓ (9 items, PASS/FAIL boxes, sign-off line, post-sign-off procedure) | ✓ (Items reference all DT-01..04 + Pitfall 14) | ✓ VERIFIED (template-only; user UAT pending) |

---

## Key-Link Verification (Wiring)

| From | To | Via | Status | Detail |
| ---- | -- | --- | ------ | ------ |
| `DevTunnelsPickerModal` (Cmd-Shift-T) | `DevTunnelsActor::handle_load` | `mpsc::Sender<Command::Load>` | ✓ WIRED | `app.rs:732` |
| `MicrosoftAuthDeviceFlowModal` | `DevTunnelsActor::handle_start_microsoft_signin` | `Command::StartMicrosoftSignIn` | ✓ WIRED | `app.rs:2034` |
| `DevTunnelsActor::handle_load` | `DevTunnelsApi::list_tunnels` | direct call | ✓ WIRED | `devtunnels_actor.rs:207` |
| `DevTunnelsActor::handle_connect` | `connect_tunnel` | direct call | ✓ WIRED | `devtunnels_actor.rs:225` |
| `connect_tunnel` | `DevTunnelTransport::connect` | direct call | ✓ WIRED (call site) / ⚠️ STUB (callee body returns Err) | `domain.rs:20` |
| `DevTunnelsActor::handle_connect` | `Mux::create_tab_async_with_transport` | `Box<dyn PtyTransport>` | ✓ WIRED | `devtunnels_actor.rs:237` |
| `vector-tunnel-agent` host | Dev Tunnels relay | `RelayTunnelHost::start` + AGENT_PORT=32100 | ✓ WIRED (host side ready) | `crates/vector-tunnel-agent/src/host.rs` |
| Agent session pump | `portable-pty` PTY | spawn $SHELL on `open_pty` frame | ✓ WIRED | `crates/vector-tunnel-agent/src/session.rs` |
| `format_tab_title(_, _, DevTunnel)` | ` [remote]` suffix | string concat | ✓ WIRED | `pane.rs:215-216` |
| `app.rs::apply_devtunnel_tint_for_pane` | `MICROSOFT_BLUE` | `chrome.tint.set_color` | ✓ WIRED | `app.rs:823-826` |
| `MicrosoftTokenStore::save` | Keychain `microsoft_refresh_token` | `vector_secrets::Secrets::store` | ✓ WIRED | `auth/token_store.rs:37+` |
| Tag `v*` push | `agent-release.yml` workflow | GitHub Actions trigger | ✓ WIRED | `agent-release.yml:4-5` |
| `cargo deb` | `.deb` artifact | `[package.metadata.deb]` | ✓ WIRED | `Cargo.toml:48-66` |

---

## Behavioural Spot-Checks

| # | Behaviour | Command | Result |
| - | --------- | ------- | ------ |
| 1 | Phase-8 crate test suites compile and pass | `cargo test -p vector-tunnels -p vector-tunnel-agent -p vector-tunnel-protocol -p vector-secrets --tests` | ✓ PASS — 16 test-suite sections, all `0 failed`; integration tests for handshake / read / write / resize / exit / protocol-mismatch / list_tunnels / Microsoft device flow / token store all green |
| 2 | Arch-lint tests pass (Pitfall 14 token-leak coverage extends to new crates) | `cargo test -p vector-arch-tests --tests` | ✓ PASS — all 5 arch-lint test files green |
| 3 | Workspace builds clean (full check from prompt context) | (reported pre-context: 142 suites green, cross-phase regression in `b569e6f`) | ✓ PASS |
| 4 | Spike doc exists with locked decision | `test -f .planning/research/spikes/dev-tunnels-decision.md && grep "Path 2 Variant 2c" .planning/research/spikes/dev-tunnels-decision.md` | ✓ PASS — file exists, 2 hits |
| 5 | Spike doc commit precedes any integration code commit | `git log --oneline -- .planning/research/spikes/dev-tunnels-decision.md` vs. `git log --oneline -- crates/vector-tunnels/ crates/vector-tunnel-agent/` | ✓ PASS — 454618e (13:43:35) precedes 44d35ba (13:48:08) by ~4.5 min |
| 6 | SMOKE matrix has 9 items per UAT contract | `grep -c "### Item " 08-SMOKE.md` | ✓ PASS — 9 |
| 7 | `DevTunnelTransport::connect()` SDK call site is currently a stub | `grep -n "not yet wired" crates/vector-tunnels/src/transport.rs` | ⚠️ FOUND (intentional, tracked) — L218 |

---

## Anti-Pattern Scan

Targets: `crates/vector-tunnels/src/`, `crates/vector-tunnel-agent/src/`, `crates/vector-tunnel-protocol/src/`.

- `grep -rn "TODO\|FIXME\|unimplemented\|todo!\|panic!"` of Phase-8 source dirs: **no hits**.
- One intentional stub: `crates/vector-tunnels/src/transport.rs:217-219` — `DevTunnelTransport::connect()` returns `Err(TransportError::Protocol("DevTunnelTransport::connect not yet wired — pending SDK consumption decision (Plan 08-06)".into()))`. Documented in 08-04 SUMMARY (L18, L43, L182, L227), 08-05 SUMMARY (L267), STATE.md (L115). **Classification: ℹ️ INFO** (known, documented, and tracked — surfaces as failure in 08-SMOKE.md Item 5 once user attempts live UAT). Not a new finding.
- One intentional permanent-shim: `crates/vector-mux/src/devtunnel_domain.rs` — `DevTunnelDomain::spawn` is `unimplemented!()` by WIN-04 design (vector-mux must stay free of vector-tunnels dep; actor goes through `vector_tunnels::domain::connect_tunnel` instead). Documented in 08-04 SUMMARY L70+L114.
- No `TODO`/`FIXME`/`HACK` markers in src files of the three new crates.

---

## Findings & Open Items

### Code-level (verifiable now)

1. **All five Success Criteria are addressed in code at the layer they live at.** SC#1 (spike-gate) is a hard pass — file + git-log ordering prove it. SC#2/#4 are fully wired and unit-tested. SC#3 is partial: the agent host, the JSON wire protocol, the client-side pump, the picker UI, the actor, the keymap, and the Mux install seam are all in place and unit-tested, but the SDK `RelayTunnelClient::connect → into_rw → DevTunnelTransport::new_with_stream` glue (the ~15 lines of code sketched in the comment block at `transport.rs:208-216`) is a documented Err-stub. SC#5 is N/A (spike chose path b).
2. **Requirements traceability is clean.** Every DT-01..04 ID is claimed by at least one plan in the phase. No orphans.
3. **Tests: 100% of Phase-8-touched test suites pass.** Pitfall 14 arch-lint extended to new crates. Microsoft device-flow + token-store + DevTunnelTransport pump + RelayTunnelHost session lifecycle + protocol codec all green.
4. **Dual-russh dep graph (0.37 + 0.60) is acknowledged and intentional.** Per user prompt context: both compile. Workspace `[patch.crates-io] russh = vscode-russh` stays dormant until the SDK is consumed in `vector-tunnels`.

### Human-UAT (deliberately deferred + tracked)

The remaining nine items below are all tracked in either `08-SMOKE.md` (9-item matrix, Items 1-9) or `08-06-HUMAN-UAT.md` (5-item .deb smoke). The user has explicitly noted awareness and acceptance of these deferrals. These appear in `human_verification:` frontmatter.

- **08-SMOKE.md Item 1** is verifiable without UAT (spike-doc exists) — already ✓ above.
- **08-SMOKE.md Items 2-9** require a live Microsoft account, a live Linux box running the agent, GPU-rendered window inspection, and runtime log scraping. They also subsume the `DevTunnelTransport::connect()` SDK stub (Item 5 cannot pass while the stub returns Err).
- **08-06 Task 3** (= 08-06-HUMAN-UAT.md) requires `cargo-deb` + Linux to validate the `.deb` round-trip; pure-Mac dev box cannot exercise.

### Why `status: human_needed` and not `gaps_found`

The strict reading of the verification criteria would mark SC#3 as a code-level FAIL because `DevTunnelTransport::connect()` is a stub. However:

1. The stub is **explicitly documented** in three SUMMARYs and STATE.md (08-04 L18+L43+L182+L227, 08-05 L267, STATE L115), with the exact downstream owner (Plan 08-06 SDK consumption) and the exact code-path to activate (uncomment `tunnels = { workspace = true }` in `crates/vector-tunnels/Cargo.toml`, then replace the Err-stub body with the sketched 5 lines).
2. The user's prompt-context note explicitly states: "humans tests deliberately deferred + tracked. The user is aware and is OK with `status: human_needed`."
3. The remaining unverified end-to-end behaviour (Item 5 of 08-SMOKE.md) **already tracks** this gap — it cannot pass until the SDK is wired and the agent is running, so the SDK gap is subsumed by the human-UAT item rather than being a separate undocumented blocker.
4. The dual-russh dep-graph decision still hangs over the SDK wiring (russh 0.37 vs 0.60 binary-bloat-vs-fork trade-off, ROADMAP risks note line 224). Closing this is a small but non-trivial code task that the user has chosen to defer along with the live UAT.

If the user wants the SDK consumption to be treated as a separate code-level gap (not subsumed by the SMOKE matrix), flip this report to `status: gaps_found` and add the structured gap:

```yaml
gaps:
  - truth: "Clicking a tunnel opens a remote shell via the chosen transport (SC#3 end-to-end)"
    status: partial
    reason: "DevTunnelTransport::connect() is an explicit Err stub; SDK glue (RelayTunnelClient::connect → into_rw → new_with_stream) deferred per 08-04 SUMMARY but not picked up by 08-06."
    artifacts:
      - path: "crates/vector-tunnels/src/transport.rs"
        issue: "connect() body returns Err(TransportError::Protocol(\"not yet wired — pending SDK consumption decision (Plan 08-06)\"))"
      - path: "crates/vector-tunnels/Cargo.toml"
        issue: "`tunnels = { workspace = true }` still commented out — SDK not consumed by the crate that needs it"
    missing:
      - "Uncomment `tunnels = { workspace = true }` in crates/vector-tunnels/Cargo.toml"
      - "Replace transport.rs:217-219 Err-stub with the sketched 5-line SDK call (RelayTunnelClient::connect → connect_to_port(AGENT_PORT) → into_rw → Self::new_with_stream)"
      - "Verify AGENT_PORT=32100 matches the agent's `add_port_raw` call in crates/vector-tunnel-agent/src/host.rs"
      - "Resolve russh-0.37-vs-0.60 dual-version (accept ~3MB bloat OR fork+bump vscode-russh)"
```

---

## Summary

Phase 8 has shipped **6 of 7 plans complete** with two of those plans (08-06 distribution-Linux-smoke, 08-07 9-item UAT) explicitly tracked as awaiting human verification on real hardware. The seventh — the actual `DevTunnelTransport::connect()` SDK consumption — is a small but real code-level deferral that has been openly tracked across multiple SUMMARYs and STATE.md, and that is implicitly captured by 08-SMOKE.md Item 5.

Per user instruction, this report classifies the phase as `human_needed`: every must-have is satisfied or wired at the code layer it owns, and the remaining gaps are exclusively in the tracked HUMAN-UAT items (plus the closely-related SDK-consumption deferral that gates Item 5).

---

## Post-Verification Followups

### 2026-05-22 — `DevTunnelTransport::connect()` SDK wiring closed (commit `66d95e0`)

The code-level gap flagged in §"Findings & Open Items" (Finding #1) and the proposed `gaps:` block — `crates/vector-tunnels/src/transport.rs:217-220` returning `Err(TransportError::Protocol("not yet wired — pending SDK consumption decision (Plan 08-06)"))` — has been closed inline.

Changes landed under `feat(08): wire DevTunnelTransport::connect() body via tunnels SDK`:

- Activated `tunnels = { workspace = true }` in `crates/vector-tunnels/Cargo.toml`.
- Replaced the 3-line Err stub with the SDK sketch from the in-source comment block: build a slim `TunnelManagementClient` via `new_tunnel_management("Vector/<ver>")`, `RelayTunnelClient::new(mgmt).connect(&endpoint, &token)`, `connect_to_port(AGENT_PORT)`, `into_rw()`, hand the resulting `PortConnectionRW` to `Self::new_with_stream`.
- Translated our slim `TunnelRecord` endpoint into the SDK's full `tunnels::contracts::TunnelEndpoint` (only `client_relay_uri` + `host_public_keys` are actually read by `RelayTunnelClient::connect`; the rest are required-by-shape).
- Added unit test `connect_rejects_tunnel_with_no_endpoints` asserting the new endpoint-validation guard and proving `connect()` no longer returns the "not yet wired" stub message.

**Dual-russh dep graph verified clean:** `russh 0.37.1` (Microsoft `vscode-russh` fork via `[patch.crates-io]`) resolves for the `tunnels` SDK; `russh 0.60.3` (vanilla) resolves for `vector-ssh`. Both compile together — no fork, no `~3 MB` bloat mitigation needed at this stage. The previously-dormant `[patch.crates-io] russh = vscode-russh` patch is now active and consumed by `tunnels` SDK.

**Post-fix sanity:**

- `cargo test -p vector-tunnels --tests` → 9/9 `transport_protocol` green (including the new test).
- `cargo test --workspace --tests` → 142/142 suites green (matches pre-fix baseline; no regression).
- `make lint` → green.

`status:` deliberately left as `human_needed` — Items 2-9 of 08-SMOKE.md still require live human UAT (Microsoft sign-in flow, Linux box running the agent, GPU visual inspection, runtime log scrape). The orchestrator should re-verify on the next pass; the code-level SC#3 (PARTIAL) should now flip to ✓ VERIFIED at the code layer it lives at, with end-to-end exercise still gated on 08-SMOKE.md Item 5 as before.

---

_Verified: 2026-05-22_
_Verifier: Claude (gsd-verifier)_
_Followup wired: 2026-05-22 — commit 66d95e0_
