# Nommie — Cursor Rules (v1.6)

> Repo-root file. Applies to **all Cursor edits, refactors, and codegen**.  

---

## General
- Follow these rules exactly. If a request would break them, **stop and ask**.  
- Don’t add rationale or comments; just implement.  

---

## Backend Code
- Handlers return `Result<T, AppError>` — never `HttpResponse`.  
- **All DB access** via `require_db(&state)` or `with_txn(&state, …)`.  
- Service/repo functions take `&dyn DbConn` (dyn trait, not generics).  
- Only adapters may `use sea_orm::*`.  
- Domain modules: no DB or Actix imports.  
- Use enums (not strings) for states/roles/phases.  
- No `unwrap()` or `expect()` outside tests.  

---

## Transactions
- Wrap multi-row/table work in `with_txn`.  
- `with_txn` closures return `Result<_, AppError>`.  
- Nested txns not supported (except via `SharedTxn` in tests).  
- Prod: commit on Ok, rollback on Err.  
- Tests: always rollback.  
- Don’t call `begin/commit/rollback` directly.  

---

## Extractors
- Use only these: `AuthToken`, `JwtClaims`, `CurrentUser`, `CurrentUserDb`, `GameId`, `ValidatedJson<T>`.  
- `ValidatedJson<T>` must map errors to `AppError` Problem Details.  

---

## Error Handling
- Errors follow Problem Details: `{ type, title, status, detail, code, trace_id }`.  
- Create all errors via central `AppError` helpers.  
- `code` comes from registry (no ad-hoc strings).  
- Never leak raw serde/SQL/OAuth errors.  

---

## Logging
- Logs must include `trace_id`.  
- No PII (mask or hash emails / google_sub).  

---

## AppState
- `AppState.db` is private. Use `state.db()` / `require_db(&state)`.  

---

## Schema & Migrations
- Schema is one init migration in `apps/backend/migration`.  
- Don’t add extra migration files, SQL, or DDL elsewhere.  

---

## Env & pnpm
- Always source `.env` once per shell session before pnpm commands:  
  set -a; source .env; set +a
- Never commit real `.env`.  
- Don’t read `DATABASE_URL` directly in code.  

---

## Testing
- Tests must be deterministic (seed time/RNG).  
- DB tests use `_test` DB (rollback enforced).  
- Use `SharedTxn` only when a test needs continuity.  
- Control tests that need committed data must use pooled DB setup.  
- Run backend tests with `pnpm be:test`.  
