# syntax=docker/dockerfile:1.6

FROM rust:1.91.1 AS builder

WORKDIR /app

# Install dependencies required for building OpenSSL-dependent crates
RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifest files first to maximize build caching
COPY Cargo.toml Cargo.lock ./
COPY apps/backend/Cargo.toml apps/backend/Cargo.toml
COPY apps/migration-cli/Cargo.toml apps/migration-cli/Cargo.toml
COPY packages/migration/Cargo.toml packages/migration/Cargo.toml
COPY packages/backend-test-support/Cargo.toml packages/backend-test-support/Cargo.toml
COPY packages/db-infra/Cargo.toml packages/db-infra/Cargo.toml

# Copy library package sources (they're dependencies, need real code)
COPY packages/migration/src packages/migration/src
COPY packages/backend-test-support/src packages/backend-test-support/src
COPY packages/db-infra/src packages/db-infra/src

# Create dummy source files only for binaries to cache dependency compilation
RUN mkdir -p apps/backend/src \
    && echo "fn main() {}" > apps/backend/src/main.rs \
    && mkdir -p apps/migration-cli/src \
    && echo "fn main() {}" > apps/migration-cli/src/main.rs

RUN cargo build --release --manifest-path apps/backend/Cargo.toml

# Remove dummy sources before copying the real ones
RUN rm -rf apps/backend/src \
    && rm -rf apps/migration-cli/src

# Copy the full workspace
COPY . .

# Build the actual backend binary
RUN cargo build --locked --release --manifest-path apps/backend/Cargo.toml

# Build migration-cli binary
RUN cargo build --locked --release --manifest-path apps/migration-cli/Cargo.toml

FROM rust:1.91.1-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    # Create non-root runtime user and set up app directory
    && useradd -m -u 1000 appuser \
    && mkdir -p /app \
    && chown -R appuser:appuser /app

# Copy CA certificate for Postgres TLS verification before switching users
# The CA cert is public and safe to include in the image
# /etc/ssl/certs/ exists in the base image (ca-certificates package creates it)
COPY docker/postgres-tls/ca.crt /etc/ssl/certs/nommie-ca.crt
RUN chmod 644 /etc/ssl/certs/nommie-ca.crt

USER appuser

WORKDIR /app

# EXPOSE documents the default port (3001) for documentation purposes only
# The actual runtime port is controlled by BACKEND_PORT environment variable
# If BACKEND_PORT differs from 3001, use that value for docker run -p mappings
EXPOSE 3001

COPY --from=builder /app/target/release/backend /usr/local/bin/backend
COPY --from=builder /app/target/release/migration /usr/local/bin/migration-cli

CMD ["backend"]

