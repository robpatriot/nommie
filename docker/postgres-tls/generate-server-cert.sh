#!/usr/bin/env sh
set -euo pipefail

# Generate Postgres server certificate during Docker build.
# This script is run with --mount=type=secret to access CA key and cert.
#
# Expected secrets:
#   - /run/secrets/nommie_ca_key  (CA private key)
#   - /run/secrets/nommie_ca_crt   (CA public certificate)

CA_KEY="/run/secrets/nommie_ca_key"
CA_CERT="/run/secrets/nommie_ca_crt"
OUTPUT_DIR="/opt/nommie/ssl"
CA_RENEWAL_THRESHOLD_SECONDS="${CA_RENEWAL_THRESHOLD_SECONDS:-2592000}" # 30 days

# Verify secrets are available
if [ ! -f "${CA_KEY}" ]; then
    echo "ERROR: CA key not found at ${CA_KEY}" >&2
    echo "Build must include --secret id=nommie_ca_key,src=/path/to/ca.key" >&2
    exit 1
fi

if [ ! -f "${CA_CERT}" ]; then
    echo "ERROR: CA cert not found at ${CA_CERT}" >&2
    echo "Build must include --secret id=nommie_ca_crt,src=/path/to/ca.crt" >&2
    exit 1
fi

# Ensure CA certificate is valid and warn when close to expiry
if ! openssl x509 -checkend 0 -noout -in "${CA_CERT}"; then
    echo "ERROR: CA certificate supplied via secrets has expired" >&2
    exit 1
fi

if ! openssl x509 -checkend "${CA_RENEWAL_THRESHOLD_SECONDS}" -noout -in "${CA_CERT}"; then
    echo "WARNING: CA certificate will expire soon; refresh the CA before it lapses" >&2
fi

echo "Generating Postgres server certificate..."

# Generate server private key (4096 bits)
openssl genrsa -out "${OUTPUT_DIR}/server.key" 4096

# Generate certificate signing request
# CN=postgres matches the Docker service name
# SAN includes both 'postgres' (service name) and 'localhost' (for direct connections)
openssl req -new -key "${OUTPUT_DIR}/server.key" \
    -out "${OUTPUT_DIR}/server.csr" \
    -subj "/CN=postgres/O=Nommie/C=US"

# Create temporary extfile for SAN
cat > "${OUTPUT_DIR}/server.ext" <<EOF
[v3_req]
subjectAltName=DNS:postgres,DNS:localhost
EOF

# Sign server certificate with CA (3-year validity, 1095 days)
openssl x509 -req \
    -in "${OUTPUT_DIR}/server.csr" \
    -CA "${CA_CERT}" \
    -CAkey "${CA_KEY}" \
    -CAcreateserial \
    -out "${OUTPUT_DIR}/server.crt" \
    -days 1095 \
    -extensions v3_req \
    -extfile "${OUTPUT_DIR}/server.ext"

# Clean up temporary files
rm -f "${OUTPUT_DIR}/server.csr" "${OUTPUT_DIR}/server.ext"

# Set permissions
chmod 600 "${OUTPUT_DIR}/server.key"
chmod 644 "${OUTPUT_DIR}/server.crt"

# Capture metadata for runtime refresh decisions
SERVER_SERIAL=$(openssl x509 -in "${OUTPUT_DIR}/server.crt" -noout -serial | cut -d= -f2)
SERVER_NOT_AFTER=$(openssl x509 -in "${OUTPUT_DIR}/server.crt" -noout -enddate | cut -d= -f2)
SERVER_FINGERPRINT_SHA256=$(openssl x509 -in "${OUTPUT_DIR}/server.crt" -noout -fingerprint -sha256 | cut -d= -f2)
CA_FINGERPRINT_SHA256=$(openssl x509 -in "${CA_CERT}" -noout -fingerprint -sha256 | cut -d= -f2)
CA_SUBJECT=$(openssl x509 -in "${CA_CERT}" -noout -subject | sed 's/^subject= //')
GENERATED_AT=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

cat > "${OUTPUT_DIR}/cert.meta" <<EOF
# Metadata for TLS artifacts baked into the image
GENERATED_AT=${GENERATED_AT}
SERVER_SERIAL=${SERVER_SERIAL}
SERVER_NOT_AFTER=${SERVER_NOT_AFTER}
SERVER_FINGERPRINT_SHA256=${SERVER_FINGERPRINT_SHA256}
CA_FINGERPRINT_SHA256=${CA_FINGERPRINT_SHA256}
CA_SUBJECT=${CA_SUBJECT}
EOF

chmod 644 "${OUTPUT_DIR}/cert.meta"

echo "Server certificate generated successfully:"
echo "  - ${OUTPUT_DIR}/server.key"
echo "  - ${OUTPUT_DIR}/server.crt"
echo "  - Valid for 3 years (1095 days)"

