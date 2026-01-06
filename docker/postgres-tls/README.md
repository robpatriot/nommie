# Postgres TLS Image

This directory contains a shared Docker image for Postgres with TLS support using a private CA. The same image is used for both `dev-db` and `prod` environments.

## Overview

- **Server certificates** are generated at **build time** using Docker build secrets
- The **CA private key** is only available during build and is **never included** in the final image
- Server certs are baked into the image and automatically copied into the Postgres data volume on first run
- This ensures certs persist across image rebuilds while keeping the CA key secure

## Prerequisites

Before building the image, you must:

1. **Generate a private CA** (one-time, manual step):
   ```bash
   mkdir -p ~/secrets/nommie-ca
   cd ~/secrets/nommie-ca
   
   # Generate CA private key (4096 bits)
   openssl genrsa -out ca.key 4096
   
   # Generate CA certificate (20-year validity)
   openssl req -new -x509 -days 7300 -key ca.key -out ca.crt \
     -subj "/CN=Nommie CA/O=Nommie/C=US"
   ```

2. **Copy CA public certificate** to the postgres-tls directory:
   ```bash
   cp ~/secrets/nommie-ca/ca.crt docker/postgres-tls/ca.crt
   ```
   
   **Why two locations?**
   - `~/secrets/nommie-ca/ca.key` and `ca.crt` → Used during **build** (via Docker build secrets)
   - `docker/postgres-tls/ca.crt` → Copied into postgres-tls and backend images during build (for TLS verification)
   
   Even though `ca.crt` is a public certificate, this repository treats it like
   other deployment-specific secrets: you should create `docker/postgres-tls/ca.crt`
   locally and **not** commit it to git (it is gitignored).
   
   **Important:** Only copy `ca.crt` (the public certificate) into
   `docker/postgres-tls/ca.crt` locally. Never commit `ca.key` or `ca.crt` to git.

## Building the Image

### Option 1: Automatic Build via Docker Compose (Recommended)

The `dev-db` and `prod` docker-compose files are configured to automatically build the image when needed. Just run:

```bash
# For dev-db
docker compose -f docker/dev-db/docker-compose.yml up --build

# For prod
docker compose -f docker/prod/docker-compose.yml up --build
```

The compose files will automatically:
- Read CA key/cert from `~/secrets/nommie-ca/` (default paths)
- Pass them as build secrets to the Dockerfile
- Build and tag the image as `nommie-postgres-tls:latest`

**Custom CA paths:** You can override the default paths using environment variables:
```bash
export NOMMIE_CA_KEY_PATH=/custom/path/to/ca.key
export NOMMIE_CA_CERT_PATH=/custom/path/to/ca.crt
docker compose -f docker/dev-db/docker-compose.yml up --build
```

### Option 2: Manual Build

You can also build the image manually using Docker build secrets:

```bash
cd /path/to/nommie

docker build \
  --secret id=nommie_ca_key,src=~/secrets/nommie-ca/ca.key \
  --secret id=nommie_ca_crt,src=~/secrets/nommie-ca/ca.crt \
  -t nommie-postgres-tls:latest \
  docker/postgres-tls
```

**What happens during build:**
- The build script (`generate-server-cert.sh`) runs with access to CA key/cert via `/run/secrets/`
- It generates a new `server.key` and `server.crt` signed by your CA
- Server certs are baked into the image at `/opt/nommie/ssl/`
- CA public cert is also included at `/opt/nommie/ssl/ca.crt`
- The CA private key is **never** included in the final image

## Using the Image

### dev-db

The `dev-db/docker-compose.yml` uses the image directly:

```yaml
postgres:
  image: nommie-postgres-tls:latest
```

On first container start:
- Server certs are copied from `/opt/nommie/ssl/` into `$PGDATA/ssl/`
- Postgres is configured with SSL in `postgresql.conf`
- Subsequent starts reuse the certs from the volume

### prod

The `prod/docker-compose.yml` also uses the same image:

```yaml
postgres:
  image: nommie-postgres-tls:latest
```

Both environments use the **same CA**, ensuring consistent TLS behavior.

## Backend Configuration

Backend containers have the CA cert baked into the image at `/etc/ssl/certs/nommie-ca.crt` (copied during build).

**Set environment variables** in `backend.env`:
   ```bash
   # TLS is enabled by default (verify-full)
   # POSTGRES_SSL_MODE=verify-full  # (default, can be omitted)
   POSTGRES_SSL_ROOT_CERT=/etc/ssl/certs/nommie-ca.crt
   ```

3. **To disable TLS** (not recommended):
   ```bash
   POSTGRES_SSL_MODE=disable
   # POSTGRES_SSL_ROOT_CERT not needed when disabled
   ```

## Certificate Expiry

- **CA certificate:** Valid for 20 years (7300 days) - set during generation
- **Server certificates:** Valid for 3 years (1095 days) - set during build

**To rotate server certs:**
1. Rebuild the image with the same CA secrets (new server certs will be generated)
2. Optionally delete old certs from the volume to force re-copy:
   ```bash
   docker compose -f docker/dev-db/docker-compose.yml exec postgres rm -f /var/lib/postgresql/data/ssl/server.key /var/lib/postgresql/data/ssl/server.crt
   ```
3. Restart the container

**To check certificate expiry:**
```bash
# Check CA cert
openssl x509 -enddate -noout -in docker/postgres-tls/ca.crt

# Check server cert (from inside container)
docker compose -f docker/dev-db/docker-compose.yml exec postgres openssl x509 -enddate -noout -in /var/lib/postgresql/data/ssl/server.crt
```

## Security Notes

- **CA private key (`ca.key`):** Keep this secure and backed up. Store it outside the repository (e.g., `~/secrets/nommie-ca/`).
- **Server private key:** Baked into the image but only accessible to the `postgres` user inside the container.
- **CA public cert (`ca.crt`):** Safe to commit to git and share. It's in `docker/postgres-tls/ca.crt` (gitignored).

## Troubleshooting

**Build fails with "CA key not found":**
- Ensure you're providing both `--secret id=nommie_ca_key` and `--secret id=nommie_ca_crt`
- Verify the paths to your CA key and cert are correct

**Backend can't connect with TLS:**
- Verify `POSTGRES_SSL_ROOT_CERT` points to the mounted CA cert path
- Check that the CA cert is mounted in `docker-compose.yml`
- Ensure `POSTGRES_SSL_MODE` is not set to `disable` (unless you want to disable TLS)

**Postgres starts without SSL:**
- Check container logs for SSL configuration messages
- Verify `$PGDATA/ssl/server.key` and `server.crt` exist in the volume
- Check `postgresql.conf` for SSL settings

