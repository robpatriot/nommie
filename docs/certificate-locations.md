# Certificate Locations and Verification

This document shows where SSL certificates are copied/mounted in the Postgres and Backend containers, and how to verify they exist and are in the right place.

## Certificate Flow Overview

### Postgres Container

1. **Build Time** (in `docker/postgres-tls/Dockerfile`):
   - Server certificates (`server.key`, `server.crt`) are generated and baked into image at `/opt/nommie/ssl/`
   - CA certificate (`ca.crt`) is copied into image at `/opt/nommie/ssl/ca.crt`

2. **Runtime** (in `docker/postgres-tls/entrypoint-ssl.sh`):
   - Server certs are copied from `/opt/nommie/ssl/` → `/var/lib/postgresql/ssl/` (on first run)
   - CA cert is copied from `/opt/nommie/ssl/` → `/var/lib/postgresql/ssl/` (on first run)
   - Postgres uses certs from `/var/lib/postgresql/ssl/` for SSL connections

3. **Volume Mount** (in `docker-compose.yml`):
   - `../shared/ca.crt` → `/etc/ssl/certs/nommie-ca.crt` (for local backend connections)

### Backend Container

1. **Volume Mount** (in `docker-compose.yml`):
   - `../shared/ca.crt` → `/etc/ssl/certs/nommie-ca.crt` (for TLS verification)

## Detailed Locations

### Postgres Container

| Certificate | Build Location | Runtime Location | Volume Mount | Purpose |
|------------|----------------|------------------|--------------|---------|
| `server.key` | `/opt/nommie/ssl/server.key` | `/var/lib/postgresql/ssl/server.key` | `nommie_dev_db_ssl` (dev-db) or inside `nommie_prod_db_data` (prod) | Postgres SSL server key |
| `server.crt` | `/opt/nommie/ssl/server.crt` | `/var/lib/postgresql/ssl/server.crt` | `nommie_dev_db_ssl` (dev-db) or inside `nommie_prod_db_data` (prod) | Postgres SSL server cert |
| `ca.crt` | `/opt/nommie/ssl/ca.crt` | `/var/lib/postgresql/ssl/ca.crt` | `nommie_dev_db_ssl` (dev-db) or inside `nommie_prod_db_data` (prod) | CA cert (reference) |
| `ca.crt` | N/A | `/etc/ssl/certs/nommie-ca.crt` | `../shared/ca.crt` (host) | For local backend connections |

### Backend Container

| Certificate | Runtime Location | Volume Mount | Purpose |
|------------|------------------|--------------|---------|
| `ca.crt` | `/etc/ssl/certs/nommie-ca.crt` | `../shared/ca.crt` (host) | TLS verification of Postgres connections |

## Code Locations

### Postgres Container

**Build-time certificate generation:**
- **File**: `docker/postgres-tls/Dockerfile` (lines 28-37)
- **Script**: `docker/postgres-tls/generate-server-cert.sh`
- **Baked location**: `/opt/nommie/ssl/`

**Runtime certificate copying:**
- **File**: `docker/postgres-tls/entrypoint-ssl.sh` (lines 20-50)
- **Source**: `/opt/nommie/ssl/`
- **Destination**: `/var/lib/postgresql/ssl/`

**Volume mounts:**
- **dev-db**: `docker/dev-db/docker-compose.yml` (line 24: separate SSL volume, line 28: CA cert mount)
- **prod**: `docker/prod/docker-compose.yml` (line 20: SSL directory created inside data volume, no separate SSL volume)

### Backend Container

**Volume mounts:**
- **dev-db**: `docker/dev-db/docker-compose.yml` (line 28)
- **prod**: `docker/prod/docker-compose.yml` (line 42)

## Verification Commands

### Check Postgres Container Certificates

#### 1. Check baked certificates in image (build-time location)
```bash
# For dev-db
docker compose -f docker/dev-db/docker-compose.yml exec postgres ls -la /opt/nommie/ssl/

# For prod
docker compose -f docker/prod/docker-compose.yml exec postgres ls -la /opt/nommie/ssl/
```

