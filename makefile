qa:
	cargo clippy --all
	cargo nextest r
	cargo fmt --all -- --check
	cargo doc --all --no-deps
	cargo machete
	#cargo audit todo enable at some point

build:
	cargo build --release

mkcert:
	mkcert localhost 127.0.0.1 ::1

echo-server:
	go run echo/main.go

test:
	cargo nextest r
