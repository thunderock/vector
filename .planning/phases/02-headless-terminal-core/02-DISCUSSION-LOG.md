# Phase 02: Headless Terminal Core - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-11
**Phase:** 02-headless-terminal-core
**Areas discussed:** Headless binary purpose & visible output, VT conformance corpus source, Domain trait shape, Scrollback search interface

---

## Area selection

| Option | Description | Selected |
|--------|-------------|----------|
| Headless binary purpose & visible output | What does `cargo run --bin vector-headless` actually DO on screen? | ✓ |
| VT conformance corpus source | Where do CORE-01 conformance tests come from? | ✓ |
| Domain trait shape — full now or minimal | How much of `Domain`/`PtyTransport` ships in Phase 2? | ✓ |
| Scrollback search interface (CORE-03) | How does regex scrollback search surface in Phase 2? | ✓ |

**User's choice:** All four areas selected.

---

## Headless binary purpose & visible output

| Option | Description | Selected |
|--------|-------------|----------|
| Pass-through proxy | User types into `vector-headless`; bytes flow to PTY; output bytes round-trip through alacritty_terminal; grid renders back to parent terminal each tick. Manually driveable with vim/htop/tmux. Largest surface. | ✓ |
| Snapshot/capture tool | `vector-headless --command 'echo hello' --rows 24` runs command, dumps grid state as ASCII/JSON. Test-oriented; smaller surface. | |
| Interactive REPL | Spawn shell; output parsed into grid AND echoed line-by-line. Less polished than pass-through; simpler. | |
| Test-only (no standalone binary) | Drop binary; ship vector-term as library validated only by `cargo test`. Requires roadmap amendment. | |

**User's choice:** Pass-through proxy (Recommended)
**Notes:** Selected the recommended option because it maximizes manual-driver utility — vim/tmux/htop can be used as live real-world fixtures, which Pitfall 1 explicitly calls for as a warning-signs canary.

---

## VT conformance corpus source

| Option | Description | Selected |
|--------|-------------|----------|
| Roll our own focused fixtures | ~20-40 hand-authored (input_bytes, expected_grid_state) pairs covering exactly CORE-01/02/06. Runs in <1s. Tight scope. | ✓ |
| Vendor a subset of Alacritty's tests | Broader coverage via Alacritty's MIT-licensed fixtures. Risk of pulling in expectations alacritty_terminal handles differently. | |
| Adapt vttest / esctest | De facto VT100/xterm conformance suites. Heaviest integration; highest fidelity. | |
| Mix: own fixtures + select Alacritty cases | Author CORE-01/02 ourselves, vendor 10-20 Alacritty edge cases. Best coverage-to-effort. | |

**User's choice:** Roll our own focused fixtures (Recommended)
**Notes:** Aligns with the "we ship a terminal and a tunnel client, full stop" anti-bloat principle from PROJECT.md. Easier to debug failures when each fixture exists for an explicit CORE-XX criterion.

---

## Domain / PtyTransport trait shape

| Option | Description | Selected |
|--------|-------------|----------|
| Full trait + LocalDomain + stubbed remote domains | `PtyTransport` + `Domain` finalized; LocalDomain working; CodespaceDomain/DevTunnelDomain stubs with `unimplemented!()` bodies. Locks contract early. | ✓ |
| Minimal: trait + LocalDomain only | Define trait, ship LocalDomain, no remote-domain files. Phase 7 may reshape trait at first remote impl. | |
| Just LocalPty struct, no trait yet | Skip trait entirely; introduce in Phase 4 or 7 when second impl emerges. YAGNI-pure but defers load-bearing decision. | |

**User's choice:** Full trait + LocalDomain + stubbed remote domains (Recommended)
**Notes:** Trait surface is load-bearing for Phases 4/7/8/9. Locking it early costs ~30 LOC of stubs but eliminates reshape risk at every later phase. Consistent with the Phase 1 pattern of locking architectural invariants on day one.

---

## Scrollback search interface

| Option | Description | Selected |
|--------|-------------|----------|
| Library API only — tests exercise it, no user UI | `fn search(&self, regex: &Regex)` in vector-term; Phase 2 tests assert against 10k+ line scrollback. Cmd-F UI deferred to Phase 5. | ✓ |
| CLI flag on vector-headless | `vector-headless --search PATTERN` runs interactively then dumps matches on exit. Library API still ships. | |
| Interactive Ctrl-/ inside vector-headless | Search mode inside pass-through proxy. Closest to eventual Phase 5 UX; largest scope. | |

**User's choice:** Library API only (Recommended)
**Notes:** CORE-03 acceptance is satisfied by construction (library + tests); user-facing search UI belongs in Phase 5 with GPU + input handling. Avoids pulling Phase 5 work into Phase 2 scope.

---

## Claude's Discretion

These were captured in CONTEXT.md as Claude's-discretion items without requiring user input:

- Shell selection logic (`$SHELL` → `/etc/passwd` → `/bin/zsh` fallback)
- Default grid size (planner picks 80×24 or 100×30; CLI overrides)
- Parser tracing levels (workspace-standard `tracing`)
- Lifecycle on shell exit (drain → render final → exit 0)
- Error reporting style (anyhow at binary, thiserror in library)
- Binary file location (`crates/vector-app/src/bin/vector-headless.rs` vs own crate — planner picks)
- Mux scope boundary (Phase 2 wires LocalDomain → Term directly; Pane/Tab/Window land in Phase 4)

## Deferred Ideas

Captured in CONTEXT.md `<deferred>` section:

- Search UI overlay (Phase 5)
- Pane/Tab/Window types in vector-mux (Phase 4)
- Alacritty test-suite vendoring / vttest / esctest integration
- Bracketed paste / mouse / DECSCUSR input handling beyond parser-state-setting
- CodespaceDomain / DevTunnelDomain bodies (Phases 7/8)
- Sixel, Kitty graphics (project-level Out of Scope)
- Custom terminfo (project-level Out of Scope)