**Expected output:**
```
-rw------- 1 postgres postgres 1704 server.key
-rw-r--r-- 1 postgres postgres 1950 server.crt
-rw-r--r-- 1 postgres postgres 1950 ca.crt
```

#### 2. Check runtime certificates (volume location)
```bash
# For dev-db
docker compose -f docker/dev-db/docker-compose.yml exec postgres ls -la /var/lib/postgresql/ssl/

# For prod
docker compose -f docker/prod/docker-compose.yml exec postgres ls -la /var/lib/postgresql/ssl/
```

**Expected output:**
```
-rw------- 1 postgres postgres 1704 server.key
-rw-r--r-- 1 postgres postgres 1950 server.crt
-rw-r--r-- 1 postgres postgres 1950 ca.crt
```

#### 3. Check mounted CA cert (for local backend)
```bash
# For dev-db
docker compose -f docker/dev-db/docker-compose.yml exec postgres ls -la /etc/ssl/certs/nommie-ca.crt

# For prod
docker compose -f docker/prod/docker-compose.yml exec postgres ls -la /etc/ssl/certs/nommie-ca.crt
```

**Expected output:**
```
-rw-r--r-- 1 root root 1950 /etc/ssl/certs/nommie-ca.crt
```

#### 4. Verify Postgres SSL configuration
```bash
# For dev-db
docker compose -f docker/dev-db/docker-compose.yml exec postgres grep -E "^ssl|ssl_cert|ssl_key|ssl_ca" /var/lib/postgresql/data/postgresql.conf

# For prod
docker compose -f docker/prod/docker-compose.yml exec postgres grep -E "^ssl|ssl_cert|ssl_key|ssl_ca" /var/lib/postgresql/data/postgresql.conf
```

**Expected output:**
```
ssl = on
ssl_cert_file = '/var/lib/postgresql/ssl/server.crt'
ssl_key_file = '/var/lib/postgresql/ssl/server.key'
ssl_ca_file = '/var/lib/postgresql/ssl/ca.crt'
```

### Check Backend Container Certificates

#### 1. Check mounted CA cert
```bash
# For prod
docker compose -f docker/prod/docker-compose.yml exec backend ls -la /etc/ssl/certs/nommie-ca.crt
```

**Expected output:**
```
-rw-r--r-- 1 root root 1950 /etc/ssl/certs/nommie-ca.crt
```

#### 2. Verify backend environment variable
```bash
# For prod
docker compose -f docker/prod/docker-compose.yml exec backend env | grep POSTGRES_SSL
```

**Expected output:**
```
POSTGRES_SSL_MODE=verify-full
POSTGRES_SSL_ROOT_CERT=/etc/ssl/certs/nommie-ca.crt
```

### Check Host Certificate File

#### 1. Verify shared CA cert exists on host
```bash
ls -la docker/shared/ca.crt
```

**Expected output:**
```
-rw-r--r-- 1 user user 1950 docker/shared/ca.crt
```

**Note**: If this file doesn't exist, you need to copy it:
```bash
cp ~/secrets/nommie-ca/ca.crt docker/shared/ca.crt
```

## Certificate Verification Script

Here's a comprehensive script to check all certificate locations:

