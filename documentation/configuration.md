# Configuration reference

Every setting the WebUI, the compose stack and the Makefile read. It's all set with environment variables; there are no config files. [`.env.example`](../.env.example) is the template: copy it to `.env` (git-ignored), and both `make` and `docker compose` read it automatically.

```bash
cp .env.example .env
```

## Backend

The backend binary reads these at startup. An invalid value stops it with a message naming the variable.

| Variable | Default | Purpose |
|---|---|---|
| `BIND_ADDR` | `0.0.0.0:8080` (`.env.example`: `127.0.0.1:8080`) | Listen address, `host:port`. The app never uses port 80; map any host port to 8080. |
| `DATABASE_URL` | `./sqlite.db` (image: `/data/sqlite.db`) | SQLite file that stores servers, clients and their credentials. Created if missing. |
| `DATABASE_MAX_MB` | `256` | Hard cap on the database size, in MiB. Writes beyond it fail with "database is full: DATABASE_MAX_MB reached", and the app keeps running. |
| `NATS_CA_FILE` | unset | PEM file with the CA(s) that signed your NATS server certificates. Leave it unset for servers without TLS or with publicly trusted certificates. See [Connecting NATS](connecting-nats.md#tls-with-a-private-ca). |
| `STATIC_DIR` | `web/dist` (image: `/app/web/dist`) | The built frontend that the backend serves. |
| `RUST_LOG` | `info` | Log filter, e.g. `debug` or `nats_webui=debug,info`. |
| `LOG_FORMAT` | `text` (image: `json`) | `text` for people, `json` (one object per line) for log aggregation. |

## Compose stack (`make up`)

[`compose.yaml`](../compose.yaml) reads these. The container itself always listens on `0.0.0.0:8080` and stores its database at `/data/sqlite.db`.

| Variable | Default | Purpose |
|---|---|---|
| `NATS_CA_HOST_PATH` | **required** | Host path of the CA certificate. It's mounted read-only at `/certs/nats-ca.crt`. |
| `WEBUI_HOST_PORT` | `8080` | Host port, bound to `127.0.0.1`. |
| `WEBUI_TARGET` | `debug` | Image target: `debug` (with network tools) or `runtime` (what production runs). |
| `WEBUI_CPUS` | `1.0` | CPU limit. |
| `WEBUI_MEM` | `256m` | Memory limit. Swap is disabled. |
| `WEBUI_PIDS` | `256` | Process and thread limit. |
| `WEBUI_DB_MAX_MB` | `256` | `DATABASE_MAX_MB` inside the container. |
| `UI_DEV_HOST_PORT` | `5173` | Port of the optional Vite container (`docker compose --profile ui-dev up`). |

`RUST_LOG` and `LOG_FORMAT` are passed through to the container.

## Make variables

These only affect `make` targets. Set them in `.env` or on the command line.

| Variable | Default | Used by |
|---|---|---|
| `IMAGE` | `nats-webui` | `docker-build`, `docker-push`, `docker-run` |
| `TAG` | `git describe --tags --always --dirty` | `docker-build`, `docker-push`, `docker-run` |
| `REGISTRY` | empty, **required** to push | `docker-push`, e.g. `registry.example.internal:5000` |
| `ALLOW_DIRTY` | `0` | `docker-push`: set `1` to push a `-dirty` tag (uncommitted work) |
| `PUSH_LATEST` | `0` | `docker-push`: set `1` to also push `:latest` |
| `SCAN_MODE` | `warn` | `scan`: `enforce` fails on blocking findings, as CI does. See [Security scanning](security-scanning.md). |
| `SEMGREP_IMAGE`, `TRIVY_IMAGE` | pinned in `scripts/security-scan.sh` | `scan`: override only to test a scanner upgrade |
| `IMAGES` | empty | `scan`: images to scan, e.g. `IMAGES="nats-webui:v0.3.0"` |

## Precedence

`make` loads `.env` with `-include .env`, so **`.env` wins over variables exported in your shell**. Pass one-off values on the make command line instead, because those win over `.env`:

```bash
make dev NATS_CA_FILE=/path/to/ca.pem      # works
NATS_CA_FILE=/path/to/ca.pem make dev      # ignored if .env sets NATS_CA_FILE
```

`docker compose` follows its own rules: shell variables win over `.env`.
