#!/usr/bin/env bash
set -euo pipefail

# Entrypoint script for Postgres TLS image.
# This script:
# 1. Copies baked server certificates from /opt/nommie/ssl/ into a separate SSL volume
#    on first run (so certs persist across image rebuilds)
# 2. Configures postgresql.conf with SSL settings
# 3. Delegates to the official Postgres entrypoint

BAKED_CERT_DIR="/opt/nommie/ssl"
VOLUME_CERT_DIR="/var/lib/postgresql/ssl"
SERVER_KEY="${VOLUME_CERT_DIR}/server.key"
SERVER_CERT="${VOLUME_CERT_DIR}/server.crt"
CA_CERT="${VOLUME_CERT_DIR}/ca.crt"
VOLUME_METADATA="${VOLUME_CERT_DIR}/cert.meta"
BAKED_METADATA="${BAKED_CERT_DIR}/cert.meta"
SSL_RENEWAL_THRESHOLD_SECONDS="${SSL_RENEWAL_THRESHOLD_SECONDS:-2592000}" # 30 days
FORCE_SSL_REFRESH="${FORCE_SSL_REFRESH:-false}"

read_metadata() {
    local file="$1"
    local prefix="$2"

    while IFS='=' read -r key value; do
        case "${key}" in
            ''|\#*) continue ;;
        esac
        # shellcheck disable=SC2086 # intentional dynamic variable
        printf -v "${prefix}_${key}" '%s' "${value}"
    done < "${file}"
}

cleanup_baked_artifacts() {
    if [ -d "${BAKED_CERT_DIR}" ]; then
        find "${BAKED_CERT_DIR}" -mindepth 1 -maxdepth 1 -exec rm -rf {} +
    fi
}

# Create SSL cert directory (in separate volume, doesn't affect $PGDATA)
mkdir -p "${VOLUME_CERT_DIR}"

baked_bundle_ready="false"
if [ -f "${BAKED_CERT_DIR}/server.key" ] && \
   [ -f "${BAKED_CERT_DIR}/server.crt" ] && \
   [ -f "${BAKED_CERT_DIR}/ca.crt" ] && \
   [ -f "${BAKED_METADATA}" ]; then
    baked_bundle_ready="true"
fi

needs_copy="false"

if [ "${FORCE_SSL_REFRESH}" = "true" ]; then
    echo "FORCE_SSL_REFRESH enabled; refreshing TLS assets"
    needs_copy="true"
fi

if [ "${needs_copy}" = "false" ]; then
    if [ ! -f "${SERVER_KEY}" ] || [ ! -f "${SERVER_CERT}" ]; then
        needs_copy="true"
    elif [ ! -f "${VOLUME_METADATA}" ]; then
        if [ "${baked_bundle_ready}" = "true" ]; then
            needs_copy="true"
        else
            echo "WARNING: SSL metadata missing from volume and no baked bundle available" >&2
        fi
    fi
fi

if [ "${needs_copy}" = "false" ] && [ "${baked_bundle_ready}" = "true" ]; then
    read_metadata "${BAKED_METADATA}" "BAKED"
    read_metadata "${VOLUME_METADATA}" "VOLUME"

    if [ "${BAKED_CA_FINGERPRINT_SHA256:-}" != "${VOLUME_CA_FINGERPRINT_SHA256:-}" ]; then
        needs_copy="true"
    fi
fi

if [ "${needs_copy}" = "false" ] && [ -f "${SERVER_CERT}" ]; then
    if ! openssl x509 -checkend "${SSL_RENEWAL_THRESHOLD_SECONDS}" -noout -in "${SERVER_CERT}"; then
        if [ "${baked_bundle_ready}" = "true" ]; then
            echo "Persisted server certificate expires soon; refreshing"
            needs_copy="true"
        else
            echo "WARNING: Server certificate near expiry but no baked bundle available" >&2
        fi
    fi
fi

if [ "${needs_copy}" = "true" ]; then
    if [ "${baked_bundle_ready}" != "true" ]; then
        echo "ERROR: TLS refresh requested but baked certificates are missing" >&2
        exit 1
    fi

    echo "Copying server certificates from image to SSL volume..."
    cp "${BAKED_CERT_DIR}/server.key" "${SERVER_KEY}"
    cp "${BAKED_CERT_DIR}/server.crt" "${SERVER_CERT}"
    cp "${BAKED_CERT_DIR}/ca.crt" "${CA_CERT}"
    cp "${BAKED_METADATA}" "${VOLUME_METADATA}"

    chown postgres:postgres "${SERVER_KEY}" "${SERVER_CERT}" "${CA_CERT}" "${VOLUME_METADATA}"
    chmod 600 "${SERVER_KEY}"
    chmod 644 "${SERVER_CERT}" "${CA_CERT}" "${VOLUME_METADATA}"

    echo "Server certificates copied to ${VOLUME_CERT_DIR}"
else
    echo "Server certificates already up to date in SSL volume, reusing them"
fi

# Copy CA cert to volume if missing and baked bundle is still available
if [ ! -f "${CA_CERT}" ] && [ "${baked_bundle_ready}" = "true" ]; then
    cp "${BAKED_CERT_DIR}/ca.crt" "${CA_CERT}"
    chown postgres:postgres "${CA_CERT}"
    chmod 644 "${CA_CERT}"
fi

# Ensure metadata exists when no refresh happened but CA/metadata missing (should be rare)
if [ ! -f "${VOLUME_METADATA}" ] && [ "${baked_bundle_ready}" = "true" ]; then
    cp "${BAKED_METADATA}" "${VOLUME_METADATA}"
    chown postgres:postgres "${VOLUME_METADATA}"
    chmod 644 "${VOLUME_METADATA}"
fi

cleanup_baked_artifacts()

# Configure SSL in postgresql.conf (only if database is initialized)
# We check for PG_VERSION to ensure postgresql.conf exists
if [ -f "${PGDATA}/PG_VERSION" ]; then
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
fi

# Delegate to official Postgres entrypoint
exec /usr/local/bin/docker-entrypoint.sh "$@"

