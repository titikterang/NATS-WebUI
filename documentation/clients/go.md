# Go client (nats.go)

Connect from Go, publish with headers and subscribe, against a plain server or one with username/password and TLS. The snippets come from [`example/clients/go/main.go`](../../example/clients/go/main.go). For the settings shared by all languages, see [Writing NATS clients](README.md).

## Install

```bash
go get github.com/nats-io/nats.go@v1.56.0
```

The example uses Go 1.26 and builds a static binary into a distroless image.

## Connect

```go
import "github.com/nats-io/nats.go"

opts := []nats.Option{nats.Name("orders-service")}            // shown in the server's connection list
if user := os.Getenv("NATS_USER"); user != "" {
	opts = append(opts, nats.UserInfo(user, os.Getenv("NATS_PASSWORD")))
}
if ca := os.Getenv("NATS_CA_FILE"); ca != "" {
	opts = append(opts, nats.RootCAs(ca))                        // PEM file(s); verifies the host name too
}

nc, err := nats.Connect(os.Getenv("NATS_URL"), opts...)        // "nats://h1:4222,nats://h2:4222" or "tls://..."
if err != nil {
	return fmt.Errorf("connect: %w", err)
}
defer nc.Drain()
```

| Server | What to pass |
|---|---|
| Plain, no auth | the URL only |
| Username/password | add `nats.UserInfo(user, password)` |
| TLS, private CA | add `nats.RootCAs("/path/ca.pem")` and use `tls://` URLs |
| TLS, public CA | `tls://` URLs; the system roots are used |

`nats.Connect` takes several servers as one comma-separated string. Print the server you're on with `nc.ConnectedUrlRedacted()`, which hides any password in the URL.

## Publish with headers

```go
body, _ := json.Marshal(map[string]any{"client": "go", "n": 1, "at": time.Now().UTC().Format(time.RFC3339Nano)})
msg := nats.NewMsg("demo.hello")
msg.Header.Set("X-Example-Client", "go")
msg.Data = body
if err := nc.PublishMsg(msg); err != nil {
	return fmt.Errorf("publish: %w", err)
}
if err := nc.Flush(); err != nil {                              // round-trip: the server has it
	return fmt.Errorf("flush: %w", err)
}
```

For a message without headers, `nc.Publish(subject, data)` is enough.

## Subscribe

```go
sub, err := nc.SubscribeSync("demo.hello")
if err != nil {
	return fmt.Errorf("subscribe: %w", err)
}
if err := nc.Flush(); err != nil {                              // the server now has the subscription
	return fmt.Errorf("flush: %w", err)
}
for {
	msg, err := sub.NextMsg(30 * time.Second)
	if err != nil {
		return err                                               // nats.ErrTimeout when nothing arrived
	}
	fmt.Println(msg.Subject, msg.Header.Get("X-Example-Client"), string(msg.Data))
}
```

For callback style, use `nc.Subscribe(subject, func(m *nats.Msg) { ... })`. Answer a request with `m.Respond(data)`.

## Error handling

- `nats.Connect` returns an error for an unreachable server, a failed TLS handshake or rejected credentials (`nats: Authorization Violation`).
- Permission violations are asynchronous. Catch them with an error handler option:

  ```go
  nats.ErrorHandler(func(_ *nats.Conn, _ *nats.Subscription, err error) {
  	log.Print(err)                                             // e.g. nats: permissions violation: Permissions Violation for Publish to "x"
  })
  ```

- Add `nats.DisconnectErrHandler`, `nats.ReconnectHandler` and `nats.ClosedHandler` to log connection changes in long-running services.

## Run the example

```bash
make -C example up-jetstream clients
docker run --rm --network nats-examples -e NATS_URL=nats://nats-js:4222 -e COUNT=3 nats-example-go             # subscriber
docker run --rm --network nats-examples -e NATS_URL=nats://nats-js:4222 -e MODE=pub -e COUNT=3 nats-example-go # publisher, in a second terminal
```

Against the auth + TLS setup, as the `app` user:

```bash
source example/.env
docker run --rm --network nats-examples -v "$PWD/example/nats-auth/certs/ca.pem:/ca.pem:ro" \
  -e NATS_URL=tls://nats-auth:4222 -e NATS_CA_FILE=/ca.pem \
  -e NATS_USER=app -e NATS_PASSWORD="$APP_PASSWORD" \
  -e SUBJECT=app.demo -e MODE=pub nats-example-go
```

Without Docker: `cd example/clients/go && NATS_URL=nats://127.0.0.1:4222 go run .`.
