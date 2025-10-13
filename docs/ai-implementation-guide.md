# Nommie AI Player Implementation Guide

This guide provides everything you need to implement a custom AI player for Nommie (Nomination Whist). Submit your implementation as a single Rust file for code review and integration.

---

## Table of Contents

1. [Game Rules](#game-rules)
2. [Quick Start](#quick-start)
3. [The AiPlayer Trait](#the-aiplayer-trait)
4. [Available Game State](#available-game-state)
5. [Core Data Types](#core-data-types)
6. [Reference Implementation: RandomPlayer](#reference-implementation-randomplayer)
7. [Advanced: GameHistory API](#advanced-gamehistory-api)
8. [Error Handling](#error-handling)
9. [Testing Your AI](#testing-your-ai)
10. [Submission Requirements](#submission-requirements)

---

## Game Rules

Nommie is a 4-player trick-taking card game with bidding. Here are the complete rules:

### Players & Setup
- **4 players** in fixed clockwise turn order (seats 0-3)
- **26 rounds** total with varying hand sizes
- **Hand sizes** per round: `13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 2, 2, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13`
- Each round uses a freshly shuffled standard 52-card deck
- Dealer rotates clockwise each round

### Bidding Phase
- Each player bids how many tricks they expect to win (0 to hand_size)
- Dealer bids last
- **Dealer restriction**: Cannot bid a value that makes the sum of all 4 bids equal to hand_size
- **Zero bid limit**: A player cannot bid 0 more than 3 rounds in a row
- **Trump selection**: Highest bidder chooses trump suit (ties broken by earliest bidder in turn order)

### Trump Selection
- Winner of bidding chooses from: Clubs, Diamonds, Hearts, Spades, or NoTrump
- Affects trick resolution (see below)

### Trick Play
- **Leading**: Player to left of dealer leads first trick; thereafter, trick winner leads next trick
- **Following suit**: Must play a card of the lead suit if you have one
- **Trick winner** is determined by:
  - Highest trump card played, OR
  - If no trumps, highest card of the lead suit
  - Card ranks: 2 < 3 < 4 < 5 < 6 < 7 < 8 < 9 < 10 < J < Q < K < A
- Each round has exactly `hand_size` tricks

### Scoring
- **+1 point** per trick won
- **+10 bonus** if tricks won exactly equals your bid (including 0)
- Scores are cumulative across all 26 rounds

### Game End
- After round 26, highest total score wins
- Multiple players can tie for the win

---

## Quick Start

Here's a minimal template you can copy and customize:

```rust
use crate::ai::{AiError, AiPlayer};
use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::{Card, GameContext, Trump};

/// MyAI - A simple AI that [describe your strategy]
pub struct MyAI {
    // Your AI's state here
}

impl MyAI {
    pub fn new() -> Self {
        Self {
            // Initialize your state
        }
    }
}

impl AiPlayer for MyAI {
    fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError> {
        // Get legal bids (handles dealer restriction automatically)
        let legal_bids = state
            .legal_bids()
            .map_err(|e| AiError::Internal(format!("Failed to get legal bids: {e}")))?;
        
        if legal_bids.is_empty() {
            return Err(AiError::InvalidMove("No legal bids available".into()));
        }
        
        // Your bidding logic here
        // Example: just bid 0
        Ok(0)
    }

    fn choose_play(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Card, AiError> {
        // Get legal cards to play (handles follow-suit rule automatically)
        let legal_plays = state
            .legal_plays()
            .map_err(|e| AiError::Internal(format!("Failed to get legal plays: {e}")))?;
        
        if legal_plays.is_empty() {
            return Err(AiError::InvalidMove("No legal plays available".into()));
        }
        
        // Your card selection logic here
        // Example: play first legal card
        Ok(legal_plays[0])
    }

    fn choose_trump(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Trump, AiError> {
        // Choose trump after winning the bid
        // Can choose from: Clubs, Diamonds, Hearts, Spades, or NoTrump
        
        // Your trump selection logic here
        // Example: choose Spades
        Ok(Trump::Spades)
    }
}
```

---

## The AiPlayer Trait

Your AI must implement three decision methods. Each receives complete visible game state.

### Common Parameters

All three methods receive the same two parameters:

**`state: &CurrentRoundInfo`** - Current round state including:
- Your hand, position, and seat
- All bids (who has bid what, or None if not yet)
- Current scores for all players
- Current trick state and plays
- Helper methods for legal moves: `legal_bids()`, `legal_plays()`, `legal_trumps()`

**`context: &GameContext`** - Game-wide context including:
- Complete game history via `context.game_history()` for strategic analysis
- Historical data persisting across all rounds (bids, trumps, scores from past rounds)

### Required Methods

#### `fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError>`

Called during the bidding phase when it's your turn.

**When called**: Once per round, in turn order (dealer bids last)

**What to return**: A legal bid value (0 to hand_size)
- Query `state.legal_bids()` for valid options
- The dealer restriction is handled automatically

**Example**:
```rust
fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError> {
    let legal_bids = state.legal_bids()
        .map_err(|e| AiError::Internal(format!("{e}")))?;
    
    // Count high cards in hand
    let high_cards = state.hand.iter()
        .filter(|c| matches!(c.rank, Rank::Jack | Rank::Queen | Rank::King | Rank::Ace))
        .count();
    
    // Bid conservatively based on high cards
    let target_bid = (high_cards / 2) as u8;
    
    // Choose closest legal bid
    legal_bids.iter()
        .min_by_key(|&&b| (b as i16 - target_bid as i16).abs())
        .copied()
        .ok_or_else(|| AiError::InvalidMove("No legal bids".into()))
}
```

#### `fn choose_play(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Card, AiError>`

Called during trick play when it's your turn.

**When called**: Once per trick in turn order

**What to return**: A legal card from your hand
- Query `state.legal_plays()` for valid cards
- Follow-suit rule is enforced automatically

**Example**:
```rust
fn choose_play(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Card, AiError> {
    let legal_plays = state.legal_plays()
        .map_err(|e| AiError::Internal(format!("{e}")))?;
    
    if legal_plays.is_empty() {
        return Err(AiError::InvalidMove("No legal plays".into()));
    }
    
    // Play highest card if leading, lowest if following
    if state.current_trick_plays.is_empty() {
        // Leading - play highest card
        Ok(*legal_plays.iter().max().unwrap())
    } else {
        // Following - play lowest card
        Ok(*legal_plays.iter().min().unwrap())
    }
}
```

#### `fn choose_trump(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Trump, AiError>`

Called when you win the bid and must select trump.

**When called**: After all bids are in, if you had the highest bid

**What to return**: Trump choice
- Can choose: `Trump::Clubs`, `Trump::Diamonds`, `Trump::Hearts`, `Trump::Spades`, or `Trump::NoTrump`
- Query `state.legal_trumps()` for all options (returns all 5 options)

**Example**:
```rust
fn choose_trump(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Trump, AiError> {
    // Get all legal trump options (5 choices)
    let legal_trumps = state.legal_trumps();
    
    // Count cards per suit in hand
    let mut suit_counts = [(Trump::Clubs, 0), (Trump::Diamonds, 0), 
                           (Trump::Hearts, 0), (Trump::Spades, 0)];
    
    for card in &state.hand {
        let idx = match card.suit {
            Suit::Clubs => 0,
            Suit::Diamonds => 1,
            Suit::Hearts => 2,
            Suit::Spades => 3,
        };
        suit_counts[idx].1 += 1;
    }
    
    // Choose suit with most cards, or NoTrump if weak hand
    let (best_trump, best_count) = suit_counts.iter()
        .max_by_key(|(_, count)| count)
        .copied()
        .unwrap_or((Trump::NoTrump, 0));
    
    // If weak in all suits (less than 3 cards), choose NoTrump
    if best_count < 3 {
        Ok(Trump::NoTrump)
    } else {
        Ok(best_trump)
    }
}
```

### Thread Safety

Your struct must be `Send + Sync` (thread-safe). This is required because the game engine may call your AI from different threads.

**If your AI is stateless**: Automatically satisfied

**If your AI needs mutable state** (e.g., RNG): Use interior mutability with `Mutex`

```rust
use std::sync::Mutex;
use rand::prelude::*;

pub struct MyAI {
    rng: Mutex<StdRng>,  // Mutex provides thread-safe interior mutability
}
```

---

## Available Game State

Every decision method receives `CurrentRoundInfo` with complete visible game state.

### CurrentRoundInfo Fields

#### Your Position & Context
```rust
player_seat: i16        // Your position (0-3)
dealer_pos: i16         // Who is dealing this round (0-3)
current_round: i16      // Round number (0-25)
hand_size: u8           // Cards per player this round
game_state: GameState   // Current phase: Bidding, TrumpSelection, or TrickPlay
```

#### Your Hand
```rust
hand: Vec<Card>         // Cards you currently hold (updated as you play)
```

#### Bidding Information
```rust
bids: Vec<Option<u8>>   // Bids by seat position
                        // Example: [Some(3), Some(2), None, None]
                        // means seats 0 and 1 have bid, 2 and 3 haven't yet
```

#### Trump Information
```rust
trump: Option<Trump>    // Trump suit if selected (None during bidding)
```

#### Current Trick
```rust
trick_no: i16                      // Which trick (0 to hand_size-1)
current_trick_plays: Vec<(i16, Card)>  // Cards played in this trick so far
                                       // Format: (seat_position, card)
                                       // Empty at start, up to 4 entries
trick_leader: Option<i16>          // Who should lead this trick
                                   // (highest bidder for trick 0,
                                   //  previous winner otherwise)
```

#### Scores
```rust
scores: [i16; 4]        // Cumulative scores for all players (by seat 0-3)
```

### Helper Methods

#### `legal_bids() -> Result<Vec<u8>, AppError>`

Returns valid bids you can make right now.

**Returns**:
- Empty vec if not your turn or not in bidding phase
- Vec of legal bid values (0 to hand_size)
- Automatically handles dealer restriction

**Always use this method** instead of implementing bid validation yourself.

```rust
let legal_bids = state.legal_bids()
    .map_err(|e| AiError::Internal(format!("{e}")))?;

// Choose from legal_bids, not from arbitrary values
```

#### `legal_plays() -> Result<Vec<Card>, AppError>`

Returns valid cards you can play right now.

**Returns**:
- Empty vec if not your turn or not in trick play phase
- Vec of cards from your hand that are legal to play
- Automatically enforces follow-suit rule

**Always use this method** instead of implementing follow-suit logic yourself.

```rust
let legal_plays = state.legal_plays()
    .map_err(|e| AiError::Internal(format!("{e}")))?;

// Choose from legal_plays, not from state.hand directly
```

#### `legal_trumps() -> Vec<Trump>`

Returns all valid trump options (always the same 5 options).

**Returns**: `[Trump::Clubs, Trump::Diamonds, Trump::Hearts, Trump::Spades, Trump::NoTrump]`

```rust
let trumps = state.legal_trumps();
// All 5 options are always legal
```

---

## Core Data Types

### Card

```rust
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}
```

**Comparison**: Cards implement `Ord` for sorting, but this is for display purposes only. Don't use `<` or `>` for trick resolution - the game engine handles that based on trump and lead suit.

### Suit

```rust
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}
```

### Rank

```rust
pub enum Rank {
    Two, Three, Four, Five, Six, Seven, Eight, Nine, Ten,
    Jack, Queen, King, Ace,
}
```

**Trick value order** (when same suit): `Two < Three < ... < Ace`

### Trump

```rust
pub enum Trump {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
    NoTrump,
}
```

**Conversion**: You can convert `Suit` to `Trump` easily (`Trump::from(suit)`), but note that `choose_trump()` requires returning a `Suit`, not a `Trump`.

### GameState

```rust
pub enum GameState {
    Bidding,        // Players are bidding
    TrumpSelection, // Highest bidder is choosing trump
    TrickPlay,      // Playing tricks
}
```

Check `state.game_state` to understand which phase the game is in.

### AiError

```rust
pub enum AiError {
    Timeout,            // Decision took too long (reserved for future use)
    Internal(String),   // Internal error in your AI logic
    InvalidMove(String),// Attempted illegal move
}
```

---

## Reference Implementation: RandomPlayer

Here's the complete implementation of `RandomPlayer`, demonstrating best practices:

```rust
use std::sync::Mutex;
use rand::prelude::*;

use crate::ai::{AiError, AiPlayer};
use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::{Card, Trump};

/// AI that makes random legal moves.
///
/// Can be seeded for deterministic behavior in tests.
pub struct RandomPlayer {
    rng: Mutex<StdRng>,  // Mutex for thread-safe interior mutability
}

impl RandomPlayer {
    /// Create a new RandomPlayer.
    ///
    /// - If `seed` is Some, uses that seed for deterministic behavior
    /// - If `seed` is None, uses system entropy for randomness
    pub fn new(seed: Option<u64>) -> Self {
        let rng = if let Some(s) = seed {
            StdRng::seed_from_u64(s)
        } else {
            StdRng::from_entropy()
        };
        Self {
            rng: Mutex::new(rng),
        }
    }
}

impl AiPlayer for RandomPlayer {
    fn choose_bid(&self, state: &CurrentRoundInfo, _context: &GameContext) -> Result<u8, AiError> {
        // Get legal bids (handles dealer restriction)
        let legal_bids = state
            .legal_bids()
            .map_err(|e| AiError::Internal(format!("Failed to get legal bids: {e}")))?;

        // Validate we have options
        if legal_bids.is_empty() {
            return Err(AiError::InvalidMove("No legal bids available".into()));
        }

        // Lock RNG and choose randomly
        let mut rng = self
            .rng
            .lock()
            .map_err(|e| AiError::Internal(format!("RNG lock poisoned: {e}")))?;
        
        let choice = legal_bids
            .choose(&mut *rng)
            .copied()
            .ok_or_else(|| AiError::Internal("Failed to choose random bid".into()))?;

        Ok(choice)
    }

    fn choose_play(&self, state: &CurrentRoundInfo, _context: &GameContext) -> Result<Card, AiError> {
        // Get legal plays (handles follow-suit rule)
        let legal_plays = state
            .legal_plays()
            .map_err(|e| AiError::Internal(format!("Failed to get legal plays: {e}")))?;

        // Validate we have options
        if legal_plays.is_empty() {
            return Err(AiError::InvalidMove("No legal plays available".into()));
        }

        // Lock RNG and choose randomly
        let mut rng = self
            .rng
            .lock()
            .map_err(|e| AiError::Internal(format!("RNG lock poisoned: {e}")))?;
        
        let choice = legal_plays
            .choose(&mut *rng)
            .copied()
            .ok_or_else(|| AiError::Internal("Failed to choose random card".into()))?;

        Ok(choice)
    }

    fn choose_trump(&self, state: &CurrentRoundInfo, _context: &GameContext) -> Result<Trump, AiError> {
        // Get all legal trump options (5 choices including NoTrump)
        let legal_trumps = state.legal_trumps();

        // Validate we have options (should always have 5)
        if legal_trumps.is_empty() {
            return Err(AiError::InvalidMove("No legal trumps available".into()));
        }

        // Lock RNG and choose randomly from all 5 options
        let mut rng = self
            .rng
            .lock()
            .map_err(|e| AiError::Internal(format!("RNG lock poisoned: {e}")))?;
        
        let choice = legal_trumps
            .choose(&mut *rng)
            .copied()
            .ok_or_else(|| AiError::Internal("Failed to choose random trump".into()))?;

        Ok(choice)
    }
}
```

### Key Patterns Demonstrated

1. **Thread-safe RNG**: Uses `Mutex<StdRng>` for interior mutability
2. **Deterministic testing**: Accepts optional seed parameter
3. **Always use legal move helpers**: Calls `legal_bids()` and `legal_plays()`
4. **Proper error handling**: Wraps errors, validates preconditions, never panics
5. **`Send + Sync` satisfied**: No shared mutable state without synchronization

---

## Advanced: GameHistory via GameContext

For advanced strategies that learn from opponent behavior, you can access complete game history via the `GameContext` parameter.

### Purpose

Access all completed rounds to:
- Analyze opponent bidding tendencies (aggressive vs conservative)
- Track trump selections by player
- Adapt strategy based on score differential
- Build statistical models for opponent behavior

### Accessing History

Game history is cached by the game engine and accessed via `context.game_history()`:

```rust
fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError> {
    // Access cached game history (no database connection needed!)
    let history = context.game_history();
    
    // Analyze opponent's recent bidding pattern
    let opponent_seat = (state.player_seat + 1) % 4;
    
    let recent_bids: Vec<u8> = history.rounds
        .iter()
        .rev()
        .take(5)  // Last 5 rounds
        .filter_map(|r| r.bids[opponent_seat as usize])
        .collect();
    
    let avg_recent = if !recent_bids.is_empty() {
        recent_bids.iter().sum::<u8>() as f64 / recent_bids.len() as f64
    } else {
        0.0
    };
    
    // Use avg_recent to inform your bid...
    
    // Your bidding logic
    let legal_bids = state.legal_bids()
        .map_err(|e| AiError::Internal(format!("{e}")))?;
    Ok(legal_bids[0])
}
```

**Key Points**:
- `context.game_history()` returns `&GameHistory` (always available in production)
- History is updated automatically after each round completes
- No database connection or async code required in your AI
- Separate from `CurrentRoundInfo` for clean architectural separation

### GameHistory Structure

```rust
pub struct GameHistory {
    pub rounds: Vec<RoundHistory>,
}
```

### RoundHistory Fields

```rust
pub struct RoundHistory {
    pub round_no: i16,                      // Round number (0-25)
    pub dealer_seat: i16,                   // Who dealt this round
    pub bids: [Option<u8>; 4],             // Bids by each player
    pub trump_selector_seat: Option<i16>,  // Who won the bid
    pub trump: Option<Trump>,              // Trump choice (if selected)
    pub scores: [RoundScoreDetail; 4],     // Scores for each player
}

pub struct RoundScoreDetail {
    pub round_score: i16,        // Points earned this round
    pub cumulative_score: i16,   // Total score after this round
}
```

### Usage Example: Opponent Analysis

```rust
fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError> {
    let legal_bids = state.legal_bids()
        .map_err(|e| AiError::Internal(format!("{e}")))?;
    
    // Analyze all opponents
    let history = context.game_history();
    let mut opponent_aggression = [0.0; 4];
    
    for (seat, aggression) in opponent_aggression.iter_mut().enumerate() {
        if seat == state.player_seat as usize {
            continue; // Skip self
        }
        
        let mut total = 0.0;
        let mut count = 0;
        
        for round in &history.rounds {
            if let Some(bid) = round.bids[seat] {
                // High bid relative to hand size = aggressive
                let hand_size = round.scores[seat].cumulative_score; // Example metric
                total += bid as f64;
                count += 1;
            }
        }
        
        *aggression = if count > 0 { total / count as f64 } else { 0.0 };
    }
    
    // Use aggression scores to inform your bid strategy...
    
    // Make bid decision
    Ok(legal_bids[0])
}

---

## Error Handling

### When to Use Each Error Type

**`AiError::Internal(String)`**
- RNG failures (lock poisoning, etc.)
- Logic errors in your AI code
- Failed assumptions in your algorithm
- Parse errors or invalid state

```rust
let mut rng = self.rng.lock()
    .map_err(|e| AiError::Internal(format!("RNG lock poisoned: {e}")))?;
```

**`AiError::InvalidMove(String)`**
- No legal moves available (shouldn't happen if using helper methods)
- Your AI somehow computed an illegal action

```rust
if legal_bids.is_empty() {
    return Err(AiError::InvalidMove("No legal bids available".into()));
}
```

**`AiError::Timeout`**
- Reserved for future timeout enforcement
- Don't use in your AI implementations

### Error Handling Pattern

```rust
fn choose_bid(&self, state: &CurrentRoundInfo) -> Result<u8, AiError> {
    // 1. Get legal moves, wrapping domain errors
    let legal_bids = state.legal_bids()
        .map_err(|e| AiError::Internal(format!("Failed to get legal bids: {e}")))?;
    
    // 2. Validate preconditions
    if legal_bids.is_empty() {
        return Err(AiError::InvalidMove("No legal bids available".into()));
    }
    
    // 3. Your logic with proper error handling
    let bid = self.compute_bid(state, &legal_bids)
        .map_err(|e| AiError::Internal(format!("Bid computation failed: {e}")))?;
    
    // 4. Validate result (optional defensive check)
    if !legal_bids.contains(&bid) {
        return Err(AiError::Internal(format!("Computed illegal bid: {bid}")));
    }
    
    Ok(bid)
}
```

### Never Panic

**Don't do this**:
```rust
fn choose_bid(&self, state: &CurrentRoundInfo) -> Result<u8, AiError> {
    let legal_bids = state.legal_bids().unwrap();  // ❌ Can panic!
    Ok(legal_bids[0])  // ❌ Can panic if empty!
}
```

**Do this instead**:
```rust
fn choose_bid(&self, state: &CurrentRoundInfo) -> Result<u8, AiError> {
    let legal_bids = state.legal_bids()
        .map_err(|e| AiError::Internal(format!("{e}")))?;  // ✅
    
    legal_bids.first()
        .copied()
        .ok_or_else(|| AiError::InvalidMove("No legal bids".into()))  // ✅
}
```

---

## Testing Your AI

### Unit Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_deterministic_behavior() {
        // Create AI with specific seed
        let ai1 = MyAI::new(Some(12345));
        let ai2 = MyAI::new(Some(12345));
        
        // Create mock game state
        // (You'll need to construct CurrentRoundInfo for testing)
        
        // Verify both AIs make same decisions
        let bid1 = ai1.choose_bid(&state).unwrap();
        let bid2 = ai2.choose_bid(&state).unwrap();
        assert_eq!(bid1, bid2);
    }
    
    #[test]
    fn test_always_makes_legal_moves() {
        let ai = MyAI::new(None);
        
        // Test with various game states
        // Verify AI always chooses from legal moves
    }
}
```

### Testing Edge Cases

Test your AI against these scenarios:

1. **Dealer bid restriction**: When last to bid with sum constraint
2. **Follow-suit enforcement**: When must play specific suit
3. **Limited options**: When only 1-2 legal moves available
4. **All hand sizes**: Test with 2 cards and 13 cards
5. **Different trump selections**: Verify trump choice logic
6. **Score-based decisions**: If your AI adapts to scores

### Integration Testing

While you won't run full game simulations yourself, consider:
- Does your AI handle all phases correctly?
- Does it never return illegal moves?
- Does it complete decisions quickly (< 1 second recommended)?
- Is behavior deterministic when seeded?

---

## Submission Requirements

### What to Submit

**Single Rust File** containing:
1. Your struct definition
2. `AiPlayer` trait implementation
3. Any helper methods or types
4. (Optional) Unit tests

**Brief Description** including:
- AI name and strategy overview
- Any notable features or algorithms
- Recommended configuration parameters (if any)

### File Structure Example

```rust
//! MyAI - Brief description of strategy
//!
//! Author: Your Name
//! Strategy: [Describe approach]

use crate::ai::{AiError, AiPlayer};
use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::{Card, Rank, Suit};

// Optional: dependencies if needed
use std::sync::Mutex;
use rand::prelude::*;

/// Your AI struct with any state needed
pub struct MyAI {
    // State here
}

impl MyAI {
    /// Constructor
    pub fn new(seed: Option<u64>) -> Self {
        // Initialize
        Self { /* ... */ }
    }
    
    // Helper methods here
}

impl AiPlayer for MyAI {
    fn choose_bid(&self, state: &CurrentRoundInfo) -> Result<u8, AiError> {
        // Implementation
    }
    
    fn choose_play(&self, state: &CurrentRoundInfo) -> Result<Card, AiError> {
        // Implementation
    }
    
    fn choose_trump(&self, state: &CurrentRoundInfo) -> Result<Suit, AiError> {
        // Implementation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Your tests here
}
```

### Required Traits

- `AiPlayer` trait implementation (all 3 methods)
- `Send + Sync` for thread safety (automatic for most implementations)

### Naming Conventions

- **Struct name**: Descriptive (e.g., `AggressivePlayer`, `CardCountingAI`, `MinimaxPlayer`)
- **File name**: `your_ai_name.rs`

### What Happens Next

1. Submit your file for code review
2. Integration testing against game engine
3. Code review feedback (if any)
4. Integration into the repository
5. Your AI can then be used in games!

---

## Best Practices Summary

### ✅ DO

- Use `state.legal_bids()` and `state.legal_plays()` for valid moves
- Return `AiError` instead of panicking
- Use `Mutex` for thread-safe mutable state (e.g., RNG)
- Support optional seed parameter for deterministic testing
- Validate preconditions and check for empty legal move lists
- Keep decision logic fast (< 1 second per decision)
- Write unit tests for your AI

### ❌ DON'T

- Don't implement follow-suit or dealer restriction logic yourself
- Don't use `unwrap()` or `expect()` (return errors instead)
- Don't panic under any circumstances
- Don't directly play from `state.hand` without checking `legal_plays()`
- Don't modify shared state without proper synchronization
- Don't make assumptions about array lengths without validation

---

## Examples of AI Strategies

### Conservative Player

```rust
// Bids conservatively (count sure tricks only)
// Plays highest cards when leading, lowest when following
```

### Aggressive Player

```rust
// Bids optimistically (counts potential tricks)
// Takes risks to win tricks even without trump advantage
```

### Card Counter

```rust
// Tracks cards played in CURRENT TRICK using CurrentRoundInfo.current_trick_plays
// Uses AI Memory (when implemented) to remember cards from previous tricks this round
// Calculates probability of remaining cards in current round
// Makes optimal decisions based on known information
```

### Adaptive Player

```rust
// Analyzes opponent behavior from GameHistory
// Adjusts bidding based on opponent patterns
// Changes strategy based on score differential
```

---

## Questions?

For questions about:
- **Game rules**: Refer to [Game Rules](#game-rules) section above
- **API usage**: Check [Available Game State](#available-game-state) and examples
- **Implementation patterns**: See [Reference Implementation](#reference-implementation-randomplayer)
- **Submission**: See [Submission Requirements](#submission-requirements)

**Key source files** (for reference):
- AI trait definition: `apps/backend/src/ai/trait_def.rs`
- Random player example: `apps/backend/src/ai/random.rs`
- Game state view: `apps/backend/src/domain/player_view.rs`
- Card types: `apps/backend/src/domain/cards_types.rs`

---

Good luck building your AI player!
