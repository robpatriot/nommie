# Backend Testing Rules

- Tests must be deterministic.
- Database tests must use the _test database.
- Use SharedTxn for tests needing continuity.
- Tests requiring committed data must use pooled DB setup.
- Use structured error assertions instead of string matching.
