# Why Use a Separate SSL Volume for Certificates?

## The Problem

When storing SSL certificates for Postgres, you have several options:
1. Store them inside `$PGDATA` (the Postgres data directory: `/var/lib/postgresql/data`)
2. Store them in a separate directory on the container filesystem
3. Store them in a **separate Docker volume** (what dev-db does)

## Why a Separate Volume?

Based on the code comments in `docker/postgres-tls/entrypoint-ssl.sh`, the separate SSL volume was chosen for these reasons:

### 1. **Doesn't Affect $PGDATA** (Line 17 comment)

The Postgres data directory (`$PGDATA = /var/lib/postgresql/data`) is critical infrastructure. Storing certificates inside it would:

- **Mix concerns**: Certificates become part of the database data structure
- **Complicate backups**: Database backups would include/exclude certificates, making it harder to manage them separately
- **Risk during reinitialization**: If you need to reinitialize the database, you'd lose the certificates
- **Volume lifecycle coupling**: Can't easily manage certificate lifecycle independently from database lifecycle

### 2. **Persist Across Image Rebuilds** (Line 7 comment)

The entrypoint script comment states:
> "Copies baked server certificates from /opt/nommie/ssl/ into a separate SSL volume on first run (so certs persist across image rebuilds)"

With a separate volume:
- Certificates persist even when you rebuild the Docker image
- The entrypoint script only copies certs if they don't exist (line 22 check)
- Existing certs are reused, preventing unnecessary regeneration

### 3. **Independent Certificate Lifecycle**

A separate volume allows you to:
- **Rotate certificates** without touching the database data
- **Delete and recreate** the SSL volume independently
- **Backup certificates separately** from database backups
- **Manage certificate expiry** without database downtime

### 4. **Clean Separation of Concerns**

- **Database data volume**: Contains actual database files, WAL, etc.
- **SSL volume**: Contains only certificates
- **Clear boundaries**: Each volume has a single, well-defined purpose

## Current Implementation

### dev-db (Has Separate Volume) ✅

```yaml
volumes:
  - nommie_dev_db_data:/var/lib/postgresql/data
  - nommie_dev_db_ssl:/var/lib/postgresql/ssl  # Separate volume
```

**Benefits:**
- Certificates persist independently
- Can be managed separately from database
- Clear separation of concerns

### local-prod (No Separate Volume) ⚠️

```yaml
volumes:
  - nommie_local_prod_db_data:/var/lib/postgresql/data
  # No separate SSL volume - certs stored in /var/lib/postgresql/ssl
```

**Current behavior:**
- The entrypoint script creates `/var/lib/postgresql/ssl/` directory
- Since it's not inside `$PGDATA` (`/var/lib/postgresql/data`), it's on the container filesystem
- **Issue**: If the container is recreated, certs would be lost (unless they're inside a volume)

**Potential problem:**
- `/var/lib/postgresql/ssl` is NOT inside the data volume mount point
- It's also NOT a separate volume
- This means certs are **ephemeral** and will be lost on container recreation

## Recommendation

The `dev-db` setup with a separate SSL volume is the correct approach. The `local-prod` setup should probably also use a separate SSL volume for consistency and to ensure certificates persist.

### Suggested Fix for local-prod

Add a separate SSL volume to `docker/local-prod/docker-compose.yml`:

```yaml
volumes:
  - nommie_local_prod_db_data:/var/lib/postgresql/data
  - nommie_local_prod_ssl:/var/lib/postgresql/ssl  # Add this
  - ../postgres/init.sh:/docker-entrypoint-initdb.d/init.sh:ro

# ... later in the file ...

volumes:
  nommie_local_prod_db_data:
  nommie_local_prod_ssl:  # Add this
  nommie_local_prod_caddy_data:
  nommie_local_prod_caddy_config:
```

## Code References

- **Entrypoint script**: `docker/postgres-tls/entrypoint-ssl.sh` (lines 6-7, 17, 21)
- **dev-db compose**: `docker/dev-db/docker-compose.yml` (line 24)
- **local-prod compose**: `docker/local-prod/docker-compose.yml` (missing SSL volume)

## Summary

The separate SSL volume was implemented to:
1. ✅ Keep certificates separate from database data (`$PGDATA`)
2. ✅ Ensure certificates persist across image rebuilds
3. ✅ Allow independent certificate lifecycle management
4. ✅ Maintain clean separation of concerns

The `dev-db` setup correctly implements this pattern. The `local-prod` setup should be updated to match for consistency and to ensure certificates persist properly.