```bash
#!/bin/bash
set -e

echo "=== Checking Postgres Container Certificates ==="
echo

echo "1. Baked certificates in image:"
docker compose -f docker/dev-db/docker-compose.yml exec postgres ls -la /opt/nommie/ssl/ 2>/dev/null || echo "  Container not running or dev-db not available"
echo

echo "2. Runtime certificates in volume:"
docker compose -f docker/dev-db/docker-compose.yml exec postgres ls -la /var/lib/postgresql/ssl/ 2>/dev/null || echo "  Container not running or dev-db not available"
echo

echo "3. Mounted CA cert in Postgres:"
docker compose -f docker/dev-db/docker-compose.yml exec postgres ls -la /etc/ssl/certs/nommie-ca.crt 2>/dev/null || echo "  Container not running or dev-db not available"
echo

echo "=== Checking Backend Container Certificates ==="
echo

echo "4. Mounted CA cert in Backend:"
docker compose -f docker/prod/docker-compose.yml exec backend ls -la /etc/ssl/certs/nommie-ca.crt 2>/dev/null || echo "  Container not running or prod not available"
echo

echo "5. Backend SSL environment variables:"
docker compose -f docker/prod/docker-compose.yml exec backend env | grep POSTGRES_SSL || echo "  Container not running or prod not available"
echo

echo "=== Checking Host Certificate File ==="
echo

echo "6. Shared CA cert on host:"
if [ -f "docker/shared/ca.crt" ]; then
    ls -la docker/shared/ca.crt
    echo "  ✓ File exists"
else
    echo "  ✗ File missing! Copy it with: cp ~/secrets/nommie-ca/ca.crt docker/shared/ca.crt"
fi
echo

echo "=== Certificate Expiry Check ==="
echo

if [ -f "docker/shared/ca.crt" ]; then
    echo "CA Certificate expiry:"
    openssl x509 -enddate -noout -in docker/shared/ca.crt
fi
```

## Common Issues and Fixes

### Issue 1: Missing `docker/shared/ca.crt` on host

**Symptom**: Backend can't connect to Postgres with TLS

**Fix**:
```bash
cp ~/secrets/nommie-ca/ca.crt docker/shared/ca.crt
```

### Issue 2: Server certificates not copied to volume

**Symptom**: Postgres logs show SSL errors

**Fix**: The entrypoint script should handle this automatically. If not, check:
```bash
# Check if baked certs exist
docker compose -f docker/dev-db/docker-compose.yml exec postgres ls -la /opt/nommie/ssl/

# Check if volume certs exist
docker compose -f docker/dev-db/docker-compose.yml exec postgres ls -la /var/lib/postgresql/ssl/

# If volume certs are missing, restart the container (entrypoint will copy them)
docker compose -f docker/dev-db/docker-compose.yml restart postgres
```

### Issue 3: Wrong path in `POSTGRES_SSL_ROOT_CERT`

**Symptom**: Backend connection fails with certificate verification error

**Fix**: Ensure `POSTGRES_SSL_ROOT_CERT` in backend env file matches the mount path:
```bash
# Should be:
POSTGRES_SSL_ROOT_CERT=/etc/ssl/certs/nommie-ca.crt
```

### Issue 4: Certificate permissions wrong

**Symptom**: Postgres can't read certificates

**Fix**: Check permissions (should be 600 for key, 644 for certs):
```bash
docker compose -f docker/dev-db/docker-compose.yml exec postgres ls -la /var/lib/postgresql/ssl/
# If wrong, the entrypoint script should fix them on next restart
```

## Summary

| Container | Certificate | Source | Destination | Verified By |
|-----------|-------------|--------|-------------|-------------|
| Postgres | `server.key` | `/opt/nommie/ssl/` (baked) | `/var/lib/postgresql/ssl/` (volume) | `entrypoint-ssl.sh` |
| Postgres | `server.crt` | `/opt/nommie/ssl/` (baked) | `/var/lib/postgresql/ssl/` (volume) | `entrypoint-ssl.sh` |
| Postgres | `ca.crt` | `/opt/nommie/ssl/` (baked) | `/var/lib/postgresql/ssl/` (volume) | `entrypoint-ssl.sh` |
| Postgres | `ca.crt` | `docker/shared/ca.crt` (host) | `/etc/ssl/certs/nommie-ca.crt` | `docker-compose.yml` |
| Backend | `ca.crt` | `docker/shared/ca.crt` (host) | `/etc/ssl/certs/nommie-ca.crt` | `docker-compose.yml` |

