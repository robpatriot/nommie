# Nommie AI Player Implementation Guide

This guide explains how to build AI players for **Nommie** (Nomination Whist). It’s designed for fast onboarding and reliable implementation, with full code listings moved to **numbered appendices** for readability.

---

## Table of Contents

1. [Game Rules](#game-rules)
2. [Indexing Reference](#indexing-reference)
3. [RNG & Determinism](#rng--determinism)
4. [Quick Start](#quick-start)
5. [The AiPlayer Trait](#the-aiplayer-trait)
6. [Available Game State](#available-game-state)
7. [Core Data Types](#core-data-types)
8. [Error Handling](#error-handling)
9. [Testing Your AI](#testing-your-ai)
10. [Best Practices](#best-practices)
11. [Submission Requirements](#submission-requirements)
12. [Appendices](#appendices)

---

## Game Rules

Nommie is a 4-player trick‑taking game with bidding.

### Players & Setup
- **4 players** in fixed clockwise order (seats 0–3)
- **26 rounds** with varying hand sizes
- **Hand sizes per round:** 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 2, 2, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13
- Fresh 52‑card deck each round
- Dealer rotates clockwise each round

### Bidding
- Each player bids expected tricks (0 → hand_size). Dealer bids last.
- **Dealer restriction:** dealer may not make total bids == hand_size.
- **Zero-bid limit (Nommie rule):** no player may bid **0** more than **3** rounds in a row.
- **Trump selection:** highest bid wins; ties → earliest bidder in order.

### Trump Selection
- Winner chooses one of **Clubs / Diamonds / Hearts / Spades / NoTrump**.

### Trick Play
- **Lead:** player to the **left of the dealer** leads trick 1; thereafter, the **previous trick winner** leads.
- **Follow suit:** must follow lead suit if possible.
- **Winner:** highest trump if any; otherwise highest of the lead suit.
- **Ranks:** 2 < 3 < … < A.
- Exactly `hand_size` tricks per round.

### Scoring
- +1 per trick won.
- +10 bonus if tricks won == bid (including 0).
- Scores accumulate across all 26 rounds.

### Game End
- After round 26, highest total wins (ties allowed).

---

## Indexing Reference

Nommie mixes 1‑based and 0‑based concepts.

| Concept | Range | Used For |
|---|---|---|
| **Rounds** | 1–26 | human reference & `round_no` |
| **Tricks** | 1–hand_size | per‑round memory/tracking |
| **Seats** | 0–3 | array indices (`bids[seat]`, `scores[seat]`) |

`round_no=1 → 13 cards • round_no=13 → first 2‑card round • round_no=26 → 13 cards`.  
Vector indices are always `round_no – 1`.

---

## RNG & Determinism

A single **game seed** controls deterministic aspects of the engine.

1. **Game creation:** `game_seed = rand::random::<i64>()`  
2. **Derived seeds:**  
   - Dealing → `derive_dealing_seed(game_seed, round_no)`  
   - Memory → `derive_memory_seed(game_seed, round_no, seat)`  
3. **AI decisions:** AIs may accept their own optional seed for tests; production may use entropy.

**Determinism notes:**
- **Engine‑controlled:** Memory degradation is fully deterministic from `(game_seed, round_no, trick_no/seat/… salt)`.
- **Stateless safety:** Salting avoids order‑of‑calls dependence and supports parallelism.
- **Replays:** Same `game_seed` + same moves ⇒ identical memory and dealing.

---

## Quick Start

Use the short template in **Appendix A.1** to get a working AI.  
For a complete, thread‑safe reference with deterministic RNG, see **Appendix A.2 (RandomPlayer)**.

Typical loop:
1. Call legal helpers in this order: `state.legal_bids()` → `state.legal_trumps()` → `state.legal_plays()`.
2. Apply your strategy (bidding heuristics, trump selection, then card choice).
3. Return a **legal** result. Never panic.

---

## The AiPlayer Trait

Your AI implements three methods, each receiving **read‑only** views:

- `choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError>`  
  Use `state.legal_bids()`; engine enforces dealer restriction and zero‑bid streak rule.

- `choose_trump(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Trump, AiError>`  
  Only called for the **bid winner** by the orchestrator.

- `choose_play(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Card, AiError>`  
  Use `state.legal_plays()`; engine enforces follow‑suit.

**Thread safety:** your AI struct must be `Send + Sync`. If you keep RNG, wrap it in `Mutex<StdRng>`.

---

## Available Game State

### `CurrentRoundInfo` (high‑level fields)
- `player_seat: i16` (0–3), `dealer_pos: i16` (0–3)  
- `current_round: i16` (1–26), `hand_size: u8`, `game_state: GameState`
- `bids: Vec<Option<u8>>`, `trump: Option<Trump>`
- `trick_no: i16` (1..=hand_size)  
- `current_trick_plays: Vec<(i16, Card)>`
- `trick_leader: Option<i16>` → **left of dealer for trick 1; previous trick winner thereafter**  
- `scores: [i16; 4]`

### Helpers (always prefer these)
- `legal_bids() -> Result<Vec<u8>, AppError>`
- `legal_trumps() -> Vec<Trump>`
- `legal_plays() -> Result<Vec<Card>, AppError>`

### `GameContext`
- `game_history() -> Option<&GameHistory>` (available once game starts)
- `round_memory() -> Option<&RoundMemory>` (**completed** tricks only; **not** the current trick)

**Memory determinism:** Outcomes are fully deterministic for a given `game_seed` and salts; see **Appendix C**.

### `GameHistory` / `RoundHistory`
- `RoundHistory` includes `hand_size: u8` field for each round
- **Use `r.hand_size` directly** instead of calculating from `round_no`
- Prevents off-by-one errors when analyzing historical data

---

## Core Data Types

- **Card** `{ suit: Suit, rank: Rank }` — `Ord` is for sorting only; trick resolution is engine‑driven by lead/trump.
- **Suit** `Clubs|Diamonds|Hearts|Spades`
- **Trump** `Clubs|Diamonds|Hearts|Spades|NoTrump`
- **Rank** `Two..Ace`
- **GameState** `Bidding|TrumpSelection|TrickPlay`
- **AiError** `Timeout|Internal(String)|InvalidMove(String)`

---

## Error Handling

- Wrap domain errors into `AiError::Internal`.
- Use `AiError::InvalidMove` when you can’t produce a legal decision.
- **Never panic**. Prefer `Result` and validate preconditions (e.g., non‑empty legal lists).

---

## Testing Your AI

**Targets**
- Always returns a **legal** bid/play/trump.
- **Deterministic when seeded** (AI seed) and **replayable** via `game_seed`.
- **Fast:** aim for **≤ 50 ms per decision**.

**Examples & scaffolding:** see **Appendix B**.

---

## Best Practices

✅ **Do**
- Call `legal_*` helpers; never re‑implement rules.
- Keep mutex‑guarded RNG sections **short**; no blocking I/O inside locks.
- Use a **single source of truth** for turn order and indexing (see table above).
- Separate **strategy** (your logic) from **legality** (engine helpers).
- Treat `game_history()` and `round_memory()` as **optional** (`Option`), with safe fallbacks.
- Use `r.hand_size` from `RoundHistory` instead of calculating from `round_no`.

❌ **Don’t**
- Double‑count the current trick by mixing `current_trick_plays` with `round_memory()`.
- Assume memory or history is available.
- Hold locks across async boundaries or long computations.
- Use `unwrap`/`expect` in decision code.

---

## Submission Requirements

Submit a single Rust file containing:
1. Your AI struct and `AiPlayer` impl
2. Helper methods/types (if any)
3. Optional unit tests

Include a short **description** (strategy overview, parameters).

---

## Appendices

The following sections contain full, copy‑ready listings and extended examples referenced above.

### Appendix A — AI Code Templates

#### A.1 — Minimal MyAI Template
```rust
use crate::ai::{AiError, AiPlayer};
use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::{Card, GameContext, Trump};

pub struct MyAI;

impl MyAI {
    pub fn new() -> Self { Self }
}

impl AiPlayer for MyAI {
    fn choose_bid(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<u8, AiError> {
        let legal = state.legal_bids().map_err(|e| AiError::Internal(format!("{e}")))?;
        legal.first().copied().ok_or_else(|| AiError::InvalidMove("No legal bids".into()))
    }

    fn choose_play(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<Card, AiError> {
        let legal = state.legal_plays().map_err(|e| AiError::Internal(format!("{e}")))?;
        legal.first().copied().ok_or_else(|| AiError::InvalidMove("No legal plays".into()))
    }

    fn choose_trump(&self, _state: &CurrentRoundInfo, _cx: &GameContext) -> Result<Trump, AiError> {
        Ok(Trump::Spades)
    }
}
```

#### A.2 — Full RandomPlayer Implementation
```rust
use std::sync::Mutex;
use rand::prelude::*;

use crate::ai::{AiError, AiPlayer};
use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::{Card, GameContext, Trump};

/// AI that makes random legal moves. Can be seeded for deterministic tests.
pub struct RandomPlayer {
    rng: Mutex<StdRng>,
}

impl RandomPlayer {
    pub fn new(seed: Option<u64>) -> Self {
        let rng = match seed { Some(s) => StdRng::seed_from_u64(s), None => StdRng::from_entropy() };
        Self { rng: Mutex::new(rng) }
    }
}

impl AiPlayer for RandomPlayer {
    fn choose_bid(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<u8, AiError> {
        let legal = state.legal_bids().map_err(|e| AiError::Internal(format!("get bids: {e}")))?;
        if legal.is_empty() { return Err(AiError::InvalidMove("No legal bids".into())); }
        let mut rng = self.rng.lock().map_err(|e| AiError::Internal(format!("rng lock: {e}")))?;
        legal.choose(&mut *rng).copied().ok_or_else(|| AiError::Internal("rng choice".into()))
    }

    fn choose_play(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<Card, AiError> {
        let legal = state.legal_plays().map_err(|e| AiError::Internal(format!("get plays: {e}")))?;
        if legal.is_empty() { return Err(AiError::InvalidMove("No legal plays".into())); }
        let mut rng = self.rng.lock().map_err(|e| AiError::Internal(format!("rng lock: {e}")))?;
        legal.choose(&mut *rng).copied().ok_or_else(|| AiError::Internal("rng choice".into()))
    }

    fn choose_trump(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<Trump, AiError> {
        let trumps = state.legal_trumps();
        if trumps.is_empty() { return Err(AiError::InvalidMove("No trumps".into())); }
        let mut rng = self.rng.lock().map_err(|e| AiError::Internal(format!("rng lock: {e}")))?;
        trumps.choose(&mut *rng).copied().ok_or_else(|| AiError::Internal("rng choice".into()))
    }
}
```

---

### Appendix B — Testing Examples

#### B.1 — Unit Test Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_with_same_seed() {
        let a1 = RandomPlayer::new(Some(7));
        let a2 = RandomPlayer::new(Some(7));
        // build state/context...
        // assert_eq!(a1.choose_bid(&state, &cx).unwrap(), a2.choose_bid(&state, &cx).unwrap());
    }

    #[test]
    fn always_returns_legal_moves() {
        let ai = RandomPlayer::new(None);
        // For various mocked states, assert chosen move ∈ legal_* outputs
    }
}
```

#### B.2 — Edge Case Testing Scenarios
1. Dealer restriction when last to bid.  
2. Zero‑bid streak (3 consecutives ⇒ 0 becomes illegal).  
3. Must‑follow suit with only one legal card.  
4. Hand sizes: 13 and 2 cards.  
5. NoTrump vs suit trumps.  
6. Score pressure (e.g., conservative vs aggressive modes).

---

### Appendix C — Advanced Strategy Examples

#### C.1 — Opponent Analysis using GameHistory
```rust
fn choose_bid(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<u8, AiError> {
    let legal = state.legal_bids().map_err(|e| AiError::Internal(format!("{e}")))?;

    if let Some(hist) = cx.game_history() {
        let me = state.player_seat as usize;
        let mut avg_pct = [0.0; 4]; // average bid as % of hand_size

        for seat in 0..4 {
            if seat == me { continue; }
            let (mut sum_pct, mut n) = (0.0_f64, 0.0_f64);

            for r in &hist.rounds {
                if let Some(b) = r.bids[seat] {
                    let hs = r.hand_size as f64;
                    if hs > 0.0 {
                        sum_pct += (b as f64) / hs;
                        n += 1.0;
                    }
                }
            }
            avg_pct[seat] = if n > 0.0 { sum_pct / n } else { 0.0 };
        }

        // Example use: target bid scaled by our current hand_size and peer aggression
        let my_hs = state.hand_size as f64;
        let opponents_aggr = (avg_pct.iter().sum::<f64>() - avg_pct[me]) / 3.0;
        let base = (my_hs * 0.35).round() as u8; // base heuristic
        let adjust = if opponents_aggr > 0.45 { 1 } else { 0 }; // nudge up if table is aggressive
        let target = base.saturating_add(adjust);

        // Clamp to legal range by choosing nearest legal value
        if let Some(&best) = legal.iter().min_by_key(|&&b| (b as i16 - target as i16).abs()) {
            return Ok(best);
        }
    }

    // Fallback
    legal.first().copied().ok_or_else(|| AiError::InvalidMove("No legal bids".into()))
}
```

#### C.2 — Void Detection (Memory Example)
```rust
fn detect_voids_in_hearts(cx: &GameContext) -> [bool; 4] {
    use crate::domain::{PlayMemory, Suit};
    let mut voids = [false; 4];
    if let Some(mem) = cx.round_memory() {
        for trick in &mem.tricks {
            if let Some((_, PlayMemory::Exact(first))) = trick.plays.first() {
                if first.suit == Suit::Hearts {
                    for (seat, m) in trick.plays.iter().skip(1) {
                        match m {
                            PlayMemory::Exact(c) if c.suit != Suit::Hearts => voids[*seat as usize] = true,
                            PlayMemory::Suit(s) if *s != Suit::Hearts => voids[*seat as usize] = true,
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    voids
}
```

#### C.3 — Card Counting (Memory Example)
```rust
fn count_highs_played(cx: &GameContext) -> usize {
    use crate::domain::{PlayMemory, Rank};
    let mut n = 0;
    if let Some(mem) = cx.round_memory() {
        for t in &mem.tricks {
            for (_, m) in &t.plays {
                match m {
                    PlayMemory::Exact(c) if matches!(c.rank, Rank::Jack | Rank::Queen | Rank::King | Rank::Ace) => n += 1,
                    PlayMemory::RankCategory(cat) if matches!(cat, RankCategory::High) => n += 1,
                    _ => {}
                }
            }
        }
    }
    n
}
```

#### C.4 — Memory‑Aware Strategy Skeleton
```rust
fn choose_play(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<Card, AiError> {
    let legal = state.legal_plays().map_err(|e| AiError::Internal(format!("{e}")))?;
    if let Some(mem) = cx.round_memory() {
        // Compute a simple memory quality score
        let (mut exact, mut total) = (0, 0);
        for t in &mem.tricks { for (_, m) in &t.plays { total += 1; if m.is_exact() { exact += 1; } } }
        let quality = if total > 0 { (exact as f64) / (total as f64) } else { 0.0 };
        // Use `quality` to choose a more/less conservative play...
    }
    legal.first().copied().ok_or_else(|| AiError::InvalidMove("No legal plays".into()))
}
```

---

### Appendix D — Data Structures

#### D.1 — GameHistory / RoundHistory
```rust
pub struct GameHistory { pub rounds: Vec<RoundHistory> }

pub struct RoundHistory {
    pub round_no: i16,                     // 1–26
    pub hand_size: u8,                     // Number of cards each player had this round
    pub dealer_seat: i16,                  // 0–3
    pub bids: [Option<u8>; 4],
    pub trump_selector_seat: Option<i16>,
    pub trump: Option<Trump>,
    pub scores: [RoundScoreDetail; 4],
}

pub struct RoundScoreDetail {
    pub round_score: i16,
    pub cumulative_score: i16,
}
```

#### D.2 — RoundMemory / PlayMemory
```rust
pub struct RoundMemory { pub mode: MemoryMode, pub tricks: Vec<TrickMemory> }

pub struct TrickMemory { pub trick_no: i16, pub plays: Vec<(i16, PlayMemory)> }

pub enum PlayMemory {
    Exact(Card),
    Suit(Suit),
    RankCategory(RankCategory),
    Forgotten,
}
```
---

**End of Guide**
