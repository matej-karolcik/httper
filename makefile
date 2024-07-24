mkcert:
	mkcert localhost 127.0.0.1 ::1

echo-server:
	cd echo && go run main.go

test:
	go test -v ./...
