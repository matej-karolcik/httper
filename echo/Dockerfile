FROM golang:1.22.3-alpine3.19 as builder

COPY main.go go.mod ./

RUN go build


FROM scratch

COPY --from=builder /go/echo /usr/local/bin/server
COPY certs certs

EXPOSE 8080

ENTRYPOINT [ "server" ]