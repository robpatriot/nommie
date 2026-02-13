# Longest-Waiting Frontend Cache Design

This document explains how the frontend caches the **Longest-Waiting (LW)** endpoint.

The goal is to explain **what the cache does, why it exists, and where it deliberately gives up and refetches**.

---

## What is the Longest-Waiting endpoint?

The backend exposes an endpoint that returns **up to 5 games** where the user is currently eligible to play.

The list is ordered by:

1. Games with other human players first
2. Oldest `waiting_since`
3. `game_id` as a tie-break

The frontend uses this endpoint only to power **“next game” navigation**.

---

## Design goals

The LW cache is designed to:

- Ensure the backend remains the single source of truth
- Avoid refetching on every turn in common cases
- Avoid pathological refetch loops when repeatedly playing the same game
- Preserve correctness when eligibility or ordering genuinely changes
- Avoid duplicating backend logic in the frontend

---

## Core philosophy

**The frontend never guesses ordering.**

Ordering is trusted only when:

- It comes directly from the backend via a refetch, or
- It is restored from a previously fetched backend snapshot that is known to still be valid, or
- Ordering is provably irrelevant (≤ 2 games)

If the frontend cannot prove a cached ordering is still correct, it refetches.

---

## High-level approach

The frontend maintains a small LW cache with three layers:

1. A local pool of game IDs used for navigation
2. A completeness flag that indicates whether the server response was truncated
3. An optional snapshot used only to break refetch-per-turn loops

The cache is **event-driven**, not time-based.

---

## Current implementation (code pointers)

This spec is implemented as an explicit LW cache state machine stored in TanStack Query under `queryKeys.games.waitingLongestCache()`.

### Backend (source of truth)

- **Endpoint**: `GET /api/games/waiting-longest` in `[apps/backend/src/routes/games.rs](apps/backend/src/routes/games.rs)` calls `GameService::game_waiting_longest` in `[apps/backend/src/services/games.rs](apps/backend/src/services/games.rs)`.
  - Returns **up to 5** `game_ids` for navigation.
- **Events**:
  - `your_turn` is user-scoped (published when a human becomes the next actor).
  - `long_wait_invalidated` is user-scoped (published for structural transitions: game started/ended/abandoned, player left/rejoined).

### Frontend (LW cache)

- **Cache state + rules**: `[apps/frontend/lib/queries/lw-cache.ts](apps/frontend/lib/queries/lw-cache.ts)`
- **Realtime integration**:
  - `[apps/frontend/lib/providers/web-socket-provider.tsx](apps/frontend/lib/providers/web-socket-provider.tsx)`
  - `[apps/frontend/hooks/useGameSync.ts](apps/frontend/hooks/useGameSync.ts)`
- **UI consumption**: `[apps/frontend/components/Header.tsx](apps/frontend/components/Header.tsx)`
- **No legacy key**: the old `queryKeys.games.waitingLongest()` key is not used.

---

## State

### LW.pool

- Ordered list of up to 5 eligible `gameId`s
- Ordering is authoritative **only immediately after a refetch or snapshot restore**
- When `LW.pool.length <= 2`, ordering is irrelevant and the pool is treated as a navigation set.
Note: the cache will only add games locally if doing so keeps the pool at ≤ 2; otherwise it refetches to restore server ordering.

### LW.isCompleteFromServer

Set **only** on refetch:

- `true` if the server returned fewer than 5 games
- `false` if the server returned exactly 5 games (list may be truncated)

### LW.snapshot (optional)

A server-authoritative snapshot used to avoid refetch loops.

- Shape: `{ gameId, pool, isCompleteFromServer }`
- Always tied to **exactly one gameId**
- Never created from local-only additions

---

## Rendering rule

The UI always renders navigation using:

LW.pool filtered to exclude the current game ID

---

## Definition: Refetch

A refetch:

- Replaces `LW.pool` with the server response
- Sets `LW.isCompleteFromServer`
- Optionally updates `LW.snapshot` (explicitly controlled)

---

## R0 — Refetch overlap and coalescing

Refetches are **coalesced** to prevent storms caused by bursts of events.

