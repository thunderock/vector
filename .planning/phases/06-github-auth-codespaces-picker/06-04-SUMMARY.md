---
phase: 06-github-auth-codespaces-picker
plan: 04
subsystem: config
tags: [toml_edit, regex, atomic-write, profile-writer, cs-03, ui-spec-5.3]

requires:
  - phase: 06-github-auth-codespaces-picker
    plan: 01
    provides: "vector-config::writer module + WriterError + toml_edit dep wired"
provides:
  - "vector-config::append_codespace_profile(path, name, codespace_name, tint) -> Result<String, WriterError> — formatting-preserving append + atomic rename, returns final name (auto-suffixed on collision)"
  - "vector-config::derive_profile_name(codespace_name, &existing) -> String — UI-SPEC §5.3 derivation: strip owner/, strip (?i)-[a-z0-9]{4,}$, fall back to owner, de-collide via -N suffix"
  - "WriterError::NoParent variant for path with no parent directory"
affects: [06-06-codespaces-modal]

tech-stack:
  added:
    - "regex.workspace dep on vector-config (was already a transitive but now direct for the random-suffix strip)"
  patterns:
    - "Atomic write pattern: write `{path}.tmp` next to target, then `std::fs::rename` — matches Plan 05-04 watcher pitfall-1 (atomic rename inode swap survives parent-dir FSEvents debounce)"
    - "toml_edit DocumentMut round-trip: parse, mutate, to_string — preserves comments, blank lines, and existing block ordering verbatim"
    - "Collision auto-suffix returns the actual name written, so callers can render a toast that matches what landed on disk"

key-files:
  created: []
  modified:
    - "crates/vector-config/src/writer.rs (replaced unimplemented!() stubs with full implementation, 117 lines)"
    - "crates/vector-config/tests/profile_writer.rs (5 stubs un-ignored + 1 new test = 6 tests)"
    - "crates/vector-config/Cargo.toml (added regex.workspace = true)"
    - "Cargo.lock (regex pulled in as direct dep of vector-config)"

key-decisions:
  - "Two-layer de-collision: derive_profile_name handles initial owner/random-suffix strip + collision against the in-memory list; append_codespace_profile re-applies a simple `-N` loop against the on-disk profile table at write time. This keeps derive_profile_name pure (no I/O, easy to test) while still defending against the caller passing a name that races with a manual edit between derive and write."
  - "WriterError gains NoParent rather than panicking on `path.parent()` returning None. Atomic write needs a sibling directory; if the caller hands us '/' or similar, we surface a typed error."
  - "Empty source path → DocumentMut::new() rather than erroring. Plan 06-06 will call this even if ~/.config/vector/config.toml doesn't yet exist (first save creates the file)."

requirements-completed: [CS-03]

duration: 2min
completed: 2026-05-14
---

# Phase 6 Plan 04: Wave 1 — vector-config writer (CS-03) Summary

**Implemented `append_codespace_profile` + `derive_profile_name` with toml_edit round-trip, regex random-suffix strip, atomic rename, and collision auto-suffix; 6/6 writer tests green and clippy clean.**

## Performance

- **Duration:** 2 min
- **Started:** 2026-05-14T19:10:06Z
- **Completed:** 2026-05-14T19:12:26Z
- **Tasks:** 2 (RED + GREEN)
- **Files modified:** 4 (0 created, 4 modified)

## Accomplishments

- `derive_profile_name` strips the `owner/` prefix, applies `(?i)-[a-z0-9]{4,}$` to drop random codespace suffixes (`octocat/hello-world-abc123` → `hello-world`, `colligo/vector-x7k2m1n8` → `vector`), preserves short tails (`adobe/design-system-v2` keeps `-v2`), and de-collides via `-2`, `-3`, ... against the supplied existing-names slice.
- `append_codespace_profile` reads the existing `~/.config/vector/config.toml` (treats missing file as empty), parses with `toml_edit::DocumentMut`, inserts `[profile.{name}]` with `kind = "codespace"` + `codespace_name` + `tint`, writes the rendered document to `{path}.tmp`, then `std::fs::rename`s it over the target.
- toml_edit round-trip verified preserving `# comment` lines, blank-line separators, and existing `[profile.work-local]` blocks across the append (test `append_preserves_existing_blocks`).
- Atomic rename verified: `.toml.tmp` does not persist after a successful append (test `append_atomic_rename_no_partial`), so Plan 05-04's `notify` watcher never sees a partial file.
- Returned `String` is the actual profile name written (after collision-suffix), enabling Plan 06-06's UI to render a toast that matches disk state when two saves of the same repo race.
- `WriterError::NoParent` added for the path-has-no-parent case (rather than panicking on `path.parent().unwrap()`).

## Task Commits

1. **Task 06-04-01: RED — un-ignore 5 stub tests + add `append_atomic_rename_no_partial` 6th test** — `7beff12` (test)
2. **Task 06-04-02: GREEN — full writer implementation + regex dep + clippy fixes (map_or_else, contains)** — `deaf4a9` (feat)

