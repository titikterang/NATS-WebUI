"""Minimal NATS producer/consumer (nats-py).

MODE=pub|sub  NATS_URL  NATS_USER  NATS_PASSWORD  NATS_CA_FILE  SUBJECT  COUNT  TIMEOUT_SECONDS
"""

import asyncio
import json
import os
import ssl
import sys
from datetime import datetime, timezone

import nats


def env(key: str, default: str = "") -> str:
    return os.environ.get(key) or default


async def main() -> int:
    mode = env("MODE", "sub")
    subject = env("SUBJECT", "demo.hello")
    count = int(env("COUNT", "5"))
    timeout = float(env("TIMEOUT_SECONDS", "30"))

    tls = None
    if ca := env("NATS_CA_FILE"):
        tls = ssl.create_default_context(cafile=ca)

    async def on_error(e: Exception) -> None:
        print(f"python: error: {e}", file=sys.stderr, flush=True)

    nc = await nats.connect(
        servers=env("NATS_URL", "nats://127.0.0.1:4222").split(","),
        user=env("NATS_USER") or None,
        password=env("NATS_PASSWORD") or None,
        tls=tls,
        name=f"example-python-{mode}",
        error_cb=on_error,
        connect_timeout=5,
        # Fail fast instead of retrying a bad handshake or password for minutes.
        max_reconnect_attempts=3,
    )
    print(f"python: connected to {nc.connected_url.netloc} as {env('NATS_USER', '(no auth)')}", flush=True)

    if mode == "pub":
        for n in range(1, count + 1):
            body = json.dumps({"client": "python", "n": n, "at": datetime.now(timezone.utc).isoformat()})
            await nc.publish(subject, body.encode(), headers={"X-Example-Client": "python"})
        await nc.flush()
        print(f"python: published {count} to {subject}", flush=True)
        await nc.drain()
        return 0

    received = 0
    done = asyncio.Event()

    async def on_msg(msg):
        nonlocal received
        received += 1
        client = (msg.headers or {}).get("X-Example-Client", "-")
        print(f"python: {msg.subject} [{client}] {msg.data.decode()}", flush=True)
        if received >= count:
            done.set()

    await nc.subscribe(subject, cb=on_msg)
    await nc.flush()  # the server has the subscription once the flush round-trips
    print("python: ready", flush=True)
    try:
        await asyncio.wait_for(done.wait(), timeout)
    except asyncio.TimeoutError:
        print(f"python: timeout, received {received}/{count}", file=sys.stderr, flush=True)
        await nc.drain()
        return 1
    print(f"python: received {count} on {subject}", flush=True)
    await nc.drain()
    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