### R0.1 — Requesting a refetch

- At most one refetch may be in flight
- Later requests are coalesced
- Snapshot creation is explicit per-refetch

### R0.2 — Applying a refetch result

- Last-response-wins semantics
- Snapshot updated only when explicitly requested

---

## R1 — Structural invalidation

Triggered by `long_wait_invalidated`.

Actions:

- Clear `LW.snapshot`
- Refetch with `createSnapshot = false`

**Reason**:
Structural changes invalidate both ordering and snapshot relevance.

---

## R2 — Taking a turn (optimistic send)

Triggered when the user sends a move in a game.

Actions:

- Remove `currentGameId` from `LW.pool`
- If `LW.snapshot` exists and `snapshot.gameId !== currentGameId`, clear `LW.snapshot`
- If `LW.pool.length < 2` AND `LW.isCompleteFromServer == false`, refetch (`createSnapshot = false`)

**Reason (cache)**:
Removing a game never changes the relative order of remaining entries and is always safe.

---

## R3 — GameState received: not your turn

Triggered by `GameState(myTurn = false)`.

Actions:

- Do nothing to `LW.pool`
- Do nothing to `LW.snapshot`

**Reason**:
The game was already removed when the action was taken; no further changes are required.

---

## R4 — YourTurn message

Triggered by explicit `your_turn(gameId)`.

### Navigation mode (pool size < 2)

If `LW.pool.length < 2` and `gameId` is not already present:

- Append `gameId` to `LW.pool`
- If `LW.snapshot` exists and `snapshot.gameId !== gameId`, clear `LW.snapshot`
- Do not refetch

**Reason**:
When there are ≤ 2 games, ordering is irrelevant for “next game” navigation.

### Snapshot reuse

If:

- `snapshot.gameId === gameId`, and
- Snapshot exists (it is always authoritative)

Then:

- Restore `LW.pool` and `LW.isCompleteFromServer` from snapshot
- Do not refetch

### Otherwise

- Refetch with `createSnapshot = true`
- Snapshot is created from the refetch result

**Reason**:
A `your_turn` message changes eligibility. Snapshot reuse is allowed only when correctness can be proven.

---

## R5 — GameState received: your turn (edge-triggered)

Triggered by `GameState(myTurn = true)`.

The frontend tracks two values for the **currently subscribed game**:

- **`prevIsUsersTurn`**: Initialized from the initial HTTP snapshot (`initialData`) when `useGameSync` mounts; updated after each `game_state` message is applied.
- **`pendingAction`**: Set when the user sends a move (`onOptimisticSend`); cleared when `game_state` is received or on server error.

### Trigger condition

Treat as `your_turn(gameId)` when `myTurn == true` **and** either:

- **Rising edge**: `prevIsUsersTurn == false` (turn just became ours), or
- **Pending action**: `pendingAction == true` (we just sent a move; server confirms we are still the actor, e.g. multi-step phases or round completion).

This ensures only GameState messages that correspond to a previously initiated action drive cache reconstruction. Incidental navigation must not mutate cache state.

### Actions (same as R4)

- If `LW.pool.length < 2`, append `gameId` to `LW.pool` (if missing) and do not refetch
- Else if `snapshot.gameId === gameId`, restore from snapshot (no refetch)
- Otherwise refetch with `createSnapshot = true`

---

## Snapshot rules summary

### Snapshot creation

A snapshot is created only when:

- A refetch completes
- That refetch was explicitly requested with `createSnapshot = true`

### Snapshot reuse

A snapshot may be reused only when:

- The same game becomes eligible again
- No invalidating events occurred

### Snapshot invalidation

A snapshot is cleared when:

- `long_wait_invalidated` is received

---

## Known cache miss (by design)

When the user alternates between multiple human games while others are eligible:

- Refetches are expected
- This is correct and intentional
- Avoiding refetches would require frontend reimplementation of backend ordering logic

---

## Summary

- Backend remains the sole authority on ordering
- Frontend updates locally only when provably safe
- Snapshots exist solely to break same-game refetch loops
- Structural ambiguity always triggers refetch
- High-entropy scenarios refetch by design
