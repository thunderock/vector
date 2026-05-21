//! Phase 8 / Plan 08-05 Task 1 — Microsoft sign-in menu row labels.
//!
//! UI-SPEC §Copywriting Contract: row strings must match verbatim.

use vector_app::menu::{microsoft_signin_menu_rows, SignInState};

#[test]
fn signed_out_shows_sign_in_with_microsoft() {
    let rows = microsoft_signin_menu_rows(SignInState::SignedOut);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "Sign in with Microsoft");
    assert!(rows[0].1, "row must be enabled");
}

#[test]
fn signed_in_shows_sign_out_of_microsoft() {
    let rows = microsoft_signin_menu_rows(SignInState::SignedIn);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, "Sign out of Microsoft");
    assert!(rows[0].1, "row must be enabled");
}
