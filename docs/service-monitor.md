# Service Monitoring and Readiness

## Scope

This document specifies how Nommie detects dependency outages, exposes backend readiness state, and how the frontend reacts to those states.

It defines:

- backend dependency monitoring
- backend readiness states
- frontend Healthy / Suspect / Degraded behavior
- HTTP mutation failure handling
- post-recovery reconciliation behavior

WebSocket error handling semantics are defined in `docs/websocket-design.md`.

---

## Core Rules

- Backend readiness is the single source of truth for system availability.
- Readiness reflects dependency health.
- `/api/readyz` reports the current readiness state and does not actively probe dependencies.
- The frontend does not infer backend readiness state solely from local error counts.
- Game state synchronization remains snapshot-based and does not rely on polling loops.

---

## Public Error Contract

Infrastructure or readiness unavailability is exposed using a single public error contract.

Responses use:

- HTTP status: `503`
- Content-Type: `application/problem+json`
- RFC 7807 response body
- `code: SERVICE_UNAVAILABLE`

Example:

{
  "type": "...",
  "title": "Service unavailable",
  "status": 503,
  "code": "SERVICE_UNAVAILABLE",
  "detail": "...",
  "trace_id": "..."
}

Dependency-specific causes (DB, Redis, etc.) are not exposed to clients.
Those details remain server-side for logging and diagnostics.

---

## Monitoring Model

- The backend runs an internal monitor that tracks dependency health and controls readiness transitions.
- The frontend probes `/readyz` when in Suspect or Degraded states.
- There is no continuous database ping loop during Healthy operation.
- Game state is not periodically polled.

Dependency outages may surface through two paths:

1. Direct request failure  
   A request reaches a handler and fails due to dependency unavailability.  
   The response returns `503 SERVICE_UNAVAILABLE`.

2. Readiness gate blocking  
   After the backend confirms an outage via its internal thresholds, the readiness gate blocks further requests and returns `503 SERVICE_UNAVAILABLE` before the handler executes.

Readiness transitions are threshold-based.

A single dependency failure does not immediately flip readiness.
The internal monitor requires multiple confirmed failures before transitioning from Healthy to Recovering.

---

## Dependency Monitoring

### Postgres

Postgres outages are recorded when the backend emits infrastructure unavailability.
Internally the backend tracks DB vs Redis; the public client contract uses:

- `SERVICE_UNAVAILABLE`

Failures may originate from:

- HTTP request handlers
- WebSocket request handlers

`require_db` does not modify readiness state.

### Redis

Redis outages are recorded from:

- startup dependency resolution
- publisher failures
- subscriber loop failures

Redis monitoring is internal to the backend and not implemented as HTTP middleware.

---

## Backend Readiness States

The backend readiness state machine contains four states:

- Startup
- Healthy
- Recovering
- Failed

### Startup

Initial state at process boot.

The service is Not Ready until:

- migrations succeed
- dependencies meet success thresholds

### Healthy

All dependencies are healthy.

`/api/readyz` returns `200`.

### Recovering

Dependency failure threshold has been exceeded.

`/api/readyz` returns `503`.

The system returns to Healthy after recovery thresholds are met.

### Failed

Irrecoverable startup failure (for example migration mismatch).

`/api/readyz` returns `503` permanently until restart.

---

## Frontend Availability States

The frontend operates in three states:

- Healthy
- Suspect
- Degraded

The UI assumes Healthy by default.

### Suspect

Suspect represents an observed dependency outage that has not yet been confirmed by backend readiness.

Entry conditions:

- HTTP response containing `SERVICE_UNAVAILABLE` (503, RFC 7807)
- WebSocket `error` frame containing `service_unavailable`

Behavior:

- UI remains usable
- a non-blocking banner or toast is shown
- `/readyz` probing begins
- no automatic game state refetch occurs

Suspect does not clear based solely on `/readyz` returning 200.

Suspect exits when either:

- `/readyz` returns 503 (transition to Degraded), or
- a subsequent successful backend operation confirms normal service behavior

Repeated dependency outages keep the frontend in Suspect.

### Degraded

Degraded represents backend-confirmed unavailability.

Entry condition:

- `/readyz` returns 503

Behavior:

- a blocking degraded screen is shown
- `/readyz` probing continues

Exit condition:

- `/readyz` returns 200 consistently

On exit to Healthy, perform post-recovery reconciliation if required.

---

## HTTP Mutation Failure Handling

### Success

- optimistic state is retained or reconciled

If the frontend is in Suspect, a successful mutation transitions to Healthy.

### Validation or Domain Error

- optimistic state rolls back
- no automatic refetch occurs

### Version Conflict

- optimistic state rolls back
- one authoritative snapshot refetch is performed

### Dependency Outage

- optimistic state rolls back
- frontend enters or remains in Suspect
- no immediate refetch occurs
- mark the current game as needing post-recovery reconciliation

---

## Post-Recovery Reconciliation

When the frontend transitions from Degraded to Healthy:

If the current game is marked as needing reconciliation:

- perform one bounded refetch of authoritative game state
- clear the reconciliation flag