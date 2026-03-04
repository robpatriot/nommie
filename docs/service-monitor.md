# 🔍 Nommie — Service Monitoring & Readiness (End-State Specification)

## Document Scope

This document defines the intended end-state behavior of Nommie’s monitoring, readiness, frontend degraded handling, and WebSocket interaction model.

It describes the authoritative semantics of:

- Backend dependency monitoring
- Readiness state transitions
- Frontend Suspect and Degraded states
- WebSocket error handling
- Reconciliation after failures

---

# 🌐 Core Principles

1. Backend readiness is the single source of truth.
2. Readiness reflects strict dependency health.
3. No artificial traffic is generated to force readiness transitions.
4. No additional database probing loop is introduced beyond the backend monitor and readiness probing mechanisms.
5. `/api/readyz` is a lightweight view of the readiness state machine and does not actively probe Postgres.
6. Frontend behavior aligns to backend readiness confirmation.
7. Eventual consistency is achieved without polling loops for game state.

---

# 🔁 Polling & Monitoring Model

- The backend runs an internal monitor loop that evaluates dependency state and manages readiness transitions.
- The frontend probes `/readyz` during Suspect and Degraded states.
- There is no continuous Postgres ping loop during Healthy state.
- There is no periodic game-state polling.

---

# ⚙️ Backend Readiness Model

## Dependency Recording

### Postgres

- Postgres health is event-driven.
- Failures are recorded when `DB_UNAVAILABLE` is emitted from:
  - HTTP request boundaries
  - WebSocket boundary handlers
- `require_db` does not update readiness.
- `/api/readyz` reports current readiness state only.

### Redis

Redis readiness is recorded from:

- Startup dependency resolution
- Publisher path failures
- Subscriber loop failures

No HTTP middleware is used for Redis.

---

## Canonical Dependency-Outage Codes

Only the following codes represent dependency outages:

- `DB_UNAVAILABLE`
- `REDIS_UNAVAILABLE`

These codes:

- Increment backend dependency failure counters
- Trigger frontend Suspect state
- Drive reconciliation behavior
- Are shared across HTTP and WebSocket transports

No generic `BACKEND_UNAVAILABLE` is defined.

---

# 🧭 Backend State Machine

## Modes

- `Startup`
- `Healthy`
- `Recovering`
- `Failed`

## Semantics

### Startup

- Initial state at boot.
- Service is Not Ready.
- Exits only when:
  - Migrations succeed, and
  - All dependencies meet success thresholds.

Failures during Startup do not create additional transitions (service is already Not Ready).

### Healthy

- All dependencies healthy.
- `/api/readyz` returns 200.

### Recovering

- Dependency failure threshold exceeded.
- `/api/readyz` returns 503.
- Recoverable when success thresholds are met.

### Failed

- Irrecoverable startup failure (e.g. migration mismatch).
- Entered immediately on migration failure.
- `/api/readyz` returns 503 permanently until restart.
- No in-process recovery path.

---

# 🖥️ Frontend Readiness Model

Frontend states:

- Healthy
- Suspect
- Degraded (Confirmed Not Ready)

Frontend assumes Healthy UX by default and does not show a banner on initial load.

---

## Suspect State

Suspect represents observed dependency failure that has not yet been confirmed by backend readiness.

### Entry Conditions

Entered when a dependency-outage signal is observed:

- HTTP response with `DB_UNAVAILABLE` or `REDIS_UNAVAILABLE`
- WebSocket error frame with those codes

### Behavior

- UI remains usable.
- A non-blocking toast/banner is shown.
- `/readyz` probing begins.
- WebSocket remains connected.
- No immediate state refetch occurs.
- The current game may set `needs_reconcile_after_recovery`.

### Transition Rules

Suspect does **not** clear based solely on `/readyz` returning 200.

Suspect remains active until one of the following occurs:

1. **Backend Confirmation of Degraded**
   - `/readyz` returns 503.
   - Transition to Degraded (blocking).

2. **Successful Normal Operation**
   - A subsequent user-driven operation that exercises the backend succeeds (e.g. a player action or authoritative state fetch).
   - Transition back to Healthy.
   - Clear any transient Suspect indicators.

Repeated dependency failures while in Suspect:
- Keep the frontend in Suspect.
- Continue showing appropriate toast messaging.
- Maintain any `needs_reconcile_after_recovery` flags.

---

## Degraded State

Degraded represents backend-confirmed not-ready state.

### Entry Condition

- `/readyz` returns 503.

### Behavior

- Blocking degraded page/screen is shown.
- Normal site usage is prevented.
- WebSocket connect/reconnect attempts are paused.
- `/readyz` probing continues.

### Exit Condition

- `/readyz` returns 200 consistently according to recovery threshold.
- Transition to Healthy.
- Perform post-recovery reconciliation if required.

---

# 🔌 WebSocket Behavior

## Dependency Outage in WS Handlers

When backend encounters DB/Redis failure during WS processing:

- Sends `error { code: DB_UNAVAILABLE | REDIS_UNAVAILABLE }`
- Keeps socket open
- Records readiness failure

*Implementation note:* Today only DB is used in the WS request path (e.g. subscribe/build_game_state), so only `DB_UNAVAILABLE` is sent over the socket. Redis failures are recorded in the broker (publisher/subscriber) and drive `/readyz`; no Redis failure is sent to clients over WS until we add a path that surfaces it (e.g. broker broadcast).

Client:

- Clears pending latches
- Enters Suspect
- Does not immediately refetch
- Sets `needs_reconcile_after_recovery` for the current game

---

## Protocol Errors (Malformed / Bad Protocol)

- Backend closes socket.
- Client reconnects with backoff (unless in Degraded).
- No automatic refetch.

---

## Authorization Errors (Forbidden)

- Backend sends error frame.
- Socket remains open.
- Client clears latches.
- No automatic refetch required.

---

# 🔁 HTTP Mutation Handling

## Success

- Optimistic update retained or reconciled via snapshot.
- If in Suspect, transition back to Healthy.
- No refetch required.

## Validation / Domain Failure

- Roll back to previous state.
- Show action-level error.
- No refetch required.

## Version Conflict / Optimistic Lock Mismatch

- Roll back.
- Perform one immediate refetch of authoritative game state.
- Converge to server truth.

## Dependency Outage

- Roll back optimistic update.
- Show service message.
- Enter or remain in Suspect.
- Do not immediately refetch.
- Set `needs_reconcile_after_recovery` for the current game.

---

# 🔄 Post-Recovery Reconciliation

Triggered when frontend transitions from Degraded back to Healthy.

If the current game has `needs_reconcile_after_recovery`:

- Perform one bounded refetch of game state.
- Clear the flag.
- Do not refetch again unless another qualifying event occurs.

Navigating to a different game naturally triggers a fresh state load.

---

# 🌐 Network Errors

Network failures are treated as client connectivity issues.

They do not assert backend dependency failure.

---

# ❌ Explicit Non-Goals

- No additional DB ping loop in Healthy state.
- No automatic refetch on dependency-outage WS errors.
- No refetch on every mutation failure.
- No closing WebSocket on dependency outage.
- No frontend escalation to Degraded based solely on local failure counts.
- No duplicate readiness updates from `require_db`.

---

# ✅ Stability Guarantees

This model guarantees:

- No frontend/backend threshold mismatch bounce.
- No infinite refresh loops during outages.
- No reliance on multiple users to confirm failure.
- Eventual convergence after recovery.
- Minimal additional load during degraded periods.
- Clear separation between dependency health, protocol correctness, domain validation, and network instability.