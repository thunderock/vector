//! AUTH-02 Keychain integration. Manual-UAT only — CI runner has no Keychain.

#[test]
#[ignore = "Manual UAT — real macOS Keychain; Plan 06-02 verifies locally"]
fn token_roundtrip_through_keychain() {
    // Plan 06-02 manual UAT: set / get / delete on real Keychain.
}
