# Nommie — Cursor Rules (v1.4.1)

> Repo-root file. Applies to **all Cursor edits, refactors, and codegen**.  

---

## General
- Follow these rules exactly. If a requested change would break them, **stop and ask for a new prompt**.  
- Don’t add rationale in code; just implement the rules.  

---

## Backend Code
- Handlers return `Result<T, AppError>` — never `HttpResponse`.  
- **All DB access** must go via `require_db(&state)` or `with_txn(&state, …)`.  
  Do not call lower-level connection helpers directly.  
- Domain modules: no DB or Actix imports.  
- Use enums (not strings) for states/roles/phases.  
- No `unwrap()` or `expect()` outside tests.  

---

## Rust Imports & `use` Rules

- Default: put all `use` at the **top of the module**, after docs/attrs.  
- Allowed inside a function/test only if the symbol is used there once or it avoids polluting scope.  
- Group order (with blank line): std::* then external crates then internal crates
- Sort alphabetically within groups.  
- Import traits explicitly; function-scoped if they’re one-off.  
- Avoid aliasing unless needed to resolve collisions.  
- Use `pub use` only in stable boundaries (`lib.rs`, `mod.rs`, prelude).  
- Tests may use `use super::*;` and local trait imports for ergonomics. 
- **No wildcard imports** in production. Allowed only in `#[cfg(test)]` and those allowed in the root clippy.toml.  

---

## Transactions
- Wrap all multi-row/table work in `with_txn`.  
- `with_txn` closures return `Result<_, AppError>`.  
- Nested/shared transactions are allowed.  

---

## Extractors
- Use only existing extractors: `AuthToken`, `JwtClaims`, `CurrentUser`, `CurrentUserDb`.  
- Don’t create new extractors unless explicitly asked.  
- Minimize DB hits (resolve user+membership efficiently).  

---

## Error Handling
- Errors follow Problem Details: `{ type, title, status, detail, code, trace_id }`.  
- `code` must come from the central registry (no ad-hoc strings).  

---

## AppState
- `AppState.db` is private. Access only via `state.db()` or `require_db(&state)`.  
- Don’t rebuild configs ad-hoc.  

---

## Schema & Migrations
- **Schema is managed exclusively by the SeaORM migrator crate** at `apps/backend/migration`.  
- There is **one init migration**: `apps/backend/migration/src/m20250823_000001_init.rs`.  
  - Do **not** add more migration files.  
  - When schema changes, update this single init migration to reflect the full current schema.  
- Do **not** create SQL files for schema.  
- Do **not** add DDL to `docker/postgres/init.sh` (that script only sets roles, extensions, and privileges).  

---

## Testing
- Tests must be deterministic (seed time/RNG).  
- No-DB tests must expect `DB_UNAVAILABLE` if they touch DB paths.  
- DB tests must use a `_test` database (guard enforced).  

### Running backend tests
- **Before any backend tests**, refresh the test DB:

    pnpm db:mig:test:refresh

- **Run all tests**:

    pnpm be:test     # backend  
    pnpm fe:test     # frontend  
    pnpm test        # all

### Running a subset of backend tests (cargo-nextest)
> ⚠️ Args to `pnpm be:test` go to `cargo nextest run`.  
> Use `--` to pass flags intended for the test binaries (e.g., `--nocapture`).

**A) By integration test binary (recommended)** — use the file name **without** `.rs`:

    pnpm be:test --test auth_login  
    pnpm be:test --test users_service_tests

**B) By test name substring (positional filter)** — matches test *function* names across all binaries:

    pnpm be:test test_me          # runs tests whose names contain "test_me"  
    pnpm be:test login_endpoint   # runs names containing "login_endpoint"

**C) By expression filter**

    pnpm be:test -- -E login  
    pnpm be:test -- -E 'test_login_endpoint_.*'

**Output control**

    pnpm be:test --test auth_login -- --nocapture

**Gotchas**
- `pnpm be:test auth_login` (bare word) filters by **test name**, not by binary.  
  If no test names contain `auth_login`, you’ll see “no tests to run.”  
  Use `--test auth_login` to select the integration-test binary named `auth_login`.  
- Don’t pass file paths or directories; nextest selects by **binary** (`--test`) or **name filter** (positional/`-E`).  

---

## Commands
- Lint:  
    pnpm be:lint  
    pnpm fe:lint  
    pnpm lint  

- Test:  
    pnpm be:test  
    pnpm be:test:v  
    pnpm be:test:q  
    pnpm fe:test  
    pnpm test  

- DB services:  
    pnpm db:svc:up  
    pnpm db:svc:down  
    pnpm db:svc:stop  
    pnpm db:svc:logs  
    pnpm db:svc:ready  
    pnpm db:svc:psql  

- Migrations:  
  - Prod:  
        pnpm db:mig:up  
        pnpm db:mig:refresh  
  - Test:  
        pnpm db:mig:test:up  
        pnpm db:mig:test:refresh  

---

## Safety
- Always source the repo-root `.env` before running backend or frontend commands:  

        set -a; source .env; set +a

  Do this **once per shell session** — you don’t need to repeat it before every command.  

- Only `.env.example` is committed; never commit real `.env` files.  
- No dotenv loaders (`dotenvx`, `dotenvy`) in code/scripts.  
- Never read `DATABASE_URL` directly.  
- Never run destructive ops against non-`_test` DBs.  
