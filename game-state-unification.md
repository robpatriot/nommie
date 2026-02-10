# GameState Model Unification

This document defines the single, cohesive task of unifying backend and frontend around one authoritative game state model. It reflects the current, observed state of the codebase and incorporates explicit decisions made during investigation.

---

## Objective

Unify backend and frontend around a single authoritative game state model derived from `GameStateMsg`, eliminating legacy snapshot payloads while preserving correctness, optimistic UI behavior, and enabling safe, incremental migration.

---

## Core Principles

- **Backend is authoritative**
  - `GameStateMsg` is the canonical representation of game + viewer state.
  - All frontend state must ultimately derive from it.

- **Frontend state is WS-driven, but locally writable**
  - WebSocket messages are the sole inbound source of authoritative updates after initial load.
  - The frontend may apply **temporary optimistic updates** for responsiveness.
  - WebSocket updates always reconcile and win.

- **One model, one cache contract**
  - Exactly one frontend cache model represents game state.
  - Transitional dual-write mechanisms are acceptable; steady-state duplication is not.

- **Correctness before cleanup**
  - Migration safety and clarity take priority over immediate deletion of legacy code.
  - Architectural decisions are permanent; migration mechanisms are temporary.

---

## What Is Already True (Confirmed)

- The WebSocket protocol is unified on `GameStateMsg`.
- Incoming WS messages are validated and version-gated as `game_state`.
- `GameStateMsg` is the wire-level source of truth.
- A stable adapter exists converting `GameStateMsg → GameRoomSnapshotPayload`.

Protocol unification is complete.  
Application-state unification is not.

---

## Current State (Observed)

### Frontend State Reality

- The frontend cache currently treats `GameRoomSnapshotPayload` as the authoritative application state.
- `useGameSync`:
  - receives `GameStateMsg`
  - converts it to `GameRoomSnapshotPayload`
  - writes only to `queryKeys.games.snapshot(gameId)`
- All consumers (hooks, components, tests) read snapshot payloads.
- There is no `GameRoomState` type or state-native cache key yet.

### Optimistic Updates

- Mutations perform optimistic updates by:
  - reading `GameRoomSnapshotPayload` from cache
  - constructing updated snapshot payloads
  - writing them back immediately
- WebSocket updates later reconcile and overwrite as needed.

The UI is responsive because the frontend temporarily authors state, but the state model itself is snapshot-based.

---

## Target State (Decided)

### Frontend State Model

The frontend will introduce a **`GameRoomState`** as its single cached state model.

This is a **thin wrapper**, not a second domain model.

Initial shape (illustrative):

- `topic`
- `version`
- `game` (from `GameStateMsg`)
- `viewer` (from `GameStateMsg`)
- optional FE-only metadata (e.g. `receivedAt`, `source`)

Characteristics:

- Derived directly from `GameStateMsg`
- No deep normalization initially
- No protocol-only fields leaked into consumers
- Accessed via selectors, not raw property reads

### Optimistic Updates (Explicitly Kept)

- Optimistic updates remain a required feature for UI responsiveness.
- Optimistic logic must migrate from snapshot payloads to `GameRoomState`.
- WebSocket updates always reconcile and remain authoritative.

The goal is **not** to remove optimism, but to make it state-native.

---

## Migration Strategy (Aligned to Decisions)

### Phase 0 — Foundations (Not Started)

- Define `GameRoomState` (thin wrapper).
- Introduce new query key: `queryKeys.games.state(gameId)`.
- Add selectors to shield consumers from raw state shape.

_No behavior change. No consumer migration._

---

### Phase 1 — Write Path (Low Risk)

- Update `useGameSync` to write `GameRoomState` into the new cache key.
- Continue writing snapshot payloads in parallel (dual-write).
- Preserve all version gating and correctness checks.

This establishes a concrete unification starting point with minimal blast radius.

---

### Phase 2 — Read Path Migration (Primary Work)

- Introduce `useGameRoomState` (or equivalent).
- Migrate:
  - page bootstrap
  - client root
  - local hooks
  - tests
- Snapshot payloads may be derived temporarily for unmigrated consumers.

This phase is wide but largely mechanical.

---

### Phase 3 — Optimistic Mutation Migration (Key Design Phase)

This phase migrates existing optimistic update behavior from snapshot payloads to the new `GameRoomState` cache model.

- Mutations will:
  - read the current `GameRoomState` from cache
  - apply **optimistic rewrites** directly to `GameRoomState` for immediate UI feedback
  - stop constructing or writing `GameRoomSnapshotPayload` objects

- Optimistic updates are **temporary and local**:
  - they exist only to keep the UI responsive
  - they do not change the authoritative source of truth

- WebSocket reconciliation rules:
  - incoming `GameStateMsg` updates always remain authoritative
  - monotonic version checks are preserved
  - authoritative WS state replaces or corrects optimistic state as required

This phase preserves existing UI responsiveness while completing the migration to a single, state-native frontend model.

---

### Phase 4 — HTTP Alignment

- HTTP bootstrap returns data convertible directly into `GameRoomState`.
- Any remaining snapshot adapters must be removable.

---

