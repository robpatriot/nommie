# Nommie Real-Time Sync Design

## Overview
This document defines the plan for evolving Nommie's game synchronization from HTTP polling to a push-based architecture (websockets first, with room for other transports later). It captures shared understanding across frontend, backend, and infrastructure so we can estimate, implement, and review the work with minimal ambiguity.

## Background & Current State
- Game clients poll `GET /api/games/:id/snapshot` every 3 seconds via `GameRoomClient` in the Next.js frontend.
- Polling logic is tightly coupled to UI state management (pending actions, toasts, concurrency guards) but all server communication funnels through `getGameRoomSnapshotAction` and `fetchWithAuth`.
- Backend exposes read-only snapshots with ETags and optimistic locking; it has no long-lived connection support or event broadcasting.
- Latency and bandwidth costs grow with the number of concurrent players because every client maintains its own polling cadence.

## Goals
1. Provide near-real-time delivery of snapshot updates to active game clients (<250 ms target after state commit).
2. Reduce redundant HTTP traffic by replacing periodic polling with server push wherever possible.
3. Maintain the existing snapshot shape (or a documented evolution) so UI rendering code requires minimal change.
4. Preserve manual refresh and/or polling fallback for resilience.

## Non-Goals
- Replacing existing REST endpoints for game actions (bid, play, etc.) in this phase.
- Implementing mobile push notifications or background sync.
- Guaranteeing exactly-once delivery; at-least-once with idempotent snapshots is acceptable.

## Proposed Architecture (High Level)
1. **Transport Layer**
   - Add a websocket endpoint (e.g., `GET /ws/games/:id`) that authenticates via the same backend JWT used for REST calls.
   - Use an Actix websocket actor per connection; manage connections in a registry keyed by game ID.
   - Implement ping/pong heartbeats and server-triggered disconnects on idle or auth failure.
2. **Update Broadcast Path**
   - When game state mutates (existing services already know when lock_version changes), enqueue a snapshot broadcast event containing the latest `GameSnapshotResponse`.
   - Use Redis pub/sub: every backend instance publishes to `game:<id>` channels; instances with local websocket subscribers receive the payload and forward it.
   - Keep a minimal in-process registry for dev/single-node setups, but treat Redis as mandatory for multi-node fan-out.
3. **Frontend Sync Service**
   - Extract polling logic from `GameRoomClient` into a dedicated `useGameSync` hook or service that owns transport state.
   - Hook accepts a `transport` implementation (polling vs websocket) so we can roll out behind a feature flag and fall back when sockets fail.
   - Websocket client handles reconnect with exponential backoff, connection status, and surfaces incoming snapshots through the same interface the UI already consumes.
4. **Protocol**
   - JSON messages with `{ "type": "snapshot", "data": GameSnapshotPayload, "version": 1 }`.
   - Optional `{ "type": "error", "code": ..., "message": ... }` for server-side issues.
   - Client sends `{ "type": "subscribe", "gameId": number }` immediately after connect (if URL doesnt already imply game ID) and `{ "type": "ping" }` for custom heartbeats as needed.

## Detailed Backend Design

### Websocket Endpoint & Auth
- New route: `GET /ws/games/{game_id}` handled by an Actix websocket actor.
- Handshake verifies the backend JWT exactly like REST endpoints (`require_backend_jwt`); rejects if token missing/invalid.
- Actor stores `game_id`, `user_id`, and connection metadata (connected_at, last_ping).

### Connection Registry & Redis Integration
- Each process keeps an in-memory map `game_id -> HashSet<Addr<WebSocketActor>>`.
- At startup, the process also subscribes to Redis channel pattern `game:*`.
- Publishing flow: whenever a game snapshot changes, code running inside the request/command publishes `{"gameId":123,"payload":...}` to `game:123`.
- Receiving flow: Redis subscription callback looks up local connections for that game and forwards the serialized payload to each actor; actors drop clients that fail to accept data (backpressure).

### Broadcast Trigger Points
- Wrap the existing state mutation paths (bids, plays, ready, AI management) so that after DB commit succeeds we:
  1. Load or reuse the latest `GameSnapshotResponse`.
  2. Publish it to Redis.
- For now we can synchronously recompute the snapshot using the existing domain call; if that proves costly we can cache snapshots per lock_version.

### Lifecycle & Resilience
- Ping/pong: server sends ping every 20 seconds; if no pong in 10 seconds, close the socket and remove from registry.
- Reconnection: clients are expected to reconnect; on connect we immediately send the latest snapshot for that game so they catch up.
- Structured logging on connect/disconnect/error/broadcast with `{game_id, user_id, reason}`.

## Frontend Design

### Sync Abstraction
- Introduce `useGameSync({ gameId, initialSnapshot })` hook responsible for:
  - Establishing the websocket.
  - Maintaining connection status (`connected | reconnecting | closed`).
  - Surfacing the latest snapshot + lockVersion + metadata.
  - Providing imperative methods `refresh()` (manual fetch fallback) and `sendAction()` hooks if needed later.
- Hook exposes same data shape currently managed inside `GameRoomClient`, so UI changes are limited to wiring.

### Websocket Client Details
- Browser connects to `${NEXT_PUBLIC_BACKEND_WS_URL}/ws/games/${gameId}` with `Authorization: Bearer <jwt>` header (Next proxy or custom fetcher).
- Implements exponential backoff reconnect (e.g., 1s, 2s, 4s up to 30s).
- Displays toast/banner when disconnected to prompt manual refresh if necessary.
- Manual refresh button still calls the REST snapshot action to cover the rare case of persistent socket failure.

### Existing Actions
- Bids/plays/AI actions remain HTTP POSTs for now; after each successful action we rely on push updates rather than forcing an immediate refetch (manual refresh remains as fallback).

## Infrastructure & Configuration
- Add Redis to dev docker-compose; define `REDIS_URL` consumed by backend.
- Backend config: enable/disable websocket feature (default on), Redis connection pool size, ping intervals.
- Frontend config: `NEXT_PUBLIC_BACKEND_WS_URL` (can reuse HTTP base with `wss://` prefix).
- Update dev docs so contributors know how to run Redis locally.

## Testing & Validation
- **Unit tests:** websocket actor handles subscribe/unsubscribe, heartbeat timeouts, JSON encoding/decoding.
- **Integration tests:** spin up redis + backend, simulate multiple connections, verify broadcast fan-out and reconnect behavior.
- **Frontend tests:** hook-level tests using mocked websocket to ensure reconnection logic and state transitions.
- **Load tests:** simulate high-frequency state updates to ensure broadcast latency stays <250 ms and Redis throughput is adequate.
- **Failure drills:** kill Redis or backend instance to confirm clients reconnect and fall back gracefully.

## Rollout Notes
- No production users yet, so once the websocket feature is functional we can remove polling entirely and update the frontend to require the new hook.
- Keep the manual refresh action exposed in UI for support/debugging even after sockets become primary.

## Decisions
- Websocket payloads will use full snapshots (same shape as REST) for v1; message versioning allows future diff-based optimizations if monitoring shows bandwidth pressure.
- Because the product is still pre-production, we will ship a direct cutover: websocket sync replaces polling entirely with no dual-support period or backwards compatibility for old clients.
- Instrument both structured logs (connect, disconnect, broadcast, error events tagged with game/user context) and basic counters/gauges (`ws_connections_active`, `ws_connect_total`, `ws_disconnect_total` with reason, `ws_broadcast_total`, `ws_broadcast_fail_total`, heartbeat failures). Even without dashboards today, emitting these metrics positions us for future telemetry aggregation.
