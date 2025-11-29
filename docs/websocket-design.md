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

## Open Questions / To Refine Next
- What level of metrics/logging do we need for socket lifecycle (connect, disconnect, errors)?

## Decisions
- Websocket payloads will use full snapshots (same shape as REST) for v1; message versioning allows future diff-based optimizations if monitoring shows bandwidth pressure.
- Because the product is still pre-production, we will ship a direct cutover: websocket sync replaces polling entirely with no dual-support period or backwards compatibility for old clients.
