//! Strategic — a memory-aware, context-aware AI for Nommie.
//!
//! Goals:
//! - Stay 100% legal using the engine's `legal_*()` helpers.
//! - Be deterministic (no RNG), but materially stronger than simple hand evaluation.
//! - Use RoundMemory for card counting and void detection.
//! - Use GameHistory for opponent modeling and pattern recognition.
//! - Adapt strategy based on game context (trick number, scores, round number).
//!
//! Bidding (context-aware EV):
//! - Estimate trick potential from suit shape and top-card density.
//! - Adjust based on score position (more aggressive when trailing).
//! - Learn from opponent patterns via GameHistory.
//! - Choose the closest legal bid to the adjusted estimate.
//!
//! Trump selection:
//! - Prefer the suit with the best (count, high-card) tuple.
//! - Consider No Trump (if legal) when the hand is balanced and has enough top cards.
//!
//! Play strategy (bid-target-driven):
//! - Every play decision is anchored to the bid target (exact tricks for +10 bonus).
//! - Track tricks won via state.tricks_won to compute remaining need.
//! - Compute target policy: need = bid - tricks_won, avoid = tricks_won >= bid.
//! - Pressure scaling: need == 0 → strongly prefer losing; need >= tricks_remaining → strongly prefer winning.
//! - Score cards by target alignment (win vs lose desirability) with heuristics as tie-breakers.
//! - Prefer cheap wins (low cards when winning needed) and conserve highs when losing needed.
//!
//! Notes & assumptions:
//! - Uses public fields (`state.hand`, `state.trump`, `state.current_trick_plays`, etc.)
//!   plus the `legal_*()` helpers and context information.
//! - `Card` is assumed to expose `suit: Suit` and implement `Ord` among same-suit ranks.
//! - `Trump` is one of the five enum variants (`Clubs`, `Diamonds`, `Hearts`, `Spades`, `NoTrumps`).
//!
//! Determinism:
//! - No randomness used. `seed` is stored for future extensions (e.g., tie-breaking knobs).
use std::collections::HashMap;

use crate::ai::{AiError, AiPlayer};
use crate::domain::player_view::{CurrentRoundInfo, GameHistory};
use crate::domain::round_memory::{PlayMemory, RoundMemory};
use crate::domain::{card_beats, Card, GameContext, Rank, Suit, Trump};

#[derive(Clone)]
pub struct Strategic {
    _seed: Option<u64>, // reserved, currently unused for strict determinism
}

/// Context for scoring cards in bid-target-driven play selection.
struct ScoringContext<'a> {
    legal: &'a [Card],
    current_plays: &'a [(u8, Card)],
    lead_suit: Option<Suit>,
    trump: Trump,
    pressure: f32,
    avoid: bool,
    memory: Option<&'a RoundMemory>,
    my_hand: &'a [Card],
    opponent_voids: [Vec<Suit>; 4],
    my_seat: u8,
    is_endgame: bool,
    need: u8,
}

impl Strategic {
    pub const NAME: &'static str = "Strategic";
    pub const VERSION: &'static str = "1.0.0";

    pub fn new(seed: Option<u64>) -> Self {
        Self { _seed: seed }
    }

    // ---------- Utilities (pure, small, deterministic) ----------

    fn is_trump(card: &Card, trump: Trump) -> bool {
        matches!(
            (trump, card.suit),
            (Trump::Clubs, Suit::Clubs)
                | (Trump::Diamonds, Suit::Diamonds)
                | (Trump::Hearts, Suit::Hearts)
                | (Trump::Spades, Suit::Spades)
        )
    }

    /// High-card score of a suit: evaluates top 5 cards (10, J, Q, K, A).
    /// Ace = 6, King = 5, Queen = 2, Jack = 1, Ten = 1.
    /// Relies on `Ord` within a suit.
    fn suit_high_score(suit_cards: &mut [Card]) -> usize {
        if suit_cards.is_empty() {
            return 0;
        }
        suit_cards.sort(); // ascending
        let mut score = 0;
        let mut high_cards_counted = 0;

        // Score from highest to lowest, but only count top 5 high cards (10-A)
        for card in suit_cards.iter().rev() {
            if high_cards_counted >= 5 {
                break;
            }
            match card.rank {
                Rank::Ace => {
                    score += 6;
                    high_cards_counted += 1;
                }
                Rank::King => {
                    score += 5;
                    high_cards_counted += 1;
                }
                Rank::Queen => {
                    score += 2;
                    high_cards_counted += 1;
                }
                Rank::Jack => {
                    score += 1;
                    high_cards_counted += 1;
                }
                Rank::Ten => {
                    score += 1;
                    high_cards_counted += 1;
                }
                _ => {
                    // Below Ten, don't count
                }
            }
        }
        score
    }

    /// Coarse hand strength estimate in tricks.
    /// - Longest suit adds weight.
    /// - High-card density adds weight.
    /// - Very short suits (void/singleton) add a little for ruffing potential.
    fn estimate_tricks(state: &CurrentRoundInfo) -> f32 {
        let hand = &state.hand;
        let n = hand.len().max(1);

        // Group cards by suit
        let mut by_suit_cards: HashMap<Suit, Vec<Card>> = HashMap::new();
        for c in hand.iter() {
            by_suit_cards.entry(c.suit).or_default().push(*c);
        }

        let counts: Vec<usize> = by_suit_cards.values().map(|v| v.len()).collect();
        let longest = counts.iter().max().copied().unwrap_or(0) as f32;

        // High-card control proxy
        let mut hi = 0usize;
        for v in by_suit_cards.values_mut() {
            hi += Self::suit_high_score(v);
        }
        let hi_norm = hi as f32 / 6.0; // max 6 if two cards in each suit considered

        // Short-suit ruffing potential (count void/singleton)
        let short_bonus = counts
            .iter()
            .map(|&c| if c <= 1 { 0.5 } else { 0.0 })
            .sum::<f32>();

        // Combine: tune weights lightly; keep stable/deterministic.
        let estimate = 0.4 * longest + 0.9 * hi_norm + 0.3 * short_bonus;

        // Scale to hand size (roughly normalize to max ~ n/2 tricks)
        let scaled = estimate.min((n as f32) * 0.6);
        scaled * 0.9 // slight conservative bias
    }

