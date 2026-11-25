# Local NATS setups and sample clients.   make -C example help
-include .env
export

SHELL := bash
.SHELLFLAGS := -eu -o pipefail -c
.DEFAULT_GOAL := help
NETWORK := nats-examples

.PHONY: help
help: ## List targets
	@awk 'BEGIN {FS = ":.*## "} /^[a-zA-Z_-]+:.*## / {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)

.PHONY: network
network:
	@docker network inspect $(NETWORK) >/dev/null 2>&1 || docker network create $(NETWORK) >/dev/null

.PHONY: up-jetstream up-cluster up-auth up-all
up-jetstream: network ## Single node + JetStream      nats://127.0.0.1:4222
	docker compose -f nats-jetstream/compose.yaml up -d --wait

up-cluster: network ## 3-node JetStream cluster      nats://127.0.0.1:4231-4233
	docker compose -f nats-cluster/compose.yaml up -d --wait

up-auth: network certs ## JetStream + userpass + TLS  tls://127.0.0.1:4242 (needs example/.env)
	@test -f .env || { echo "cp example/.env.example example/.env and set the passwords"; exit 1; }
	docker compose --env-file .env -f nats-auth/compose.yaml up -d --wait

up-all: up-jetstream up-cluster up-auth ## Start all three setups

.PHONY: certs
certs: ## Generate the local CA and server certificate for nats-auth
	nats-auth/gen-certs.sh

.PHONY: clients
clients: ## Build the node, python and go client images
	docker build -q -t nats-example-node clients/node >/dev/null
	docker build -q -t nats-example-python clients/python >/dev/null
	docker build -q -t nats-example-go clients/go >/dev/null
	@echo "built nats-example-{node,python,go}"

.PHONY: test
test: clients ## Round-trip every client against every running setup
	./test.sh

.PHONY: down-all
down-all: ## Stop all setups and delete their data volumes
	docker compose -f nats-jetstream/compose.yaml down -v
	docker compose -f nats-cluster/compose.yaml down -v
	docker compose --env-file .env.example -f nats-auth/compose.yaml down -v
	@docker network rm $(NETWORK) >/dev/null 2>&1 || true
