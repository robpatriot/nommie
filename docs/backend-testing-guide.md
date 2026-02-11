# Backend Testing Guide

## Document Scope

Explains how automated tests interact with the database layer, the guard rails
that prevent accidental prod access, and the available harness helpers.
Complementary docs: `backend-error-handling.md` for error assertions,
`database-url-calculation.md` for env vars and URL construction, and
`project-milestones.md` for when new suites are expected.

## Database Environment Policy

### Code Requirements
- **Backend uses env parts, not `DATABASE_URL`** - `POSTGRES_HOST`, `POSTGRES_PORT`, `TEST_DB`, `APP_DB_USER`, `APP_DB_PASSWORD`, etc. URLs are constructed in code
- **Tests use `TEST_DB`** - must end with `_test`; no `TEST_DATABASE_URL` or URL derivation
- **Schema is automatically initialized** - `build_state()` handles empty databases
- **Source env before running tests** - `set -a; source apps/backend/.env; source docker/dev-db/db.env; set +a` (see global rules)

### Selecting the Test Database Backend
- **Default backend is PostgreSQL.** Tests fall back to Postgres when no override is provided.
- **Override per run with `NOMMIE_TEST_DB_KIND`.** Supported values are:
  - `postgres`
  - `sqlite-file`
  - `sqlite-memory`
- **Helper usage:** `build_test_state()` and `test_state_builder()` consult the env var so suites automatically pick up the configured backend.
- **Backend-specific suites:** Postgres-only tests detect the current backend and skip when it does not match, while SQLite-only suites remain active when appropriate.
- **Example:**
  ```bash
  # Run backend tests against SQLite memory
  NOMMIE_TEST_DB_KIND=sqlite-memory pnpm be:test

  # Run regression suites that require Postgres
  NOMMIE_TEST_DB_KIND=postgres pnpm be:test:full
  ```

### Database Setup
- **Postgres:** Tests use `TEST_DB` (must end with `_test`)
- **SQLite:** Tests use file or in-memory DB per `NOMMIE_TEST_DB_KIND`; no `TEST_DB` required
- **Schema is automatically initialized by `build_state()`** - no manual preparation needed
- Fresh databases are automatically migrated on first connection

## Test Support Functions

**Location:** `apps/backend/tests/support/test_state.rs`

- `build_test_state()` - Builds a complete `AppState` with test database configured
- `test_state_builder()` - Returns a `StateBuilder` for custom configuration
- `resolve_test_db_kind()` - Resolves the database backend from `NOMMIE_TEST_DB_KIND` env var

These functions automatically:
- Set `RuntimeEnv::Test`
- Respect `NOMMIE_TEST_DB_KIND` environment variable
- Default to PostgreSQL if no override is provided

## Safety Features

- **Database Guard**: Tests fail if `TEST_DB` doesn't end with `_test` (enforced in `packages/db-infra/src/config/db.rs`)
- **Auto-Migration**: Empty databases are automatically migrated on connection
- **Environment Isolation**: Tests use the same env vars as dev (source `docker/dev-db/db.env` before running)
- **Automatic Schema Migration**: `build_state()` automatically migrates empty databases for both prod and test

## Running Tests

```bash
# Source env once per shell (required)
set -a; source apps/backend/.env; source docker/dev-db/db.env; set +a

# Run all backend tests (schema auto-migrates on first connection)
pnpm be:test

# Run tests with verbose output (requires TEST_DB ending with _test when using Postgres)
pnpm be:test:v

# Run specific test
pnpm be:test -- test_health_endpoint
```

**Note**: Schema is automatically initialized on first connection to empty databases.

**Note**: When using Postgres (`NOMMIE_TEST_DB_KIND=postgres` or default), `TEST_DB` must end with `_test` or the build will fail with a clear config error.

## Example `docker/dev-db/db.env`

```env
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
PROD_DB=nommie
TEST_DB=nommie_test   # must end with "_test"
APP_DB_USER=nommie_app
APP_DB_PASSWORD=your-password
NOMMIE_OWNER_USER=nommie_owner
NOMMIE_OWNER_PASSWORD=your-owner-password
```

For local dev without TLS, set `POSTGRES_SSL_MODE=disable` in `apps/backend/.env` (see `database-url-calculation.md`).

## Architecture Notes

- Tests connect to the database and schema is auto-migrated if empty
- `build_state()` automatically handles schema initialization for all database types
- Integration tests use `actix_web::test` for in-process service testing
- The `_test` suffix guard ensures tests never run against production databases
- Schema validation is automatic - both prod and test startup validate schema readiness