    /// Compute "exactness difficulty" - how hard it is to hit exactly the estimated number of tricks.
    /// Higher difficulty means more swinginess, making it harder to control the exact trick count.
    /// Returns a factor: >1.0 = harder to control (nudge bid down), <1.0 = easier to control (closer to estimate).
    fn compute_exactness_difficulty(state: &CurrentRoundInfo) -> f32 {
        let hand = &state.hand;
        let hand_size = hand.len() as f32;

        if hand_size == 0.0 {
            return 1.0;
        }

        // Count cards by suit
        let mut by_suit_cards: HashMap<Suit, Vec<Card>> = HashMap::new();
        for c in hand.iter() {
            by_suit_cards.entry(c.suit).or_default().push(*c);
        }

        // Factor 1: Void count (more voids → higher swinginess)
        // There are 4 suits total, so voids = 4 - suits_with_cards
        let suits_with_cards = by_suit_cards.len();
        let void_count = (4 - suits_with_cards) as f32;
        let void_difficulty = 1.0 + (void_count * 0.15); // +15% per void

        // Factor 2: Longest suit length (very long suits → forced wins late, harder to control)
        let counts: Vec<usize> = by_suit_cards.values().map(|v| v.len()).collect();
        let longest = counts.iter().max().copied().unwrap_or(0) as f32;
        let avg_suit_length = hand_size / 4.0;
        let length_difficulty = if longest > avg_suit_length * 1.5 {
            // Very long suit (e.g., 6+ in a 13-card hand) = harder to control
            1.0 + ((longest - avg_suit_length * 1.5) * 0.1)
        } else {
            1.0
        };

        // Factor 3: Number of top winners (A/K) relative to hand size
        // More A/K = easier to control (can choose when to win)
        let ace_king_count = hand
            .iter()
            .filter(|c| matches!(c.rank, Rank::Ace | Rank::King))
            .count() as f32;
        let control_ratio = ace_king_count / hand_size;
        // Higher control ratio = easier to hit exactly (lower difficulty)
        // Normalize: at 0 A/K → difficulty 1.2, at hand_size/4 A/K → difficulty 0.9
        let control_difficulty = 1.2 - (control_ratio * 1.2); // Scale down as control increases
        let control_difficulty = control_difficulty.clamp(0.8, 1.3); // Clamp reasonable range

        // Factor 4: Suit balance vs extreme shapes
        // Extreme shapes (e.g., 7-3-2-1) are harder to control than balanced (4-3-3-3)
        // For suits with cards, use their count; for voids, use 0
        let variance = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
            .iter()
            .map(|&suit| {
                let count = by_suit_cards.get(&suit).map(|v| v.len()).unwrap_or(0) as f32;
                let diff = count - avg_suit_length;
                diff * diff
            })
            .sum::<f32>()
            / 4.0;
        // Normalize variance - higher variance = more extreme shape = higher difficulty
        // For a 13-card hand: balanced (4-3-3-3) has variance ~0.25, extreme (7-3-2-1) has ~4.25
        let balance_difficulty = 1.0 + (variance * 0.05); // Scale variance impact

        // Combine factors (multiplicative, but keep in reasonable range)
        let combined =
            void_difficulty * length_difficulty * control_difficulty * balance_difficulty;

        // Normalize to reasonable range: 0.85 to 1.15
        // This means: easy hands allow closer to estimate, hard hands nudge down by up to 15%
        combined.clamp(0.85, 1.15)
    }

    /// Pick the legal bid closest to the estimate (bias downward on ties).
    fn choose_bid_from_estimate(legal: &[u8], estimate: f32) -> Option<u8> {
        if legal.is_empty() {
            return None;
        }
        let mut best = legal[0];
        let mut best_delta = (best as f32 - estimate).abs();
        for &b in &legal[1..] {
            let d = (b as f32 - estimate).abs();
            if d < best_delta || (d == best_delta && b < best) {
                best = b;
                best_delta = d;
            }
        }
        Some(best)
    }

    /// Trump scoring: (count * 10) + high_score.
    /// Larger is better; favors length, then top-card quality.
    fn trump_score_for_suit(hand: &[Card], suit: Suit) -> i32 {
        let mut suit_cards: Vec<Card> = hand.iter().filter(|c| c.suit == suit).copied().collect();
        let count = suit_cards.len() as i32;
        let hi = Self::suit_high_score(&mut suit_cards) as i32;
        count * 10 + hi
    }

    /// Consider No Trump only when hand is extremely balanced with exceptional high cards
    /// and the best suit is weak (score < 45).
    fn prefer_no_trump(hand: &[Card], best_suit_score: i32) -> bool {
        // Best suit must be weak - if we have a decent trump suit, prefer it
        if best_suit_score >= 45 {
            return false;
        }

        // Extremely balanced: no suit >= 4 and no suit <= 1 (all suits 2-3)
        let mut by_suit_counts: HashMap<Suit, usize> = HashMap::new();
        let mut total_aces_and_kings = 0;

        for c in hand {
            *by_suit_counts.entry(c.suit).or_insert(0) += 1;
            if matches!(c.rank, Rank::Ace | Rank::King) {
                total_aces_and_kings += 1;
            }
        }

        let balanced = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
            .iter()
            .all(|&suit| {
                let count = by_suit_counts.get(&suit).copied().unwrap_or(0);
                (2..=3).contains(&count)
            });
        if !balanced {
            return false;
        }

        // Count unique suits with Ace or King
        let mut suits_with_ace_or_king = 0;
        for &suit in &[Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let has_ace_or_king = hand
                .iter()
                .any(|c| c.suit == suit && matches!(c.rank, Rank::Ace | Rank::King));
            if has_ace_or_king {
                suits_with_ace_or_king += 1;
            }
        }

        // Exceptional high-card quality: at least 4 total Aces and Kings (or 3 if hand is small)
        let hand_size = hand.len();
        let min_aces_kings = if hand_size <= 6 { 3 } else { 4 };
        if total_aces_and_kings < min_aces_kings {
            return false;
        }

        // Additional check: hand has at least one Ace or King in at least 3 of the 4 suits
        if suits_with_ace_or_king < 3 {
            return false;
        }

        true
    }

    // ---------- Bid Target Tracking ----------

    /// Check if we're in the endgame (final tricks where exact bid matters most).
    /// Returns true if tricks_remaining <= 4.
    fn is_endgame(tricks_remaining: u8, _hand_size: u8) -> bool {
        tricks_remaining <= 4
    }

    /// Compute bid target state: how many tricks we still need and whether to avoid winning.
    fn bid_target_state(state: &CurrentRoundInfo, tricks_won_so_far: u8) -> (u8, bool, f32) {
        let my_bid = state.bids[state.player_seat as usize].unwrap_or(0);
        let tricks_remaining = state.hand_size.saturating_sub(state.trick_no - 1);

        let need = my_bid.saturating_sub(tricks_won_so_far);
        let avoid = tricks_won_so_far >= my_bid;

        // Compute pressure: how urgent it is to win tricks
        // If need == 0 → strongly prefer losing (pressure = -2.0)
        // If need >= tricks_remaining → strongly prefer winning (pressure = 2.0)
        // Otherwise → neutral (pressure = 0.0, or scale based on need/tricks_remaining)
        let pressure = if need == 0 {
            -2.0 // Strongly prefer losing
        } else if tricks_remaining > 0 && need >= tricks_remaining {
            2.0 // Strongly prefer winning
        } else if tricks_remaining > 0 {
            // Scale pressure based on how critical the need is
            let criticality = need as f32 / tricks_remaining as f32;
            criticality * 1.5 // Scale to 0.0-1.5 range
        } else {
            0.0
        };

        (need, avoid, pressure)
    }

    // ---------- Memory Analysis Helpers ----------