### Phase 5 — Cleanup

- Remove snapshot cache keys.
- Delete snapshot-era types, helpers, adapters, and tests.
- Rename files to reflect state-native terminology.

---

## Completion Criteria

- `GameRoomSnapshotPayload` is no longer used as application state.
- Frontend cache stores only `GameRoomState`.
- Optimistic updates operate on `GameRoomState`.
- WebSocket updates are the sole authoritative reconciliation mechanism.
- Exactly one backend state builder and one frontend ingestion path exist.
- Tests use state-native fixtures.

---

## Previous notes that may be relevant

- **Remove legacy snapshot artefacts**
  - Identify and delete unused snapshot response types / helpers
  - GameSnapshotResponse and legacy snapshot envelope parsing types
  - Confirm exactly one backend snapshot builder
  - Confirm exactly one FE adapter

It referred to this internal extraction step:

Build the existing GameSnapshotResponse
then extract { version, snapshot, viewer } from it
instead of serializing it directly.

Example conceptually (not code you keep forever):

existing builder ──▶ GameSnapshotResponse
                        ├─ snapshot
                        ├─ viewer fields
                        └─ version
                  ──▶ destructure
                        ├─ version
                        ├─ GameSnapshot
                        └─ ViewerState
                  ──▶ WS GameState

That lets you:
Fix the protocol immediately
Remove duplication immediately
Keep correctness while refactoring safely
Later (once things are stable), you might:
Rename the builder
Return (GameSnapshot, ViewerState, version) directly
Or split “shared” vs “viewer” builders more cleanly
That later cleanup is what was “temporary” — not the architectural decision.
- Then we had this list of potential improvements to useGameState.ts
Improvements We Should Not Forget (Recommended, Still In-Scope)
These are not required to make tests pass, but are correctness/robustness wins worth doing now or immediately after.

---

