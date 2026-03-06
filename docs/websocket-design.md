# WebSocket Real-Time Synchronization

## Scope

This document specifies the WebSocket-based real-time synchronization system used by Nommie.

It defines:

- connection lifecycle
- authentication
- broadcast architecture
- message semantics
- error handling semantics
- snapshot delivery model

Game snapshot structure is defined in `docs/game-snapshot-contract.md`.

## Transport and Authentication

Clients establish a WebSocket connection to:

GET /ws

Authentication is performed using a JWT.

Supported authentication mechanisms:

- `Authorization: Bearer <jwt>`
- query parameter `token=<jwt>` (used by browsers)

Browser clients obtain short-lived WebSocket tokens from:

GET /api/ws-token

which proxies to the backend endpoint:

GET | POST /api/ws/token

Tokens are used only to establish the WebSocket connection.

## Connection Lifecycle

After the WebSocket connection is established:

1. Client sends `hello`.
2. Server responds with `hello_ack`.
3. Client sends `subscribe` for a topic.
4. Server acknowledges the subscription and sends the current game snapshot.

The server periodically sends ping frames and closes connections that do not respond.

## Topics

Subscriptions are topic-based.

Current topic types:

- `game`

Each game subscription represents a stream of snapshots for a single game.

Clients may subscribe and unsubscribe during a connection.

## Broadcast Architecture

Realtime delivery uses two layers.

### Instance-local session registry

Each server instance maintains a registry of sessions subscribed to topics.

### Redis fan-out

Redis pub/sub distributes update notifications between instances.

When a game changes:

1. The backend publishes `{ game_id, version }` to Redis.
2. Each instance receiving the message notifies its local sessions.
3. Each session rebuilds the latest snapshot and sends it to the client.

Redis messages contain identifiers and version numbers only.  
Snapshots are rebuilt locally on each instance.

## Message Semantics

Server → client messages include:

- `hello_ack` — confirms connection and negotiated protocol.
- `ack` — confirms subscription or unsubscription requests.
- `game_state` — authoritative snapshot of game state.
- `your_turn` — hint that the current user may need to act.
- `long_wait_invalidated` — invalidates a previously issued wait condition.
- `error` — indicates a request or protocol failure.

Client → server messages include:

- `hello` — initiates the connection handshake.
- `subscribe` — subscribes to a topic.
- `unsubscribe` — removes a subscription.

Exact message schemas are defined in the backend and frontend types.

## Error Handling Semantics

### Dependency outage during WS processing

When a dependency outage occurs during WebSocket processing:

- the backend sends an `error` frame with code `service_unavailable`
- the connection remains open
- the backend records the dependency failure for readiness monitoring

Client behavior on dependency outage:

- clear any pending latches for in-flight WS-driven waits
- enter or remain in frontend Suspect state
- do not immediately refetch state
- mark the current game as needing post-recovery reconciliation (if applicable)

### Protocol errors

Protocol errors include malformed frames or invalid message ordering (for example missing `hello`).

On protocol error:

- the backend closes the WebSocket connection

Client behavior:

- reconnect with backoff unless the UI is in Degraded state
- do not automatically refetch state as part of reconnect logic (state convergence occurs via normal snapshot delivery after subscribe)

### Authorization errors

Authorization errors include forbidden subscriptions.

On authorization error:

- the backend sends an `error` frame
- the connection remains open

Client behavior:

- clear pending latches
- do not refetch state solely due to the authorization error

## Snapshot Delivery Model

WebSocket updates deliver complete snapshots, not diffs.

Properties:

- snapshots are idempotent
- clients replace local state using the highest version received
- ordering guarantees are not required
- reconnecting clients receive the latest snapshot
