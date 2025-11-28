# Contributing to Nommie

This document outlines the development workflow and conventions for contributing to Nommie.

## Development Setup

### Prerequisites
- Node.js 18+
- pnpm 8+
- Rust stable
- Docker

### Initial Setup
1. Clone the repository
2. Install dependencies: `pnpm i`
3. Copy and source environment:
   ```bash
   cp docs/env.example.txt .env
   set -a; . ./.env; set +a
   ```
4. Start PostgreSQL (run manually):
   ```bash
   docker compose -f docker/dev-db/docker-compose.yml up -d postgres
   ```
5. Create/refresh databases (run manually using migration-cli):
   ```bash
   # Dev database
   cargo run --bin migration-cli -- --env prod --db postgres fresh
   # Test database
   cargo run --bin migration-cli -- --env test --db postgres fresh
   ```

## Development Commands

### Backend
- **Start:** `pnpm be:up` (logs to `.dev/dev.log`)
- **Stop:** `pnpm be:down`
- **Build:** `pnpm be:build`
- **Test:** `pnpm be:test`
- **Lint:** `pnpm be:lint`
- **Format:** `pnpm be:format`

### Frontend
- **Start:** `pnpm fe:up`
- **Stop:** `pnpm fe:down`
- **Build:** `pnpm fe:build`
- **Lint:** `pnpm fe:lint`
- **Format:** `pnpm fe:format`

### Database
Run docker-compose commands manually:
- **Start:** `docker compose -f docker/dev-db/docker-compose.yml up -d postgres`
- **Stop:** `docker compose -f docker/dev-db/docker-compose.yml stop postgres`
- **Destroy:** `docker compose -f docker/dev-db/docker-compose.yml down -v`
- **Check readiness:** `pnpm db:svc:ready`
- **View logs:** `docker compose -f docker/dev-db/docker-compose.yml logs -f postgres`
- **Connect via psql:** `docker compose -f docker/dev-db/docker-compose.yml exec postgres psql -U "${POSTGRES_USER}" -d "${POSTGRES_DB}"`

For migrations, use the migration-cli binary:
- **Migrate prod DB:** `cargo run --bin migration-cli -- --env prod --db postgres up`
- **Fresh prod DB:** `cargo run --bin migration-cli -- --env prod --db postgres fresh`
- **Fresh test DB:** `cargo run --bin migration-cli -- --env test --db postgres fresh`

### Combined
- **Start all:** `pnpm up` (starts backend and frontend)
- **Stop all:** `pnpm down` (stops backend and frontend)
- **Status:** `pnpm status` (shows backend and frontend status)
- **Lint all:** `pnpm lint`
- **Format all:** `pnpm format`

---

## Cursor Rules

This repo uses [Cursor](https://cursor.sh) for AI-assisted development.  
Project-specific conventions are locked in **`.cursor/rules.md`** — covering schema design, error handling, extractors, testing, and more.  

➡️ Always check that file before making changes; update it when project policies evolve.

---

## Environment Management

### Shell Sourcing
Environment variables must be sourced in your shell before running any commands:

```bash
set -a; . ./.env; set +a
```

**Important:** This must be done in each new shell session. The project does not use `dotenvx` or `dotenvy` - all environment loading is done via shell sourcing.

### Key Environment Variables
- `POSTGRES_HOST`, `POSTGRES_PORT` - Database connection
- `PROD_DB`, `TEST_DB` - Database names (test DB must end with `_test`)
- `APP_DB_USER`, `APP_DB_PASSWORD` - App role credentials
- `NOMMIE_OWNER_USER`, `NOMMIE_OWNER_PASSWORD` - Owner role credentials
- `APP_JWT_SECRET` - JWT signing secret
- `CORS_ALLOWED_ORIGINS` - Allowed CORS origins

## Code Conventions

### Rust (Backend)
- Follow `cargo fmt` and `cargo clippy` guidelines
- Use explicit error handling with `Result<T, AppError>`
- Domain logic stays pure (no DB/framework imports)
- Use enums over strings for states/roles/phases
- Prefer small, focused functions over large ones

### TypeScript/JavaScript (Frontend)
- Follow ESLint and Prettier configurations
- Prefer single formatted strings over concatenation

### Module Organization
- Module declarations at top of parent files (`mod`, `pub mod`)
- Grouped `use` statements at top of each file (std, extern, crate)
- `pub use` only in `lib.rs` or `prelude` modules

## Testing

### Backend Tests
- Run with: `pnpm be:test`
- Tests use `TEST_DB` (enforced `_test` suffix)
- All tests must be deterministic
- Use `StateBuilder` for test state creation

### Test Database Safety
- All destructive operations require `_test` suffix
- Never run destructive ops against production databases
- Use `MIGRATION_TARGET=test` for test database operations

## Database Migrations

Migrations run with the **Owner** role using the migration-cli binary:

- **Production:** `cargo run --bin migration-cli -- --env prod --db postgres up`
- **Test:** `cargo run --bin migration-cli -- --env test --db postgres up`
- **Fresh (prod):** `cargo run --bin migration-cli -- --env prod --db postgres fresh`
- **Fresh (test):** `cargo run --bin migration-cli -- --env test --db postgres fresh`

## Architecture Guidelines

### Config vs Infra Separation
- **Config** (`apps/backend/src/config/`): Pure environment parsing and URL construction
- **Infra** (`apps/backend/src/infra/`): Database connections and state building
- **Test Support** (`apps/backend/src/test_support/`): Test-only helpers

### Error Handling
- Use `AppError` with Problem Details format
- Include `trace_id` in all error responses
- Use `SCREAMING_SNAKE_CASE` error codes from central registry

### State Management
- `AppState` holds DB pool and `SecurityConfig`
- Inject via `web::Data<AppState>`
- Don't clone/rebuild security config ad-hoc

## Pull Request Process

1. Create a feature branch from `main`
2. Make your changes following the conventions above
3. Run tests: `pnpm be:test`
4. Run linters: `pnpm lint`
5. Run formatters: `pnpm format`
6. Ensure all tests pass
7. Submit a pull request with a clear description

## Questions?

Check the main [README.md](README.md) for more details about the project architecture and setup.
