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

# Copy the CA certificate from build context so changes bust cache
COPY ca.crt /opt/nommie/ssl/ca.crt
RUN chmod 644 /opt/nommie/ssl/ca.crt

# Copy certificate generation script
COPY generate-server-cert.sh /usr/local/bin/generate-server-cert.sh
RUN chmod +x /usr/local/bin/generate-server-cert.sh

# Generate server certificate during build (using build secrets)
# This script reads CA key from secrets and CA cert from the copied file
RUN --mount=type=secret,id=nommie_ca_key \
    /usr/local/bin/generate-server-cert.sh

# Copy entrypoint script that handles volume persistence
COPY entrypoint-ssl.sh /usr/local/bin/entrypoint-ssl.sh
RUN chmod +x /usr/local/bin/entrypoint-ssl.sh

# Copy database initialization script
COPY init.sh /docker-entrypoint-initdb.d/00-init.sh
RUN chmod +x /docker-entrypoint-initdb.d/00-init.sh

# Set permissions on server certs (they'll be copied to volume on first run)
RUN chown -R postgres:postgres /opt/nommie/ssl && \
    chmod 600 /opt/nommie/ssl/server.key && \
    chmod 644 /opt/nommie/ssl/server.crt

ENTRYPOINT ["/usr/local/bin/entrypoint-ssl.sh"]
CMD ["postgres"]

