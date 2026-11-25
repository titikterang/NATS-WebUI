# Local development

Run the WebUI from source with hot reload, run the quality gate, and find where a change belongs. It takes about five minutes on a machine with the prerequisites installed.

## Prerequisites

| Tool | Version | Needed for |
|---|---|---|
| Rust | stable (edition 2021) | Backend |
| Node.js | 24 or newer | Frontend |
| GNU Make, bash | any recent | All commands |
| Docker with Compose v2 | any recent | Optional: the compose stack, images, the [example NATS setups](../example/README.md) |

## Run it

1. Clone and create your settings file:

   ```bash
   git clone <your fork URL> nats-webui && cd nats-webui
   cp .env.example .env
   ```

2. Install the dependencies:

   ```bash
   make install         # cargo fetch + npm ci
   ```

3. Start a NATS server to talk to, if you don't have one. The [examples](../example/README.md) give you one in a few seconds:

   ```bash
   cp example/.env.example example/.env     # set your own passwords
   make -C example up-jetstream             # nats://127.0.0.1:4222, monitoring :8222
   ```

4. Start the backend and the frontend together:

   ```bash
   make dev
   ```

   | Process | Address | Reloads on change |
   |---|---|---|
   | Backend (`cargo run`) | http://127.0.0.1:8080 | No: stop with Ctrl-C and run `make dev` again |
   | Vite dev server | **http://localhost:5173** | Yes, instantly |

   Vite proxies `/api`, `/client` and `/healthz` to the backend, WebSockets included. Ctrl-C stops both.

5. Open http://localhost:5173 and add the server: host `127.0.0.1`, NATS port `4222`, monitoring port `8222`, no username. See [Connecting NATS](connecting-nats.md) for auth and TLS.

Run the halves separately with `make dev-backend` and `make dev-frontend`. To point Vite at another backend, set `NATS_WEBUI_BACKEND=http://host:port` before `make dev-frontend`.

To use a server with a private CA, pass the CA on the command line (see [precedence](configuration.md#precedence)):

```bash
make dev NATS_CA_FILE=$PWD/example/nats-auth/certs/ca.pem
```

## Run the gate before every commit

```bash
make check
```

`make check` runs the following, and any warning fails it:

| Step | Command |
|---|---|
| Rust formatting | `cargo fmt --check` |
| Rust lints | `cargo clippy --all-targets --locked -- -D warnings` |
| Frontend lints, including the layer rules | `npx eslint . --max-warnings 0` |
| Frontend types | `vue-tsc --noEmit` |
| All unit tests | `cargo test --locked` and `vitest run` |

`make fmt` fixes formatting and the lint issues ESLint can fix.

The unit tests are the project's source of truth: they pin the domain rules, the API contract and the logging guarantees (no password in any log line). Add or change a test with every behaviour change, and run a narrower set while you work:

```bash
cargo test domain::client                 # one Rust module
cargo test -- --nocapture                 # show output
cd web && npx vitest run src/domain       # one frontend folder
cd web && npm run test:watch              # re-run on save
```

## Where a change belongs

The layers and their rules are described in [Architecture](architecture.md). For the usual changes:

| You want to… | Change |
|---|---|
| Add or change a validation rule | `src/domain/` (+ a test there). If the UI validates the same thing, mirror it in `web/src/domain/` (for example `subject.ts` ↔ `SubjectPattern`). |
| Add a backend use case | A service in `src/application/`, using the port traits in `ports.rs`. Test it with the fakes in `application/fakes.rs`. |
| Talk to a new external system | A new port trait in `ports.rs`, an adapter in `src/infrastructure/`, wired in `main.rs`. |
| Add an endpoint | A handler in `src/presentation/http/handlers/`, the wire type in `dto.rs`, the route in `mod.rs`'s route table comment, then the client function in `web/src/infrastructure/api.ts` and the type in `web/src/domain/types.ts`. |
| Change the database | Append a migration to `src/infrastructure/sqlite/migrations.rs`. Only add; never edit a released migration. |
| Add a page | `web/src/features/<area>/`, plus a route in `web/src/app/router.ts`. Every screen gets a URL. |
| Reuse UI | `web/src/shared/ui/` for components, `web/src/composables/` for stateful logic. |

### Logging in new code

Follow the conventions at the top of `src/logging.rs` (summarized in [Architecture → Logging](architecture.md#logging)): structured fields instead of prose, `warn` for trouble outside the process, nothing per message, and every credential wrapped in `Secret`. Read logs as JSON with `make dev-backend LOG_FORMAT=json`, and increase detail with `make dev-backend RUST_LOG=nats_webui=debug,info`.

## Run in containers

The compose stack runs the WebUI as it runs in production, with CPU, memory, process and disk limits. It needs `NATS_CA_HOST_PATH` in `.env` (any PEM CA file works when you only use plain servers):

```bash
make up        # build and start on http://127.0.0.1:8080
make ps        # health and live resource usage
make logs
make down      # stop; data stays in the webui-data volume
```

The stack uses the `debug` image, which adds curl, dig, nc, tcpdump, openssl, sqlite3 and the `nats` CLI. `make debug-shell` opens a shell with those tools in the WebUI's network namespace, which is the fastest way to check what the container can reach:

```bash
make debug-shell          # with the stack running (make up)
nc -zv nats.example.internal 4222
curl -s http://nats.example.internal:8222/varz | head
```

How the limits work, and how to run in production, is in [Deployment](deployment.md).

## Security scan

`make scan` runs the same Semgrep CE and Trivy checks as the pull-request workflow, from public Docker images. It scans a clean copy of the files git would commit, so `.env`, `sandbox/` and local databases never reach a scanner.

```bash
make docker-build TAG=dev
make scan IMAGES="nats-webui:dev"
```

Add `SCAN_MODE=enforce` to fail like CI. What's checked, what blocks and how to handle a finding are in [Security scanning](security-scanning.md).

## Troubleshooting

| Symptom | Cause and fix |
|---|---|
| `make dev` ignores `NATS_CA_FILE=… make dev` | `.env` wins over shell variables. Use `make dev NATS_CA_FILE=…`. |
| Backend exits with `BIND_ADDR must be host:port` or `DATABASE_MAX_MB must be a positive number` | Fix the value in `.env`; see [Configuration](configuration.md). |
| `Address already in use` on 8080 or 5173 | Another process holds the port. Change `BIND_ADDR` (and `NATS_WEBUI_BACKEND` for Vite), or stop the other process. |
| UI on :5173 shows no servers and API calls fail | The backend isn't running or crashed. Check the `make dev` output for the backend's error. |
| `make up` fails with `Set NATS_CA_HOST_PATH in .env` | Compose needs a CA file to mount; set it in `.env`. |
| Server shows "Auth failed", "Error" or monitor "Unreachable" | See [Connecting NATS → Troubleshooting](connecting-nats.md#troubleshooting). |
