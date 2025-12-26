#!/usr/bin/env bash
set -euo pipefail

# Entrypoint script for Caddy that generates Caddyfile from template
# using environment variable substitution

: "${CADDY_DOMAIN:?CADDY_DOMAIN environment variable is required}"

CADDYFILE_TEMPLATE="/etc/caddy/Caddyfile.template"
CADDYFILE="/etc/caddy/Caddyfile"

echo "Generating Caddyfile for domain: ${CADDY_DOMAIN}"

# Substitute CADDY_DOMAIN in template and write to Caddyfile
# Escape special sed replacement characters: & (matched text) and \ (escape)
# Using | as delimiter, so | is not special in replacement string
DOMAIN_ESCAPED=$(printf '%s' "${CADDY_DOMAIN}" | sed 's/[\\&]/\\&/g')
sed "s|\${CADDY_DOMAIN}|${DOMAIN_ESCAPED}|g" < "${CADDYFILE_TEMPLATE}" > "${CADDYFILE}"

echo "Caddyfile generated successfully"

# Delegate to Caddy's default entrypoint
exec /usr/bin/caddy "$@"

