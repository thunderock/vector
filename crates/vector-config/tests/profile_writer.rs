//! CS-03 writer tests. Plan 06-04 un-ignores these.

#[test]
#[ignore = "Wave-0 stub — Plan 06-04 fills in"]
fn append_codespace_profile_writes_correct_block() {
    // Plan 06-04: tempfile config, call append_codespace_profile, parse result,
    // assert [profile.hello-world] kind="codespace" codespace_name=... tint="#7a3aaf".
}

#[test]
#[ignore = "Wave-0 stub — Plan 06-04 fills in"]
fn derive_profile_name_strips_random_suffix() {
    // Plan 06-04: "octocat/hello-world-abc123" → "hello-world".
}

#[test]
#[ignore = "Wave-0 stub — Plan 06-04 fills in"]
fn derive_profile_name_keeps_non_random_tail() {
    // Plan 06-04: "adobe/design-system-v2" → "design-system-v2" (suffix < 4 chars).
}

#[test]
#[ignore = "Wave-0 stub — Plan 06-04 fills in"]
fn derive_profile_name_decollides() {
    // Plan 06-04: existing=["vector"], "colligo/vector-x7k2m1n8" → "vector-2".
}

#[test]
#[ignore = "Wave-0 stub — Plan 06-04 fills in"]
fn append_preserves_existing_blocks() {
    // Plan 06-04: existing [default] + [profile.work-local] survive verbatim
    // (toml_edit round-trip preserves comments + ordering).
}