**Plan metadata commit:** (to follow — SUMMARY/STATE/ROADMAP)

## Files Modified

- `crates/vector-config/src/writer.rs` — replaced two `unimplemented!()` stubs with the full implementation (regex strip, de-collide loop, toml_edit insert, atomic_write helper, NoParent error variant)
- `crates/vector-config/tests/profile_writer.rs` — flipped 5 `#[ignore]` stubs to active tests; added `append_atomic_rename_no_partial` 6th test
- `crates/vector-config/Cargo.toml` — added `regex.workspace = true` to `[dependencies]`
- `Cargo.lock` — pulled regex transitively into the vector-config dep tree as a direct dep

## Decisions Made

- **Two-layer de-collision:** `derive_profile_name` handles the strip + initial collision against the in-memory list of existing names (caller's responsibility to supply current profile keys); `append_codespace_profile` re-applies a simple `-N` loop against the on-disk parsed `[profile]` table at write time, so a race between "derive" and "write" still produces a unique name. This keeps `derive_profile_name` pure (no I/O, trivially unit-testable).
- **WriterError::NoParent:** `path.parent()` returning None is a real edge case for `path = "/"` or other rootless paths; surface it as a typed error rather than panicking.
- **Empty source → `DocumentMut::new()`:** Plan 06-06 will call this on first save, before `~/.config/vector/config.toml` exists. Treating `NotFound` as empty source yields a fresh document; on success the parent dir must already exist (Plan 06-06 ensures `~/.config/vector/` is created during initial save).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Lint hygiene] clippy::manual-contains on `refs.iter().any(|n| *n == profile_name)`**
- **Found during:** Task 06-04-02 (first clippy run)
- **Issue:** Workspace `-D warnings` lint forbids `iter().any(==)` when `contains` works.
- **Fix:** Switched to `refs.contains(&profile_name)` and `refs.contains(&candidate.as_str())` inside the collision loop.
- **Files modified:** `crates/vector-config/src/writer.rs`
- **Committed in:** `deaf4a9`

**2. [Rule 1 — Lint hygiene] clippy::map_unwrap_or on `path.file_name().map(...).unwrap_or_else(...)`**
- **Found during:** Task 06-04-02
- **Issue:** Workspace lint forbids `map(...).unwrap_or_else(...)`.
- **Fix:** Switched to `path.file_name().map_or_else(|| "config.toml".to_string(), |s| s.to_string_lossy().into_owned())`.
- **Files modified:** `crates/vector-config/src/writer.rs`
- **Committed in:** `deaf4a9`

Plan's `<action>` block included both a "messy" and a "clean" version of `append_codespace_profile`; the implementation lands the clean version (single function, no duplicate `final_name`). Plan-anticipated cleanup, not a deviation.

---

**Total deviations:** 2 auto-fixed (Rule-1 lint hygiene)
**Impact on plan:** Zero scope change. Both lints are workspace-enforced policy that the plan's action block didn't pre-anticipate.

## Issues Encountered

The two clippy lints above were the only friction; resolved inline.

## User Setup Required

None — purely internal data-layer work.

## Next Phase Readiness

- **Plan 06-06 (Wave 2 — Codespaces picker modal):** Can now wire the `Save as profile` button to:
  1. Compute existing profile names from the live `ConfigFile` (already available via Plan 05-04 hot-reload).
  2. Call `derive_profile_name(codespace.name, &existing)` to suggest a name in the modal field.
  3. On confirm, call `append_codespace_profile(path, suggested_name, codespace.name, tint)` and render a toast using the returned `String` (so the user sees the actual `-N` suffix if a collision occurred).
- **CS-03 acceptance:** saved codespace profile survives app restart because Plan 05-04's watcher will pick up the atomic-rename event, re-parse, and hot-reload the new `[profile.X]` block into `ConfigState`.

## Self-Check: PASSED

Verified each created/modified file + commit on disk:

- `crates/vector-config/src/writer.rs` — FOUND (full implementation, single `append_codespace_profile`, single `derive_profile_name`)
- `crates/vector-config/tests/profile_writer.rs` — FOUND (6 tests, 0 ignored)
- `crates/vector-config/Cargo.toml` — FOUND (regex.workspace added)
- `.planning/phases/06-github-auth-codespaces-picker/06-04-SUMMARY.md` — FOUND
- Commit `7beff12` (RED) — FOUND
- Commit `deaf4a9` (GREEN) — FOUND

Verification commands:
- `cargo test -p vector-config --test profile_writer` → 6 passed; 0 failed; 0 ignored
- `cargo test -p vector-config --tests` → all suites green (schema_and_loader, watcher_debounce, profile_writer)
- `cargo clippy -p vector-config --all-targets -- -D warnings` → exits 0

---
*Phase: 06-github-auth-codespaces-picker*
*Completed: 2026-05-14*
