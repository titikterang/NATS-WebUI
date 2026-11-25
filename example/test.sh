#!/usr/bin/env bash
# Round-trip test for the example setups and clients:   make -C example test
#
# For each running setup: node, python and go subscribers start first, then node, python and go
# publishers each send COUNT messages. Every subscriber must receive all 3 x COUNT messages, which
# proves cross-language interop (and cross-node routing on the cluster). The auth setup also
# checks that a wrong password is rejected.
set -uo pipefail
cd "$(dirname "$0")"
[[ -f .env ]] && set -a && source .env && set +a

COUNT=5
NETWORK=nats-examples
CLIENTS=(node python go)
pass=0
fail=0

ok() { echo "  PASS $*"; pass=$((pass + 1)); }
ko() { echo "  FAIL $*"; fail=$((fail + 1)); }

running() { docker compose -f "$1/compose.yaml" ps --status running -q 2>/dev/null | grep -q .; }

# run <name> <env...>: runs a client container on the examples network
client() {
  local lang=$1; shift
  docker run --rm --network "$NETWORK" "${extra[@]}" "$@" "nats-example-$lang"
}

roundtrip() { # <label> <sub-url> <pub-url> <subject> [env...]
  local label=$1 sub_url=$2 pub_url=$3 subject=$4; shift 4
  local total=$((COUNT * ${#CLIENTS[@]})) logs pids=()
  logs=$(mktemp -d)
  for lang in "${CLIENTS[@]}"; do
    client "$lang" -e MODE=sub -e NATS_URL="$sub_url" -e SUBJECT="$subject" -e COUNT="$total" -e TIMEOUT_SECONDS=30 "$@" \
      >"$logs/sub-$lang.log" 2>&1 &
    pids+=($!)
  done
  # Publish only after every subscriber has confirmed its subscription with the server.
  for lang in "${CLIENTS[@]}"; do
    for _ in $(seq 1 60); do
      grep -q "^$lang: ready" "$logs/sub-$lang.log" 2>/dev/null && break
      sleep 0.5
    done
  done
  for lang in "${CLIENTS[@]}"; do
    if client "$lang" -e MODE=pub -e NATS_URL="$pub_url" -e SUBJECT="$subject" -e COUNT="$COUNT" "$@" >"$logs/pub-$lang.log" 2>&1; then
      ok "$label: $lang published $COUNT"
    else
      ko "$label: $lang publish ($(tail -1 "$logs/pub-$lang.log"))"
    fi
  done
  for i in "${!CLIENTS[@]}"; do
    lang=${CLIENTS[$i]}
    if wait "${pids[$i]}"; then
      from=$(grep -oE '\[(node|python|go)\]' "$logs/sub-$lang.log" | sort -u | tr -d '[]' | tr '\n' ' ')
      ok "$label: $lang received $total (from: $from)"
    else
      ko "$label: $lang subscriber ($(tail -1 "$logs/sub-$lang.log"))"
    fi
  done
  rm -rf "$logs"
}

extra=()
if running nats-jetstream; then
  echo "== nats-jetstream"
  roundtrip jetstream nats://nats-js:4222 nats://nats-js:4222 demo.hello
else
  echo "== nats-jetstream: not running (make -C example up-jetstream), skipped"
fi

if running nats-cluster; then
  echo "== nats-cluster (subscribe on nats-1, publish on nats-3)"
  roundtrip cluster nats://nats-1:4222 nats://nats-3:4222 demo.cluster
else
  echo "== nats-cluster: not running (make -C example up-cluster), skipped"
fi

if running nats-auth; then
  echo "== nats-auth (TLS + user app)"
  extra=(-v "$PWD/nats-auth/certs/ca.pem:/ca.pem:ro")
  auth=(-e NATS_CA_FILE=/ca.pem -e NATS_USER=app -e NATS_PASSWORD="${APP_PASSWORD:-}")
  roundtrip auth tls://nats-auth:4222 tls://nats-auth:4222 app.demo "${auth[@]}"
  if client go -e MODE=pub -e NATS_URL=tls://nats-auth:4222 -e NATS_CA_FILE=/ca.pem -e NATS_USER=app \
    -e NATS_PASSWORD=wrong-password -e COUNT=1 >/dev/null 2>&1; then
    ko "auth: wrong password was accepted"
  else
    ok "auth: wrong password rejected"
  fi
else
  echo "== nats-auth: not running (make -C example up-auth), skipped"
fi

echo "== $pass passed, $fail failed"
[[ $fail -eq 0 && $pass -gt 0 ]]
