// Minimal NATS producer/consumer (nats.go).
//
//	MODE=pub|sub  NATS_URL  NATS_USER  NATS_PASSWORD  NATS_CA_FILE  SUBJECT  COUNT  TIMEOUT_SECONDS
package main

import (
	"encoding/json"
	"fmt"
	"os"
	"strconv"
	"time"

	"github.com/nats-io/nats.go"
)

func env(key, def string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return def
}

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, "go:", err)
		os.Exit(1)
	}
}

func run() error {
	mode := env("MODE", "sub")
	subject := env("SUBJECT", "demo.hello")
	count, _ := strconv.Atoi(env("COUNT", "5"))
	timeoutSec, _ := strconv.Atoi(env("TIMEOUT_SECONDS", "30"))

	opts := []nats.Option{nats.Name("example-go-" + mode)}
	if user := os.Getenv("NATS_USER"); user != "" {
		opts = append(opts, nats.UserInfo(user, os.Getenv("NATS_PASSWORD")))
	}
	if ca := os.Getenv("NATS_CA_FILE"); ca != "" {
		opts = append(opts, nats.RootCAs(ca))
	}

	nc, err := nats.Connect(env("NATS_URL", nats.DefaultURL), opts...)
	if err != nil {
		return fmt.Errorf("connect: %w", err)
	}
	defer nc.Drain()
	fmt.Printf("go: connected to %s as %s\n", nc.ConnectedUrlRedacted(), env("NATS_USER", "(no auth)"))

	if mode == "pub" {
		for n := 1; n <= count; n++ {
			body, _ := json.Marshal(map[string]any{"client": "go", "n": n, "at": time.Now().UTC().Format(time.RFC3339Nano)})
			msg := nats.NewMsg(subject)
			msg.Header.Set("X-Example-Client", "go")
			msg.Data = body
			if err := nc.PublishMsg(msg); err != nil {
				return fmt.Errorf("publish: %w", err)
			}
		}
		if err := nc.Flush(); err != nil {
			return fmt.Errorf("flush: %w", err)
		}
		fmt.Printf("go: published %d to %s\n", count, subject)
		return nil
	}

	sub, err := nc.SubscribeSync(subject)
	if err != nil {
		return fmt.Errorf("subscribe: %w", err)
	}
	if err := nc.Flush(); err != nil { // the server has the subscription once the flush round-trips
		return fmt.Errorf("flush: %w", err)
	}
	fmt.Println("go: ready")
	deadline := time.Now().Add(time.Duration(timeoutSec) * time.Second)
	for received := 0; received < count; received++ {
		msg, err := sub.NextMsg(time.Until(deadline))
		if err != nil {
			return fmt.Errorf("timeout, received %d/%d: %w", received, count, err)
		}
		fmt.Printf("go: %s [%s] %s\n", msg.Subject, msg.Header.Get("X-Example-Client"), msg.Data)
	}
	fmt.Printf("go: received %d on %s\n", count, subject)
	return nil
}
