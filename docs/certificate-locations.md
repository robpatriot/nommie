# Certificate Locations and Verification

## Scope

Defines where Postgres and backend containers expect TLS certificates to exist, and the commands used to verify the installation.

## Required Certificate Paths

### Backend container

The backend verifies Postgres TLS using a CA certificate at:

/etc/ssl/certs/nommie-ca.crt

The backend must be configured with:

POSTGRES_SSL_ROOT_CERT=/etc/ssl/certs/nommie-ca.crt

### Postgres container

Postgres TLS uses certificate files at:

/var/lib/postgresql/ssl/server.key  
/var/lib/postgresql/ssl/server.crt  
/var/lib/postgresql/ssl/ca.crt

For local backend connections, the CA cert is also mounted at:

/etc/ssl/certs/nommie-ca.crt

## Dev and Prod Differences

- Dev and prod compose stacks may differ in how the SSL directory is persisted (separate SSL volume vs inside the data volume).
- Certificate paths inside the containers must remain consistent with the locations above.

## Verification Commands

### Host

Verify the host CA file exists:

ls -la docker/shared/ca.crt

### Postgres container

Verify runtime certificate files exist:

docker compose -f docker/dev-db/compose.yaml exec postgres ls -la /var/lib/postgresql/ssl/

Verify mounted CA exists (for local backend):

docker compose -f docker/dev-db/compose.yaml exec postgres ls -la /etc/ssl/certs/nommie-ca.crt

Verify Postgres configuration points at the expected files:

docker compose -f docker/dev-db/compose.yaml exec postgres grep -E "^ssl|ssl_cert|ssl_key|ssl_ca" /var/lib/postgresql/data/postgresql.conf

### Backend container

Verify mounted CA exists:

docker compose -f docker/prod/compose.yaml exec backend ls -la /etc/ssl/certs/nommie-ca.crt

Verify backend TLS env is set:

docker compose -f docker/prod/compose.yaml exec backend env | grep POSTGRES_SSL

## Common Failures

Missing host CA file:

cp ~/secrets/nommie-ca/ca.crt docker/shared/ca.crt

Wrong backend CA path:

POSTGRES_SSL_ROOT_CERT must equal /etc/ssl/certs/nommie-ca.crt

Certificate permissions:

server.key must be readable by Postgres and typically requires restrictive permissions (e.g. 600).
