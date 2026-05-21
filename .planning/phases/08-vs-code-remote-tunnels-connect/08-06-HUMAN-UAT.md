---
status: partial
phase: 08-vs-code-remote-tunnels-connect
source: [08-06-agent-distribution-PLAN.md, 08-06-SUMMARY.md]
started: 2026-05-21T21:40:00Z
updated: 2026-05-21T21:51:48Z
---

## Current Test

[awaiting human testing on a Linux box / Ubuntu VM / `rust:1.88-bookworm` Docker container — Wave 4 of Phase 8 is NOT blocked on this]

Prerequisites before starting:

```bash
# Either:
#   - real Linux box / Ubuntu VM (apt available), OR
#   - Docker on the dev Mac:
docker run -it --rm -v "$PWD:/work" -w /work rust:1.88-bookworm bash
```

## Tests

### 1. cargo-deb builds the .deb on Linux (DT-01)
expected: `cargo install cargo-deb && cargo build --release -p vector-tunnel-agent && cargo deb -p vector-tunnel-agent --no-build` exits 0; produces `target/debian/vector-tunnel-agent_<ver>_<arch>.deb`.
result: [pending]

### 2. dpkg-deb metadata sanity (DT-01)
expected: `dpkg-deb --info target/debian/vector-tunnel-agent_*.deb` shows `Section: net`, `Maintainer: Vector contributors <noreply@github.com>`, the extended-description from Cargo.toml, and the correct license/copyright lines.
result: [pending]

### 3. dpkg-deb contents lists /usr/bin/vector-tunnel-agent (DT-01)
expected: `dpkg-deb --contents target/debian/vector-tunnel-agent_*.deb` lists `./usr/bin/vector-tunnel-agent` (mode 0755) and `./usr/share/doc/vector-tunnel-agent/README.md` (0644).
result: [pending]

### 4. apt install runs postinst (DT-01)
expected: `sudo apt install ./target/debian/vector-tunnel-agent_*.deb` succeeds; postinst echoes `vector-tunnel-agent installed.` + first-run hint. `which vector-tunnel-agent` returns `/usr/bin/vector-tunnel-agent`.
result: [pending]

### 5. Installed binary runs --version (DT-01)
expected: `vector-tunnel-agent --version` prints a version string and exits 0.
result: [pending]

### 6. apt remove runs prerm cleanly (DT-01)
expected: `sudo apt remove vector-tunnel-agent` succeeds; prerm exits 0; `which vector-tunnel-agent` afterwards returns nothing.
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0
blocked: 0

## Gaps

- No Linux env available on the dev Mac at execution time (2026-05-21). Macs cannot run `cargo deb` (Linux-only Debian tooling) and cannot exercise `apt install`/`apt remove`. Path forward: either ad-hoc Docker smoke (`rust:1.88-bookworm`) by the user, or roll the smoke into Plan 08-07's UAT matrix where a live tagged release exercises the workflow end-to-end. Until then, the .deb distribution path is structurally complete (Cargo.toml metadata + debian/ scripts + agent-release.yml + xtask agent-dist all verified) but operationally unproven on real Linux.

## Notes

- Mac-side smoke (item 2 in the plan's how-to-verify list — `cargo xtask agent-dist` printing the CI hint and exiting 0) **already PASSED** during executor self-check; see `08-06-SUMMARY.md §Self-Check`.
- Phase 8 Wave 4 (Plan 08-07 UAT smoke matrix) explicitly inherits this smoke: a live tagged release will fire `.github/workflows/agent-release.yml` end-to-end, after which the published `.deb` artifacts can be downloaded and installed on a real Linux host.
