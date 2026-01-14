# syntax=docker/dockerfile:1.6

FROM rust:1.91.1 AS chef

WORKDIR /app

# Build deps (OpenSSL, pkg-config) + cargo-chef tool
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install cargo-chef

# ---- planner: compute dependency recipe (fast) ----
FROM chef AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---- builder: cook deps from recipe, then build real bins ----
FROM chef AS builder
WORKDIR /app

# Cook dependencies (cached by recipe.json content)
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo chef cook --release --recipe-path recipe.json

# Copy full source and build actual binaries
COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --locked --release --bin backend --manifest-path apps/backend/Cargo.toml

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --locked --release --bin migration --manifest-path apps/migration-cli/Cargo.toml

# ---- runtime: unchanged payload ----
FROM rust:1.91.1-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 appuser \
    && mkdir -p /app \
    && chown -R appuser:appuser /app

COPY docker/postgres-tls/ca.crt /etc/ssl/certs/nommie-ca.crt
RUN chmod 644 /etc/ssl/certs/nommie-ca.crt

USER appuser
WORKDIR /app
EXPOSE 3001

COPY --from=builder /app/target/release/backend /usr/local/bin/backend
COPY --from=builder /app/target/release/migration /usr/local/bin/migration-cli

CMD ["backend"]

