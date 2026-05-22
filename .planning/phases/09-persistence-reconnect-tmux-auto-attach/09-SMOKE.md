# Phase 9 SMOKE — PERSIST-04 + reconnect end-to-end

**User:** astiwari
**Date:** {YYYY-MM-DD — user fills}
**Tunnel id:** {redacted-on-publish}
**Remote OS:** Ubuntu 22.04 (or your equivalent)
**Remote tmux:** 3.4+ (`tmux -V` must report ≥ 3.4)
**Remote agent:** `vector-tunnel-agent` running (Phase 8 install procedure)

## USER-RUN setup (REQUIRED before any test below)

CONTEXT D-04/D-05: Vector does not detect, create, attach, share, or name tmux sessions. The user must prepare tmux on the remote BEFORE running any smoke test.

On the remote box (separate SSH session, NOT via Vector), run:

```
tmux new -s smoke
tmux set-option -g allow-passthrough on
```

Leave that tmux session attached. The smoke tests connect a new shell into the same tunnel; the OSC 52 round-trip test pre-checks `$TMUX` and aborts with a clear error if you skip this setup.

## Automated portion (PERSIST-04)

Set env vars locally:
```
export VECTOR_E2E_TUNNEL_ID=<your-tunnel-id>
export VECTOR_E2E_MICROSOFT_TOKEN=<from-keychain>
```
Run:
```
cargo test -p vector-tunnels --test live_devtunnel_smoke -- --ignored --test-threads=1 --nocapture
```

| Test | Result |
|------|--------|
| osc52_round_trip                  | ⬜ pass / ❌ fail / 📝 notes |
| decscusr_and_mouse_modes          | ⬜ pass / ❌ fail / 📝 notes |
| term_xterm_256color_advertised    | ⬜ pass / ❌ fail / 📝 notes |

## Manual portion (full reconnect UX, end-to-end)

Connect to the tunnel via Vector UI (Cmd-Shift-T → pick tunnel). The user's tmux session from "USER-RUN setup" above is already running on the remote.

| # | Step | Expected | Actual |
|---|------|----------|--------|
| 1 | Inside the user's tmux session on the remote, run `vim hello.txt` | vim opens, cursor visible | ⬜ |
| 2 | Type some text, save with `:wq` | file written | ⬜ |
| 3 | Run `htop` inside the same tmux session | htop renders correctly w/ colors + ProMotion-smooth scrolling | ⬜ |
| 4 | OSC 52 inside tmux: `printf '\e]52;c;%s\a' "$(echo hello | base64)"` | macOS clipboard contains `hello` | ⬜ |
| 5 | OSC 52 large payload: `printf '\e]52;c;%s\a' "$(head -c 200 /dev/urandom | base64)"` | macOS clipboard contains 200 bytes of base64 (verify by `pbpaste | base64 -d | wc -c`) | ⬜ |
| 6 | DECSCUSR cursor shapes: `printf '\e[1 q'` (blink block), `printf '\e[3 q'` (blink underline), `printf '\e[5 q'` (blink bar) | Cursor shape visibly changes in Vector | ⬜ |
| 7 | Mouse mode SGR 1006: paste `printf '\e[?1000h\e[?1006h'`, click in the terminal area | Vector sends SGR 1006 sequences to the remote (visible via `cat` if shell echoes) | ⬜ |
| 8 | `tput cols && tput lines` | reports the actual viewport size | ⬜ |
| 9 | echo $TERM | reports `xterm-256color` | ⬜ |
| 10 | **Reconnect:** with htop still running inside the user's tmux, in a separate SSH session run `pkill -f vector-tunnel-agent` | Vector inline status bar appears; tab title flips to `[reconnecting]`; cursor stops blinking | ⬜ |
| 11 | Wait 8 s. Try typing. | Toast `Input ignored — reconnecting` appears once; subsequent keystrokes silent-drop | ⬜ |
| 12 | Restart `vector-tunnel-agent` on remote | Vector reconnects within next backoff slot; tab title returns to `[remote]`; reattach to user's tmux: `tmux attach -t smoke` shows htop STILL RUNNING (user's tmux session persisted) | ⬜ |
| 13 | During a forced disconnect (re-run step 10), close pane with Cmd-W during reconnect attempt | Pane closes immediately (does NOT wait for next backoff slot) | ⬜ |

## Sign-off

- [ ] USER-RUN tmux setup completed before any test.
- [ ] All automated tests pass.
- [ ] All manual matrix items pass (or known-deviations documented inline).
- [ ] PERSIST-04 acceptance: tmux pass-through correctness verified end-to-end through Dev Tunnels relay with user-managed tmux.

**Approved by:** ___________  **Date:** ___________
