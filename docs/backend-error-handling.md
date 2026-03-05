# Backend Error Handling

## Purpose

Defines the backend error model and how errors are mapped to HTTP Problem Details responses.

## Error Types

The backend uses two layers of error types.

### DomainError

Domain-level errors that are independent of HTTP and database specifics.

### AppError

HTTP-aware errors used at request boundaries.

AppError is the canonical error type for HTTP responses.

DomainError is automatically mapped into AppError at the boundary.

## Error Codes

Errors are identified by a machine-readable error code enum.

Rules:

- error codes are type-safe (no ad-hoc strings)
- error codes are stable once published
- error responses expose only the code and sanitized human-readable detail

## HTTP Mapping

All HTTP error responses follow RFC 7807 Problem Details format.

Fields:

type  
title  
status  
detail  
code  
trace_id

HTTP status codes are determined by AppError variant class.

## Database Error Mapping

Database errors are sanitized and translated through a centralized mapper.

Rules:

- raw database errors must not be exposed
- dependency failures map to a distinct unavailability error class
- constraint violations map to structured domain error kinds

## Optimistic Lock Conflicts

Optimistic locking uses a version column incremented on each update.

On version mismatch:

- the backend returns a conflict error
- the response includes a machine-readable code
- the response detail includes expected and actual versions when available

## PII Safety

Rules:

- do not expose raw database messages to clients
- do not include sensitive identifiers in error details
- logs may include trace identifiers for correlation

## Response Headers

Status-specific headers are added where required.

Examples:

- 401 responses include `WWW-Authenticate: Bearer`
- 503 responses may include `Retry-After`

## Testing Requirements

Tests must assert:

- HTTP status
- error code
- Problem Details structure
- required headers for relevant status classes
