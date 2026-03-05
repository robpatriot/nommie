# Nommie Documentation

## Purpose

This directory contains the **authoritative documentation for the Nommie system**.  
Each document has a clearly defined scope to prevent duplication and drift.

Documents are intentionally **concise and architectural**.  
Detailed implementation logic should live in code unless it defines a **stable
system contract**.

---

# Document Index

## Core System Architecture

**architecture-overview.md**  
High-level system structure: frontend, backend layers, infrastructure,
and core engineering principles.

**architecture-game-context.md**  
Defines the `GameContext` concept, request-scoped caching behavior, and the
trust boundary between HTTP handlers, orchestration, and services.

---

## Gameplay & Game State

**game-rules.md**  
Canonical rule reference for Nomination Whist gameplay.  
All game logic, tests, AI behavior, and UI must conform to this document.

**game-snapshot-contract.md**  
Defines the serialized **Game Snapshot wire format** delivered to clients.
Frontend rendering must treat this as the authoritative representation of game
state.

---

## Backend Infrastructure

**backend-error-handling.md**  
Defines the backend error architecture and mapping to RFC 7807 Problem Details.

**backend-testing-guide.md**  
Explains backend test harness behavior, database safety guards, and test
environment setup.

**database-url-calculation.md**  
Explains how database connection URLs are constructed from environment variables.

---

## Frontend Architecture

**frontend-theme-system.md**  
Defines the semantic theme token system used by the frontend and how Tailwind
utilities map to it.

---

## Cross-System Interaction Models

**event-model.md**  
Defines the **event model used across the system** to describe state changes
originating from HTTP responses, WebSocket messages, and client actions.

This document provides the canonical terminology for events used by the
frontend state machines and backend messaging systems.

---

## AI Systems

**ai-player-implementation-guide.md**  
Defines the contract for implementing AI players, including the decision
interface, determinism requirements, and testing expectations.

---

## Operational Runbooks

These documents describe operational behavior rather than system architecture.

**certificate-locations.md**  
Explains TLS certificate locations in Docker environments and how to verify
correct configuration.

---

# Canonical Ownership Rules

To avoid duplication and drift:

| Topic | Canonical Document |
|------|--------------------|
| Game rules | `game-rules.md` |
| Game snapshot structure | `game-snapshot-contract.md` |
| Backend error semantics | `backend-error-handling.md` |
| Backend test harness | `backend-testing-guide.md` |
| AI interface | `ai-player-implementation-guide.md` |
| Event semantics | `event-model.md` |
| System architecture overview | `architecture-overview.md` |

Other documents **must reference these rather than restating their rules**.

---

# Documentation Principles

All Nommie documentation follows these rules:

1. **Minimal duplication**  
   A topic should have exactly one canonical document.

2. **Architecture over implementation**  
   Documents describe system behavior and contracts, not detailed code listings.

3. **Stable concepts only**  
   Avoid referencing line numbers or transient implementation details.

4. **Clear scope boundaries**  
   Each document should define what it covers and what it intentionally leaves
   to other documents.

5. **Code is the ultimate source of truth**  
   Documentation describes system design; implementation details live in code.


