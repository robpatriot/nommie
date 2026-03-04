# Error Handling (Backend)

Errors must follow Problem Details format:

type, title, status, detail, code, trace_id

- Create errors via AppError helpers.
- Error codes must come from the registry.
- Do not leak raw serde, SQL, or OAuth errors.

## Handlers
- Handlers return Result<T, AppError>.
- Request bodies must use ValidatedJson<T>.
