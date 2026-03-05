# Event Model

## Purpose

Defines the system-wide **event model** used to describe state changes across the
application.

Events originate from multiple sources and are consumed by the frontend and
backend to update UI state, trigger workflows, and synchronize game state.

This document provides a **shared conceptual model** used throughout the system.

---

# Event Sources

Events can originate from three categories of sources.

## HTTP Responses

Synchronous events produced by client requests.

Examples:

- submitting a bid
- selecting trump
- playing a card
- joining a game

HTTP responses may contain:

- an updated game snapshot
- a structured error response
- acknowledgement of a successful action

HTTP responses are treated as **authoritative state updates**.

---

## WebSocket Messages

Asynchronous events pushed by the backend.

Typical uses:

- notifying other players of actions
- broadcasting game state updates
- signaling round or phase transitions

WebSocket messages generally carry:

- event notifications
- updated snapshots

WebSocket messages are treated as **state synchronization events**.

---

## Client-Local Events

Events produced locally by the frontend without an immediate server response.

Examples:

- optimistic UI updates
- navigation state changes
- local interaction events

Local events must always reconcile with authoritative server state when it
arrives.

---

# Event Properties

## Events Are Immutable

An event represents a **fact that occurred**.

Once emitted, an event must not be modified.

Consumers derive new state from events but do not mutate them.

---

## Events Represent State Transitions

Events signal that a transition has occurred.

Examples:

Bidding → TrumpSelect  
TrickPlay → TrickResolved  
RoundComplete → NextRound

Events describe the transition but do not contain business logic.

---

## Events May Arrive Out of Order

Network conditions may cause:

- delayed WebSocket messages
- duplicated messages
- messages arriving after HTTP responses

Consumers must tolerate these conditions.

Snapshots provide the mechanism for resolving ordering differences.

---

# Snapshot Synchronization

The system uses **snapshots** to synchronize authoritative state.

Rules:

- HTTP responses may contain a snapshot.
- WebSocket events may contain a snapshot.
- the frontend must treat the most recent snapshot as authoritative.

Snapshots replace client state rather than mutating it incrementally.

This ensures client state can recover from delayed or reordered events.

---

# Event Consumers

## Frontend State Machines

Frontend components consume events to update:

- UI phase transitions
- rendered game state
- interaction availability

UI state must always reconcile with snapshots rather than assuming local state
is correct.

---

## Backend Orchestration

Backend workflows produce events after domain transitions occur.

Examples:

- after a bid is accepted
- after a trick resolves
- after a round completes

These events are propagated to clients through WebSockets.

---

# Error Events

Errors also function as events.

Examples include:

- invalid player actions
- authorization failures
- dependency failures

HTTP endpoints return structured error responses defined in
`backend-error-handling.md`.

---

# Design Principles

The event model follows these rules:

1. Snapshots are authoritative.  
2. Events signal transitions, not business logic.  
3. Clients remain reconcilable with server state.  
4. Multiple event sources are expected.  
5. Event handling must be idempotent.

---

# Related Documents

Game snapshot structure  
`game-snapshot-contract.md`

WebSocket architecture  
`websocket-design.md`

Backend error responses  
`backend-error-handling.md`

System architecture overview  
`architecture-overview.md`
