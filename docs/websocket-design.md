# Nommie Real-Time Sync (WebSockets)

## Scope
This document describes the **current** realtime sync architecture for Nommie (websockets + Redis pub/sub fan-out), including the on-the-wire protocol, authentication, and the intended testing strategy.

If you are looking for game snapshot shape: see `docs/game-snapshot-contract.md`.

## Current State (Reality)
- The web client uses **WebSockets as the primary sync mechanism** for active game sessions.
- Backend provides:
  - `GET /ws` websocket upgrade endpoint (game subscription happens via subscribe messages after connect).
  - `GET|POST /api/ws/token` short-lived websocket token issuance for browser clients.
  - Redis pub/sub fan-out to support multi-instance realtime delivery.
- HTTP `GET /api/games/{game_id}/snapshot` remains available and is used for:
  - initial page load (server-rendered snapshot)
  - manual refresh fallback in the UI

## Goals
- Provide low-latency delivery of game updates to connected clients.
- Reduce redundant traffic compared to periodic polling.
- Keep the client rendering model snapshot-based (idempotent “latest snapshot wins”).

## Non-Goals
- Exactly-once delivery.
- Replacing HTTP mutation endpoints (bids/plays/etc.) with websocket RPC.

## Architecture

### Transport & Auth
Clients connect to:
- `GET /ws?token=<jwt>`

Authentication is performed by `JwtExtract` middleware:
- Primary: `Authorization: Bearer <jwt>`
- Fallback (for browsers): query string `token=<jwt>`

Browser clients obtain a short-lived websocket token via:
- Frontend route: `GET /api/ws-token`
- Backend route: `GET|POST /api/ws/token`

Tokens are minted with a short TTL (currently ~90 seconds) and are intended only for establishing websocket connections.

### Connection Lifecycle
- Each websocket connection is handled by an Actix actor (`WsSession`).
- The client must send `hello` first; the server responds with `hello_ack`. The client then sends `subscribe` with a topic; the server sends `ack` followed by `game_state`.
  - sends ping periodically
  - disconnects clients that fail to respond within the timeout window

### Fan-out & Broadcast
Realtime fan-out is implemented with two layers:
- **In-process registry**: a map of `game_id -> sessions` held by `WsRegistry`.
- **Redis pub/sub**: a cross-process signal bus so any instance can notify all other instances of updates.

The broadcast contract is intentionally minimal:
- When a game changes, the backend publishes `{ game_id, version }` to Redis.
- Each instance that receives that signal forwards a `SnapshotBroadcast { version }` to local sessions.
- Each session rebuilds the latest snapshot on-demand and sends it to the client.

This keeps Redis messages small and avoids having to serialize/deserialize the full snapshot into Redis.

## Wire Protocol (What We Actually Send)
Messages are JSON with a `type` discriminator.

### Server → Client
- `hello_ack`: `{ "type": "hello_ack", "protocol": 1, "user_id": <id> }` — response to client `hello`
- `ack`: Machine-correlatable acknowledgement of a client command. Identifies the command and topic:
  - Subscribe: `{ "type": "ack", "command": "subscribe", "topic": { "kind": "game", "id": <game_id> } }`
  - Unsubscribe: `{ "type": "ack", "command": "unsubscribe", "topic": { "kind": "game", "id": <game_id> } }`
- `game_state`: `{ "type": "game_state", "topic": { "kind": "game", "id": <game_id> }, "version": <n>, "game": <GameSnapshot>, "viewer": <ViewerState> }`
- `your_turn`: `{ "type": "your_turn", "game_id": <id>, "version": <n> }` — user-scoped hint
- `long_wait_invalidated`: `{ "type": "long_wait_invalidated", "game_id": <id> }`
- `error`: `{ "type": "error", "code": "<code>", "message": "<string>" }`

### Client → Server
- `hello`: `{ "type": "hello", "protocol": 1 }` — must be sent first
- `subscribe`: `{ "type": "subscribe", "topic": { "kind": "game", "id": <game_id> } }`
- `unsubscribe`: `{ "type": "unsubscribe", "topic": { "kind": "game", "id": <game_id> } }`

## Frontend Integration
- `WebSocketProvider` manages the low-level connection (token fetch, connect, handshake, reconnection).
- `useGameSync` consumes `useWebSocket` and handles per-game subscribe/unsubscribe and applies snapshots to the React Query cache.
- `GameRoomClient` consumes `useGameSync` and treats websocket updates as the source of truth.
- Manual refresh remains available (HTTP snapshot fetch using ETags) for resilience/debugging.

## Configuration
- Backend:
  - `REDIS_URL` enables realtime fan-out (when present, realtime is enabled in `AppState`).
- Frontend:
  - `NEXT_PUBLIC_BACKEND_WS_URL` (optional explicit WS base)
  - `NEXT_PUBLIC_BACKEND_BASE_URL` (fallback; converted `http(s) -> ws(s)` by the client)

## Testing Strategy

### Frontend
- Unit/component tests mock the browser `WebSocket` API and validate UI updates and error handling.

### Backend (✅ Complete)
Backend integration tests cover:
- **Connection tests**: WebSocket connect with JWT authentication, hello/hello_ack handshake, subscribe → ack + game_state delivery
- **Broadcast tests**: Multi-client broadcast fan-out (all connected clients receive updates, game isolation ensures broadcasts only reach same-game clients)
- **Reconnect tests**: Client reconnection receives latest snapshot, multiple disconnect/reconnect cycles
- **Shutdown tests**: Registry cleanup and connection count management
- **Error handling tests**: Invalid JSON, missing hello, forbidden subscription
- **YourTurn tests**: User-scoped your_turn hints and broadcast behaviour

**Test Implementation Details:**
- Tests use **in-memory `WsRegistry`** (not Redis) for concurrency safety and deterministic test execution
- Each test uses **transaction-per-test isolation** via `SharedTxn` injection into request extensions
- Tests run against a **real HTTP server** (not `test::init_service()`) to support WebSocket upgrade
- Test infrastructure includes `TestTxnInjector` middleware, `WebSocketClient` wrapper, and connection count polling helpers
- Tests are located in `apps/backend/tests/suites/websocket/`

**Note:** Redis pub/sub is not tested directly (we assume Redis and the library work correctly). Tests focus on **our use** of WebSockets: connection lifecycle, broadcast delivery, and session management.

## Operational Notes
- Websocket payloads use full snapshots (not diffs). This is intentional: snapshots are idempotent and simplify correctness.
- Ordering: clients should treat snapshots as “latest version wins”.