    /// Track which suits opponents are void in based on RoundMemory.
    /// Returns a set of suits for each opponent seat that we've observed them discard in.
    fn detect_opponent_voids(memory: Option<&RoundMemory>, my_seat: u8) -> [Vec<Suit>; 4] {
        let mut voids: [Vec<Suit>; 4] = [vec![], vec![], vec![], vec![]];

        if let Some(mem) = memory {
            for trick in &mem.tricks {
                // Find the lead suit for this trick
                let lead_suit = trick
                    .plays
                    .first()
                    .and_then(|(_, play_mem)| match play_mem {
                        PlayMemory::Exact(card) => Some(card.suit),
                        PlayMemory::Suit(suit) => Some(*suit),
                        _ => None,
                    });

                if let Some(lead) = lead_suit {
                    // Check each player's play
                    for (seat, play_mem) in &trick.plays {
                        if *seat == my_seat {
                            continue; // Skip ourselves
                        }

                        // If opponent played a different suit (or we remember it as different suit),
                        // they're likely void in the lead suit
                        match play_mem {
                            PlayMemory::Exact(card) if card.suit != lead => {
                                if !voids[*seat as usize].contains(&lead) {
                                    voids[*seat as usize].push(lead);
                                }
                            }
                            PlayMemory::Suit(suit) if *suit != lead => {
                                if !voids[*seat as usize].contains(&lead) {
                                    voids[*seat as usize].push(lead);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        voids
    }

    /// Check if a rank is in the tracked range (10, J, Q, K, A).
    fn is_tracked_rank(rank: Rank) -> bool {
        matches!(
            rank,
            Rank::Ten | Rank::Jack | Rank::Queen | Rank::King | Rank::Ace
        )
    }

    /// Comprehensive card counting for ranks 10-A with all memory fidelity levels.
    ///
    /// Returns estimated count of remaining tracked cards (10-A) in the specified suit.
    /// Handles all memory fidelity levels: Exact, Suit, RankCategory, and Forgotten.
    fn count_tracked_cards(memory: Option<&RoundMemory>, suit: Suit, my_hand: &[Card]) -> f32 {
        use crate::domain::round_memory::PlayMemory;

        const TOTAL_TRACKED_PER_SUIT: f32 = 5.0; // 10, J, Q, K, A

        // Count cards we have in this suit (rank 10+)
        let my_count = my_hand
            .iter()
            .filter(|c| c.suit == suit && Self::is_tracked_rank(c.rank))
            .count() as f32;

        // Track what we remember being played
        let mut exact_played = 0.0; // Exact cards we saw
        let mut suit_only_count = 0.0; // Suit known but rank unknown

        if let Some(mem) = memory {
            for trick in &mem.tricks {
                for (_, play_mem) in &trick.plays {
                    match play_mem {
                        PlayMemory::Exact(card) => {
                            if card.suit == suit && Self::is_tracked_rank(card.rank) {
                                exact_played += 1.0;
                            }
                        }
                        PlayMemory::Suit(s) => {
                            if *s == suit {
                                // Suit known, rank unknown - could be any of the 5 tracked ranks
                                // We'll count this probabilistically using fractional counting
                                suit_only_count += 1.0;
                            }
                        }
                        // RankCategory (High/Medium/Low) doesn't include suit information,
                        // so we can't use it for suit-specific counting. This is a limitation
                        // of the memory system - we can't know which suit a RankCategory memory
                        // refers to, so we must ignore it for deterministic counting.
                        PlayMemory::RankCategory(_) => {
                            // Cannot use - no suit information
                        }
                        PlayMemory::Forgotten => {
                            // Forgotten - no information
                        }
                    }
                }
            }
        }

        // Calculate estimated remaining
        // Known: exact_played are definitely out
        // Uncertain: suit_only_count cards of this suit were played, but we don't know which ranks
        // For deterministic counting, we estimate that suit_only_count reduces available tracked cards
        // by suit_only_count * (5 tracked ranks / 13 total ranks) ≈ suit_only_count * 0.385
        // But to be conservative and deterministic, we'll use a simpler approach:
        // Treat each suit-only memory as potentially removing a tracked card with probability
        // Since we need deterministic behavior, we'll use a fractional count

        // Fractional reduction: on average, suit_only_count plays remove
        // (suit_only_count * 5 / 13) tracked cards from this suit
        let estimated_suit_only_removal = suit_only_count * (5.0 / 13.0);

        // Total known to be out: exact_played + estimated_suit_only_removal
        let estimated_removed = exact_played + estimated_suit_only_removal;

        // Remaining = total - what we have - what we estimate is out
        (TOTAL_TRACKED_PER_SUIT - my_count - estimated_removed).max(0.0)
    }

    // ---------- Opponent Modeling Helpers ----------

    /// Calculate average bid for an opponent over recent rounds.
    /// Returns None if insufficient data.
    fn opponent_avg_bid(
        history: Option<&GameHistory>,
        opponent_seat: u8,
        recent_rounds: usize,
    ) -> Option<f32> {
        let history = history?;
        let mut total = 0u32;
        let mut count = 0u32;

        for round in history.rounds.iter().rev().take(recent_rounds) {
            if let Some(bid) = round.bids.get(opponent_seat as usize).and_then(|&b| b) {
                total += bid as u32;
                count += 1;
            }
        }

        if count > 0 {
            Some(total as f32 / count as f32)
        } else {
            None
        }
    }

    // ---------- Context-Aware Adjustments ----------

    /// Adjust bid estimate based on score position.
    /// Returns multiplier: >1.0 if trailing (more aggressive), <1.0 if leading (more conservative).
    fn score_adjustment_multiplier(scores: &[i16; 4], my_seat: u8) -> f32 {
        let my_score = scores[my_seat as usize];
        let max_score = scores.iter().max().copied().unwrap_or(0);
        let min_score = scores.iter().min().copied().unwrap_or(0);

        if max_score == min_score {
            return 1.0; // All tied
        }

        let score_range = (max_score - min_score) as f32;
        let score_position = (my_score - min_score) as f32 / score_range;

        // If trailing (score_position < 0.5): more aggressive (up to 1.15)
        // If leading (score_position > 0.5): more conservative (down to 0.90)
        if score_position < 0.5 {
            1.0 + (0.5 - score_position) * 0.3 // 1.0 to 1.15
        } else {
            1.0 - (score_position - 0.5) * 0.2 // 1.0 to 0.90
        }
    }

    /// Determine if a card would win the current trick.
    /// Returns true if playing this card would beat the current winner (and likely win the trick).
    /// Note: This is an approximation - when we're not last to play, subsequent players might beat us.
    fn would_win_trick(
        card: Card,
        current_plays: &[(u8, Card)],
        lead_suit: Option<Suit>,
        trump: Trump,
        legal: &[Card],
    ) -> bool {
        match lead_suit {
            None => {
                // On lead: use heuristic
                // High trump cards are more likely to win
                // Simple heuristic: trump cards win, non-trump might win if high
                if Self::is_trump(&card, trump) {
                    return true;
                }
                // For non-trump on lead, assume high cards might win
                matches!(card.rank, Rank::Ace | Rank::King | Rank::Queen)
            }
            Some(lead) => {
                // Check if we're void in lead suit
                let have_lead_suit = legal.iter().any(|c| c.suit == lead);

                if !have_lead_suit {
                    // Void in lead suit: trump cards can ruff (winning), non-trumps discard (losing)
                    return Self::is_trump(&card, trump);
                }

                if current_plays.is_empty() {
                    // Shouldn't happen if lead_suit is Some
                    return false;
                }

                // Following suit: determine current winner from plays so far
                let mut current_winner = current_plays[0].1;
                for &(_, c) in &current_plays[1..] {
                    if card_beats(c, current_winner, lead, trump) {
                        current_winner = c;
                    }
                }

                // Check if our card beats the current winner
                card_beats(card, current_winner, lead, trump)
            }
        }
    }

    /// Score a card based on bid target alignment and heuristics.
    /// Returns (target_score, heuristic_score) where higher is better.
    /// target_score: positive for winning cards when we need wins, negative for losing cards when we need to avoid.
    /// heuristic_score: tie-breaker based on card quality (cheap wins, conserve highs).
    fn score_card_for_target(card: Card, ctx: &ScoringContext) -> (f32, f32) {
        // Determine if this card would win
        let wins =
            Self::would_win_trick(card, ctx.current_plays, ctx.lead_suit, ctx.trump, ctx.legal);

        // Target alignment score
        let target_score = if ctx.avoid {
            // Want to lose - prefer losing cards
            if wins {
                -10.0 * ctx.pressure.abs()
            } else {
                10.0 * ctx.pressure.abs()
            }
        } else if ctx.pressure > 0.0 {
            // Need wins - prefer winning cards
            if wins {
                10.0 * ctx.pressure
            } else {
                -10.0 * ctx.pressure
            }
        } else {
            // Neutral pressure - slight preference based on card quality
            // In endgame with low need, slightly prefer losing to avoid accidental wins
            if ctx.is_endgame && ctx.need > 0 && ctx.need < 3 {
                // Low but non-zero need in endgame: slight penalty for winning to avoid overtricks
                if wins {
                    -2.0
                } else {
                    0.0
                }
            } else {
                0.0
            }
        };

        // Heuristic score (tie-breaker)
        let mut heuristic_score = 0.0;

        // Prefer cheap wins (low cards when winning)
        if wins {
            // Lower rank cards are "cheaper" - give bonus
            let rank_value = match card.rank {
                Rank::Ace => 13.0,
                Rank::King => 12.0,
                Rank::Queen => 11.0,
                Rank::Jack => 10.0,
                Rank::Ten => 9.0,
                Rank::Nine => 8.0,
                Rank::Eight => 7.0,
                Rank::Seven => 6.0,
                Rank::Six => 5.0,
                Rank::Five => 4.0,
                Rank::Four => 3.0,
                Rank::Three => 2.0,
                Rank::Two => 1.0,
            };
            heuristic_score -= rank_value; // Negative = prefer lower cards
        } else {
            // When losing, prefer playing high cards to conserve low ones
            let rank_value = match card.rank {
                Rank::Ace => 13.0,
                Rank::King => 12.0,
                Rank::Queen => 11.0,
                Rank::Jack => 10.0,
                Rank::Ten => 9.0,
                Rank::Nine => 8.0,
                Rank::Eight => 7.0,
                Rank::Seven => 6.0,
                Rank::Six => 5.0,
                Rank::Five => 4.0,
                Rank::Four => 3.0,
                Rank::Three => 2.0,
                Rank::Two => 1.0,
            };
            heuristic_score += rank_value; // Positive = prefer higher cards when losing
        }

        // Bonus for conserving high cards when following suit
        if let Some(lead) = ctx.lead_suit {
            if card.suit == lead {
                let remaining_tracked = Self::count_tracked_cards(ctx.memory, lead, ctx.my_hand);
                if remaining_tracked > 10.0 && Self::is_tracked_rank(card.rank) {
                    // Many tracked cards remain - prefer conserving this high card
                    heuristic_score -= 5.0;
                }
            }
        }

        // Penalize leading suits where opponents are void (they'll trump and waste our lead)
        // Exception: if the suit is trump, opponents being void is GOOD (they can't trump it)
        if ctx.lead_suit.is_none() {
            let is_trump_suit = Self::is_trump(&card, ctx.trump);
            if !is_trump_suit {
                // We're on lead - avoid leading non-trump suits where opponents are void
                let void_count = ctx
                    .opponent_voids
                    .iter()
                    .enumerate()
                    .filter(|(seat, _)| *seat != ctx.my_seat as usize)
                    .map(|(_, voids)| if voids.contains(&card.suit) { 1 } else { 0 })
                    .sum::<usize>();
                if void_count > 0 {
                    // Penalize based on how many opponents are void (more void = bigger penalty)
                    heuristic_score -= 10.0 * void_count as f32;
                }
            }
        }

        (target_score, heuristic_score)
    }

    /// Endgame play selection for final tricks when exact bid conversion is critical.
    /// Returns a card chosen using strict endgame rules, or None if not in endgame.
    fn choose_endgame_card(
        legal: &[Card],
        current_plays: &[(u8, Card)],
        lead_suit: Option<Suit>,
        trump: Trump,
        need: u8,
        tricks_remaining: u8,
    ) -> Option<Card> {
        if need == 0 {
            // At or above bid: strongly avoid winning
            if lead_suit.is_none() {
                // Leading: prefer lowest card from short/weak suit, avoid trump
                let mut by_suit: HashMap<Suit, Vec<Card>> = HashMap::new();
                for &c in legal {
                    by_suit.entry(c.suit).or_default().push(c);
                }

                // Prefer non-trump suits, sorted by length (shorter = weaker)
                let mut suit_scores: Vec<(Suit, usize, bool)> = by_suit
                    .iter()
                    .map(|(&suit, cards)| {
                        let is_trump_suit = trump.try_into().ok() == Some(suit);
                        (suit, cards.len(), is_trump_suit)
                    })
                    .collect();

                // Sort: non-trump first, then by length (shortest first)
                suit_scores.sort_by(|a, b| {
                    match (a.2, b.2) {
                        (false, true) => std::cmp::Ordering::Less, // a (non-trump) < b (trump)
                        (true, false) => std::cmp::Ordering::Greater,
                        _ => a.1.cmp(&b.1), // Both same type, sort by length
                    }
                });

                // Try to find lowest card from shortest non-trump suit
                for (suit, _, _is_trump) in suit_scores {
                    if let Some(cards) = by_suit.get(&suit) {
                        if !cards.is_empty() {
                            let mut suit_cards = cards.clone();
                            suit_cards.sort(); // ascending
                            return Some(suit_cards[0]);
                        }
                    }
                }

                // Fallback: lowest legal card
                let mut sorted = legal.to_vec();
                sorted.sort();
                return Some(sorted[0]);
            } else if let Some(lead) = lead_suit {
                // Following: choose lowest legal card that does NOT win
                let current_winner = if !current_plays.is_empty() {
                    let mut winner = current_plays[0].1;
                    for &(_, c) in &current_plays[1..] {
                        if card_beats(c, winner, lead, trump) {
                            winner = c;
                        }
                    }
                    Some(winner)
                } else {
                    None
                };

                if let Some(winner) = current_winner {
                    // Find non-winning cards
                    let non_winners: Vec<Card> = legal
                        .iter()
                        .copied()
                        .filter(|&c| !card_beats(c, winner, lead, trump))
                        .collect();

                    if !non_winners.is_empty() {
                        let mut sorted = non_winners;
                        sorted.sort();
                        return Some(sorted[0]);
                    }
                }

                // All cards win or no current winner - choose lowest legal
                let mut sorted = legal.to_vec();
                sorted.sort();
                return Some(sorted[0]);
            } else {
                // No lead suit - shouldn't happen when following, but fallback
                let mut sorted = legal.to_vec();
                sorted.sort();
                return Some(sorted[0]);
            }
        } else if need >= tricks_remaining {
            // Must win out: strongly prefer winning
            if lead_suit.is_none() {
                // Leading: lead strongest likely winner (high trump or Ace)
                // Prefer high trump cards first
                let mut trumps: Vec<Card> = legal
                    .iter()
                    .copied()
                    .filter(|&c| Self::is_trump(&c, trump))
                    .collect();

                if !trumps.is_empty() {
                    trumps.sort(); // ascending
                    return Some(trumps[trumps.len() - 1]); // highest trump
                }

                // No trump, prefer Aces/Kings
                let mut high_cards: Vec<Card> = legal
                    .iter()
                    .copied()
                    .filter(|&c| matches!(c.rank, Rank::Ace | Rank::King))
                    .collect();

                if !high_cards.is_empty() {
                    high_cards.sort();
                    return Some(high_cards[high_cards.len() - 1]); // highest
                }

                // Fallback: highest legal card
                let mut sorted = legal.to_vec();
                sorted.sort();
                return Some(sorted[sorted.len() - 1]);
            } else if let Some(lead) = lead_suit {
                // Following: choose cheapest winning card if available
                let current_winner = if !current_plays.is_empty() {
                    let mut winner = current_plays[0].1;
                    for &(_, c) in &current_plays[1..] {
                        if card_beats(c, winner, lead, trump) {
                            winner = c;
                        }
                    }
                    Some(winner)
                } else {
                    None
                };

                if let Some(winner) = current_winner {
                    let winners: Vec<Card> = legal
                        .iter()
                        .copied()
                        .filter(|&c| card_beats(c, winner, lead, trump))
                        .collect();

                    if !winners.is_empty() {
                        let mut sorted = winners;
                        sorted.sort(); // ascending = cheapest first
                        return Some(sorted[0]);
                    }
                }

                // No winning cards - choose lowest legal
                let mut sorted = legal.to_vec();
                sorted.sort();
                return Some(sorted[0]);
            } else {
                // No lead suit - shouldn't happen when following, but fallback
                let mut sorted = legal.to_vec();
                sorted.sort();
                return Some(sorted[0]);
            }
        }

        // Neutral endgame - return None to use regular scoring
        None
    }

    /// Bid-target-driven play selection.
    /// Every play decision considers how many tricks we still NEED to hit our bid exactly.
    fn decide_card_with_target_policy(state: &CurrentRoundInfo, cx: &GameContext) -> Card {
        let legal = state.legal_plays();
        debug_assert!(!legal.is_empty(), "No legal plays provided to strategic AI");

        let current_plays = &state.current_trick_plays;
        let trump = state.trump.unwrap_or(Trump::NoTrumps);
        let lead = current_plays.first().map(|&(_, c)| c.suit);
        let memory = cx.round_memory();
        let my_hand = &state.hand;

        // Get tricks won so far from state (calculated from completed tricks)
        let tricks_won_so_far = state.tricks_won[state.player_seat as usize];

        // Detect opponent voids to avoid leading suits they can trump
        let opponent_voids = Self::detect_opponent_voids(memory, state.player_seat);

        // Compute bid target state
        let (need, avoid, pressure) = Self::bid_target_state(state, tricks_won_so_far);

        // Check if we're in endgame
        let tricks_remaining = state.hand_size.saturating_sub(state.trick_no - 1);
        if Self::is_endgame(tricks_remaining, state.hand_size) {
            // Try endgame-specific rules
            if let Some(endgame_card) = Self::choose_endgame_card(
                &legal,
                current_plays,
                lead,
                trump,
                need,
                tricks_remaining,
            ) {
                return endgame_card;
            }
            // If endgame rules returned None (neutral case), fall through to regular scoring
            // but increase weight of avoiding accidental wins when need is low
        }

        // Score all legal cards (with endgame awareness for neutral cases)
        let is_endgame = Self::is_endgame(tricks_remaining, state.hand_size);
        let scoring_ctx = ScoringContext {
            legal: &legal,
            current_plays,
            lead_suit: lead,
            trump,
            pressure,
            avoid,
            memory,
            my_hand,
            opponent_voids,
            my_seat: state.player_seat,
            is_endgame,
            need,
        };

        let mut scored: Vec<(Card, f32, f32)> = legal
            .iter()
            .map(|&card| {
                let (target, heuristic) = Self::score_card_for_target(card, &scoring_ctx);
                (card, target, heuristic)
            })
            .collect();

        // Sort by target score (primary), then heuristic (secondary)
        scored.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
        });
        scored.reverse(); // Highest scores first

        // If all cards have same target score, use heuristic tie-breaker
        // Otherwise, pick the best target-aligned card
        if let Some(best) = scored.first() {
            return best.0;
        }

        // Fallback: should never happen
        legal[0]
    }
}

impl AiPlayer for Strategic {
    fn choose_bid(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<u8, AiError> {
        let legal = state.legal_bids();
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal bids".into()));
        }

        // Base estimate from hand strength
        let mut est = Self::estimate_tricks(state);

        // Adjust based on exactness difficulty (how hard it is to hit exactly)
        // Higher difficulty → nudge bid toward safer value (lower)
        let exactness_factor = Self::compute_exactness_difficulty(state);
        est *= exactness_factor;

        // Adjust based on score position (more aggressive when trailing)
        let score_mult = Self::score_adjustment_multiplier(&state.scores, state.player_seat);
        est *= score_mult;

        // Adjust for hand size context (small hands require different strategy)
        // Small hands (2-3 cards) are more unpredictable, so be more conservative
        if state.hand_size <= 3 {
            est *= 0.85;
        } else if state.hand_size >= 10 {
            est *= 1.05; // Larger hands, be slightly more aggressive
        }

        // Learn from opponent patterns (optional adjustment based on opponent bids)
        // Check if opponents have already bid - if they bid high, we might need to be more conservative
        let existing_bids_sum: u8 = state.bids.iter().filter_map(|&b| b).sum();
        let bids_count = state.bids.iter().filter(|b| b.is_some()).count();

        if bids_count > 0 && bids_count < 4 {
            // Some opponents have bid - check if they're being aggressive
            let avg_existing_bid = existing_bids_sum as f32 / bids_count as f32;
            let expected_avg = state.hand_size as f32 / 4.0;
            if avg_existing_bid > expected_avg * 1.2 {
                // Opponents are bidding high - be slightly more conservative
                est *= 0.95;
            }
        }

        // Mild adjustment based on opponent historical bidding patterns
        // Look at what opponents typically bid in recent rounds (useful signal but not strong)
        let history = cx.game_history();
        let expected_bid = state.hand_size as f32 / 4.0;
        let mut opponent_bid_adjustment = 0.0;
        let mut opponent_count = 0;

        for opponent_seat in 0..4 {
            if opponent_seat != state.player_seat {
                if let Some(avg_bid) = Self::opponent_avg_bid(history, opponent_seat, 3) {
                    // Compare their historical average to expected - if they bid higher/lower, adjust slightly
                    let diff = avg_bid - expected_bid;
                    opponent_bid_adjustment += diff * 0.05; // Very mild impact (5% of difference)
                    opponent_count += 1;
                }
            }
        }

        if opponent_count > 0 {
            // Average the adjustment across opponents and apply mildly
            let avg_adjustment = opponent_bid_adjustment / opponent_count as f32;
            est += avg_adjustment;
        }

        let Some(bid) = Self::choose_bid_from_estimate(&legal, est) else {
            return Err(AiError::InvalidMove("No legal bids".into()));
        };
        Ok(bid)
    }

    fn choose_trump(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<Trump, AiError> {
        let legal = state.legal_trumps();
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal trumps".into()));
        }

        let hand = state.hand.clone();

        // Score each legal suit and pick the best
        let mut best: Option<(i32, Trump)> = None;
        for &t in &legal {
            let score = match t {
                Trump::Clubs => Self::trump_score_for_suit(&hand, Suit::Clubs),
                Trump::Diamonds => Self::trump_score_for_suit(&hand, Suit::Diamonds),
                Trump::Hearts => Self::trump_score_for_suit(&hand, Suit::Hearts),
                Trump::Spades => Self::trump_score_for_suit(&hand, Suit::Spades),
                Trump::NoTrumps => continue,
            };

            match best {
                None => best = Some((score, t)),
                Some((bs, _)) if score > bs => best = Some((score, t)),
                _ => {}
            }
        }

        // Get the best suit score (or 0 if no suits were evaluated)
        let best_suit_score = best.map(|(score, _)| score).unwrap_or(0);

        // Only consider No Trump if the best suit is weak and hand meets strict criteria
        if Self::prefer_no_trump(&hand, best_suit_score) {
            return Ok(Trump::NoTrumps);
        }

        // If we found a best suit, return it
        if let Some((_, t)) = best {
            Ok(t)
        } else {
            // If only NoTrumps was legal or no suit evaluated, default to first legal.
            Ok(legal[0])
        }
    }

    fn choose_play(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<Card, AiError> {
        let legal = state.legal_plays();
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal plays".into()));
        }

        // Use bid-target-driven play selection
        Ok(Self::decide_card_with_target_policy(state, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::player_view::{CurrentRoundInfo, GameHistory};
    use crate::domain::round_memory::{PlayMemory, TrickMemory};
    use crate::domain::state::Phase;

    fn build_test_state_with_memory(
        bids: [Option<u8>; 4],
        trick_no: u8,
        hand_size: u8,
        hand: Vec<Card>,
        current_trick_plays: Vec<(u8, Card)>,
        tricks_won_memory: Vec<TrickMemory>,
        trump: Option<Trump>,
    ) -> (CurrentRoundInfo, GameContext) {
        // Calculate tricks_won from memory (simulating what the backend does)
        let trump_val = trump.unwrap_or(Trump::NoTrumps);
        let mut tricks_won = [0u8; 4];
        for trick in &tricks_won_memory {
            // Only count if we have exact cards for all plays (can determine winner reliably)
            let mut all_exact = true;
            let mut plays_with_cards = Vec::new();
            for (seat, play_mem) in &trick.plays {
                match play_mem {
                    PlayMemory::Exact(card) => {
                        plays_with_cards.push((*seat, *card));
                    }
                    _ => {
                        all_exact = false;
                        break;
                    }
                }
            }
            if all_exact && plays_with_cards.len() == 4 {
                // Determine winner: first card is lead
                let lead_suit = plays_with_cards[0].1.suit;
                let mut winner = plays_with_cards[0];
                for &(seat, card) in &plays_with_cards[1..] {
                    if card_beats(card, winner.1, lead_suit, trump_val) {
                        winner = (seat, card);
                    }
                }
                if winner.0 < 4 {
                    tricks_won[winner.0 as usize] += 1;
                }
            }
        }

        let state = CurrentRoundInfo {
            game_id: 1,
            player_seat: 0,
            game_state: Phase::Trick { trick_no },
            current_round: 1,
            hand_size,
            dealer_pos: 0,
            hand,
            bids,
            trump,
            trick_no,
            current_trick_plays,
            scores: [0, 0, 0, 0],
            tricks_won,
            trick_leader: Some(0),
        };

        let memory = RoundMemory::new(crate::ai::memory::MemoryMode::Full, tricks_won_memory);

        let history = GameHistory { rounds: vec![] };

        let context = GameContext::new(1)
            .with_history(history)
            .with_round_memory(Some(memory));

        (state, context)
    }

    #[test]
    fn test_avoid_winning_when_need_zero() {
        // Scenario: We bid 2, already won 2 tricks, need 0 more
        // Should prefer losing cards over winning cards
        let hand = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace, // High card that could win
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two, // Low card that would lose
            },
        ];

        // Create memory showing we won 2 tricks (player 0 won both)
        let trick1 = TrickMemory::new(
            1,
            vec![
                (
                    0,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Ace,
                    }),
                ),
                (
                    1,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::King,
                    }),
                ),
                (
                    2,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Queen,
                    }),
                ),
                (
                    3,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Jack,
                    }),
                ),
            ],
        );
        let trick2 = TrickMemory::new(
            2,
            vec![
                (
                    1,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Two,
                    }),
                ),
                (
                    2,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Three,
                    }),
                ),
                (
                    3,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Four,
                    }),
                ),
                (
                    0,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Ace,
                    }),
                ),
            ],
        );

        // Following suit - player 3 led Hearts with a low card, now it's our turn (seat 0)
        let leader_card = Card {
            suit: Suit::Hearts,
            rank: Rank::Three,
        };

        let (mut state, context) = build_test_state_with_memory(
            [Some(2), Some(1), Some(1), Some(0)], // We (seat 0) bid 2
            3,                                    // Trick 3
            5,                                    // Hand size 5
            hand,
            vec![(3, leader_card)], // Player 3 led, now it's player 0's turn (0 = (3 + 1) % 4)
            vec![trick1, trick2],
            Some(Trump::Spades),
        );

        // Fix: Ensure trick_leader is set correctly for turn calculation
        state.trick_leader = Some(3);

        let ai = Strategic::new(None);
        let chosen = ai
            .choose_play(&state, &context)
            .expect("Should choose a card");

        // Should choose the losing card (Two) over the winning card (Ace)
        // Since we need 0 more tricks, we want to avoid winning
        assert_eq!(chosen.rank, Rank::Two);
        assert_eq!(chosen.suit, Suit::Hearts);
    }

    #[test]
    fn test_prefer_winning_when_critical_need() {
        // Scenario: We bid 3, won 0 tricks, on trick 3 of 5, need 3 more tricks
        // Should strongly prefer winning cards
        let hand = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two, // Low card that would lose
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace, // High card that would win
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::King,
            },
        ];

        // Player 3 led Hearts with a low card, now it's our turn (seat 0)
        let leader_card = Card {
            suit: Suit::Hearts,
            rank: Rank::Three,
        };

        let (mut state, context) = build_test_state_with_memory(
            [Some(3), Some(1), Some(1), Some(0)], // We (seat 0) bid 3
            3,                                    // Trick 3 of 5
            5,                                    // Hand size 5
            hand,
            vec![(3, leader_card)], // Player 3 led, now it's player 0's turn
            vec![],                 // No tricks won yet
            Some(Trump::Spades),
        );

        state.trick_leader = Some(3);

        let ai = Strategic::new(None);
        let chosen = ai
            .choose_play(&state, &context)
            .expect("Should choose a card");

        // Should choose the winning card (Ace) to get closer to our bid
        assert_eq!(chosen.rank, Rank::Ace);
        assert_eq!(chosen.suit, Suit::Hearts);
    }

    #[test]
    fn test_avoid_winning_when_over_bid() {
        // Scenario: We bid 1, already won 2 tricks (over bid)
        // Should try to lose
        let hand = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace, // Would win
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two, // Would lose
            },
        ];

        // Memory showing we won 2 tricks
        let trick1 = TrickMemory::new(
            1,
            vec![
                (
                    0,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Ace,
                    }),
                ),
                (
                    1,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::King,
                    }),
                ),
                (
                    2,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Queen,
                    }),
                ),
                (
                    3,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Jack,
                    }),
                ),
            ],
        );
        let trick2 = TrickMemory::new(
            2,
            vec![
                (
                    1,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Two,
                    }),
                ),
                (
                    2,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Three,
                    }),
                ),
                (
                    3,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Four,
                    }),
                ),
                (
                    0,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Ace,
                    }),
                ),
            ],
        );

        let leader_card = Card {
            suit: Suit::Hearts,
            rank: Rank::Three,
        };

        let (mut state, context) = build_test_state_with_memory(
            [Some(1), Some(1), Some(1), Some(0)], // We (seat 0) bid 1, but won 2
            3,
            5,
            hand,
            vec![(3, leader_card)], // Player 3 led, now it's player 0's turn
            vec![trick1, trick2],
            Some(Trump::Spades),
        );

        state.trick_leader = Some(3);

        let ai = Strategic::new(None);
        let chosen = ai
            .choose_play(&state, &context)
            .expect("Should choose a card");

        // Should choose the losing card since we're already over bid
        assert_eq!(chosen.rank, Rank::Two);
        assert_eq!(chosen.suit, Suit::Hearts);
    }

    fn build_test_state_for_bidding(
        hand: Vec<Card>,
        hand_size: u8,
        bids: [Option<u8>; 4],
        player_seat: u8,
    ) -> (CurrentRoundInfo, GameContext) {
        // Determine dealer position - if player_seat is 0, dealer is 3 (player_seat bids first)
        let dealer_pos = if player_seat == 0 { 3 } else { player_seat - 1 };

        let state = CurrentRoundInfo {
            game_id: 1,
            player_seat,
            game_state: Phase::Bidding,
            current_round: 1,
            hand_size,
            dealer_pos,
            hand,
            bids,
            trump: None,
            trick_no: 0,
            current_trick_plays: Vec::new(),
            scores: [0, 0, 0, 0],
            tricks_won: [0, 0, 0, 0],
            trick_leader: None,
        };

        let context = GameContext::new(1).with_history(GameHistory { rounds: vec![] });

        (state, context)
    }

    #[test]
    fn test_swingy_hand_produces_conservative_bid() {
        // Swingy hand: 7 cards in one suit, 2 voids, few A/K
        // This should produce a more conservative bid due to high exactness difficulty
        let hand = vec![
            // 7 Hearts
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Three,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Four,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Five,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Six,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Seven,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Eight,
            },
            // 2 Spades, 4 Diamonds
            Card {
                suit: Suit::Spades,
                rank: Rank::Nine,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Ten,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Jack,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Queen,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
        ];

        let (state, context) = build_test_state_for_bidding(
            hand,
            13,
            [None, None, None, None],
            0, // Player seat 0 (bids first after dealer 3)
        );

        let ai = Strategic::new(None);
        let bid_swingy = ai
            .choose_bid(&state, &context)
            .expect("Should produce a bid");

        // Compare with a balanced hand
        let balanced_hand = vec![
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Queen,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Jack,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ten,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Nine,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Nine,
            },
        ];

        let (balanced_state, balanced_context) = build_test_state_for_bidding(
            balanced_hand,
            13,
            [None, None, None, None],
            0, // Player seat 0
        );

        let bid_balanced = ai
            .choose_bid(&balanced_state, &balanced_context)
            .expect("Should produce a bid");

        // Swingy hand should bid lower (more conservative) than balanced hand
        // Note: exact values depend on estimate_tricks, but swingy should be <= balanced
        assert!(
            bid_swingy <= bid_balanced,
            "Swingy hand should produce conservative bid. Swingy: {}, Balanced: {}",
            bid_swingy,
            bid_balanced
        );
    }

    #[test]
    fn test_balanced_hand_preserves_bid() {
        // Balanced hand with good control (multiple A/K, balanced shape)
        // Should allow closer-to-estimate bids (low difficulty factor)
        let hand = vec![
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Queen,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Jack,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ten,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Nine,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Queen,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Eight,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Seven,
            },
        ];

        let (state, context) = build_test_state_for_bidding(
            hand,
            13,
            [None, None, None, None],
            0, // Player seat 0
        );

        let ai = Strategic::new(None);
        let bid = ai
            .choose_bid(&state, &context)
            .expect("Should produce a bid");

        // Balanced hand with control should produce a reasonable bid
        // (4-4-3-2 shape, multiple A/K = low difficulty)
        assert!(
            (3..=7).contains(&bid),
            "Balanced hand should produce reasonable bid: {}",
            bid
        );
    }

    #[test]
    fn test_void_hand_increases_difficulty() {
        // Hand with 2 voids (swingy, hard to control exactly)
        let hand = vec![
            // Only Hearts and Diamonds, voids in Clubs and Spades
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Queen,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Jack,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ten,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Nine,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Eight,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Queen,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Jack,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ten,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Nine,
            },
        ];

        let (state, context) = build_test_state_for_bidding(
            hand,
            13,
            [None, None, None, None],
            0, // Player seat 0
        );

        let ai = Strategic::new(None);
        let bid_voids = ai
            .choose_bid(&state, &context)
            .expect("Should produce a bid");

        // Compare with similar hand but no voids (just fewer cards in those suits)
        let no_void_hand = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Queen,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Jack,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ten,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Nine,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Eight,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Queen,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            }, // One card in Clubs
            Card {
                suit: Suit::Spades,
                rank: Rank::Two,
            }, // One card in Spades
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Jack,
            },
        ];

        let (no_void_state, no_void_context) = build_test_state_for_bidding(
            no_void_hand,
            13,
            [None, None, None, None],
            0, // Player seat 0
        );

        let bid_no_voids = ai
            .choose_bid(&no_void_state, &no_void_context)
            .expect("Should produce a bid");

        // Hand with voids should be more conservative (lower or equal bid)
        assert!(
            bid_voids <= bid_no_voids,
            "Hand with voids should be more conservative. With voids: {}, Without: {}",
            bid_voids,
            bid_no_voids
        );
    }

    #[test]
    fn test_endgame_need_zero_chooses_losing_line() {
        // Scenario: Endgame (3 tricks remaining), bid 3, won 3 tricks (need 0)
        // Should choose a losing card if available
        let hand = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            }, // Would win
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            }, // Would lose
        ];

        // Memory showing we won 3 tricks
        let trick1 = TrickMemory::new(
            1,
            vec![
                (
                    0,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Ace,
                    }),
                ),
                (
                    1,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::King,
                    }),
                ),
                (
                    2,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Queen,
                    }),
                ),
                (
                    3,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Jack,
                    }),
                ),
            ],
        );
        let trick2 = TrickMemory::new(
            2,
            vec![
                (
                    1,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Two,
                    }),
                ),
                (
                    2,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Three,
                    }),
                ),
                (
                    3,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Four,
                    }),
                ),
                (
                    0,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Ace,
                    }),
                ),
            ],
        );
        let trick3 = TrickMemory::new(
            3,
            vec![
                (
                    2,
                    PlayMemory::Exact(Card {
                        suit: Suit::Spades,
                        rank: Rank::Two,
                    }),
                ),
                (
                    3,
                    PlayMemory::Exact(Card {
                        suit: Suit::Spades,
                        rank: Rank::Three,
                    }),
                ),
                (
                    0,
                    PlayMemory::Exact(Card {
                        suit: Suit::Spades,
                        rank: Rank::Ace,
                    }),
                ),
                (
                    1,
                    PlayMemory::Exact(Card {
                        suit: Suit::Spades,
                        rank: Rank::King,
                    }),
                ),
            ],
        );

        // Following suit - player 3 led Hearts with a low card, now it's our turn (seat 0)
        let leader_card = Card {
            suit: Suit::Hearts,
            rank: Rank::Three,
        };

        // 5-card hand, trick 4 of 5 (1 remaining after this trick = endgame)
        let (mut state, context) = build_test_state_with_memory(
            [Some(3), Some(1), Some(1), Some(0)], // We (seat 0) bid 3, won 3
            4,                                    // Trick 4 of 5
            5,                                    // Hand size 5
            hand,
            vec![(3, leader_card)], // Player 3 led
            vec![trick1, trick2, trick3],
            Some(Trump::Spades),
        );

        state.trick_leader = Some(3);

        let ai = Strategic::new(None);
        let chosen = ai
            .choose_play(&state, &context)
            .expect("Should choose a card");

        // Should choose the losing card (Two) since we need 0 more tricks and we're in endgame
        assert_eq!(chosen.rank, Rank::Two);
        assert_eq!(chosen.suit, Suit::Hearts);
    }

    #[test]
    fn test_endgame_must_win_out_chooses_winning_line() {
        // Scenario: Endgame (3 tricks remaining), bid 5, won 2 tricks (need 3, must win out)
        // Should choose a winning card
        let hand = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            }, // Would lose
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            }, // Would win
            Card {
                suit: Suit::Clubs,
                rank: Rank::King,
            },
        ];

        // Memory showing we won 2 tricks
        let trick1 = TrickMemory::new(
            1,
            vec![
                (
                    0,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Ace,
                    }),
                ),
                (
                    1,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::King,
                    }),
                ),
                (
                    2,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Queen,
                    }),
                ),
                (
                    3,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Jack,
                    }),
                ),
            ],
        );
        let trick2 = TrickMemory::new(
            2,
            vec![
                (
                    1,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Two,
                    }),
                ),
                (
                    2,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Three,
                    }),
                ),
                (
                    3,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Four,
                    }),
                ),
                (
                    0,
                    PlayMemory::Exact(Card {
                        suit: Suit::Diamonds,
                        rank: Rank::Ace,
                    }),
                ),
            ],
        );

        // Following suit - player 3 led Hearts with a low card
        let leader_card = Card {
            suit: Suit::Hearts,
            rank: Rank::Three,
        };

        // 5-card hand, trick 3 of 5 (2 remaining after this trick = endgame)
        let (mut state, context) = build_test_state_with_memory(
            [Some(5), Some(0), Some(0), Some(0)], // We (seat 0) bid 5, won 2
            3,                                    // Trick 3 of 5
            5,                                    // Hand size 5
            hand,
            vec![(3, leader_card)],
            vec![trick1, trick2],
            Some(Trump::Spades),
        );

        state.trick_leader = Some(3);

        let ai = Strategic::new(None);
        let chosen = ai
            .choose_play(&state, &context)
            .expect("Should choose a card");

        // Should choose the winning card (Ace) since we need 3 more tricks and only 2 remain
        assert_eq!(chosen.rank, Rank::Ace);
        assert_eq!(chosen.suit, Suit::Hearts);
    }

    #[test]
    fn test_non_endgame_behavior_unchanged() {
        // Scenario: Not endgame (5 tricks remaining), bid 3, won 0 tricks
        // Should use regular scoring, not endgame rules
        let hand = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::King,
            },
        ];

        // Following suit
        let leader_card = Card {
            suit: Suit::Hearts,
            rank: Rank::Three,
        };

        // 10-card hand, trick 5 of 10 (5 remaining after this trick = not endgame for hand_size < 10)
        let (mut state, context) = build_test_state_with_memory(
            [Some(3), Some(2), Some(2), Some(1)], // We (seat 0) bid 3, won 0
            5,                                    // Trick 5 of 10
            10,                                   // Hand size 10 (endgame threshold is 4 remaining)
            hand,
            vec![(3, leader_card)],
            vec![], // No tricks won yet
            Some(Trump::Spades),
        );

        state.trick_leader = Some(3);

        let ai = Strategic::new(None);
        let chosen = ai
            .choose_play(&state, &context)
            .expect("Should choose a card");

        // Should use regular scoring (will choose based on need and pressure, not strict endgame rules)
        // This test just ensures it doesn't panic and returns a legal card
        assert_eq!(chosen.suit, Suit::Hearts); // Must follow suit
        assert!(state.hand.contains(&chosen)); // Should be from our hand
    }
}
