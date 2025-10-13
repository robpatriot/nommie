# In-Memory Game Engine - Implementation Guide

## Purpose

Fast in-memory game simulation for AI training, bypassing database persistence entirely.

**Target Performance:** 10-30ms per game (vs 9.15s with DB = **300-900x speedup**)

**Use Case:** AI training requiring 10k-100k game simulations
- Current: 9.15s per game = 25 hours for 10,000 games
- Target: 10-30ms per game = 3-5 minutes for 10,000 games

---

## Architecture Overview

### Two Separate "In-Memory" Systems

This doc covers **AI Training** only. Integration testing is separate future work.

#### 1. In-Memory Game Engine (This Document)
**Purpose:** AI training at maximum speed

```
SimAI → InMemoryGame → GameState (domain)
```

- **No repos, no services** - bypasses persistence layer entirely
- Synchronous, panics on invalid moves
- 10-30ms per game
- **Location:** `apps/backend/src/simulation/`

#### 2. In-Memory Repos (Future Work - Not This Document)
**Purpose:** Fast integration tests of full production code path

```
Test → Handlers → Services → InMemoryRepos → HashMap
```

- Tests full service layer logic
- Async, Result-based, validates normally
- 100-500ms per game
- **Location:** `apps/backend/src/repos/memory/` (not implemented yet)

**Important:** These are fundamentally different systems solving different problems. They share the domain layer but have separate implementations.

---

## Design Decisions

### 1. Dual-System Architecture
- **Database version:** Production web game (persistence, async, resumable)
- **In-memory version:** AI training (speed, ephemeral, batch)
- **Shared:** Domain layer (`domain/state.rs`, game logic) - already database-free

### 2. Code Sharing Strategy
**Share:**
- ✅ Domain layer (state, tricks, scoring, bidding)
- ✅ Card dealing logic
- ✅ Core game rules

**Duplicate:**
- ❌ Game loop orchestration (sync vs async, different error handling)
- ❌ AI interfaces (different traits for different use cases)
- ❌ Validation strategies (simulation: panic, DB: Result)

**Mitigation:** Comprehensive cross-validation tests (same seed → same outcome)

### 3. AI Trait Design
Two separate traits optimized for different use cases:

```rust
// Simulation: Fast, synchronous, training-focused
pub trait SimAI {
    fn choose_bid(&mut self, state: &GameState, seat: PlayerId) -> u8;
    fn choose_trump(&mut self, state: &GameState, seat: PlayerId) -> Trump;
    fn choose_card(&mut self, state: &GameState, seat: PlayerId) -> Card;
}
```

**Key differences from database `AiPlayer` trait:**
- `&mut self` - AI maintains internal state without `Arc<Mutex<>>`
- `&GameState` - Full state access (no information hiding needed)
- No `Result` - Invalid moves panic (fail-fast for training)
- `seat: PlayerId` - Explicit perspective parameter

### 4. Validation & Error Handling
**Validate with panics/debug assertions**

```rust
impl InMemoryGame {
    fn play_card(&mut self, card: Card) {
        debug_assert!(self.is_legal_play(card), "Illegal card: {card:?}");
        // ... proceed
    }
}
```

