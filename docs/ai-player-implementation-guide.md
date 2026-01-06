# Nommie AI Player Implementation Guide

## Document Scope

Step-by-step guide for integrating AI players with the production backend. It
focuses on the `AiPlayer` trait, deterministic tooling, and submission
requirements. For the canonical ruleset see `game-rules.md`; for high-throughput
offline simulation refer to `backend-in-memory-game-engine.md`.

This guide explains how to build AI players for **Nommie** (Nomination Whist). It’s designed for fast onboarding and reliable implementation, with full code listings moved to **numbered appendices** for readability.

---

## Table of Contents

1. [Game Rules](#game-rules)
2. [Indexing Reference](#indexing-reference)
3. [RNG & Determinism](#rng--determinism)
4. [Quick Start](#quick-start)
5. [AI Registry](#ai-registry)
6. [The AiPlayer Trait](#the-aiplayer-trait)
7. [Available Game State](#available-game-state)
8. [Core Data Types](#core-data-types)
9. [Error Handling](#error-handling)
10. [Testing Your AI](#testing-your-ai)
11. [AI Simulator](#ai-simulator)
12. [Best Practices](#best-practices)
13. [Submission Requirements](#submission-requirements)
14. [Appendices](#appendices)

---

## Game Rules

Nommie is a 4-player trick‑taking game with bidding. This section summarises the
critical rules the AI must honour; the full canonical source lives in
`game-rules.md`.

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
- Winner chooses one of **Clubs / Diamonds / Hearts / Spades / NoTrumps**.

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
| **Rounds (history)** | 1–26 | `RoundHistory.round_no` (1-based) |
| **Rounds (current)** | 0–25 | `CurrentRoundInfo.current_round` (0-based) |
| **Tricks** | 1–hand_size | `trick_no` in memory/tracking (1-based) |
| **Seats** | 0–3 | array indices (`bids[seat]`, `scores[seat]`) |

`round_no=1 → 13 cards • round_no=13 → first 2‑card round • round_no=26 → 13 cards`.  
For `CurrentRoundInfo.current_round`: value 0 = round 1, value 12 = first 2‑card round, value 25 = round 26.

**Arrays:** All multi-player arrays (`bids`, `scores`) index by seat 0–3.

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
- **Seed functions:** `derive_dealing_seed` and `derive_memory_seed` are **engine‑internal** and not exposed to AIs.
- **Stateless safety:** Salting avoids order‑of‑calls dependence and supports parallelism.
- **Replays:** Same `game_seed` + same moves ⇒ identical memory and dealing.

**AI RNG wiring:** See **Appendix A.2** for constructing AIs with optional seeds (e.g., `MyAI::new(Some(42))` for tests, `MyAI::new(None)` for production entropy).

---

## Quick Start

Use the short template in **Appendix A.1** to get a working AI.  
For a complete, thread‑safe reference with deterministic RNG, see **Appendix A.2 (RandomPlayer)**.

Typical loop:
1. Call legal helpers in this order: `context.legal_bids(state)` → `state.legal_trumps()` → `state.legal_plays()`.
2. Apply your strategy (bidding heuristics, trump selection, then card choice).
3. Return a **legal** result. Never panic.

---

## AI Registry

All production AIs are surfaced through a static, deterministic registry in `apps/backend/src/ai/registry.rs`. Every contributor must register their implementation manually:

1. **Implement** `AiPlayer` for your type in your module.
2. **Add** a new `AiFactory` entry to the `AI_FACTORIES` slice:
   - Choose a stable `name` and semantic `version`.
   - Provide a constructor function that returns `Box<dyn AiPlayer + Send + Sync>` and accepts the optional seed.
3. **Preserve ordering** in the slice—treat it as append-only unless you have a breaking reason.
4. Avoid side effects in the constructor. Respect the seed input so deterministic tests can reproduce decisions.

Once registered, your AI becomes available via `crate::ai::registry::{registered_ais, by_name}` and is automatically covered by the conformance suite (see [Testing Your AI](#testing-your-ai)).

---

## The AiPlayer Trait

Your AI implements three methods, each receiving **read‑only** views:

- `choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError>`  
  Called during **Bidding** phase only. Use `context.legal_bids(state)`; enforces dealer restriction and zero‑bid streak rule.

- `choose_trump(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Trump, AiError>`  
  Called during **TrumpSelection** phase only, and **only for the bid winner**. Use `state.legal_trumps()`; all 5 options are valid.

- `choose_play(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<Card, AiError>`  
  Called during **TrickPlay** phase only. Use `state.legal_plays()`; engine enforces follow‑suit.

**Phase guarantee:** The engine only calls methods during the correct phase. AIs are never invoked out-of-phase.

**Thread safety:** your AI struct must be `Send + Sync`. If you keep RNG, wrap it in `Mutex<StdRng>`.

**Lifecycle:** The engine creates a **new AI instance for each decision** (bid/trump/play). The same AI instance may be called from different games or concurrently. **Do not store per‑game state** in your AI struct; use only the provided `state` and `context`.

---

## Available Game State

### `CurrentRoundInfo` (high-level fields)
- `game_id: i64` — useful for fetching cached `GameHistory`/`RoundMemory`
- `player_seat: u8` (0–3), `dealer_pos: u8` (0–3)  
- `current_round: u8` (0–25), `hand_size: u8`, `game_state: Phase`
- `hand: Vec<Card>` — remaining cards in your hand after removing plays already recorded
- `bids: [Option<u8>; 4]`, `trump: Option<Trump>`
- `trick_no: u8` (1..=hand_size)  
- `current_trick_plays: Vec<(u8, Card)>` — **in play order** (leader first), seats 0–3, empty if no one has played
- `trick_leader: Option<u8>` → **left of dealer for trick 1; previous trick winner thereafter**  
- `tricks_won: [u8; 4]` — tricks won by each player this round (completed tricks only)
- `scores: [i16; 4]` — **cumulative** scores from all completed rounds (current round not included)

### Helpers (always prefer these)

**On `GameContext`:**
- `context.legal_bids(state) -> Vec<u8>` — sorted 0..=hand_size; enforces dealer restriction and zero‑bid streak rule

**On `CurrentRoundInfo`:**
- `state.legal_trumps() -> Vec<Trump>` — all 5 options (Clubs, Diamonds, Hearts, Spades, NoTrumps)
- `state.legal_plays() -> Vec<Card>` — arbitrary order; returns empty if not your turn

**Note:** The engine calls AIs only when it's their turn, so `legal_*` should never be empty. If needed, advance with `(seat + 1) % 4`.

### `GameContext`
- `game_history() -> Option<&GameHistory>` (available once game starts)
- `round_memory() -> Option<&RoundMemory>` (**completed** tricks only; updates after each completed trick; **not** during the current trick)

**Memory determinism:** Outcomes are fully deterministic for a given `game_seed` and salts; see **RNG & Determinism** and **Appendix C**.

### `GameHistory` / `RoundHistory`
- `RoundHistory` includes `hand_size: u8` field for each round
- **Use `r.hand_size` directly** instead of calculating from `round_no`
- Prevents off-by-one errors when analyzing historical data
- **See Appendix D.1** for full struct definitions

### Memory Degradation Enums

**MemoryMode** — Controls memory fidelity:  
- `Full` — Perfect recall of all plays (memory level 100)  
- `Partial { level }` — Degraded memory (levels 1-99)  
- `None` — No historical card memory (level 0)

**PlayMemory** — What you remember about a single card play:  
- `Exact(Card)` — Perfect recall of suit and rank  
- `Suit(Suit)` — Remember suit, forgot rank  
- `RankCategory(RankCategory)` — Only vague memory (high/medium/low)  
- `Forgotten` — No memory of this play

**RankCategory** — Vague rank memory:  
- `High` — Jack, Queen, King, Ace  
- `Medium` — 7, 8, 9, 10  
- `Low` — 2, 3, 4, 5, 6

**Degradation order:** `Exact → Suit → RankCategory → Forgotten`. Higher-value cards (Ace, King) are more memorable than low cards (2, 3).

**Helpers:** `is_exact()`, `is_forgotten()`, `exact_card()` → `Option<Card>`. See **Appendix D.2** for full struct definitions.

---

## Core Data Types

- **Card** `{ suit: Suit, rank: Rank }` — `Ord` is for sorting only; trick resolution is engine‑driven by lead/trump.
- **Suit** `Clubs|Diamonds|Hearts|Spades`
- **Trump** `Clubs|Diamonds|Hearts|Spades|NoTrumps`
- **Rank** `Two..Ace`
- **Phase** `Init|Bidding|TrumpSelect|Trick { trick_no }|Scoring|Complete|GameOver`
- **AiError** `Timeout|Internal(String)|InvalidMove(String)`

---

## Error Handling

- Wrap domain errors into `AiError::Internal`.
- Use `AiError::InvalidMove` when you can't produce a legal decision.
- **Never panic**. Prefer `Result` and validate preconditions (e.g., non‑empty legal lists).

**Engine policy:** On illegal move or `AiError`, the engine retries up to 3 times. If all retries fail, the game aborts (no random fallback).

---

## Testing Your AI

**Targets**
- Always returns a **legal** bid/play/trump.
- **Deterministic when seeded** (AI seed) and **replayable** via `game_seed`.
- **Fast:** aim for **≤ 50 ms per decision** (guideline only; no engine timeout enforcement).

**Examples & scaffolding:** see **Appendix B**.

### Conformance Suite

The backend owns a deterministic conformance suite that exercises every AI registered in `crate::ai::registry`:

- Command: `cargo test ai_conformance_suite`
- Coverage:
  - Dealer-restriction bidding scenario (last seat with forbidden total).
  - Must-follow suit trick play.
  - Trump selection legality.
  - Seeded determinism checks for bid/play/trump.
- Expectation: all registered AIs pass with no flaky behavior and complete in \< 1 second.

You should run this suite after registering or changing an AI. It is complementary to any custom unit tests in your module.

---

## AI Simulator

The **ai-simulator** runs fast in-memory games without database overhead for rapid iteration on AI strategies.

### Simulator

Located in `packages/ai-simulator/`, the simulator runs games entirely in memory and outputs detailed metrics. Available options include:

**Example usage:**

```bash
cargo run -p ai-simulator -- --games 100 --seats MyNewAI --output-dir /tmp/sim-results

./packages/ai-simulator/analyze_results.py /tmp/sim-results
```

This runs 100 games with your AI in all seats, then analyzes the latest results file automatically.

- Number of games to simulate
- AI type per seat (strategic, heuristic, reckoner, random)
- Fixed game seeds for reproducibility
- Output directory and format (JSONL, CSV)
- Compression options

Results are written to timestamped JSONL and CSV files containing:
- Per-game metrics (final scores, winners, duration)
- Per-round metrics (bids, tricks won, bid accuracy, trump selection)
- Per-player metrics (total score, rounds won, bid accuracy statistics)

### Analysis Script

The `analyze_results.py` script processes simulation JSONL output and provides:

**Bid Accuracy Analysis**
- Overall exact/overbid/underbid percentages
- Mean error, MAE, RMSE, and error distribution histograms
- Breakdown by hand size, trump type, and seat position

**Auction Dynamics**
- Performance comparison when winning the auction (choosing trump) vs not
- First leader analysis (performance when leading first trick)

**Calibration Tables**
- Bid value → actual tricks won mapping
- Breakdown by hand-size buckets

**Score Metrics**
- Win rate, average score per game, points per hand
- Bonus hit rate (percentage of exact bid matches)

**Contract Conversion Stats**
- Round-level success rates for making bids

The analysis script can process a specific JSONL file or automatically find the latest simulation results.

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

**Compatibility:** This guide targets Nommie backend v0.1.0, Rust edition 2021. The `AiPlayer` trait API is stable; we'll document any breaking changes if we bump to v0.2.0+.

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
    fn choose_bid(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<u8, AiError> {
        let legal = cx.legal_bids(state);
        legal.first().copied().ok_or_else(|| AiError::InvalidMove("No legal bids".into()))
    }

    fn choose_play(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<Card, AiError> {
        let legal = state.legal_plays();
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
    fn choose_bid(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<u8, AiError> {
        let legal = cx.legal_bids(state);
        if legal.is_empty() { return Err(AiError::InvalidMove("No legal bids".into())); }
        let mut rng = self.rng.lock().map_err(|e| AiError::Internal(format!("rng lock: {e}")))?;
        legal.choose(&mut *rng).copied().ok_or_else(|| AiError::Internal("rng choice".into()))
    }

    fn choose_play(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<Card, AiError> {
        let legal = state.legal_plays();
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
5. NoTrumps vs suit trumps.  
6. Score pressure (e.g., conservative vs aggressive modes).

---

### Appendix C — Advanced Strategy Examples

#### C.1 — Opponent Analysis using GameHistory
```rust
fn choose_bid(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<u8, AiError> {
    let legal = cx.legal_bids(state);

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
        // Use avg_pct (opponent bid % by hand_size) together with your current hand evaluation
// to compute a target bid here. Then choose a legal bid closest to that target.
        if let Some(&best) = legal.first() { return Ok(best); }
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
    let legal = state.legal_plays();
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
    pub round_no: u8,                      // 1–26 (1-based for history)
    pub hand_size: u8,                     // Number of cards each player had this round
    pub dealer_seat: u8,                   // 0–3
    pub bids: [Option<u8>; 4],
    pub trump_selector_seat: Option<u8>,
    pub trump: Option<Trump>,
    pub scores: [RoundScoreDetail; 4],
}

pub struct RoundScoreDetail {
    pub round_score: u8,
    pub cumulative_score: i16,
}
```

#### D.2 — RoundMemory / PlayMemory
```rust
pub struct RoundMemory { pub mode: MemoryMode, pub tricks: Vec<TrickMemory> }

pub struct TrickMemory {
    pub trick_no: u8, // 1..=hand_size (1-based)
    pub plays: Vec<(u8, PlayMemory)> // (seat, memory)
}

pub enum PlayMemory {
    Exact(Card),
    Suit(Suit),
    RankCategory(RankCategory),
    Forgotten,
}
```
---

**End of Guide**
