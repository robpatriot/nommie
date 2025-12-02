#!/usr/bin/env bash
set -euo pipefail

# Entrypoint script for Postgres TLS image.
# This script:
# 1. Copies baked server certificates from /opt/nommie/ssl/ into $PGDATA/ssl/
#    on first run (so certs persist in the data volume)
# 2. Configures postgresql.conf with SSL settings
# 3. Delegates to the official Postgres entrypoint

BAKED_CERT_DIR="/opt/nommie/ssl"
VOLUME_CERT_DIR="${PGDATA}/ssl"
SERVER_KEY="${VOLUME_CERT_DIR}/server.key"
SERVER_CERT="${VOLUME_CERT_DIR}/server.crt"
CA_CERT="${VOLUME_CERT_DIR}/ca.crt"

# Ensure volume cert directory exists
mkdir -p "${VOLUME_CERT_DIR}"

# Copy server certificates from baked location to volume on first run
# This ensures certs persist across image rebuilds
if [ ! -f "${SERVER_KEY}" ] || [ ! -f "${SERVER_CERT}" ]; then
    echo "Copying server certificates from image to data volume..."
    
    if [ ! -f "${BAKED_CERT_DIR}/server.key" ] || [ ! -f "${BAKED_CERT_DIR}/server.crt" ]; then
        echo "ERROR: Baked server certificates not found in ${BAKED_CERT_DIR}" >&2
        exit 1
    fi
    
    cp "${BAKED_CERT_DIR}/server.key" "${SERVER_KEY}"
    cp "${BAKED_CERT_DIR}/server.crt" "${SERVER_CERT}"
    
    # Set ownership and permissions
    chown postgres:postgres "${SERVER_KEY}" "${SERVER_CERT}"
    chmod 600 "${SERVER_KEY}"
    chmod 644 "${SERVER_CERT}"
    
    echo "Server certificates copied to ${VOLUME_CERT_DIR}"
else
    echo "Server certificates already exist in volume, reusing them"
fi

# Copy CA cert to volume (for reference, though we'll also use the baked one)
if [ ! -f "${CA_CERT}" ]; then
    if [ -f "${BAKED_CERT_DIR}/ca.crt" ]; then
        cp "${BAKED_CERT_DIR}/ca.crt" "${CA_CERT}"
        chown postgres:postgres "${CA_CERT}"
        chmod 644 "${CA_CERT}"
    fi
fi

# Configure SSL in postgresql.conf if not already configured
if ! grep -q "^ssl[[:space:]]*=" "${PGDATA}/postgresql.conf" 2>/dev/null; then
    echo "Configuring Postgres SSL in postgresql.conf..."
    cat >> "${PGDATA}/postgresql.conf" <<EOF

# SSL configuration (added by entrypoint-ssl.sh)
ssl = on
ssl_cert_file = '${SERVER_CERT}'
ssl_key_file = '${SERVER_KEY}'
EOF
    
    # Add CA cert if available (optional, but useful for client cert verification)
    if [ -f "${CA_CERT}" ]; then
        echo "ssl_ca_file = '${CA_CERT}'" >> "${PGDATA}/postgresql.conf"
    fi
    
    echo "SSL configuration added to postgresql.conf"
else
    echo "SSL already configured in postgresql.conf"
fi

# Delegate to official Postgres entrypoint
exec /usr/local/bin/docker-entrypoint.sh "$@"

