//! D-38: Box<dyn PtyTransport> and Box<dyn Domain> are object-safe. Plan 02-04.

#[test]
#[ignore = "Plan 02-04"]
fn pty_transport_is_object_safe() {
    // Compile-time check: this code must compile, runtime body unused.
    unimplemented!("Plan 02-04");
}

#[test]
#[ignore = "Plan 02-04"]
fn domain_is_object_safe() {
    unimplemented!("Plan 02-04");
}
