# Longest-Waiting Cache

## Scope

This document specifies the frontend cache used for the **Longest-Waiting (LW)** endpoint.

The cache supports “next game” navigation while preserving the backend as the
authoritative source of ordering.

## Longest-Waiting Endpoint

The backend exposes an endpoint returning **up to 5 games** where the user is
eligible to act.

Ordering is defined by the backend as:

1. Games with other human players first
2. Oldest `waiting_since`
3. `game_id` as a tie-break

The frontend uses this list only for navigation.

## Design Constraints

The LW cache must satisfy:

- Backend remains the sole authority for ordering.
- The frontend does not reproduce backend ordering logic.
- Refetch loops are avoided during repeated play in the same game.
- Eligibility or ordering changes must converge to backend truth.

## Cache Model

The LW cache maintains three pieces of state.

### LW.pool

Ordered list of up to five eligible `gameId`s.

Ordering is authoritative only immediately after:

- a refetch, or
- restoration from a previously fetched snapshot.

When `LW.pool.length ≤ 2`, ordering is irrelevant and the pool is treated as a
navigation set.

Local additions are permitted only if the pool remains ≤ 2 entries.

### LW.isCompleteFromServer

Set only during refetch.

- `true` when the server returned fewer than 5 games.
- `false` when the server returned exactly 5 games (list may be truncated).

### LW.snapshot (optional)

Server-authoritative snapshot used to prevent refetch loops.

Structure:

`{ gameId, pool, isCompleteFromServer }`

Properties:

- Tied to exactly one `gameId`
- Created only from refetch results
- Never derived from locally constructed state

## Rendering Rule

Navigation uses:

`LW.pool` filtered to exclude the current game ID.

## Refetch Definition

A refetch:

- replaces `LW.pool` with the server response
- updates `LW.isCompleteFromServer`
- may optionally update `LW.snapshot`

Refetch requests are coalesced so that only one refetch is active at a time.

The most recent response wins.

## State Transitions

### R1 — Structural invalidation

Triggered by `long_wait_invalidated`.

Actions:

- clear `LW.snapshot`
- refetch without snapshot creation

Structural changes invalidate both ordering and snapshot relevance.

### R2 — Taking a turn

Triggered when the user sends a move.

Actions:

- remove `currentGameId` from `LW.pool`
- clear snapshot if it refers to a different game
- if `LW.pool.length < 2` and `LW.isCompleteFromServer == false`, refetch

Removing a game never changes the relative order of remaining entries.

### R3 — GameState: not your turn

Triggered by `GameState(myTurn = false)`.

Actions:

- no change to `LW.pool`
- no change to `LW.snapshot`

### R4 — `your_turn` event

Triggered by `your_turn(gameId)`.

#### Navigation mode (pool < 2)

If the pool has fewer than two entries and the game is not already present:

- append the game
- clear snapshot if it refers to another game
- do not refetch

#### Snapshot reuse

If `snapshot.gameId == gameId`:

- restore `LW.pool` and `LW.isCompleteFromServer`
- do not refetch

#### Otherwise

- refetch and create snapshot

### R5 — GameState: your turn

Triggered by `GameState(myTurn = true)`.

The frontend tracks:

- `prevIsUsersTurn`
- `pendingAction`

Treat as `your_turn(gameId)` when:

- the turn transitions from false to true, or
- the user previously sent an action and the server confirms they remain the actor.

Actions are identical to rule R4.

## Snapshot Rules

### Snapshot creation

Snapshots are created only when:

- a refetch completes, and
- snapshot creation was explicitly requested.

### Snapshot reuse

A snapshot may be reused only when:

- the same game becomes eligible again, and
- no invalidating events occurred.

### Snapshot invalidation

Snapshots are cleared when:

- `long_wait_invalidated` is received.

## Expected Refetch Scenario

If the user alternates between multiple human games while other eligible games
exist, refetches will occur.

This behavior is intentional. Avoiding it would require the frontend to
replicate backend ordering logic.

## Invariants

- Backend ordering is authoritative.
- Frontend updates ordering only when correctness can be proven.
- Snapshots exist only to prevent repeated refetches in the same game.
- Ambiguous situations always trigger refetch.
