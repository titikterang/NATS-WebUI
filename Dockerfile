# syntax=docker/dockerfile:1.7
#
# Targets:
#   runtime (default)  distroless, non-root, no shell. This is what you deploy.
#   debug              Debian slim + network/debug tools (curl, dig, nc, tcpdump, openssl, sqlite3,
#                      nats CLI). For local troubleshooting only; don't push it as a release.
#
#   docker build -t nats-webui .                  # runtime
#   docker build --target debug -t nats-webui:debug .
#
# The app always listens on 8080 inside the container (non-root can't bind 80). Map any host port.

ARG RUST_VERSION=1
ARG NODE_VERSION=26

# ---- backend ----------------------------------------------------------------------------------
FROM rust:${RUST_VERSION}-trixie AS backend
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked \
 && install -D target/release/nats-webui /out/nats-webui \
 && install -D "$(gcc -print-file-name=libgcc_s.so.1)" /out/lib/libgcc_s.so.1 \
 && mkdir -p /out/data

# ---- frontend ---------------------------------------------------------------------------------
FROM node:${NODE_VERSION}-trixie-slim AS frontend
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci --no-audit --no-fund
COPY web ./
RUN npm run build

# ---- debug ------------------------------------------------------------------------------------
FROM debian:trixie-slim AS debug
ARG NATSCLI_VERSION=0.5.2
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates curl dnsutils iproute2 iputils-ping jq netcat-openbsd openssl procps sqlite3 tcpdump unzip \
 && rm -rf /var/lib/apt/lists/*
# nats CLI, verified against the release's SHA256SUMS.
RUN set -eu; arch="$(dpkg --print-architecture)"; zip="nats-${NATSCLI_VERSION}-linux-${arch}.zip"; \
    base="https://github.com/nats-io/natscli/releases/download/v${NATSCLI_VERSION}"; \
    cd /tmp && curl -fsSLO "${base}/${zip}" && curl -fsSL "${base}/SHA256SUMS" -o SHA256SUMS; \
    grep " ${zip}\$" SHA256SUMS | sha256sum -c -; \
    unzip -j "${zip}" '*/nats' -d /usr/local/bin && rm -f /tmp/*
RUN useradd --uid 65532 --no-create-home --shell /usr/sbin/nologin nonroot
COPY --from=backend /out/nats-webui /app/nats-webui
COPY --from=backend --chown=65532:65532 /out/data /data
COPY --from=frontend /web/dist /app/web/dist
ENV BIND_ADDR=0.0.0.0:8080 DATABASE_URL=/data/sqlite.db DATABASE_MAX_MB=256 STATIC_DIR=/app/web/dist RUST_LOG=info LOG_FORMAT=text
USER 65532:65532
EXPOSE 8080
VOLUME /data
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD ["/app/nats-webui", "healthcheck"]
ENTRYPOINT ["/app/nats-webui"]

# ---- runtime (default) ------------------------------------------------------------------------
# glibc + CA certificates only: no shell, package manager, OpenSSL or OS userland to patch.
# The binary links only libc, libm and libgcc_s (TLS is rustls/ring), so the OpenSSL that the
# distroless `cc` image ships is dead weight that only adds CVEs. libgcc_s comes from the
# Debian 13 builder (same glibc as this Debian 13 base).
FROM gcr.io/distroless/base-nossl-debian13:nonroot AS runtime
COPY --from=backend /out/lib/libgcc_s.so.1 /usr/lib/libgcc_s.so.1
COPY --from=backend /out/nats-webui /app/nats-webui
# A /data owned by the nonroot user, so a fresh named volume is writable.
COPY --from=backend --chown=65532:65532 /out/data /data
COPY --from=frontend /web/dist /app/web/dist
# JSON logs by default for log aggregation; set LOG_FORMAT=text for humans.
ENV BIND_ADDR=0.0.0.0:8080 DATABASE_URL=/data/sqlite.db DATABASE_MAX_MB=256 STATIC_DIR=/app/web/dist RUST_LOG=info LOG_FORMAT=json
USER 65532:65532
EXPOSE 8080
VOLUME /data
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD ["/app/nats-webui", "healthcheck"]
ENTRYPOINT ["/app/nats-webui"]
