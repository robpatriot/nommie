# Testing Guide

This document describes the testing setup for the Nommie backend.

## Database Environment Policy

### Code Requirements
- **Backend code reads only `DATABASE_URL`** - no references to `TEST_DATABASE_URL`
- **Tests must run against a database whose name ends with `_test`**
- **Tests must NEVER run migrations** - use `pnpm db:fresh:test` to prepare schema
- **The test runner automatically derives the test database URL** by appending `_test` to the existing `DATABASE_URL`

### Database Setup
- Tests run against a database suffixed with `_test`
- **Schema must be prepared manually using `pnpm db:fresh:test` before running tests**
- Tests only connect to the database and build the app - no migrations are run
- **Running migrations in tests will panic with clear instructions**

## Test Structure

### Test Support Module (`src/test_support/`)
- `assert_test_db_url(url: &str)` - Validates database URL ends with `_test`
- `load_test_env()` - Loads `.env.test` configuration
- `get_test_db_url()` - Retrieves database URL from environment
- **`migrate_test_db()` - Panics with instructions to use `pnpm db:fresh:test`**

### Health Endpoint (`src/health.rs`)
- Simple `GET /health` endpoint returning `200` with body `"ok"`
- `configure()` function returns a closure that configures the Actix Web App
- Avoids generic type issues with `actix_web::App`

### Integration Tests (`tests/`)
- `healthcheck.rs` - Tests the health endpoint with full service setup
- Uses in-process Actix Web testing with the configurator closure
- **Only connects to database - does not run migrations**

## Safety Features

- **Database Guard**: Tests panic if `DATABASE_URL` doesn't end with `_test`
- **Migration Prevention**: Any attempt to run migrations in tests will panic
- **Environment Isolation**: Test-specific environment via `.env.test`

## Running Tests

```bash
# First, prepare the test database schema
pnpm db:fresh:test

# Then run all backend tests (automatically sets DATABASE_URL to *_test)
pnpm test

# Run tests with verbose output (requires DATABASE_URL to end with _test)
cargo test --verbose

# Run specific test (requires DATABASE_URL to end with _test)
cargo test test_health_endpoint
```

**Important**: You must run `pnpm db:fresh:test` before running tests. Tests will panic if you try to run migrations.

**Note**: Running `cargo test` directly (without `pnpm test`) will fail fast with a clear `_test` guard panic if `DATABASE_URL` doesn't end with `_test`.

## Example `.env.test`

```env
DATABASE_URL=postgresql://user:password@localhost:5432/nommie_test
```

## Architecture Notes

- Tests only connect to the database and build the app
- No migrations are run during tests - schema must be prepared beforehand
- Integration tests use `actix_web::test` for in-process service testing
- The `_test` suffix guard ensures tests never run against production databases
