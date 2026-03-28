# syntax=docker/dockerfile:1.6

# -----------------------------------------------------------------------------
# Base image update guidance:
# - This Dockerfile intentionally uses mutable tags (rust:trixie, debian:trixie-slim).
# - Rebuild with `--pull` to refresh base layers:
#     docker build --pull -t <image>:<tag> .
# - Pin digests only if you need strict reproducibility/auditability.
# -----------------------------------------------------------------------------

############################
# Build stage
############################
FROM rust:trixie AS builder

ENV CARGO_TARGET_DIR=/src/target

# Optional pin for cargo-leptos. Leave empty to install latest locked release.
ARG CARGO_LEPTOS_VERSION=""

RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libsqlite3-dev \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# cargo-leptos builds SSR binary + hydrated frontend assets (target/site)
RUN set -eux; \
    if [ -n "${CARGO_LEPTOS_VERSION}" ]; then \
      cargo install --locked cargo-leptos --version "${CARGO_LEPTOS_VERSION}"; \
    else \
      cargo install --locked cargo-leptos; \
    fi

RUN rustup target add wasm32-unknown-unknown --toolchain nightly

WORKDIR /src
COPY . .

# Build app + supporting operational binaries
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    set -eux; \
    cargo leptos build --release; \
    cargo build -p migration --release --locked; \
    cargo build -p leptos-auth-template-community \
      --bin seed_admin \
      --features ssr \
      --no-default-features \
      --release \
      --locked; \
    mkdir -p /out/bin /out/site; \
    cp /src/target/release/leptos-auth-template-community /out/bin/app; \
    cp /src/target/release/migration /out/bin/migration; \
    cp /src/target/release/seed_admin /out/bin/seed_admin; \
    cp -R /src/target/site/. /out/site/; \
    cp -R /src/crates/app/public/. /out/site/

############################
# Runtime stage
############################
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libsqlite3-0 \
    tini \
 && rm -rf /var/lib/apt/lists/*

# Non-root runtime user
RUN useradd --system --create-home --uid 10001 --shell /usr/sbin/nologin appuser

WORKDIR /app

COPY --from=builder /out/bin /app/bin
COPY --from=builder /out/site /app/site

RUN mkdir -p /data && \
    chown -R appuser:appuser /app /data

# Small boot wrapper:
# - optional migrations on startup (RUN_MIGRATIONS=1)
# - optional admin seed on startup (SEED_ON_BOOT=1 + SEED_ALLOW_ADMIN=1)
RUN set -eux; \
    cat > /app/entrypoint.sh <<'SH'
#!/usr/bin/env sh
set -eu

if [ "${RUN_MIGRATIONS:-1}" = "1" ]; then
  /app/bin/migration up
fi

if [ "${SEED_ON_BOOT:-0}" = "1" ]; then
  /app/bin/seed_admin
fi

exec /app/bin/app
SH

RUN chmod +x /app/entrypoint.sh /app/bin/*

USER appuser:appuser

EXPOSE 3000
VOLUME ["/data"]

# App defaults (override as needed)
ENV DATABASE_URL="sqlite:///data/app.sqlite?mode=rwc" \
    LEPTOS_SITE_ROOT="site" \
    LEPTOS_SITE_ADDR="0.0.0.0:3000" \
    LEPTOS_ENV="production" \
    APP_BASE_URL="https://example.com" \
    WEBAUTHN_RP_ORIGIN="http://localhost:3000" \
    WEBAUTHN_RP_ID="localhost" \
    WEBAUTHN_RP_NAME="example-app" \
    RUST_LOG="info"

ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["/app/entrypoint.sh"]
