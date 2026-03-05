# GameContext

## Purpose

Defines the `GameContext` concept and how it is used by HTTP handlers and AI orchestration.

`GameContext` is a read-only view that bundles game-wide and player-specific state.

## Design Principles

- Stateless server: no cross-request in-memory cache in AppState.
- Request-scoped caching: cache may exist only within a single request boundary.
- Services are trust boundaries: services load their own authoritative validation data.
- Progressive enhancement: context fields may be absent depending on game state.

## Context Contents

`GameContext` combines:

- game identifier
- optional game history
- optional current-round player view
- optional round memory

Each of these fields may be unavailable depending on when the context is assembled.

## Progressive Enhancement States

- Lobby: game id only
- Game started: adds history
- Player action: adds current-round info
- AI decision: adds round memory

## HTTP Boundary

HTTP handlers may use an extractor that:

- builds a `GameContext` for the authenticated user
- caches it in request extensions for the duration of the request

Within a request, repeated access must not introduce additional database queries.

## Service Boundary

Services must not trust caller-provided `GameContext` for validation.

Rules:

- service APIs accept identifiers and request parameters
- services load validation data from the database within their transaction
- cached context is permitted for response shaping, not for authorization or validation

This preserves a security boundary: validation data must be loaded from the authoritative store.

## AI Orchestration

AI orchestration may cache game history locally within its own loop for performance.

Rules:

- orchestration may reuse history for AI strategy
- services still load their own validation data
- orchestration must treat `GameContext` as read-only input only
