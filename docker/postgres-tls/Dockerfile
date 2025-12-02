FROM postgres:16-alpine

# This image builds Postgres with TLS support using a private CA.
# Server certificates are generated at build time using Docker build secrets.
#
# Build with:
#   docker build \
#     --secret id=nommie_ca_key,src=/path/to/ca.key \
#     --secret id=nommie_ca_crt,src=/path/to/ca.crt \
#     -t nommie-postgres-tls:latest \
#     docker/postgres-tls
#
# The CA key is only available during build and is NOT included in the final image.
# Server certs are baked into the image at /opt/nommie/ssl/ and will be copied
# into the Postgres data volume on first container start.

# Install openssl for certificate generation
RUN apk add --no-cache openssl

# Create directory for baked certificates
RUN mkdir -p /opt/nommie/ssl && \
    chmod 755 /opt/nommie/ssl

# Copy certificate generation script
COPY generate-server-cert.sh /usr/local/bin/generate-server-cert.sh
RUN chmod +x /usr/local/bin/generate-server-cert.sh

# Generate server certificate during build (using build secrets)
# This script reads CA key/cert from /run/secrets/ and generates server.key/server.crt
RUN --mount=type=secret,id=nommie_ca_key \
    --mount=type=secret,id=nommie_ca_crt \
    /usr/local/bin/generate-server-cert.sh

# Copy CA public certificate into image (safe to include, it's public)
RUN --mount=type=secret,id=nommie_ca_crt \
    cp /run/secrets/nommie_ca_crt /opt/nommie/ssl/ca.crt && \
    chmod 644 /opt/nommie/ssl/ca.crt

# Copy entrypoint script that handles volume persistence
COPY entrypoint-ssl.sh /usr/local/bin/entrypoint-ssl.sh
RUN chmod +x /usr/local/bin/entrypoint-ssl.sh

# Set permissions on server certs (they'll be copied to volume on first run)
RUN chown -R postgres:postgres /opt/nommie/ssl && \
    chmod 600 /opt/nommie/ssl/server.key && \
    chmod 644 /opt/nommie/ssl/server.crt

ENTRYPOINT ["/usr/local/bin/entrypoint-ssl.sh"]
CMD ["postgres"]

