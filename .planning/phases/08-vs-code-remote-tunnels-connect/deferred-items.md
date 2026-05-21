# Phase 8 Deferred Items

## From Plan 08-02 (Microsoft OAuth) — 2026-05-21

- **vector-tunnels::transport** uses `vector_mux::transport::{PtyTransport, TransportKind}` but `vector_mux::transport` is a private module. Out of Plan 08-02 scope (transport.rs is owned by Plan 08-04 Mac client transport). Sibling agent (Plan 08-04) authored these references during parallel execution. Fix: switch to `vector_mux::{PtyTransport, TransportKind}` re-exports. Affects `make test` workspace build; in-scope `cargo test -p vector-tunnels --test microsoft_device_flow` + `cargo test -p vector-tunnels --test microsoft_token_store` both pass.
