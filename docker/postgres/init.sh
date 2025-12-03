#!/usr/bin/env bash
set -euo pipefail

# Expected env from docker-compose:
#   POSTGRES_DB, POSTGRES_USER, POSTGRES_PASSWORD, APP_DB_PASSWORD

: "${POSTGRES_DB:?POSTGRES_DB is required and must be set in the environment}"
: "${POSTGRES_USER:?POSTGRES_USER is required and must be set in the environment}"
: "${POSTGRES_PASSWORD:?POSTGRES_PASSWORD is required and must be set in the environment}"
: "${APP_DB_PASSWORD:?APP_DB_PASSWORD is required and must be set in the environment}"
: "${NOMMIE_OWNER_PASSWORD:?NOMMIE_OWNER_PASSWORD is required and must be set in the environment}"

########################################
# 1) Roles + Databases (idempotent)
#    Use psql variables + \gexec for safe quoting.
########################################
echo "Creating databases and roles..."
psql -v ON_ERROR_STOP=1 \
     --dbname "$POSTGRES_DB" \
     --username "$POSTGRES_USER" \
     -v owner_pw="${NOMMIE_OWNER_PASSWORD}" \
     -v app_pw="${APP_DB_PASSWORD}" <<'PSQL'
-- Create nommie_owner if missing, then ensure password
SELECT format('CREATE ROLE nommie_owner LOGIN PASSWORD %L', :'owner_pw')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname='nommie_owner');
\gexec
SELECT format('ALTER ROLE nommie_owner WITH PASSWORD %L', :'owner_pw');
\gexec

-- Create nommie_app if missing, then ensure password
SELECT format('CREATE ROLE nommie_app LOGIN PASSWORD %L', :'app_pw')
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname='nommie_app');
\gexec
SELECT format('ALTER ROLE nommie_app WITH PASSWORD %L', :'app_pw');
\gexec

-- Create application databases if missing
SELECT 'CREATE DATABASE nommie OWNER nommie_owner'
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname='nommie');
\gexec
SELECT 'CREATE DATABASE nommie_test OWNER nommie_owner'
WHERE NOT EXISTS (SELECT 1 FROM pg_database WHERE datname='nommie_test');
\gexec
PSQL

########################################
# 2) Configure 'nommie' DB (extensions & privileges)
########################################
echo "Configuring 'nommie' database..."
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "nommie" <<'PSQL'
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- Public schema ownership & privileges
ALTER SCHEMA public OWNER TO nommie_owner;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
GRANT USAGE, CREATE ON SCHEMA public TO nommie_owner;
GRANT USAGE ON SCHEMA public TO nommie_app;

ALTER DEFAULT PRIVILEGES FOR USER nommie_owner IN SCHEMA public
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO nommie_app;

ALTER DEFAULT PRIVILEGES FOR USER nommie_owner IN SCHEMA public
GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO nommie_app;

-- (No table creation here; schema is managed by the SeaORM migrator.)
PSQL

########################################
# 3) Configure 'nommie_test' DB (extensions & privileges)
########################################
echo "Configuring 'nommie_test' database..."
psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --dbname "nommie_test" <<'PSQL'
CREATE EXTENSION IF NOT EXISTS pgcrypto;

ALTER SCHEMA public OWNER TO nommie_owner;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;
GRANT USAGE, CREATE ON SCHEMA public TO nommie_owner;
GRANT USAGE ON SCHEMA public TO nommie_app;

ALTER DEFAULT PRIVILEGES FOR USER nommie_owner IN SCHEMA public
GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO nommie_app;

ALTER DEFAULT PRIVILEGES FOR USER nommie_owner IN SCHEMA public
GRANT USAGE, SELECT, UPDATE ON SEQUENCES TO nommie_app;
PSQL

########################################
# 4) Configure SSL in postgresql.conf (if not already configured)
########################################
echo "Configuring SSL in postgresql.conf..."
PGDATA="${PGDATA:-/var/lib/postgresql/data}"
SSL_CERT_DIR="/var/lib/postgresql/ssl"
SERVER_CERT="${SSL_CERT_DIR}/server.crt"
SERVER_KEY="${SSL_CERT_DIR}/server.key"
CA_CERT="${SSL_CERT_DIR}/ca.crt"

if [ -f "${PGDATA}/postgresql.conf" ] && ! grep -q "^ssl[[:space:]]*=" "${PGDATA}/postgresql.conf" 2>/dev/null; then
    cat >> "${PGDATA}/postgresql.conf" <<EOF

# SSL configuration (added by init.sh)
ssl = on
ssl_cert_file = '${SERVER_CERT}'
ssl_key_file = '${SERVER_KEY}'
EOF
    
    if [ -f "${CA_CERT}" ]; then
        echo "ssl_ca_file = '${CA_CERT}'" >> "${PGDATA}/postgresql.conf"
    fi
    
    echo "SSL configuration added to postgresql.conf"
else
    echo "SSL already configured in postgresql.conf or postgresql.conf not found"
fi

