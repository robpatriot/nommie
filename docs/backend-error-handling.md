# Error Handling Architecture

## Document Scope

This guide covers the backend error model: how database, domain, and HTTP layers
map errors to RFC 7807 responses while staying free of PII. Architectural
context lives in `architecture-overview.md`; testing guidance for these patterns
is captured in `backend-testing-guide.md`.

**Version:** 1.2  
**Last Updated:** 2025-10-10  
**Status:** Active

## Overview

This document describes Nommie's error handling architecture across all layers of the backend application. The system follows RFC 7807 Problem Details for HTTP APIs and implements a two-layer error model that separates domain concerns from HTTP/transport concerns.

> **Note:** This document focuses on architectural patterns and design rationale. For complete type definitions, error codes, and implementation details, refer to the source files referenced throughout.

### Design Principles

1. **Layer Separation:** Domain errors (`DomainError`) are HTTP-agnostic; HTTP errors (`AppError`) handle web concerns
2. **Type Safety:** Machine-readable error codes via enums, no ad-hoc strings
3. **PII Safety:** Structured error mapping sanitizes database and internal errors before exposure
4. **Automatic Conversion:** Seamless error propagation via `From` implementations
5. **Problem+JSON:** RFC 7807 compliance for all HTTP error responses

### Error Flow

```
Database/Adapter Layer → Domain Layer → HTTP Layer → Client
    (DbErr)          →  (DomainError) → (AppError) → (Problem+JSON)
```

---

## Architecture Layers

### Layer 1: Top-Level Error Types

**Location:** `apps/backend/src/error.rs`, `apps/backend/src/errors/`

#### AppError (HTTP-Aware)

The canonical application error type that implements `ResponseError` for Actix-web. See `src/error.rs` for the complete definition.

**Variant categories:**
- **Domain errors:** `Validation`, `NotFound`, `BadRequest`, `Conflict` (carry `ErrorCode` + detail)
- **Auth errors:** `Unauthorized`, `Forbidden` (+ specific variants for JWT/Bearer issues)
- **Infrastructure errors:** `Db`, `DbUnavailable`, `Timeout`, `Internal`, `Config`

**Key characteristics:**
- Most variants carry an `ErrorCode` for machine-readable identification
- Status codes determined by variant type
- Helper constructors for common patterns (e.g., `AppError::not_found()`, `AppError::conflict()`)

#### ErrorCode (Machine-Readable Codes)

Type-safe enum providing machine-readable error codes for the HTTP API. See `src/errors/error_code.rs` for the complete registry.

**Categories:**
- **Auth** (~8 variants): `Unauthorized`, `UnauthorizedMissingBearer`, `UnauthorizedInvalidJwt`, etc.
- **Validation** (~11 variants): `InvalidGameId`, `InvalidBid`, `MustFollowSuit`, `PhaseMismatch`, etc.
- **NotFound** (~4 variants): `GameNotFound`, `UserNotFound`, `PlayerNotFound`, etc.
- **Conflicts** (~6 variants): `OptimisticLock`, `SeatTaken`, `UniqueEmail`, `JoinCodeConflict`, etc.
- **Infrastructure** (~10 variants): `DbError`, `DbUnavailable`, `DbTimeout`, `DataCorruption`, etc.

All codes map to SCREAMING_SNAKE_CASE strings via `as_str()` for HTTP responses (e.g., `ErrorCode::OptimisticLock` → `"OPTIMISTIC_LOCK"`).

#### DomainError (HTTP/DB-Agnostic)

Domain-level errors used by services and repositories. See `src/errors/domain.rs` for the complete definition.

**Conversion:** `impl From<DomainError> for AppError` provides automatic mapping.

---

### Layer 2: Repository & Adapter Conventions

**Location:** `apps/backend/src/repos/`, `apps/backend/src/adapters/`

#### Repository Functions

Repositories are **stateless** and **generic** over `ConnectionTrait`. They return `DomainError`.

#### SeaORM Adapters

Adapters handle ORM-specific concerns and return `sea_orm::DbErr`.

#### Automatic Error Conversion

**Location:** `apps/backend/src/infra/db_errors.rs`

Central `map_db_err` function translates SeaORM errors to domain errors. This enables seamless error propagation with `?` operator across all layers.

#### Helper Constructors

Both error types provide convenience constructors:

```rust
// AppError helpers
AppError::not_found(ErrorCode::GameNotFound, "Game not found")
AppError::conflict(ErrorCode::OptimisticLock, "Resource modified; retry")
AppError::bad_request(ErrorCode::InvalidGameId, "Invalid ID")

// DomainError helpers
DomainError::not_found(NotFoundKind::Game, "Game not found")
DomainError::conflict(ConflictKind::OptimisticLock, detail)
DomainError::validation(ValidationKind::PhaseMismatch, detail)
```

---

### Layer 3: HTTP Mapping

**Location:** `apps/backend/src/error.rs`

#### ResponseError Implementation

`AppError` implements `ResponseError` for Actix-web, automatically converting to HTTP responses.

#### HTTP Status Code Mapping

Status codes are determined by the `AppError` variant. See `src/error.rs` for complete mapping logic.

