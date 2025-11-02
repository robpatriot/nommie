# Testing Guide

This document describes the testing setup for the Nommie backend.

## Database Environment Policy

### Code Requirements
- **Backend code reads only `DATABASE_URL`** - no references to `TEST_DATABASE_URL`
- **Tests must run against a database whose name ends with `_test`**
- **Schema is automatically initialized** - `build_state()` handles empty databases
- **The test runner automatically derives the test database URL** by appending `_test` to the existing `DATABASE_URL`

### Selecting the Test Database Backend
- **Default backend is PostgreSQL.** Tests fall back to Postgres when no override is provided.
- **Override per run with `NOMMIE_TEST_DB_KIND`.** Supported values are:
  - `postgres`
  - `sqlite-file`
  - `sqlite-memory`
- **Helper usage:** `build_test_state()` and `test_state_builder()` consult the env var so suites automatically pick up the configured backend.
- **Backend-specific suites:** Postgres-only tests (e.g. `regression/game_flow_ai_pg`, migration lock tests) detect the current backend and skip when it does not match, while SQLite-only suites (`services/sqlite_backend_file.rs`, `services/sqlite_backend_mem.rs`) remain active when appropriate.
- **Example:**
  ```bash
  # Run backend tests against SQLite memory
  NOMMIE_TEST_DB_KIND=sqlite-memory pnpm be:test

  # Run regression suites that require Postgres
  NOMMIE_TEST_DB_KIND=postgres pnpm be:test:full
  ```

### Database Setup
- Tests run against a database suffixed with `_test`
- **Schema is automatically initialized by `build_state()`** - no manual preparation needed
- Tests connect to the database and schema is auto-migrated if empty
- **Fresh databases are automatically migrated on first connection**

## Test Structure

### Test Support Module (`src/test_support/`)
- `assert_test_db_url(url: &str)` - Validates database URL ends with `_test`
- `load_test_env()` - Loads `.env.test` configuration
- `get_test_db_url()` - Retrieves database URL from environment
- `ensure_schema_ready(db)` - Ensures schema is ready, auto-migrates empty databases (automatically called by `build_state`)

### Health Endpoint (`src/health.rs`)
- Simple `GET /health` endpoint returning `200` with body `"ok"`
- `configure()` function returns a closure that configures the Actix Web App
- Avoids generic type issues with `actix_web::App`

### Integration Tests (`tests/`)
- `healthcheck.rs` - Tests the health endpoint with full service setup
- Uses in-process Actix Web testing with the configurator closure
- **Connects to database with auto-migration support**

## Safety Features

- **Database Guard**: Tests panic if `DATABASE_URL` doesn't end with `_test`
- **Auto-Migration**: Empty databases are automatically migrated on connection
- **Environment Isolation**: Test-specific environment via `.env.test`
- **Automatic Schema Migration**: `build_state()` automatically migrates empty databases for both prod and test

## Running Tests

```bash
# Run all backend tests (schema auto-migrates on first connection)
pnpm test

# Run tests with verbose output (requires DATABASE_URL to end with _test)
cargo test --verbose

# Run specific test (requires DATABASE_URL to end with _test)
cargo test test_health_endpoint
```

**Note**: Schema is automatically initialized on first connection to empty databases.

**Note**: Running `cargo test` directly (without `pnpm test`) will fail fast with a clear `_test` guard panic if `DATABASE_URL` doesn't end with `_test`.

## Example `.env.test`

```env
DATABASE_URL=postgresql://user:password@localhost:5432/nommie_test
```

## Architecture Notes

- Tests connect to the database and schema is auto-migrated if empty
- `build_state()` automatically handles schema initialization for all database types
- Integration tests use `actix_web::test` for in-process service testing
- The `_test` suffix guard ensures tests never run against production databases
- Schema validation is automatic - both prod and test startup validate schema readiness