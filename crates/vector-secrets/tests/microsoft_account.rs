use vector_secrets::Secrets;

#[test]
fn microsoft_account_constant_value() {
    assert_eq!(
        Secrets::MICROSOFT_REFRESH_ACCOUNT,
        "microsoft_refresh_token"
    );
}
