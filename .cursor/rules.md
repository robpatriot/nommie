# Nommie — Cursor Rules (v1.3.2)

> Keep this file at repo root. It applies to all AI actions (generate, edit, refactor, move/rename). If you must deviate, leave a short code comment explaining why.

## Purpose
- Apply these rules to all AI-assisted edits, refactors, and codegen.
- Prefer clarity and small, reviewable diffs. Explain non-obvious changes in code comments adjacent to the change.

## Tech Stack Hints
- Backend: Rust, Actix Web, SeaORM, PostgreSQL.
- Frontend: Next.js (App Router), TypeScript, Tailwind.
- Monorepo layout: /apps/frontend, /apps/backend, /packages, /docs.

## Environment & Commands Policy
- We do not use dotenv loaders (dotenvx, dotenvy) in code or scripts.
- Always source env in your shell before running commands: run `set -a; . ./.env; set +a` once per shell session.
- .env.example contains component vars only (no DATABASE_URL). Code builds URLs from parts.
- Never read DATABASE_URL directly in code. Use `config::db::db_url(profile, owner)` or `infra::db::connect_db(profile, owner)`.

## Module Boundaries
- config/ — pure settings (env parsing, small helpers). No I/O.
  - config/db.rs → DbProfile, DbOwner, db_url(profile, owner).
- infra/ — runtime infrastructure (does I/O, owns handles).
  - infra/db.rs → connect_db(profile, owner) using db_url; enforces “_test” safety.
  - infra/state.rs → StateBuilder; builds AppState (uses App role).
- test_support/ — test-only helpers (e.g., logging init). No production wiring.
- Domain modules contain no DB/framework code.

## Imports & Re-exports Policy (Rust)
1) Module declarations first at the top: `pub mod ...; mod ...;`
2) One imports block per file, placed after module declarations.
3) Group imports with a blank line between groups and alphabetical sorting within each group:
   - `use std::...`
   - external crates (not `std`, `crate`, or `super`)
   - `use crate::...` and `use super::...`
4) No wildcard imports in production code. Tests may use wildcard imports (e.g., `use crate::prelude::*;`) when it improves readability.
5) Centralize re-exports:
   - Re-export the public surface from `apps/backend/src/lib.rs` under a single re-exports block.
   - Provide a minimal `pub mod prelude` in `lib.rs` for tests. Production code should import explicit items from the crate root (e.g., `use crate::{DbProfile, DbOwner};`), not wildcard preludes.
   - Avoid `pub use` in leaf modules. If a leaf module must expose items publicly, re-export them from `lib.rs` instead.
6) Rustfmt and Clippy enforcement:
   - rustfmt.toml at repo root should include:
     - edition = "2021"
     - group_imports = "StdExternalCrate"
     - imports_granularity = "Module"
     - reorder_imports = true
     - reorder_modules = true
   - Deny wildcard imports in non-test code via crate attributes at the top of lib.rs and main.rs:
     - `#![deny(clippy::wildcard_imports)]`
     - `#![cfg_attr(test, allow(clippy::wildcard_imports))]`

## Global Conventions
- Domain logic stays pure: no DB access and no web framework imports in domain modules.
- Use enums over strings for states/roles/phases (code + schema).
- Prefer explicit, narrow interfaces and small functions over god objects.
- Respect linters/formatters: Rustfmt + Clippy, ESLint + Prettier.
- Tests must be deterministic; avoid time/RNG leaks unless seeded/injected.

## String Interpolation (JS/TS & Shell)
- JS/TS: use template literals like `${value}`.
- Shell: expand with `${VAR}` or `"$VAR"` as appropriate.
- Prefer a single formatted string over ad-hoc concatenation where clarity matters.

## Error Handling & Responses
- Handlers return `Result<T, AppError>` — never raw `HttpResponse`.
- Problem Details shape: `{ type, title, status, detail, code, trace_id }`.
- `code` is SCREAMING_SNAKE_CASE from a central registry (no ad-hoc strings).
- Ensure per-request `trace_id` flows into logs and responses.

## AppState & Configuration
- AppState holds DB pool and SecurityConfig; inject via `web::Data<AppState>`.
- Do not clone/rebuild security config ad-hoc; use shared state.
- Build DB URLs from env parts; tests must use a DB whose name ends with `_test`.

## Extractors
- Available now: AuthToken, JwtClaims, CurrentUser, CurrentUserDb.
- Planned (do not synthesize until requested): ValidatedJson<T>, GameId, GameMembership.

## Schema & Migrations
- Canonical schema is managed via the SeaORM migrator crate.
- Migrations run with the Owner role via `infra::db::connect_db(profile, DbOwner::Owner)`.
- Fresh/reset operations are allowed only against `_test` databases.
- Primary keys: bigint identity; timestamps in UTC; add indexes for FKs/frequent queries.
- Use enums for persistent states/roles/phases (no string literals in schema).

## Persistence Patterns (SeaORM)
- Prefer explicit column selection over `select(*)`.
- On updates/deletes check `rows_affected`; treat `0` as meaningful.
- No `unwrap()`/`expect()` outside tests; use `?` and map to `AppError`.
- Use explicit transactions for multi-row or multi-table flows.

## Transactions & Concurrency
- Use `DatabaseTransaction` for atomic sequences.
- Keep transactions short; avoid long-lived locks.
- For “only-one-writes,” use row-level locking or safe uniqueness constraints; never rely on in-memory cross-request invariants.

## Time, Randomness & Determinism
- No `SystemTime::now()` in domain code. Inject a Clock trait; use a fixed TestClock in tests.
- RNG must be injected and seeded in tests for reproducibility.

## Testing Policy
- Unit tests for new code; integration tests for complex flows.
- Provide assertion helpers for Problem Details and `trace_id`.
- Prefer property tests for trick/scoring invariants where practical.
- Tests are isolated and reset DB state; no test-order coupling.
- Tests that touch the DB must validate they’re using a “_test” database (guard enforced by infra/db.rs).

## Testing Commands (current)
- Backend: `pnpm be:test` (cargo test with nocapture).
- Migrations:
  - Prod: `pnpm db:migrate`
  - Fresh prod: `pnpm db:fresh`
  - Fresh test: `pnpm db:fresh:test`
- Postgres helpers:
  - Ready check: `pnpm db:pg_isready`
  - psql shell: `pnpm db:psql`

## Safety Rails
- Do not introduce dotenvx/dotenvy in code or scripts.
- Do not read DATABASE_URL directly; always compose via db_url or connect via connect_db.
- Never run destructive operations against non-“_test” databases.
