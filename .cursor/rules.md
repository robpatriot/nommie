# Nommie — Cursor Rules (v1.4)

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

---

## Commands
- Lint: `pnpm be:lint`, `pnpm fe:lint`, `pnpm lint`  
- Test: `pnpm be:test`, `pnpm be:test:v`, `pnpm be:test:q`, `pnpm fe:test`, `pnpm test`  
- DB services: `pnpm db:svc:up|down|stop|logs|ready|psql`  
- Migrations:  
  - Prod: `pnpm db:mig:up`, `pnpm db:mig:refresh`  
  - Test: `pnpm db:mig:test:up`, `pnpm db:mig:test:refresh`  

---

## Safety
- No dotenv loaders (`dotenvx`, `dotenvy`) in code/scripts.  
- Never read `DATABASE_URL` directly.  
- Never run destructive ops against non-`_test` DBs.  
