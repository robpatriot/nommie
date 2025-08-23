# Testing Guide

This document describes the testing setup for the Nommie backend.

## Requirements

### Environment Configuration
- Tests require a `.env.test` file with test-specific configuration
- The `DATABASE_URL` must end with `_test` to prevent accidental writes to production databases
- The test environment is loaded automatically via `dotenvy::from_filename(".env.test").ok()`

### Database Setup
- Tests run against a database suffixed with `_test`
- SeaORM migrations are automatically applied for tests
- The `migrate_test_db()` function ensures migrations only run once per test process

## Test Structure

### Test Support Module (`src/test_support/`)
- `assert_test_db_url(url: &str)` - Validates database URL ends with `_test`
- `load_test_env()` - Loads `.env.test` configuration
- `migrate_test_db(db_url: &str)` - Connects and migrates test database
- `get_test_db_url()` - Retrieves database URL from environment

### Health Endpoint (`src/health.rs`)
- Simple `GET /health` endpoint returning `200` with body `"ok"`
- `build_app()` function returns a closure that configures the Actix Web App
- Avoids generic type issues with `actix_web::App`

### Integration Tests (`tests/`)
- `healthcheck.rs` - Tests the health endpoint with full service setup
- Uses in-process Actix Web testing with the configurator closure
- Validates database connectivity and migration

## Running Tests

```bash
# Run all backend tests
pnpm test

# Run tests with verbose output
cargo test --verbose

# Run specific test
cargo test test_health_endpoint
```

## Safety Features

- **Database Guard**: Tests panic if `DATABASE_URL` doesn't end with `_test`
- **Migration Isolation**: Migrations run only once per test process
- **Environment Isolation**: Test-specific environment via `.env.test`

## Example `.env.test`

```env
DATABASE_URL=postgresql://user:password@localhost:5432/nommie_test
```

## Architecture Notes

- The `build_app()` function returns a closure to avoid Actix Web generic type issues
- Test database migrations use `OnceCell` for process-level singleton behavior
- Integration tests use `actix_test` for in-process service testing
