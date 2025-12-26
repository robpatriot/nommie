FROM caddy:2.10.2-builder AS builder

# Allow Go to automatically fetch a newer toolchain if needed
ENV GOTOOLCHAIN=auto

# Build Caddy with the Cloudflare DNS plugin for ACME DNS-01
RUN xcaddy build \
    --with github.com/caddy-dns/cloudflare

FROM caddy:2.10.2

# Use the custom-built Caddy binary
COPY --from=builder /usr/bin/caddy /usr/bin/caddy

# Copy Caddyfile template and entrypoint script
COPY Caddyfile.template /etc/caddy/Caddyfile.template
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["caddy", "run", "--config", "/etc/caddy/Caddyfile", "--adapter", "caddyfile"]

