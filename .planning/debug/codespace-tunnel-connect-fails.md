---
status: awaiting_human_verify
trigger: "codespace-tunnel-connect-fails: After GitHub OAuth completes, user cannot connect to a Codespace tunnel — stays on local shell"
created: 2026-05-18T00:00:00Z
updated: 2026-05-18T00:00:00Z
---

## Current Focus

hypothesis: Two distinct problems —
  (1) UX: post-auth handler does NOT auto-open the Codespaces picker; user is left at local shell unaware Cmd-Shift-G is available.
  (2) Scope: `codespaces_connect_selected` is an explicit stub that toasts "codespace ssh transport not yet wired — phase 7"; `vector-ssh` and `vector-tunnels` are placeholder lib.rs files; `CodespaceDomain::spawn` is `unimplemented!("Phase 7")`.
test: Trace AuthCompleted → picker → connect; read vector-ssh / vector-tunnels / CodespaceDomain stubs; cross-check ROADMAP + STATE.
expecting: Confirm (1) is a real wiring gap fixable today and (2) is intentionally deferred to Phase 7 / Phase 8.
next_action: Report scope honestly and offer in-scope Phase-6 fix for (1); recommend opening Phase 7 plan for (2).

## Symptoms

expected:
  1. After auth succeeds, app shows Codespace picker
  2. User picks Codespace → tunnel connects → remote shell in terminal

actual:
  1. Auth completes but user remains in local zsh
  2. No Codespace picker / tunnel fails silently

errors: (none captured — silent failure; toast text "codespace ssh transport not yet wired — phase 7" is emitted if user does reach Connect)
reproduction: Sign in to GitHub → auth succeeds → try to open Codespace
started: After OAuth device-flow fix (Phase 4 active dev)

## Eliminated

- hypothesis: "auth never actually completes / token not stored"
  evidence: `handle_auth_completed` dismisses the modal, rebuilds menu with Sign-out, shows "signed in as @{user}" toast; `handle_open_codespaces_picker` later loads the access token from Keychain via `build_client_from_keychain` without complaint; the user reports auth "succeeds". Token storage path works.
  timestamp: 2026-05-18

- hypothesis: "codespace listing is missing"
  evidence: `vector-codespaces::CodespacesClient::list` + `list_with_refresh` are fully implemented against `/user/codespaces?per_page=100`; `codespaces_actor::spawn_fetch_codespaces` spawns the call; `handle_codespaces_loaded` populates the picker. CS-01/02/03 in PROJECT.md noted "code-complete 2026-05-14".
  timestamp: 2026-05-18

- hypothesis: "picker UI not implemented"
  evidence: `crates/vector-app/src/codespaces_modal.rs::CodespacesPickerModal` is a full NSPanel (640x480), shows rows with state + repo + branch + last-used, handles selection / filtering / load/error states.
  timestamp: 2026-05-18

## Evidence

- timestamp: 2026-05-18
  checked: crates/vector-app/src/app.rs::handle_auth_completed (lines 360–372)
  found: After AuthCompleted, the handler dismisses the auth modal, rebuilds the menu, shows a toast, and calls request_redraw_all(). It does NOT emit `UserEvent::OpenCodespacesPicker` or otherwise advance the user to the next step.
  implication: User stays staring at the local shell with only a toast acknowledging sign-in. There is no signal that the next action is Cmd-Shift-G / `Vector → Codespaces…`. This matches the screenshot exactly: local zsh, no picker visible.

- timestamp: 2026-05-18
  checked: crates/vector-app/src/app.rs::codespaces_connect_selected (lines 468–480)
  found: Body is a pure stub: `self.toasts.show(ToastBanner::info("codespace ssh transport not yet wired — phase 7"));`. No SSH, no tunnel, no terminal pane swap, no PTY replacement.
  implication: Even if the user discovers Cmd-Shift-G, opens the picker, selects a Codespace, and presses Enter — they get a toast telling them this is Phase 7 work. The "tunnel connect fails" symptom is fundamentally an unimplemented-feature symptom, not a regression.

- timestamp: 2026-05-18
  checked: crates/vector-ssh/src/lib.rs and crates/vector-tunnels/src/lib.rs
  found: Both crates contain only their module doc-comment. `vector-ssh`: "Generic async SSH client. Filled in Phase 7 atop russh." `vector-tunnels`: "Microsoft Dev Tunnels client. Filled in Phase 8 atop microsoft/dev-tunnels."
  implication: The transport layer required for a real connect is literally not written yet.

- timestamp: 2026-05-18
  checked: crates/vector-mux/src/codespace_domain.rs
  found: `CodespaceDomain::spawn` is `unimplemented!("Phase 7")`. The mux trait surface (`PtyTransport`, `Domain`) is in place, but the codespace implementation is not.
  implication: Even with vector-ssh and vector-tunnels filled in, there's no glue yet that swaps a pane's local PTY for a remote SSH channel — the `Domain` impl is a stub.

