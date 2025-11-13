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

```rust
pub struct GameContext {
    /// Game ID
    pub game_id: i64,

    /// Complete game history (all rounds, bids, scores)
    /// Available once game has started (round 1+)
    history: Option<GameHistory>,

    /// Current round state from a specific player's perspective
    /// Only available when context is loaded for a specific player
    round_info: Option<CurrentRoundInfo>,

    /// AI's memory of completed tricks in the current round
    /// Only present for AI players
    round_memory: Option<RoundMemory>,
}
```

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
Automatically loads and caches GameContext for authenticated users in HTTP requests.

### Implementation

```rust
#[derive(Clone)]
pub struct CachedGameContext(pub Arc<GameContext>);

impl FromRequest for CachedGameContext {
    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        // 1. Check request extensions cache
        // 2. Extract GameId and GameMembership
        // 3. Load game from DB
        // 4. If game started: load GameHistory + CurrentRoundInfo
        // 5. Cache in request extensions
        // 6. Return
    }
}
```

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

```rust
// services/game_flow/player_actions.rs

pub async fn submit_bid_internal(
    &self,
    txn: &DatabaseTransaction,
    game_id: i64,           // ← Just the IDs
    player_seat: i16,
    bid_value: u8,
) -> Result<(), AppError>
```

### Why Not Accept Context?

**Security**: If services accepted pre-built context, callers could manipulate validation data:

```rust
// BAD: Service accepts context (trust issue)
pub async fn submit_bid(context: &GameContext, ...) {
    validate_consecutive_zero_bids(context.game_history(), ...)?;
    // ← What if context.game_history() was manipulated by caller?
}

// GOOD: Service loads its own data (secure)
pub async fn submit_bid(game_id: i64, ...) {
    let history = GameHistory::load(txn, game_id).await?;
    validate_consecutive_zero_bids(&history, ...)?;
    // ← Service controls validation data
}
```

### Service Implementation

```rust
// Service loads and validates using its own data
if bid_value == 0 {
    // Load fresh from DB (service owns validation data)
    let history = GameHistory::load(txn, game_id).await?;
    validate_consecutive_zero_bids(&history, player_seat, current_round)?;
}
```

### Performance Note

Yes, this means loading `GameHistory` even when it might be cached elsewhere. This is intentional:
- **Security over micro-optimization**
- Single transaction may cache query results
- Adds ~1 extra query per action
- Clear trust boundaries worth the cost

---

## AI Orchestration Path

### Local Cache Strategy

AI orchestration maintains its own local cache:

```rust
// orchestration.rs

pub async fn process_game_state(
    txn: &DatabaseTransaction,
    game_id: i64,
) -> Result<(), AppError> {
    // Local cache for orchestration loop
    let game_history = GameHistory::load(txn, game_id).await?;
    
    for _iteration in 0..MAX_ITERATIONS {
        // Load per-player state
        let round_info = CurrentRoundInfo::load(txn, game_id, player_seat).await?;
        
        // Load AI memory (filtered by memory_level)
        let memory = RoundMemory::load(txn, game_id, player_seat, memory_level).await?;
        
        // Build complete context for AI
        let game_context = GameContext::new(game_id)
            .with_history(game_history.clone())    // ← Reused from cache
            .with_round_info(round_info)           // ← Fresh per iteration
            .with_round_memory(Some(memory));      // ← Fresh per iteration
            
        // AI methods accept GameContext for strategy
        let bid = ai.choose_bid(&round_info, &game_context)?;
        
        // Service loads its own validation data (trust boundary)
        self.submit_bid_internal(txn, game_id, player_seat, bid).await?;
    }
}
```

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

## Consecutive Zero Bids Rule

### Implementation

```rust
// domain/bidding.rs

pub fn validate_consecutive_zero_bids(
    history: &GameHistory,
    player_seat: i16,
    current_round: i16,
) -> Result<(), DomainError> {
    // Need at least 3 previous rounds
    if current_round < 4 {
        return Ok(());
    }

    // Get last 3 completed rounds
    let recent_rounds: Vec<_> = history.rounds
        .iter()
        .filter(|r| r.round_no < current_round)
        .rev()
        .take(3)
        .collect();

    // Check if all 3 have zero bids
    let all_zeros = recent_rounds.iter()
        .all(|round| round.bids[player_seat as usize] == Some(0));

    if all_zeros {
        Err(DomainError::validation(
            ValidationKind::InvalidBid,
            "Cannot bid 0 four times in a row"
        ))
    } else {
        Ok(())
    }
}
```

### Where It's Called

```rust
// services/game_flow/player_actions.rs

pub async fn submit_bid_internal(..., context: Option<&GameContext>, ...) {
    if bid_value == 0 {
        // Get history from context or load
        let history = /* get from context or load fresh */;
        
        // Validate
        validate_consecutive_zero_bids(history, player_seat, current_round)?;
    }
}
```

### Test Coverage

Comprehensive tests in `domain/tests_consecutive_zeros.rs`:
- ✅ Allow 0 bid in first 3 rounds
- ✅ Allow third consecutive 0 bid
- ✅ Reject fourth consecutive 0 bid
- ✅ Allow 0 after non-zero breaks streak
- ✅ Track players independently
- ✅ Reset after non-zero bid
- ✅ Late game enforcement
- ✅ Only look at last 3 rounds
- ✅ Handle incomplete history

---

## Benefits

### 1. Single Cohesive Concept
Instead of passing 3-4 separate parameters:
```rust
// Before
fn submit_bid(
    game_id: i64,
    player_seat: i16,
    bid_value: u8,
    game_history: Option<&GameHistory>,
)

// After
fn submit_bid(
    context: &GameContext,  // ← Everything in one place
    bid_value: u8,
)
```

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
```rust
// Clean and focused
pub async fn submit_bid(
    txn: &DatabaseTransaction,
    context: &GameContext,
    bid_value: u8,
) -> Result<(), AppError>
```

---

## Migration Path

Services have simple, clean signatures - no context parameter needed:

### AI Orchestration
```rust
// AI coordinator
let game_context = build_context_for_ai(...);
let bid = ai.choose_bid(&state, &game_context)?;

// Service loads its own validation data
service.submit_bid_internal(txn, game_id, player_seat, bid).await?;
```

### HTTP Handlers (future)
```rust
async fn submit_bid(
    context: CachedGameContext,  // ← For UI rendering
    body: ValidatedJson<BidRequest>,
) -> Result<HttpResponse, AppError> {
    // Use context for response building
    let game_id = context.game_id();
    
    // Service loads its own validation data (trust boundary)
    service.submit_bid_internal(txn, game_id, player_seat, body.bid_value).await?;
    
    // Use context for response
    Ok(HttpResponse::Ok().json(context.game_history()))
}
```

---

## Future Extensions

The `GameContext` structure can naturally grow to support:

1. **Opponent Models** (AI)
   ```rust
   context.opponent_models: Option<HashMap<PlayerId, OpponentModel>>
   ```

2. **Game Statistics**
   ```rust
   context.stats: Option<GameStatistics>
   ```

3. **Undo/Replay State**
   ```rust
   context.replay_state: Option<ReplayState>
   ```

4. **Spectator View**
   ```rust
   context.spectator_view: Option<SpectatorInfo>
   ```

All additions are backward compatible (Option fields).

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

