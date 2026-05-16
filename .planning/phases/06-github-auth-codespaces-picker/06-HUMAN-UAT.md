---
status: partial
phase: 06-github-auth-codespaces-picker
source: [06-VERIFICATION.md, 06-07-PLAN.md]
started: 2026-05-14T22:12:24Z
updated: 2026-05-14T22:12:24Z
---

## Current Test

[awaiting human testing — drive via `/gsd:verify-work 6` when ready]

Prerequisites before starting:

```bash
security delete-generic-password -s vector -a github_oauth_token 2>/dev/null
security delete-generic-password -s vector -a github_refresh_token 2>/dev/null
cargo build -p vector-app --release && ./target/release/vector-app
```

## Tests

### 1. Sign in with GitHub (AUTH-01)
expected: AuthDeviceFlowModal opens 440x280 floating, 8-char user-code in large mono font, countdown ticks, `pbpaste` returns user-code, primary opens github.com/device in Safari, after auth the modal auto-dismisses within ~5s, toast `signed in as @{login}` appears, menu flips to `Sign out (@{login})`, `pbpaste` returns the PRE-modal clipboard (Pitfall 7 restore).
result: [pending]

### 2. Token persisted in Keychain (AUTH-02)
expected: `security find-generic-password -s vector -a github_oauth_token -w` returns a `gho_*` or `ghu_*` token. `grep -r 'gho_' ~/Library/Logs/` returns 0. `grep -r 'gho_' ./target/` returns 0.
result: [pending]

### 3. Token survives app restart (AUTH-02)
expected: After Cmd-Q and relaunch, `Vector` menu shows `Sign out (@{login})` on launch with no re-auth prompt.
result: [pending]

### 4. Sign out clears Keychain
expected: Click `Sign out` → toast `signed out` → menu reverts to `Sign in with GitHub`. `security find-generic-password -s vector -a github_oauth_token -w` returns `... could not be found.`
result: [pending]

### 5. Codespaces picker via menu + keyboard (CS-01)
expected: Cmd-Shift-G opens 640px NSPanel titled `Codespaces` with brief `loading codespaces…` then rows showing state / repo / branch / last-used (e.g. `2 hours ago`). Footer reads `{N} codespaces · last refreshed just now`. Re-open via `Vector → Codespaces…` menu identical.
result: [pending]

### 6. Connect button placeholder toast (CS-04 deferred to Phase 7)
expected: Arrow to Available row + Enter → toast `codespace ssh transport not yet wired — phase 7`. Modal stays open.
result: [pending]

### 7. Start a Shutdown codespace (CS-02)
expected: Select Shutdown row + Enter → toast `starting codespace…` → state label flips to `Starting` within ~5s → eventually flips to `Available` within 2min.
result: [pending]

### 8. 409 swallow when codespace already starting (Pitfall 5)
expected: Click `Start` while codespace is Starting → NO `could not start` toast; polling continues uninterrupted.
result: [pending]

### 9. Save as profile (CS-03)
expected: Cmd-S on selected row → toast `profile saved as "{derived-name}"`. `~/.config/vector/config.toml` contains `[profile.{derived-name}]` with `kind = "codespace"`, `codespace_name = "..."`, `tint = "#7a3aaf"`. Pre-existing blocks/comments preserved. Cmd-Shift-P picker shows the new profile not dimmed.
result: [pending]

### 10. Saved profile survives app restart (CS-03)
expected: Cmd-Q + relaunch + Cmd-Shift-P → saved profile still visible.
result: [pending]

### 11. 401 silent refresh / re-auth (AUTH-03)
expected: Delete only the access token from Keychain. Cmd-Shift-G → either picker loads silently (refresh-token chain succeeded) OR AuthDeviceFlowModal opens automatically. NO empty list with no explanation. NO silent failure.
result: [pending]

## Summary

total: 11
passed: 0
issues: 0
pending: 11
skipped: 0
blocked: 0

## Gaps
