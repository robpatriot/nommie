# Caching, Versioning, and ETags

## Version field
- Games use version (i32) for optimistic locking.
- Version is stored in the database and included in JSON payloads.

## Mutable endpoints
- Must accept version in request body JSON.
- Must return updated version.
- On mismatch return 409 Conflict with OPTIMISTIC_LOCK.

## Safe endpoints
- Must include version in response body.
- Must send strong ETag derived from version.
- Must support If-None-Match and return 304 when matched.
