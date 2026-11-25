# Examples: local NATS setups and sample clients

Three ready-to-run NATS setups and a producer/consumer in Node.js, Python and Go. Use them to try NATS, to test the WebUI without touching real servers, or as a starting point for your own services.

All three setups can run at the same time. They share the Docker network `nats-examples` and listen on different localhost ports.

| Setup | What it shows | Client URL | Monitoring |
|---|---|---|---|
| `nats-jetstream` | Single server with JetStream | `nats://127.0.0.1:4222` | `http://127.0.0.1:8222` |
| `nats-cluster` | 3-node JetStream cluster | `nats://127.0.0.1:4231`, `:4232`, `:4233` | `http://127.0.0.1:8231`, `:8232`, `:8233` |
| `nats-auth` | JetStream + username/password + TLS | `tls://127.0.0.1:4242` | `http://127.0.0.1:8242` |

From containers on the `nats-examples` network, use the service names instead: `nats://nats-js:4222`, `nats://nats-1:4222` (also `nats-2`, `nats-3`) and `tls://nats-auth:4222`.

## Quick start

```bash
cp example/.env.example example/.env   # then set your own passwords
make -C example up-all                  # start all three setups (waits until healthy)
make -C example test                    # round-trip every client against every setup
make -C example down-all                # stop everything and delete the example data
```

Run `make -C example help` to see every target. Start setups one at a time with `up-jetstream`, `up-cluster` or `up-auth`.

## The auth setup

`nats-auth` mirrors a typical production setup: TLS, username/password authentication and per-user permissions.

| User | Can publish | Can subscribe | Use it for |
|---|---|---|---|
| `admin` | everything | everything | Administration |
| `app` | `app.>`, `$JS.API.>` | `app.>`, `_INBOX.>` | An application |
| `monitor` | nothing | everything | Read-only monitoring, e.g. the WebUI |

- **Passwords** come from `example/.env` (git-ignored). `nats-auth/nats.conf` only references them as `$ADMIN_PASSWORD`, `$APP_PASSWORD` and `$MONITOR_PASSWORD`.
- **Certificates** are generated on first start by `nats-auth/gen-certs.sh` into `nats-auth/certs/` (git-ignored): a throwaway local CA (`ca.pem`) and a server certificate for `localhost`, `127.0.0.1` and `nats-auth`. Give clients `ca.pem` to verify the server. To regenerate, delete `nats-auth/certs/` and run `make -C example certs`.

## Sample clients

Each client is one small program with a Dockerfile. They're configured the same way:

| Variable | Default | Meaning |
|---|---|---|
| `MODE` | `sub` | `pub` sends `COUNT` messages, `sub` waits for `COUNT` messages |
| `NATS_URL` | `nats://127.0.0.1:4222` | Server URL(s), comma-separated |
| `NATS_USER`, `NATS_PASSWORD` | unset | Credentials, if the server requires them |
| `NATS_CA_FILE` | unset | CA certificate for TLS servers with a private CA |
| `SUBJECT` | `demo.hello` | Subject to publish to or subscribe on |
| `COUNT` | `5` | Messages to send or wait for |
| `TIMEOUT_SECONDS` | `30` | `sub` exits with an error if `COUNT` isn't reached in time |

Messages are JSON and carry an `X-Example-Client` header naming the sender's language. A subscriber prints `ready` once the server has confirmed its subscription, so scripts can wait for it before publishing.

| Client | Library | Image |
|---|---|---|
| `clients/node` | [nats.js](https://github.com/nats-io/nats.js) (`@nats-io/transport-node`) | `node:26-alpine`, non-root |
| `clients/python` | [nats.py](https://github.com/nats-io/nats.py) (`nats-py`) | `python:3.14-slim`, non-root |
| `clients/go` | [nats.go](https://github.com/nats-io/nats.go) | distroless static, non-root |

Build the images with `make -C example clients`, then run them on the examples network:

```bash
# Subscribe on the single server…
docker run --rm --network nats-examples -e NATS_URL=nats://nats-js:4222 -e COUNT=3 nats-example-go

# …and publish to it from another terminal.
docker run --rm --network nats-examples -e NATS_URL=nats://nats-js:4222 -e MODE=pub -e COUNT=3 nats-example-node

# The auth setup: TLS with the local CA, as the app user.
source example/.env
docker run --rm --network nats-examples -v "$PWD/example/nats-auth/certs/ca.pem:/ca.pem:ro" \
  -e NATS_URL=tls://nats-auth:4222 -e NATS_CA_FILE=/ca.pem \
  -e NATS_USER=app -e NATS_PASSWORD="$APP_PASSWORD" \
  -e SUBJECT=app.demo -e MODE=pub nats-example-python
```

## What `make -C example test` checks

For each running setup, Node, Python and Go subscribers start first, then each language publishes 5 messages. Every subscriber must receive all 15, which proves the three libraries interoperate.

- **Cluster:** subscribers connect to `nats-1` and publishers to `nats-3`, so messages cross the cluster routes.
- **Auth:** everything runs over TLS as the `app` user, and a wrong password must be rejected.

## Connect the WebUI to the examples

Run the WebUI from source ([Local development](../documentation/local-development.md)) with the example CA, then add each setup as a server in the UI. Use `127.0.0.1`, the client port and the monitoring port from the first table. For a cluster, add each node as its own server.

```bash
make dev NATS_CA_FILE=$PWD/example/nats-auth/certs/ca.pem
```

- For `nats-auth`, use the `monitor` user. It can watch every subject, and publishing is refused, which makes it a good way to see permission errors in the UI.
- Plain and TLS servers work side by side. TLS is used whenever a server requires it.
- To reach both the examples and your own TLS servers, put both CA certificates in one PEM file and point `NATS_CA_FILE` at it.

For connecting your own servers, see [Connecting NATS](../documentation/connecting-nats.md). For writing clients, see [Writing NATS clients](../documentation/clients/README.md).
