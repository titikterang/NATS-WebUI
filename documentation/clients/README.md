# Writing NATS clients: Node.js, Python and Go

How to connect your own services to NATS, with or without username/password and TLS, publish messages with headers, subscribe, and watch the traffic in the WebUI. Each language page follows the same structure and quotes the runnable programs in [`example/clients/`](../../example/clients/).

| Language | Library | Page | Full program |
|---|---|---|---|
| Node.js 24+ | [nats.js v3](https://github.com/nats-io/nats.js) (`@nats-io/transport-node`) | [node.md](node.md) | [`client.mjs`](../../example/clients/node/client.mjs) |
| Python 3.13+ | [nats.py](https://github.com/nats-io/nats.py) (`nats-py`) | [python.md](python.md) | [`client.py`](../../example/clients/python/client.py) |
| Go 1.26+ | [nats.go](https://github.com/nats-io/nats.go) | [go.md](go.md) | [`main.go`](../../example/clients/go/main.go) |

## Connection settings

The three libraries need the same four things. The example programs read them from the same environment variables:

| Setting | Env var in the examples | Plain server | Username/password | TLS with a private CA |
|---|---|---|---|---|
| Server URL(s) | `NATS_URL` (comma-separated) | `nats://host:4222` | `nats://host:4222` | `tls://host:4222` |
| Username | `NATS_USER` | — | required | as the server requires |
| Password | `NATS_PASSWORD` | — | required | as the server requires |
| CA certificate (PEM) | `NATS_CA_FILE` | — | — | required |

Rules that apply in every language:

- **Read credentials from the environment or a secret store,** never from source code.
- **List several cluster nodes** in the URL list. The client fails over between them, and it also learns the other nodes from the server.
- **Use `tls://`** for TLS servers. The client then refuses to connect without TLS, instead of silently using plain text when a server allows it.
- **Host names must match the certificate.** Connect with a DNS name or IP that's in the server certificate's subject alternative names.

## Message conventions in the examples

- The payload is JSON: `{"client": "<language>", "n": <sequence>, "at": "<UTC timestamp>"}`.
- Every message carries the header `X-Example-Client: <language>`. Headers need nats-server 2.2 or newer.
- A subscriber **flushes after subscribing** and only then prints `ready`. The flush round-trip guarantees the server has the subscription, so a publisher started afterwards can't race it. Use the same pattern in tests.
- Drain the connection before exiting (`drain()`), so buffered messages are sent and in-flight ones are processed.

## Try them against the example servers

The [example setups](../../example/README.md) give you a plain server, a 3-node cluster and a username/password + TLS server. Then:

```bash
make -C example clients        # build nats-example-node, -python, -go
make -C example test           # every language publishes, every language receives, on every setup
```

To run a single client, see [Sample clients](../../example/README.md#sample-clients) in the examples guide.

## Watch your messages in the WebUI

1. Add the server your service uses (see [Connecting NATS](../connecting-nats.md)). Use a read-only user.
2. Open **Clients**, select **Add client**, choose the server, and add subject patterns such as `app.>` or `orders.*.created`.
3. Select **Subscribe**. Messages appear in a table; expand a row to see the headers (for example `X-Example-Client`), the reply subject and the pretty-printed JSON payload.

More about the screens in [Using the UI](../using-the-ui.md).
