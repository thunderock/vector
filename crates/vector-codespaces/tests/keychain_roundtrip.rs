//! AUTH-02 keychain roundtrip. #[ignore]-gated because CI runners lack Keychain;
//! run locally with: cargo test -p vector-codespaces --test keychain_roundtrip -- --ignored

use vector_codespaces::TokenStore;
use zeroize::Zeroizing;

#[test]
#[ignore = "Manual UAT — requires real macOS Keychain"]
fn token_roundtrip_through_keychain() {
    let store = TokenStore::new();
    let _ = store.clear();
    let access = Zeroizing::new("gho_uat_token_value_local_only".to_string());
    store.save_access(&access).expect("save");
    let loaded = store.load_access().expect("load");
    assert_eq!(loaded.as_str(), "gho_uat_token_value_local_only");
    store.clear().expect("clear");
    assert!(store.load_access().is_none(), "post-clear load_access");
}
