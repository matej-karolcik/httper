qa:
	cargo clippy --all
	cargo test --all
	cargo fmt --all -- --check
	cargo doc --all --no-deps
	cargo machete
	#cargo audit todo enable at some point

build:
	cargo build --release

mkcert:
	mkcert localhost 127.0.0.1 ::1

echo-server:
	go run main.go
