---
phase: 05-polish-local-daily-driver
plan: 15
subsystem: ime
tags: [IME, NSTextInputClient, objc2, AppKit, POLISH-08]
dependency_graph:
  requires: [05-12]
  provides: [NSTextInputClient-subclass, ImeState-wired-to-winit]
  affects: [app.rs, ime.rs]
tech_stack:
  added: []
  patterns:
    - objc2 0.6.4 define_class! macro for NSView subclassing
    - Mutex<ImeState> as define_class! Ivars (MainThreadOnly inherits from NSView)
    - winit WindowEvent::Ime dispatch to pure-Rust ImeState
key_files:
  created:
    - crates/vector-app/tests/ime_shim.rs
  modified:
    - crates/vector-app/src/ime.rs
    - crates/vector-app/src/app.rs
decisions:
  - "objc2 0.6.4 uses define_class! not declare_class! (renamed in 0.6.x)"
  - "NSView is MainThreadOnly; alloc() requires MainThreadMarker parameter"
  - "Ivars = Mutex<ImeState>; Send+Sync verified via compile-time assert"
  - "Use AnyObject::downcast_ref for NSString/NSAttributedString coercion in insertText:"
  - "Winit Ime::Preedit(String, Option<(usize,usize)>) not a struct variant"
  - "set_ime_allowed(true) owned by this plan for resumed() + handle_new_tab() only; SpawnNewWindow site is Plan 05-14's"
metrics:
  duration: ~15min
  completed: 2026-05-13T16:58:54Z
  tasks: 3
  files: 3
---

# Phase 05 Plan 15: NSTextInputClient Subclass + ImeState Wiring Summary

NSTextInputClient `VectorInputView` subclass implemented via objc2 0.6.4 `define_class!` macro with six selectors; `App.ime: ImeState` field wired to winit `WindowEvent::Ime` dispatch; `set_ime_allowed(true)` added to `resumed()` and `handle_new_tab()`.

## Objective

Close gap #4 from 05-VERIFICATION.md: the `declare_class!` NSTextInputClient subclass was previously deferred (05-09's smoke matrix passed because winit's own NSTextInputClient implementation sufficed for the end-to-end test, but the source requirement was unfulfilled). Plan 05-15 ships the AppKit subclass and wires App.ime for Pitfall-9-safe dispatch.

## What Was Implemented

### Task 1: define_class! NSTextInputClient Subclass

**Pre-flight Step 0a result:** No existing `declare_class!` usage in the codebase (only in comments). This is the first `define_class!` subclass in the repo.

**Key discovery:** In objc2 0.6.4, `declare_class!` was renamed to `define_class!` with updated syntax:
- `#[unsafe(super(NSView))]` attribute replaces `unsafe impl ClassType`
- `#[ivars = Mutex<ImeState>]` attribute replaces `impl DeclaredClass`
- `#[unsafe(method(...))]` replaces `#[method(...)]`
- `msg_send_id!` deprecated — use `msg_send!` or `downcast_ref`
- `MainThreadOnly::alloc(mtm)` replaces `Self::alloc()` (NSView is MainThreadOnly)

**Ivars Send+Sync:** `Mutex<ImeState>` verified with compile-time assertion:
```rust
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Mutex<ImeState>>();
};
```

**Six selectors implemented (D-81 five-selector minimum + hasMarkedText):**

| Selector | Rust fn | ImeState method |
|----------|---------|-----------------|
| `insertText:replacementRange:` | `insert_text` | `commit()` → writes UTF-8 to PTY |
| `setMarkedText:selectedRange:replacementRange:` | `set_marked_text` | `set_preedit()` → NEVER PTY |
| `unmarkText` | `unmark_text` | `clear()` |
| `markedRange` | `marked_range` | `marked_range()` |
| `selectedRange` | `selected_range` | returns NSNotFound (usize::MAX, 0) |
| `hasMarkedText` | `has_marked_text` | `is_active()` |

**NSAttributedString coercion:** AppKit may deliver `NSAttributedString` to `insertText:` and `setMarkedText:`. Both methods use `downcast_ref::<NSString>()` first, then `downcast_ref::<NSAttributedString>()`, with a raw `msg_send![text, string]` fallback.

**cfg gate:** `#[cfg(all(target_os = "macos", not(feature = "test-hooks")))]` — the AppKit macro block is excluded from `cargo test` (no AppKit runtime); the pure-Rust ImeState layer is tested via `tests/ime_shim.rs`.

### Task 2: App.ime Field + Winit IME Dispatch

**W7 fix (clone-before-move):** `write_tx` was moved into `InputBridge::new`. Added `let ime_write_tx = write_tx.clone()` BEFORE the `InputBridge::new(write_tx, ...)` call so `ImeState::new(ime_write_tx)` gets its own channel sender.

**Winit 0.30.13 Ime enum (actual vs plan):** The plan described `Preedit { text, cursor_range }` struct variant, but winit 0.30.13 uses tuple variants:
- `Ime::Preedit(String, Option<(usize, usize)>)` — second element is `Option<(cursor_start, cursor_end)>`
- `Ime::Commit(String)`
- Adapted dispatch uses `cursor_range.map(|(start, _)| start).unwrap_or(0)` for offset.

**set_ime_allowed call sites (MEDIUM-3):** EXACTLY 2 in this plan:
1. `resumed()` — bootstrap window (Plan 05-15 site)
2. `handle_new_tab()` — Cmd-T window (Plan 05-15 site)

The SpawnNewWindow site is owned by Plan 05-14 (depends_on includes 05-15). Count will be 3 after 05-14 runs.

