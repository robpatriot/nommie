# Backend Testing Guide

## Scope

Defines how backend automated tests select and initialize databases, and the safety checks that prevent accidental production access.

## Database Configuration Rules

- The backend constructs database connection URLs from env parts (not `DATABASE_URL`).
- PostgreSQL tests use `TEST_DB` and it must end with `_test`.
- Test schema is initialized automatically for empty databases by the test state builder.

## Test State Helpers

The test harness provides helpers that:

- set the runtime environment to Test
- initialize schema for empty databases

## Safety Guarantees

- PostgreSQL tests fail fast if `TEST_DB` does not end with `_test`.
- Tests are isolated from production databases by configuration rules and guards.

## Running Tests

Source env once per shell:

set -a; source apps/backend/.env; source docker/dev-db/db.env; set +a

Run all backend tests:

pnpm be:test

Run with verbose output:

pnpm be:test:v

Run a specific test:

pnpm be:test -- test_health_endpoint

## Example Runs

Run all backend tests:

pnpm be:test

Run full test suite:

pnpm be:test:full


