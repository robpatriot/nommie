# Persistence and Transactions

## Architecture boundaries
- Repositories and services are stateless and accept connections as parameters.
- Only SeaORM adapters may depend on SeaORM.
- Services depend only on repository traits.
- Domain modules must not import database or Actix code.

## Connection types
- Single reads: ConnectionTrait
- Multiple unrelated reads: ConnectionTrait
- Multiple related reads requiring consistent snapshot: DatabaseTransaction
- Any mutation: DatabaseTransaction

## Transaction rules
- All database work uses require_db(&state) or with_txn(&state, ...).
- with_txn closures return Result<_, AppError>.
- Do not call begin, commit, or rollback directly.
- Production commits on Ok and rolls back on Err.
- Tests always roll back.
- Nested transactions only via SharedTxn in tests.