**Rationale:**
- Fast (debug assertions compile out in release builds)
- Catches AI bugs during development
- Training crashes loudly if AI is broken (good - don't train on broken AI)
- Achieves 10-30ms per game target

### 5. Event History
**Optional and configurable (default: off)**

```rust
pub struct SimulationConfig {
    pub seed: u64,
    pub track_history: bool,  // Default: false for speed
    pub on_game_complete: Option<Box<dyn Fn(&GameResult)>>,
}
```

**Benefits:**
- Fast training runs with history disabled
- Detailed debugging when enabled for specific games
- Can investigate anomalous results

### 6. GameResult Structure
**Rich but focused metrics for training evaluation**

```rust
pub struct GameResult {
    // Core identification & outcome
    pub seed: u64,
    pub final_scores: [i32; 4],
    pub winner: PlayerId,
    
    // Training metrics
    pub round_scores: Vec<[i32; 4]>,  // 26 rounds × 4 players
    pub tricks_won: [u8; 4],          // Total tricks per player
    
    // Performance tracking
    pub duration_ms: u64,
    
    // Debugging (only when history enabled)
    pub history: Option<Vec<GameEvent>>,
}
```

**Rationale:**
- `round_scores`: Analyze AI performance trajectory across rounds
- `tricks_won`: Additional performance signal beyond scores
- `duration_ms`: Track performance, identify slow AIs
- `history`: Debug weird outcomes when needed

### 7. Parallel Execution
**Include in Phase 1 using Rayon**

```rust
use rayon::prelude::*;

pub fn run_parallel_games(
    seeds: Vec<u64>,
    ai_factory: impl Fn() -> [Box<dyn SimAI>; 4] + Sync
) -> Vec<GameResult> {
    seeds.par_iter()
        .map(|&seed| {
            let mut game = InMemoryGame::new(seed);
            let mut ais = ai_factory();
            game.play_full_game(&mut ais)
        })
        .collect()
}
```

**Benefits:**
- 8-core parallelism: ~8x speedup on top of 400x base speedup
- 10,000 games in ~25 seconds instead of ~3 minutes
- Essential for large-scale training

### 8. Testing Strategy
**Strict seed-based cross-validation**

```rust
#[test]
fn identical_outcomes_memory_vs_db() {
    let seed = 42;
    let mem_result = run_memory_game(seed);
    let db_result = run_db_game(seed);
    
    // Must match exactly
    assert_eq!(mem_result.final_scores, db_result.final_scores);
    assert_eq!(mem_result.round_scores, db_result.round_scores);
}
```

**Match criteria:**
- ✅ Final scores (critical)
- ✅ Round-by-round scores (important)
- ✅ Trick-by-trick outcomes (if feasible)

---

## Module Structure

```rust
// apps/backend/src/lib.rs
pub mod simulation;  // NEW MODULE

// apps/backend/src/simulation/mod.rs
mod engine;
mod ai_trait;
mod results;
mod batch;

pub use engine::InMemoryGame;
pub use ai_trait::{SimAI, RandomSimAI};
pub use results::{GameResult, SimulationConfig};
pub use batch::run_parallel_games;
```

---

## Implementation Files

### 1. `apps/backend/src/simulation/mod.rs` (~10 lines)
Module exports only.

### 2. `apps/backend/src/simulation/engine.rs` (~250 lines)
Core simulation engine.

```rust
use crate::domain::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

pub struct InMemoryGame {
    state: GameState,  // From domain/state.rs
    rng: StdRng,
    config: SimulationConfig,
}

impl InMemoryGame {
    pub fn new(seed: u64) -> Self {
        Self {
            state: GameState::new(),
            rng: StdRng::seed_from_u64(seed),
            config: SimulationConfig::default(),
        }
    }
    
    /// Play complete 26-round game
    pub fn play_full_game(
        &mut self, 
        ais: &mut [&mut dyn SimAI; 4]
    ) -> GameResult {
        let start = std::time::Instant::now();
        
        for round_no in 1..=26 {
            self.play_round(round_no, ais);
        }
        
        self.build_result(start.elapsed())
    }
    
    fn play_round(&mut self, round_no: u8, ais: &mut [&mut dyn SimAI; 4]) {
        // 1. Deal cards using domain::deal_hands()
        let hands = deal_hands(&mut self.rng, round_no);
        self.state.hands = hands;
        
        // 2. Bidding phase
        for seat in PlayerId::all() {
            let bid = ais[seat as usize].choose_bid(&self.state, seat);
            debug_assert!(is_valid_bid(bid, round_no), "Invalid bid");
            self.state.bids[seat] = bid;
        }
        
        // 3. Trump selection
        let winning_bidder = determine_winning_bidder(&self.state.bids);
        let trump = ais[winning_bidder as usize].choose_trump(&self.state, winning_bidder);
        self.state.trump = Some(trump);
        
        // 4. Play tricks
        for _trick_no in 0..round_no {
            self.play_trick(ais);
        }
        
        // 5. Score using domain::score_round()
        let scores = score_round(&self.state);
        self.state.apply_scores(scores);
    }
    
    fn play_trick(&mut self, ais: &mut [&mut dyn SimAI; 4]) {
        for _ in 0..4 {
            let seat = self.state.current_player();
            let card = ais[seat as usize].choose_card(&self.state, seat);
            
            debug_assert!(
                self.state.is_legal_play(card, seat),
                "Illegal play: {card:?} by {seat:?}"
            );
            
            // Use domain layer's play_card
            play_card(&mut self.state, card, seat);
        }
        
        // Resolve trick using domain layer
        resolve_trick(&mut self.state);
    }
}
```

### 3. `apps/backend/src/simulation/ai_trait.rs` (~100 lines)
Trait definition and RandomSimAI implementation.

```rust
use crate::domain::{Card, GameState, PlayerId, Trump};

/// AI trait for fast synchronous simulation
pub trait SimAI {
    fn choose_bid(&mut self, state: &GameState, seat: PlayerId) -> u8;
    fn choose_trump(&mut self, state: &GameState, seat: PlayerId) -> Trump;
    fn choose_card(&mut self, state: &GameState, seat: PlayerId) -> Card;
}

/// Random AI that makes random valid moves
pub struct RandomSimAI {
    rng: StdRng,
}

impl RandomSimAI {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }
}

impl SimAI for RandomSimAI {
    fn choose_bid(&mut self, state: &GameState, seat: PlayerId) -> u8 {
        let valid_range = valid_bid_range(state.current_round_hand_size());
        valid_range.choose(&mut self.rng).copied().unwrap()
    }
    
    fn choose_trump(&mut self, state: &GameState, _seat: PlayerId) -> Trump {
        // Random trump or based on hand composition
        [Trump::Clubs, Trump::Diamonds, Trump::Hearts, Trump::Spades]
            .choose(&mut self.rng)
            .copied()
            .unwrap()
    }
    
    fn choose_card(&mut self, state: &GameState, seat: PlayerId) -> Card {
        let legal_plays = state.legal_plays(seat);
        legal_plays.choose(&mut self.rng).copied().unwrap()
    }
}
```

### 4. `apps/backend/src/simulation/results.rs` (~50 lines)
Result structures and configuration.

```rust
pub struct SimulationConfig {
    pub seed: u64,
    pub track_history: bool,
    pub on_game_complete: Option<Box<dyn Fn(&GameResult)>>,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            seed: 0,
            track_history: false,
            on_game_complete: None,
        }
    }
}

pub struct GameResult {
    pub seed: u64,
    pub final_scores: [i32; 4],
    pub winner: PlayerId,
    pub round_scores: Vec<[i32; 4]>,
    pub tricks_won: [u8; 4],
    pub duration_ms: u64,
    pub history: Option<Vec<GameEvent>>,
}

impl GameResult {
    pub fn winner(&self) -> PlayerId {
        let (winner_idx, _) = self.final_scores
            .iter()
            .enumerate()
            .max_by_key(|(_, score)| *score)
            .unwrap();
        PlayerId::from_index(winner_idx)
    }
}
```

### 5. `apps/backend/src/simulation/batch.rs` (~50 lines)
Parallel execution with Rayon.

```rust
use rayon::prelude::*;

/// Run multiple games in parallel across CPU cores
pub fn run_parallel_games(
    seeds: Vec<u64>,
    ai_factory: impl Fn() -> [Box<dyn SimAI>; 4] + Sync
) -> Vec<GameResult> {
    seeds.par_iter()
        .map(|&seed| {
            let mut game = InMemoryGame::new(seed);
            let mut ais = ai_factory();
            game.play_full_game(&mut ais)
        })
        .collect()
}

/// Aggregate statistics across multiple games
pub struct BatchStatistics {
    pub total_games: usize,
    pub avg_duration_ms: f64,
    pub win_distribution: [usize; 4],
    pub avg_final_scores: [f64; 4],
}

pub fn compute_batch_stats(results: &[GameResult]) -> BatchStatistics {
    // Compute aggregates for training analysis
    // ...
}
```

### 6. `apps/backend/tests/simulation_test.rs` (~150 lines)
Unit tests and cross-validation.

```rust
#[test]
fn test_deterministic_single_game() {
    let seed = 42;
    
    let result1 = run_game_with_seed(seed);
    let result2 = run_game_with_seed(seed);
    
    assert_eq!(result1.final_scores, result2.final_scores);
    assert_eq!(result1.round_scores, result2.round_scores);
}

#[test]
fn test_cross_validation_memory_vs_db() {
    let seed = 12345;
    
    // Run in-memory game
    let mem_result = run_memory_game(seed);
    
    // Run database game (same seed, same AIs)
    let db_result = run_db_game(seed);
    
    // Must produce identical results
    assert_eq!(mem_result.final_scores, db_result.final_scores);
    assert_eq!(mem_result.round_scores, db_result.round_scores);
}

#[test]
fn test_performance_benchmark() {
    let start = Instant::now();
    let _result = run_game_with_seed(999);
    let duration = start.elapsed();
    
    assert!(duration.as_millis() < 100, 
        "Game took {duration:?}, expected < 100ms");
}

#[test]
fn test_parallel_batch() {
    let seeds: Vec<u64> = (0..100).collect();
    let results = run_parallel_games(seeds, || {
        [
            Box::new(RandomSimAI::new(1)),
            Box::new(RandomSimAI::new(2)),
            Box::new(RandomSimAI::new(3)),
            Box::new(RandomSimAI::new(4)),
        ]
    });
    
    assert_eq!(results.len(), 100);
}
```

---

## Files to Modify

### 1. `apps/backend/src/lib.rs`
Add:
```rust
pub mod simulation;
```

### 2. `Cargo.toml`
Add dependencies:
```toml
[dependencies]
rayon = "1.10"
rand = "0.8"  # Already present, ensure version
```

---

## Domain Layer Reference

**These already exist - reuse as-is:**

- `apps/backend/src/domain/state.rs` - `GameState` struct
- `apps/backend/src/domain/dealing.rs` - `deal_hands()` function
- `apps/backend/src/domain/scoring.rs` - `score_round()` function
- `apps/backend/src/domain/tricks.rs` - `play_card()`, trick resolution
- `apps/backend/src/domain/bidding.rs` - Bid validation

**Reference (don't modify):**
- `apps/backend/src/services/game_flow.rs` - Database version orchestration
- `apps/backend/tests/game_flow_ai_test.rs` - Full game test pattern

---

## Critical Bug Awareness

**Recent fix:** Cards could be played multiple times in DB version.

**In-memory engine must:**
- ✅ Use domain layer's `play_card()` (already handles removal correctly)
- ✅ Validate hand state in debug builds
- ✅ Not introduce same bug

**Good news:** Domain layer already correct - just use it properly!

---

## Expected Outcomes

### Deliverables
- ✅ `InMemoryGame` struct with full 26-round game loop
- ✅ `SimAI` trait definition
- ✅ `RandomSimAI` implementation
- ✅ `GameResult` with rich metrics
- ✅ Parallel batch execution
- ✅ Cross-validation test proving correctness
- ✅ Performance benchmark showing 300-500x speedup
- ✅ Deterministic (same seed → same result)

### Performance Targets
- **Single game:** 10-30ms (vs 9.15s = **300-900x faster**)
- **10,000 games (sequential):** 3-5 minutes (vs 25 hours)
- **10,000 games (parallel 8-core):** 30-60 seconds

### Estimated Effort
- **Implementation:** 8-10 hours
- **Lines of code:** ~600-700 new lines
- **Testing:** Included in estimate

---

## Follow Cursor Rules

- ✅ No DB dependencies in domain layer (already satisfied)
- ✅ Services are stateless (InMemoryGame is stateless struct)
- ✅ Use enums for states/phases (reusing domain enums)
- ✅ No `unwrap()` in production code (use `debug_assert!` for validation)
- ✅ Inline format strings: `format!("value: {variable}")`
- ✅ Never run git commands (except diff)

---

## Future Work (Out of Scope)

### In-Memory Repos for Integration Testing
**Separate effort, not covered in this doc:**

```rust
// apps/backend/src/repos/memory/
mod game_repo.rs    // impl GameRepository using HashMap
mod round_repo.rs   // impl RoundRepository
// etc.
```

**Purpose:** Fast integration tests using production services

**Performance:** 100-500ms per game (vs 10s DB)

**When:** After optimizing DB tests, if still too slow

---

## Ready to Implement

This document contains everything needed to implement the in-memory game engine when ready. All decisions have been made, architecture is defined, and implementation approach is clear.

**Next steps when implementing:**
1. Create `apps/backend/src/simulation/` directory
2. Implement files in order: results.rs → ai_trait.rs → engine.rs → batch.rs
3. Add tests in `apps/backend/tests/simulation_test.rs`
4. Run cross-validation tests
5. Benchmark performance
6. Iterate based on results

