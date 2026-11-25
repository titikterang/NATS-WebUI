# Python client (nats.py)

Connect from Python with asyncio, publish with headers and subscribe, against a plain server or one with username/password and TLS. The snippets come from [`example/clients/python/client.py`](../../example/clients/python/client.py). For the settings shared by all languages, see [Writing NATS clients](README.md).

## Install

```bash
pip install nats-py==2.17.0
```

The package is `nats-py`; the module you import is `nats`. The example runs on Python 3.14 (`python:3.14-slim`).

## Connect

```python
import os, ssl
import nats

tls = None
if ca := os.environ.get("NATS_CA_FILE"):
    tls = ssl.create_default_context(cafile=ca)      # verifies the certificate and the host name

async def on_error(e: Exception) -> None:
    print(f"nats error: {e}")                         # auth failures, permission violations, ...

nc = await nats.connect(
    servers=os.environ["NATS_URL"].split(","),         # "nats://host:4222" or "tls://host:4222"
    user=os.environ.get("NATS_USER") or None,          # None → no auth
    password=os.environ.get("NATS_PASSWORD") or None,
    tls=tls,
    name="orders-service",
    error_cb=on_error,
    connect_timeout=5,
    max_reconnect_attempts=3,                           # fail fast instead of retrying for minutes
)
```

| Server | What to pass |
|---|---|
| Plain, no auth | `servers` only |
| Username/password | add `user` and `password` |
| TLS, private CA | add `tls=ssl.create_default_context(cafile="/path/ca.pem")` and use `tls://` URLs |
| TLS, public CA | `tls=ssl.create_default_context()` and `tls://` URLs |

## Publish with headers

```python
import json
from datetime import datetime, timezone

body = json.dumps({"client": "python", "n": 1, "at": datetime.now(timezone.utc).isoformat()})
await nc.publish("demo.hello", body.encode(), headers={"X-Example-Client": "python"})
await nc.flush()       # returns once the server has processed everything sent so far
```

Payloads are `bytes`. Permission violations aren't raised by `publish`; they reach `error_cb`.

## Subscribe

```python
async def on_msg(msg):
    client = (msg.headers or {}).get("X-Example-Client", "-")
    print(msg.subject, client, msg.data.decode())

await nc.subscribe("demo.hello", cb=on_msg)
await nc.flush()       # the server now has the subscription
...
await nc.drain()
```

Use `json.loads(msg.data)` for JSON payloads and `await msg.respond(data)` to answer a request.

## Gotchas

- **Silent retries.** By default nats.py retries a failed connect or a rejected password for about two minutes (60 attempts, 2 seconds apart) before giving up, and reports each failure only to `error_cb`. Without an `error_cb` it looks like a hang. Set `error_cb`, and lower `max_reconnect_attempts` in tools and tests.
- **Strict certificate checks (Python 3.13+).** `ssl.create_default_context()` enables `VERIFY_X509_STRICT`, which rejects certificates that other languages accept:
  - the CA must have a `keyUsage` extension (with `keyCertSign`);
  - the server certificate must have an `authorityKeyIdentifier`.

  Most CAs already do this. Self-made certificates often don't: see [`gen-certs.sh`](../../example/nats-auth/gen-certs.sh) for a set of `openssl` options that passes. Fix the certificates rather than turning the check off.

## Run the example

```bash
make -C example up-jetstream clients
docker run --rm --network nats-examples -e NATS_URL=nats://nats-js:4222 -e COUNT=3 nats-example-python             # subscriber
docker run --rm --network nats-examples -e NATS_URL=nats://nats-js:4222 -e MODE=pub -e COUNT=3 nats-example-python # publisher, in a second terminal
```

Against the auth + TLS setup, as the `app` user:

```bash
source example/.env
docker run --rm --network nats-examples -v "$PWD/example/nats-auth/certs/ca.pem:/ca.pem:ro" \
  -e NATS_URL=tls://nats-auth:4222 -e NATS_CA_FILE=/ca.pem \
  -e NATS_USER=app -e NATS_PASSWORD="$APP_PASSWORD" \
  -e SUBJECT=app.demo -e MODE=pub nats-example-python
```

Without Docker: `cd example/clients/python && pip install -r requirements.txt && NATS_URL=nats://127.0.0.1:4222 python client.py`.
