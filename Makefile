# NATS-WebUI: one entry point for local development, checks, images and scans.
# Settings come from .env (cp .env.example .env); any variable can also be overridden:
#   make docker-push TAG=v0.3.0 REGISTRY=registry.example.internal:5000

-include .env
export

SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c
.DEFAULT_GOAL := help

IMAGE           ?= nats-webui
TAG             ?= $(shell git describe --tags --always --dirty 2>/dev/null || echo dev)
REGISTRY        ?=
WEBUI_HOST_PORT ?= 8080
BIND_ADDR       ?= 127.0.0.1:8080

.PHONY: help
help: ## List targets
	@awk 'BEGIN {FS = ":.*## "} /^## / {printf "\n\033[1m%s\033[0m\n", substr($$0, 4)} /^[a-zA-Z_-]+:.*## / {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

## Local development (no Docker)
.PHONY: install dev dev-backend dev-frontend build run
install: ## Install Rust and npm dependencies
	cargo fetch --locked
	cd web && npm ci --no-audit --no-fund

dev: ## Run backend (:8080) and Vite dev server (:5173) together; Ctrl-C stops both
	@trap 'kill 0' EXIT; $(MAKE) --no-print-directory dev-backend & $(MAKE) --no-print-directory dev-frontend & wait

dev-backend: ## Run the backend with cargo (reads .env)
	cargo run

dev-frontend: ## Run the Vite dev server with hot reload (proxies to the backend)
	cd web && npm run dev

build: ## Build the release binary and the frontend bundle
	cargo build --release --locked
	cd web && npm run build

run: build ## Build, then run the release binary serving web/dist
	./target/release/nats-webui

## Quality gate (run before every commit)
.PHONY: check test lint typecheck fmt
check: lint typecheck test ## Everything below: the gate
	@echo "check: OK"

test: ## Backend and frontend unit tests
	cargo test --locked
	cd web && npm test

lint: ## rustfmt check, clippy (warnings are errors), ESLint (warnings are errors)
	cargo fmt --check
	cargo clippy --all-targets --locked -- -D warnings
	cd web && npx eslint . --max-warnings 0

typecheck: ## vue-tsc
	cd web && npm run typecheck

fmt: ## Format Rust and fix lint issues ESLint can fix
	cargo fmt
	cd web && npx eslint . --fix

## Docker images
.PHONY: docker-build docker-build-debug docker-run docker-push
docker-build: ## Build the runtime (distroless) image as $(IMAGE):$(TAG)
	docker build --target runtime -t $(IMAGE):$(TAG) .

docker-build-debug: ## Build the debug image (network/debug tools) as $(IMAGE):debug
	docker build --target debug -t $(IMAGE):debug .

docker-run: ## Run $(IMAGE):$(TAG) on 127.0.0.1:$(WEBUI_HOST_PORT) (needs NATS_CA_HOST_PATH)
	@test -n "$(NATS_CA_HOST_PATH)" || { echo "Set NATS_CA_HOST_PATH in .env"; exit 1; }
	docker run --rm -p 127.0.0.1:$(WEBUI_HOST_PORT):8080 -v nats-webui-data:/data \
	  -v "$(NATS_CA_HOST_PATH)":/certs/nats-ca.crt:ro -e NATS_CA_FILE=/certs/nats-ca.crt $(IMAGE):$(TAG)

docker-push: ## Tag and push $(IMAGE):$(TAG) to $(REGISTRY) (PUSH_LATEST=1 also pushes :latest)
	@test -n "$(REGISTRY)" || { echo "Set REGISTRY in .env (e.g. registry.example.internal:5000)"; exit 1; }
	@case "$(TAG)" in *-dirty) test "$${ALLOW_DIRTY:-0}" = 1 || { echo "Refusing to push a -dirty tag ($(TAG)); commit first or set ALLOW_DIRTY=1"; exit 1; };; esac
	docker image inspect $(IMAGE):$(TAG) >/dev/null || $(MAKE) --no-print-directory docker-build
	docker tag $(IMAGE):$(TAG) $(REGISTRY)/$(IMAGE):$(TAG)
	docker push $(REGISTRY)/$(IMAGE):$(TAG)
	@if [ "$${PUSH_LATEST:-0}" = 1 ]; then docker tag $(IMAGE):$(TAG) $(REGISTRY)/$(IMAGE):latest && docker push $(REGISTRY)/$(IMAGE):latest; fi

## Docker compose (local stack with resource limits)
.PHONY: up down logs ps debug-shell
up: ## Build and start the stack in the background
	docker compose up -d --build
	@echo "UI: http://127.0.0.1:$(WEBUI_HOST_PORT)"

down: ## Stop the stack (keeps data; add -v to docker compose down to delete it)
	docker compose down

logs: ## Follow the webui logs
	docker compose logs -f webui

ps: ## Show containers, health and limits
	docker compose ps
	docker stats --no-stream $$(docker compose ps -q)

debug-shell: ## Shell with network tools in the webui network namespace (tcpdump/ping allowed)
	docker run --rm -it --network container:$$(docker compose ps -q webui) --cap-add NET_RAW --cap-add NET_ADMIN \
	  --entrypoint bash $(IMAGE):debug

## Security
.PHONY: scan
scan: ## Semgrep CE + Trivy, same as CI (clean copy; IMAGES="img:tag ...", SCAN_MODE=enforce)
	scripts/security-scan.sh $(IMAGES)

.PHONY: clean
clean: ## Remove build outputs (keeps databases and volumes)
	cargo clean
	rm -rf web/dist
