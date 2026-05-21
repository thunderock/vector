.PHONY: build lint test run start dmg help _xtask

export CARGO_TERM_COLOR := always
export MACOSX_DEPLOYMENT_TARGET := 13.0

build:
	cargo build --release -p vector-app

lint:
	cargo fmt --all -- --check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --workspace --tests

run:
	cargo run --release -p vector-app

start:
	./target/release/vector-app

dmg: _xtask
	./xtask/target/release/xtask dmg

_xtask:
	cargo build --release --manifest-path xtask/Cargo.toml

help:
	@echo "build   build vector-app (host arch, release)"
	@echo "lint    cargo fmt --check + clippy"
	@echo "test    cargo test --workspace --tests"
	@echo "run     run vector-app (release, via cargo)"
	@echo "start   launch pre-built binary (target/release/vector-app)"
	@echo "dmg     build a local .dmg via xtask"
