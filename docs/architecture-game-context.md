# GameContext Architecture

## Document Scope

This document dives into the `GameContext` concept: how we assemble and cache
it in HTTP handlers, how AI orchestration consumes it, and why services treat it
as read-only input. For a system-wide overview, start with
`architecture-overview.md`. Error propagation patterns are covered in
`backend-error-handling.md`.

## Overview

This document describes the unified `GameContext` architecture that consolidates game-wide and player-specific state into a single cohesive structure used by both HTTP handlers and AI systems.

---

## Core Design Principles

1. **Stateless Server** - No in-memory state stored in AppState
2. **Request-Scoped Caching** - Cache within a single HTTP request, dies when request ends  
3. **No HTTP Coupling in Services** - Services work with plain domain types
4. **Progressive Enhancement** - Context fields are optional to support different game states

---

## The GameContext Structure

`GameContext` combines game identification, historical data, and optional player-specific round information. See `apps/backend/src/domain/game_context.rs` for the complete definition.

### Progressive Enhancement States

| State | game_id | history | round_info | round_memory |
|-------|---------|---------|------------|--------------|
| **Lobby** (pre-game) | ✅ | ❌ | ❌ | ❌ |
| **Game Started** | ✅ | ✅ | ❌ | ❌ |
| **Player Action** (HTTP) | ✅ | ✅ | ✅ | ❌ |
| **AI Decision** | ✅ | ✅ | ✅ | ✅ |

---

## HTTP Layer: CachedGameContext Extractor

### Purpose
Automatically loads and caches GameContext for authenticated users in HTTP requests. The context is cached in request extensions, so multiple consumers within the same request get the same instance without additional database queries.

### Usage in Handlers

```rust
async fn submit_bid(
    context: CachedGameContext,  // ← One parameter with everything
    body: ValidatedJson<BidRequest>,
) -> Result<HttpResponse, AppError> {
    let ctx = context.context();
    
    // Access everything:
    let game_id = ctx.game_id;
    let history = ctx.game_history();      // Option<&GameHistory>
    let round_info = ctx.round_info();     // Option<&CurrentRoundInfo>
    
    Ok(HttpResponse::Ok().finish())
}
```

### Caching Behavior

- **First access**: Loads from DB, caches in `req.extensions()`
- **Subsequent access**: Returns cached instance (no DB queries)
- **Lifetime**: Dies when HTTP request completes (stateless)

---

## Service Layer Integration

### Services Are Trust Boundaries

**Key principle**: Services do NOT accept `GameContext` for validation. They load their own data from the database.

This is a security boundary: if services accepted pre-built context, callers could manipulate validation data. Services must control their own validation data by loading from the authoritative source (database).

### Service Implementation Pattern

Services accept only IDs and load their own data:

```rust
pub async fn submit_bid_internal(
    &self,
    txn: &DatabaseTransaction,
    game_id: i64,           // ← Just the IDs
    player_seat: i16,
    bid_value: u8,
) -> Result<(), AppError>
```

### Performance Note

Yes, this means loading `GameHistory` even when it might be cached elsewhere. This is intentional:
- **Security over micro-optimization**
- Single transaction may cache query results
- Adds ~1 extra query per action
- Clear trust boundaries worth the cost

---

## AI Orchestration Path

AI orchestration maintains its own local cache within the orchestration loop:

- Game history is loaded once and reused across all AI decisions in the loop
- Per-player state (`CurrentRoundInfo`, `RoundMemory`) is loaded fresh per iteration
- Services still load their own validation data (trust boundary)

See `apps/backend/src/services/game_flow/ai_coordinator.rs` for the implementation.

---

## Request Flow Comparison

### Human Player (HTTP Request)

```
HTTP POST /api/games/123/bid
    │
    ├─> CachedGameContext::from_request()
    │   ├─> GameHistory::load()        [DB Query #1]
    │   ├─> CurrentRoundInfo::load()   [DB Query #2]
    │   └─> Cache in req.extensions()
    │
    ├─> Handler: submit_bid(context)
    │   ├─> Use context for UI rendering
    │   └─> service.submit_bid_internal(game_id, ...)
    │       └─> GameHistory::load()     [DB Query #3 - trust boundary]
    │           validate_consecutive_zeros(&history, ...)
    │
    └─> Response (uses cached context)
    
Total DB queries: 3 (service loads its own validation data)
```

### AI Player (No HTTP Request)

```
Orchestration Loop
    │
    ├─> GameHistory::load()            [DB Query #1]
    │   (cached in local variable for entire loop)
    │
    ├─> AI Decision (uses cached history for strategy)
    │   └─> ai.choose_bid(&round_info, &game_context)
    │
    ├─> service.submit_bid_internal(game_id, ...)
    │   └─> GameHistory::load()        [DB Query #2 - trust boundary]
    │       validate_consecutive_zeros(&history, ...)
    │
    └─> Loop continues
    
Total DB queries per action: 2 (orchestration cache + service validation)
Note: Within same transaction, DB may cache query results
```

---

## Benefits

### 1. Single Cohesive Concept
Instead of passing 3-4 separate parameters, everything is unified in one `GameContext` structure.

### 2. Request-Scoped Caching
- HTTP requests: Load once, use many times within request
- Cache dies with request (stateless server)
- No memory leaks, no cache invalidation complexity

### 3. Services Are Trust Boundaries
- Services don't accept `GameContext` for validation
- They load their own data from authoritative source (DB)
- No risk of caller manipulation
- Clear security model: services control validation data

### 4. Separation of Concerns
- **GameContext**: Read-only view for UI and AI strategy
- **Services**: Load their own data for validation (security)
- **HTTP layer**: Uses cached context for response building
- Each layer has clear responsibilities

### 5. Clean Service Signatures
Services have simple, focused signatures that accept only what they need (IDs, not full context).

---

## Summary

The `GameContext` architecture provides:

✅ **Unified State** - One concept for game data (ID, history, round info)  
✅ **Request-Scoped Caching** - Efficient HTTP without server-side state  
✅ **Trust Boundaries** - Services load their own validation data (secure)  
✅ **Separation of Concerns** - Context for UI/AI, services for validation  
✅ **No HTTP Coupling** - Services work with plain types  
✅ **Tested** - Comprehensive validation tests  
✅ **Scalable** - Stateless server design  

This design provides efficient caching for UI and AI strategy while maintaining proper security boundaries: services are authoritative about validation data and never trust caller-provided context.