- timestamp: 2026-05-18
  checked: .planning/PROJECT.md, .planning/ROADMAP.md, .planning/STATE.md, .planning/REQUIREMENTS.md
  found:
    - PROJECT.md line 24: "`Connect` placeholder toast points at Phase 7. Connect/transport stays in Phase 7 (Dev Tunnels + gRPC + russh)."
    - PROJECT.md line 103: "Phase 7 (Dev Tunnels + gRPC SSH transport via russh) is next."
    - ROADMAP.md line 20: "Phase 7: SSH Transport + Codespaces Connect — `gh codespace ssh --stdio` subprocess transport, `CodespaceDomain`, end-to-end remote shell with tab tint and resize."
    - ROADMAP.md line 21: "Phase 8: Dev Tunnels Integration — Day-1 spike resolves the subprocess/vendor/defer decision tree."
    - STATE.md line 111: Phase 8 is spike-gated; defer-to-v2 is acceptable.
    - REQUIREMENTS.md lines 189–196: CS-04..07 and DT-01..04 all "Phase 7"/"Phase 8" / "Pending".
  implication: The transport gap is a known, planned, intentionally-deferred Phase 7/8 scope. It is NOT a defect introduced by recent OAuth fixes; it is the next phase of work.

- timestamp: 2026-05-18
  checked: crates/vector-app/src/app.rs lines 669–672, crates/vector-app/src/menu.rs lines 417–426
  found: `AppShortcut::OpenCodespacesPicker` (Cmd-Shift-G) and the `Vector → Codespaces…` menu item BOTH dispatch `UserEvent::OpenCodespacesPicker` and `handle_open_codespaces_picker` works correctly. The picker IS reachable post-auth; the user just isn't told how.
  implication: We can close the post-auth UX gap by either (a) auto-opening the picker on AuthCompleted, or (b) changing the success toast to mention Cmd-Shift-G. (a) matches user expectation per the symptoms ("After auth succeeds (token stored), the app shows a Codespace picker") and is one extra line of code.

## Resolution

root_cause: |
  Two-part diagnosis:

  (1) UX gap (in-scope fix today): `handle_auth_completed` does not transition the
      user into the Codespaces picker. It just dismisses the auth modal and
      shows a "signed in" toast. Users have no signal that the next step is
      Cmd-Shift-G or `Vector → Codespaces…`, so they sit in the local shell —
      which is exactly what the screenshot shows. Auto-opening the picker on
      AuthCompleted closes this gap with one line and matches the spec
      ("After auth succeeds, the app shows a Codespace picker listing the
      user's available Codespaces").

  (2) Transport gap (Phase 7/8 work — NOT a debug-session fix):
      `codespaces_connect_selected` is an explicit stub. `vector-ssh` and
      `vector-tunnels` are empty. `CodespaceDomain::spawn` is
      `unimplemented!("Phase 7")`. PROJECT.md / ROADMAP.md / REQUIREMENTS.md /
      STATE.md all confirm SSH transport + Dev Tunnels are deferred to
      Phase 7 and Phase 8 respectively, with Phase 8 explicitly spike-gated.
      Implementing real connect inside a Phase 4 debug session would bypass
      the GSD workflow, the Phase 7 plan, and the Phase 8 spike decision —
      and would conflict with whatever transport choice that spike makes
      (subprocess `gh codespace ssh --stdio` vs vendored russh+gRPC).

fix: |
  In-scope (Phase 6 UX polish, ~1 line + tweak toast wording):

  In `crates/vector-app/src/app.rs::handle_auth_completed` (after the toast,
  before request_redraw_all), emit `UserEvent::OpenCodespacesPicker` so the
  picker auto-opens on a successful sign-in. Update the success toast copy to
  reflect that the picker is now visible (e.g. "signed in as @{user} — pick a
  Codespace below"). Phase-6 spec already calls for this; the wiring just
  wasn't added.

  Out-of-scope (Phase 7 work):
  - Fill in `vector-ssh` with russh client OR a thin `gh codespace ssh --stdio`
    subprocess wrapper (Phase 7 spike decides which — ROADMAP picks subprocess).
  - Implement `CodespaceDomain::spawn` to return a `PtyTransport` whose reader
    is the SSH channel's stdout and whose writer feeds the channel's stdin.
  - Replace the `codespaces_connect_selected` stub with: dismiss picker → ask
    mux to spawn a new pane (or replace the current pane's transport) using
    `CodespaceDomain` keyed on the selected codespace name → forward
    SIGWINCH-equivalent resize events through the channel.
  - Phase 8 Dev Tunnels spike then decides whether to wire `DevTunnelDomain`
    on top of `microsoft/dev-tunnels/rs/` or defer.

verification: |
  In-scope UX fix applied:
    - `cargo build -p vector-app` — clean.
    - `cargo clippy -p vector-app --all-targets -- -D warnings` — clean.
    - `cargo fmt -p vector-app -- --check` — clean.
    - `cargo test -p vector-app --tests` — all suites pass (zero failures across
      ~20 test binaries; no regressions).

  Manual verification still required (UAT, not automatable from headless tools):
    1. Launch app, click `Vector → Sign in with GitHub`.
    2. Complete device-flow code in browser.
    3. After AuthCompleted, the Codespaces picker NSPanel auto-appears with
       a "loading codespaces…" footer that resolves to the user's list.
    4. Toast reads "signed in as @{user} — pick a Codespace".

  The transport gap (codespaces_connect_selected stub) is NOT fixed; that is
  Phase 7 work and requires a fresh plan + new commits.

files_changed:
  - crates/vector-app/src/app.rs  # handle_auth_completed: emit OpenCodespacesPicker + tweak toast copy