**Key mappings:**
- `NotFound` → 404
- `BadRequest`, `Validation` → 400 or 422
- `Unauthorized` → 401
- `Forbidden` → 403
- `Conflict` (including optimistic locks) → 409
- `DbUnavailable` → 503
- `Timeout` → 504
- `Db`, `Internal` → 500

#### Problem Details Response Format (RFC 7807)

All error responses follow RFC 7807 Problem Details format:

```json
{
  "type": "https://nommie.app/errors/OPTIMISTIC_LOCK",
  "title": "Optimistic Lock",
  "status": 409,
  "detail": "Resource was modified concurrently (expected version 12, actual version 13). Please refresh and retry.",
  "code": "OPTIMISTIC_LOCK",
  "trace_id": "abc123-def456-..."
}
```

#### HTTP Headers

Status-specific headers are automatically added:
- **401 Unauthorized:** `WWW-Authenticate: Bearer`
- **503 Service Unavailable:** `Retry-After: 1`

---

## Special Cases

### Optimistic Locking

**Location:** `apps/backend/src/adapters/games_sea/mod.rs`

Optimistic locking is implemented using a `version` column (i32) that increments on every update.

#### Detection Pattern

When an update affects zero rows due to a lock version mismatch, the adapter returns a structured error payload that includes both expected and actual version numbers.

#### Structured Error Payload

Optimistic lock errors include version information:
- **Adapter:** Returns `DbErr::Custom("OPTIMISTIC_LOCK:{"expected":12,"actual":13}")`
- **Mapper:** Parses JSON and creates detailed error message
- **HTTP:** Returns 409 with human-readable detail including version numbers

**Error flow:**
```
Adapter: DbErr::Custom("OPTIMISTIC_LOCK:{...}")
    ↓
DomainError: Conflict(ConflictKind::OptimisticLock, "...expected X, actual Y...")
    ↓
AppError: Conflict { code: OptimisticLock, detail: "..." }
    ↓
HTTP: 409 with Problem+JSON
```

---

### Database Error Mapping

**Location:** `apps/backend/src/infra/db_errors.rs`

Database errors are sanitized and mapped based on error type and SQLSTATE codes. The `map_db_err` function provides centralized translation of all `DbErr` variants to `DomainError`.

**Mapping strategy:**
- `RecordNotFound` → `DomainError::not_found()`
- `Custom("OPTIMISTIC_LOCK:...")` → parsed and mapped to conflict (see Optimistic Locking section)
- `ConnectionAcquire`, `Conn` → `DomainError::infra(DbUnavailable)`
- Constraint violations → inspected via SQLSTATE codes (see below)
- Timeout/pool errors → `DomainError::infra(Timeout)`
- Unrecognized errors → `DomainError::infra(Other)` with sanitized message

#### SQLSTATE Mappings

- **23505** (Unique violation) → `Conflict` (409)
  - Specific constraint name inspection for detailed error codes (e.g., `user_credentials_email_key` → `UniqueEmail`)
- **23503** (Foreign key violation) → `Validation` (400)
- **23514** (Check constraint violation) → `Validation` (400)

---

## Best Practices

### When Adding New Errors

1. **Add error code to `ErrorCode` enum** in `errors/error_code.rs`
2. **Add corresponding variant to domain error kinds** if needed (e.g., `ConflictKind`, `ValidationKind`)
3. **Update mapping in `From<DomainError> for AppError`** if new kind added
4. **Use helper constructors** for consistency
5. **Test error responses** to verify Problem+JSON format

### Error Construction

✅ **DO:**
```rust
AppError::conflict(ErrorCode::OptimisticLock, "Resource modified")
DomainError::validation(ValidationKind::PhaseMismatch, "Expected BIDDING phase")
```

❌ **DON'T:**
```rust
AppError::Conflict { 
    code: ErrorCode::Conflict,  // too generic
    detail: format!("error: {}", raw_db_error)  // PII leakage
}
```

### PII Safety

- Never expose raw database errors to HTTP responses
- Sanitize error messages through `map_db_err`
- Use `Redacted` wrapper for logging sensitive data
- All errors include `trace_id` for correlation without exposing internals

### Testing

- Verify HTTP status codes for each error variant
- Check Problem+JSON structure compliance
- Test error code uniqueness
- Validate header presence (401/503)
- Assert structured error messages (e.g., optimistic lock versions)

---

## Future Enhancements

Potential improvements for consideration:

1. **HTTP ETag/If-Match support:** RESTful conditional requests for optimistic locking
2. **Problem+JSON extensions:** Add custom fields per RFC 7807 section 3.2
3. **Structured error context:** Carry typed data in error variants (e.g., `OptimisticLock(expected, actual)`)
4. **Error analytics:** Aggregate error codes for monitoring
5. **Client-side retry hints:** Add machine-readable retry policies to responses

---

## Related Documentation

- [Architecture Overview](./architecture-overview.md)
- [Backend Testing Guide](./backend-testing-guide.md)
- [RFC 7807 - Problem Details for HTTP APIs](https://tools.ietf.org/html/rfc7807)
