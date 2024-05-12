qa:
	cargo clippy --all
	cargo test --all
	cargo fmt --all -- --check
	cargo doc --all --no-deps
	cargo machete
	#cargo audit

build:
	cargo build --release