Convert GameRoomSnapshotPayload in consumers to new GameStateMsg
Where you are now
Transport protocol (WS): GameStateMsg is authoritative.
Frontend cache contract: React Query stores GameRoomSnapshotPayload under queryKeys.games.snapshot(gameId).
Adapter: useGameSync.transformGameStateMsg converts GameStateMsg → GameRoomSnapshotPayload.
To “convert consumers”, you’re changing the cache contract and the types flowing through hooks/components/mutations.
Goal state
Pick one of these “end states” (both are valid):
End state A: Cache stores GameStateMsg directly
Query cache holds { type:'game_state', topic, version, game, viewer } (or a small wrapper)
Components/hooks read from that and derive UI props via selectors
Mutations update cache in terms of GameStateMsg (harder, but clean)
End state B: Cache stores a frontend “RoomState” derived from GameStateMsg
RoomState is not GameRoomSnapshotPayload and not the raw GameStateMsg, but a normalized internal model
Still driven by WS protocol, but with FE-friendly shape
This usually wins if you want derived fields like playerNames, timestamps, etc.
If your stated goal is “every consumer expects the new format”, that’s basically End state A (or B if you want a normalized type).
Migration strategy (safe, incremental)
Phase 0: Introduce the new type alongside the old one
Define a new cache type
Example:
type GameRoomState = GameStateMsg
or type GameRoomState = { version; game; viewer; topic } (no type field)
Put it somewhere stable, e.g. lib/game-room/state/types.ts.
Introduce a new query key
Keep existing: queryKeys.games.snapshot(gameId)
Add new: queryKeys.games.state(gameId)
This allows migration without breaking everything at once.
Create selectors
Functions that replace today’s direct property access to payload:
selectSnapshot(state): GameSnapshot
selectViewerSeat(state): Seat | null
selectViewerHand(state): string[]
selectBidConstraints(state): BidConstraints | null
selectPlayerNames(state): string[] (derived from state.game.game.seating)
This prevents “raw state shape” from leaking everywhere.
This phase should not change behavior; it just sets up rails.
Phase 1: Make useGameSync write the new cache (in addition to old)
Right now useGameSync calls applySnapshot(payload) which writes GameRoomSnapshotPayload into the snapshot cache.
Change it to:
always store raw/normalized GameStateMsg (or GameRoomState) into queryKeys.games.state(gameId)
optionally keep writing the legacy payload during transition (or derive it from state for old consumers)
Concretely:
handleGameStateMessage still does topic filtering + monotonic version checks
then:
queryClient.setQueryData(queryKeys.games.state(gameId), message) (or normalized)
invalidate waitingLongest as you do now
During the transition you have two options:
Option 1 (cleaner): derive payload from state where needed
Stop writing payload in the sync path, and migrate consumers rapidly.
Option 2 (safer short-term): dual-write
Write both caches for a while:
state cache gets the authoritative message
snapshot cache gets transformGameStateMsg(message) as compatibility
Given your desire to avoid drift, I’d do dual-write briefly, then remove once consumers are migrated.
Phase 2: Move the read-path (consumers) to the new cache
From your rg output, these are the key consumer areas:
A) Query hook: hooks/queries/useGameSnapshot.ts
Today: returns GameRoomSnapshotPayload and reads queryKeys.games.snapshot(gameId).
Change to:
useGameRoomState(gameId) that reads queryKeys.games.state(gameId)
initialData should be state-shaped too (see Phase 3)
It should return the new state type.
This is the center of the migration.
B) Page bootstrap: app/game/[gameId]/page.tsx
Today: constructs initialPayload: GameRoomSnapshotPayload.
Change to:
initialState: GameRoomState
If the HTTP endpoint still returns payload, you can transform once here:
payload -> state (temporary adapter)
Long-term: change HTTP endpoint to return GameStateMsg-shaped data.
C) Client root: app/game/[gameId]/_components/game-room-client.tsx
Prop initialData: GameRoomSnapshotPayload becomes initialState: GameRoomState.
Downstream, you’ll update:
selectors used inside
props passed to child components/hooks
D) Local hooks consuming payload
useAiSeatManagement.ts takes snapshot: GameRoomSnapshotPayload
useGameRoomReadyState.ts takes snapshot: GameRoomSnapshotPayload
Change these to take GameRoomState (or GameSnapshot + ViewerState separately).
Often best is:
useGameRoomReadyState({ state })
and use selectors internally.
E) Helper: lib/queries/game-snapshot-helpers.ts
Likely reads cached payload to do “should refresh” / “etag” / etc.
This will need refactoring because etag may disappear in a pure-state world.
Short-term:
keep etag on the side (HTTP concern)
or keep a small “http meta” cache separate.
F) Mutations: hooks/mutations/useGameRoomMutations.ts
This is the biggest chunk.
Today it:
reads GameRoomSnapshotPayload from cache
constructs updatedSnapshot: GameRoomSnapshotPayload = { ... }
writes back to queryKeys.games.snapshot(gameId)
In the new world, mutations should:
read GameRoomState
update it (optimistic update) in a state-native way
But there’s an important question:
Do you still do optimistic updates at all, or do you rely on WS echo?
You can go either way:
Keep optimistic updates: update state cache immediately, then let WS reconcile.
Prefer WS-driven state: mutations only invalidate/await server, and UI updates when WS sends authoritative game_state.
Given you’re emphasizing “backend as source of truth”, #2 is attractive, but it can feel laggy unless WS turnaround is fast. A hybrid is common:
optimistic only for immediate UI feel
but always accept WS as authoritative and replace state.
Either way, useGameRoomMutations.ts will be a meaningful refactor: all its local construction of GameRoomSnapshotPayload must be replaced.
Phase 3: Fix the HTTP bootstrap contract
Right now the HTTP endpoint gives you GameRoomSnapshotPayload. If you want “all consumers expect the new format”, you eventually want HTTP to return the same shape you store in cache:
either GameStateMsg (or GameRoomState) directly
or { game, viewer, version, topic } without the type field
So you’ll need:
backend endpoint response change
or a frontend adapter while backend remains unchanged
Best is backend alignment: HTTP initial load returns the same structure as WS snapshots, so FE can treat them uniformly.
Phase 4: Delete compatibility layers
Once all reads/writes are state-native:
remove queryKeys.games.snapshot(gameId) usage
remove GameRoomSnapshotPayload from most of the FE (it can stay as a server DTO type if HTTP still uses it)
remove transformGameStateMsg
delete snapshot-era helpers/tests
rename files like game-snapshot-helpers.ts to game-room-state-helpers.ts
What you’ll actually touch, file-by-file
Based on your grep list, here’s the “worklist” grouped by kind:
Core types / protocol
apps/frontend/lib/game-room/protocol/types.ts (stop depending on payload types long-term)
new: apps/frontend/lib/game-room/state/* (types + selectors)
Read path (most important first)
hooks/useGameSync.ts (write state cache, dual-write during transition)
hooks/queries/useGameSnapshot.ts → new useGameRoomState.ts
app/game/[gameId]/page.tsx
app/game/[gameId]/_components/game-room-client.tsx
app/game/[gameId]/_components/hooks/useGameRoomReadyState.ts
app/game/[gameId]/_components/hooks/useAiSeatManagement.ts
lib/queries/game-snapshot-helpers.ts (rethink: etag / refresh logic)
Write path (mutations)
hooks/mutations/useGameRoomMutations.ts (largest refactor)
Tests
All tests importing GameRoomSnapshotPayload fixtures will move to state fixtures:
test/setup/game-room-client-helpers.ts
test/hooks/useGameRoomReadyState.test.tsx
test/components/game-room-client.*.test.tsx
test/hooks/useGameRoomActions.test.tsx
etc.
Key design decisions you should make up front
What is the new cached type?
raw GameStateMsg vs normalized GameRoomState
Do we keep etag/HTTP refresh?
If yes, store HTTP meta separately (don’t pollute state)
Optimistic updates policy
keep, reduce, or remove
Dual-write duration
short window (recommended) vs long compatibility
Suggested migration order (lowest risk)
Introduce GameRoomState + selectors + new query key
useGameSync writes state cache (dual-write to keep app working)
Convert read-path: useGameSnapshot → useGameRoomState, page/client root, local hooks
Convert tests for read-path
Convert mutations
Remove payload cache usage + delete compatibility
