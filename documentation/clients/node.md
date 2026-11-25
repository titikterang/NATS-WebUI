# Node.js client (nats.js v3)

Connect from Node.js, publish with headers and subscribe, against a plain server or one with username/password and TLS. The snippets come from [`example/clients/node/client.mjs`](../../example/clients/node/client.mjs). For the settings shared by all languages, see [Writing NATS clients](README.md).

## Install

nats.js v3 is split into packages. For Node.js, install the transport, which re-exports the core API:

```bash
npm install @nats-io/transport-node
```

It needs Node.js 24 or newer here (`"engines": { "node": ">=24" }` in the example). The old single package `nats` (v2) is superseded; don't mix it with `@nats-io/*`.

## Connect

```js
import { connect, headers } from '@nats-io/transport-node'

const nc = await connect({
  servers: process.env.NATS_URL.split(','),          // 'nats://host:4222' or 'tls://host:4222'
  user: process.env.NATS_USER,                       // undefined → no auth
  pass: process.env.NATS_PASSWORD,
  tls: process.env.NATS_CA_FILE ? { caFile: process.env.NATS_CA_FILE } : undefined,
  name: 'orders-service',                            // shown in the server's connection list
})
```

| Server | What to pass |
|---|---|
| Plain, no auth | `servers` only |
| Username/password | add `user` and `pass` |
| TLS, private CA | add `tls: { caFile: '/path/ca.pem' }` and use `tls://` URLs |
| TLS, public CA | `tls: {}` and `tls://` URLs |

## Publish with headers

```js
const h = headers()
h.set('X-Example-Client', 'node')
nc.publish('demo.hello', JSON.stringify({ client: 'node', n: 1, at: new Date().toISOString() }), { headers: h })
await nc.flush()        // returns once the server has processed everything sent so far
```

`publish` only buffers. `flush()` confirms the server received the message, but not that a permission allowed it: refused publishes are reported asynchronously (see [Error handling](#error-handling)).

## Subscribe

```js
const sub = nc.subscribe('demo.hello', { max: 5 })   // max: unsubscribe after 5 messages
await nc.flush()                                      // the server now has the subscription
for await (const m of sub) {
  console.log(m.subject, m.headers?.get('X-Example-Client'), m.string())
}
await nc.drain()
```

Use `m.json()` to parse JSON payloads and `m.respond(data)` to answer a request.

## Error handling

- `connect()` throws when no server is reachable, or when authentication or TLS fails. The error's message names the cause, for example `Authorization Violation`.
- Permission violations arrive later as status events. Watch them with:

  ```js
  for await (const s of nc.status()) {
    if (s.type === 'error') console.error(s.error.message)   // e.g. Permissions Violation for Publish to "x"
  }
  ```

- `await nc.closed()` resolves when the connection is closed for good, with an error if it closed because of one.

## Run the example

```bash
make -C example up-jetstream clients
docker run --rm --network nats-examples -e NATS_URL=nats://nats-js:4222 -e COUNT=3 nats-example-node             # subscriber
docker run --rm --network nats-examples -e NATS_URL=nats://nats-js:4222 -e MODE=pub -e COUNT=3 nats-example-node # publisher, in a second terminal
```

Against the auth + TLS setup, as the `app` user:

```bash
source example/.env
docker run --rm --network nats-examples -v "$PWD/example/nats-auth/certs/ca.pem:/ca.pem:ro" \
  -e NATS_URL=tls://nats-auth:4222 -e NATS_CA_FILE=/ca.pem \
  -e NATS_USER=app -e NATS_PASSWORD="$APP_PASSWORD" \
  -e SUBJECT=app.demo -e MODE=pub nats-example-node
```

Without Docker: `cd example/clients/node && npm ci && NATS_URL=nats://127.0.0.1:4222 node client.mjs`.
