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

## Core Rules

- Backend readiness is the single source of truth for system availability.
- Readiness reflects dependency health.
- `/api/readyz` reports the current readiness state and does not actively probe dependencies.
- The frontend does not infer backend readiness state solely from local error counts.
- Game state synchronization remains snapshot-based and does not rely on polling loops.

## Monitoring Model

- The backend runs an internal monitor that tracks dependency health and controls readiness transitions.
- The frontend probes `/readyz` when in Suspect or Degraded states.
- There is no continuous database ping loop during Healthy operation.
- Game state is not periodically polled.

## Dependency Monitoring

### Postgres

Postgres outages are recorded when the backend emits the error code:

- `DB_UNAVAILABLE`

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

## Canonical Dependency-Outage Codes

Only the following codes represent dependency outages:

- `DB_UNAVAILABLE`
- `REDIS_UNAVAILABLE`

These codes:

- record dependency failure in the backend monitor
- cause the frontend to enter Suspect state

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

## Frontend Availability States

The frontend operates in three states:

- Healthy
- Suspect
- Degraded

The UI assumes Healthy by default.

### Suspect

Suspect represents an observed dependency outage that has not yet been confirmed by backend readiness.

Entry conditions:

- HTTP response containing `DB_UNAVAILABLE` or `REDIS_UNAVAILABLE`
- WebSocket `error` frame containing those codes

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

## Post-Recovery Reconciliation

When the frontend transitions from Degraded to Healthy:

If the current game is marked as needing reconciliation:

- perform one bounded refetch of authoritative game state
- clear the reconciliation flag
