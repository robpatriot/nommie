FROM caddy:2.10.2-builder AS builder

# Allow Go to automatically fetch a newer toolchain if needed
ENV GOTOOLCHAIN=auto

# Build Caddy with the Cloudflare DNS plugin for ACME DNS-01
RUN xcaddy build \
    --with github.com/caddy-dns/cloudflare

FROM caddy:2.10.2

# Use the custom-built Caddy binary
COPY --from=builder /usr/bin/caddy /usr/bin/caddy

# Default Caddyfile path (will be overridden if you also mount it via volumes)
COPY Caddyfile /etc/caddy/Caddyfile