**WindowEvent::Ime arm:** Added before `_ => {}`:
```rust
WindowEvent::Ime(ime_event) => {
    use winit::event::Ime;
    match ime_event {
        Ime::Enabled => { tracing::debug!("Ime enabled"); }
        Ime::Preedit(text, cursor_range) => {
            // Pitfall 9: preedit NEVER reaches PTY
            let offset = cursor_range.map(|(start, _)| start).unwrap_or(0);
            if text.is_empty() { self.ime.clear(); } else { self.ime.set_preedit(&text, offset); }
            self.request_redraw(id);
        }
        Ime::Commit(text) => { let _ = self.ime.commit(&text); self.request_redraw(id); }
        Ime::Disabled => { self.ime.clear(); }
    }
}
```

### Task 3: Manual IME Smoke (Hiragana preedit)

**Status: AWAITING HUMAN VERIFICATION**

The manual smoke (Task 3) requires the user to:
1. Build and launch the app: `cargo run --release`
2. Switch macOS input source to Japanese — Hiragana
3. Type `aiueo` — expect `あいうえお` preedit visible at cursor (underlined)
4. Press Enter — kana commits to shell as single UTF-8 write
5. Confirm preedit text did not appear byte-by-byte before commit (Pitfall 9)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] objc2 0.6.4 uses `define_class!` not `declare_class!`**
- Found during: Task 1 Step 0a pre-flight
- Issue: Plan pseudocode used `declare_class!` with old `DeclaredClass/ClassType` API. In 0.6.4, the macro is `define_class!` with attribute-based syntax (`#[unsafe(super(...))]`, `#[ivars = ...]`, `#[unsafe(method(...))]`).
- Fix: Used `define_class!` with 0.6.4 attribute API. All selector semantics preserved.
- Files modified: `crates/vector-app/src/ime.rs`
- Commit: db04ea8

**2. [Rule 1 - Bug] NSView is MainThreadOnly; alloc requires mtm parameter**
- Found during: Task 1 Step 0a pre-flight build
- Issue: Plan's `new_with_state` called `Self::alloc()` but NSView inherits MainThreadOnly — alloc requires `MainThreadOnly::alloc(mtm)` with a `MainThreadMarker`.
- Fix: `new_with_state` now takes `mtm: objc2::MainThreadMarker` as first arg; `Self::alloc(mtm)` call.
- Files modified: `crates/vector-app/src/ime.rs`
- Commit: db04ea8

**3. [Rule 1 - Bug] msg_send_id! deprecated in objc2 0.6.4**
- Found during: Task 1 compile
- Issue: Plan used `msg_send_id![text, string]` which is deprecated in 0.6.4.
- Fix: Used `AnyObject::downcast_ref::<NSString>()` / `downcast_ref::<NSAttributedString>()` with a raw `msg_send!` fallback — no deprecated API.
- Files modified: `crates/vector-app/src/ime.rs`
- Commit: db04ea8

**4. [Rule 1 - Bug] winit 0.30.13 Ime::Preedit is a tuple variant, not a struct variant**
- Found during: Task 2 implementation
- Issue: Plan described `Ime::Preedit { text, cursor_range }` struct variant. Actual winit 0.30.13 API: `Ime::Preedit(String, Option<(usize, usize)>)` — tuple variant with `Option<(start, end)>`.
- Fix: Adapted pattern match to tuple destructuring: `Ime::Preedit(text, cursor_range)`.
- Files modified: `crates/vector-app/src/app.rs`
- Commit: 39ba9ca

## Commits

| Task | Hash | Message |
|------|------|---------|
| Task 1 | db04ea8 | feat(05-15): declare_class! NSTextInputClient subclass + ImeState regression tests |
| Task 2 | 39ba9ca | feat(05-15): App.ime field + WindowEvent::Ime dispatch + set_ime_allowed |
| Task 3 | — | (checkpoint:human-verify — no code commit) |

## Forward Dependencies

- **Plan 05-14 (SpawnNewWindow):** Must add `set_ime_allowed(true)` to its `AppShortcut::SpawnNewWindow` arm when that branch is created. The `grep -c "set_ime_allowed(true)" crates/vector-app/src/app.rs` count is currently 2; Plan 05-14 brings it to 3.
- **Plan 05-16 (render):** If winit's IME path doesn't produce the visible preedit underline attribute through the Phase-3 cell pipeline, Plan 05-16 may need to add a render hook that reads `App.ime.preedit()` and draws the underline overlay at the cursor position.

## Architecture: winit-primary + define_class!-foundation

This plan implements BOTH approaches as planned:
1. **winit primary path:** `set_ime_allowed(true)` + `WindowEvent::Ime` dispatch → `ImeState` → PTY. This is fully working and sufficient for D-81 Hiragana preedit in v1.
2. **define_class! subclass (VectorInputView):** Built and cfg-gated. Not installed as first-responder in v1 (winit's own NSTextInputClient implementation handles the OS events). The subclass is the v1.x foundation for making the custom class primary when needed.

Migration path to make VectorInputView primary: call `NSWindow.makeFirstResponder:(&VectorInputView)` after window spawn, replacing winit's first-responder assignment.

## Known Stubs

None — the App.ime field is fully wired to write_tx; preedit/commit/clear all have real implementations. The VectorInputView subclass is complete but not installed as first-responder in v1 (documented above as intentional; winit's path is primary).

## Self-Check: PASSED

- `crates/vector-app/src/ime.rs` exists with `define_class!` block ✓
- `crates/vector-app/tests/ime_shim.rs` exists with 3 tests ✓
- `crates/vector-app/src/app.rs` contains `ime: crate::ime::ImeState` ✓
- `grep -c "set_ime_allowed(true)" app.rs` == 2 ✓
- `cargo test -p vector-app --test ime_shim` 3 passed ✓
- `cargo build --workspace --release` clean ✓
- Commits db04ea8 and 39ba9ca exist ✓
