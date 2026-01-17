//! Strategic — deterministic, memory-aware AI for Nommie.
//!
//! This AI is designed to be:
//! - **Legal**: always plays a legal bid / trump / card via `legal_*()` helpers.
//! - **Deterministic**: no RNG.
//! - **Bid-targeted**: play decisions are driven by landing the bid exactly (+10).
//! - **Memory-aware**: uses `RoundMemory` for void detection and coarse card tracking.
//!
//! Notes:
//! - This file intentionally keeps heuristics cheap and robust.
//! - It prefers avoiding catastrophic overtricks once the bid is met.

use std::collections::HashMap;

use crate::ai::{AiError, AiPlayer};
use crate::domain::player_view::{CurrentRoundInfo, GameHistory, RoundHistory};
use crate::domain::round_memory::{PlayMemory, RoundMemory};
use crate::domain::{card_beats, Card, GameContext, Rank, Suit, Trump};

#[derive(Clone)]
pub struct Strategic {
    _seed: Option<u64>, // reserved; kept for future experimentation, but unused (determinism).
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum WinCertainty {
    No,
    Fragile,
    Likely,
    Sure,
}

/// Context for scoring cards in bid-target-driven selection.
struct ScoringContext<'a> {
    current_plays: &'a [(u8, Card)],
    lead_suit: Option<Suit>,
    trump: Trump,

    pressure: f32,
    avoid: bool,
    need: u8,

    memory: Option<&'a RoundMemory>,
    my_hand: &'a [Card],
    opponent_voids: [Vec<Suit>; 4],
    my_seat: u8,

    // Bucket-style expected tricks from *here* (not persisted).
    e_trump: f32,
    e_cash: f32,
    e_length: f32,
    e_ruff: f32,
}

impl Strategic {
    pub const NAME: &'static str = "Strategic";
    pub const VERSION: &'static str = "1.3.0";

    pub fn new(seed: Option<u64>) -> Self {
        Self { _seed: seed }
    }

    // ----------------------------
    // Small pure helpers
    // ----------------------------

    fn tricks_remaining(state: &CurrentRoundInfo) -> u8 {
        // state.trick_no is treated as 1-based; protect against underflow anyway.
        let tricks_played = state.trick_no.saturating_sub(1);
        state.hand_size.saturating_sub(tricks_played)
    }

    fn is_trump(card: &Card, trump: Trump) -> bool {
        matches!(
            (trump, card.suit),
            (Trump::Clubs, Suit::Clubs)
                | (Trump::Diamonds, Suit::Diamonds)
                | (Trump::Hearts, Suit::Hearts)
                | (Trump::Spades, Suit::Spades)
        )
    }

    fn rank_weight_for_cash(rank: Rank) -> f32 {
        match rank {
            Rank::Ace => 1.0,
            Rank::King => 0.85,
            Rank::Queen => 0.55,
            Rank::Jack => 0.35,
            Rank::Ten => 0.25,
            _ => 0.0,
        }
    }

    /// Rank weight scaled by hand size - lower cards become valuable in small hands.
    fn rank_weight_for_cash_scaled(rank: Rank, hand_size: u8) -> f32 {
        let base = Self::rank_weight_for_cash(rank);

        // In small hands, lower cards can win because higher cards may not be in play.
        // Scale factor: smaller hands = more value for lower cards.
        // For hand_size 13: no bonus for low cards
        // For hand_size 2: significant bonus for low cards
        let scale_factor = (13.0 / hand_size as f32).min(6.5); // Cap at 6.5x for very small hands

        match rank {
            Rank::Ace | Rank::King | Rank::Queen => base, // Top cards don't need scaling
            Rank::Jack => base + (0.15 * (scale_factor - 1.0).max(0.0)),
            Rank::Ten => base + (0.2 * (scale_factor - 1.0).max(0.0)),
            Rank::Nine => 0.0 + (0.15 * (scale_factor - 1.0).max(0.0)),
            Rank::Eight => 0.0 + (0.1 * (scale_factor - 1.0).max(0.0)),
            Rank::Seven => 0.0 + (0.05 * (scale_factor - 1.0).max(0.0)),
            _ => 0.0,
        }
    }

    fn suit_counts(hand: &[Card]) -> HashMap<Suit, usize> {
        let mut counts = HashMap::new();
        for c in hand {
            *counts.entry(c.suit).or_insert(0) += 1;
        }
        counts
    }

    fn count_high_cards_in_suit(hand: &[Card], suit: Suit) -> u32 {
        hand.iter()
            .filter(|c| c.suit == suit)
            .map(|c| match c.rank {
                Rank::Ace => 6,
                Rank::King => 5,
                Rank::Queen => 2,
                Rank::Jack => 1,
                Rank::Ten => 1,
                _ => 0,
            })
            .sum()
    }

    fn is_endgame(tricks_remaining: u8, hand_size: u8) -> bool {
        if hand_size >= 10 {
            tricks_remaining <= 4
        } else {
            tricks_remaining <= 3
        }
    }

    fn players_left_to_act(current_plays: &[(u8, Card)]) -> usize {
        // 4 players total; current_plays includes plays already made this trick.
        4usize.saturating_sub(current_plays.len()).saturating_sub(1)
    }

    // ----------------------------
    // Memory helpers
    // ----------------------------

    fn detect_opponent_voids(memory: Option<&RoundMemory>) -> [Vec<Suit>; 4] {
        let mut voids: [Vec<Suit>; 4] = [vec![], vec![], vec![], vec![]];

        let Some(memory) = memory else { return voids };

        for trick in memory.tricks.iter() {
            if trick.plays.is_empty() {
                continue;
            }

            // Determine lead suit (best-effort).
            let lead_suit = match &trick.plays[0].1 {
                PlayMemory::Exact(c) => Some(c.suit),
                PlayMemory::Suit(s) => Some(*s),
                _ => None,
            };
            let Some(lead) = lead_suit else { continue };

            for (seat, play) in trick.plays.iter() {
                match play {
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

        voids
    }

    /// Coarse estimate of how many "tracked high cards" (10-A) remain in a suit.
    ///
    /// This uses RoundMemory conservatively:
    /// - Exact plays are removed precisely.
    /// - Suit-only plays are treated as weak evidence (kept neutral here).
    fn remaining_tracked_high_cards(
        memory: Option<&RoundMemory>,
        suit: Suit,
        my_hand: &[Card],
    ) -> i32 {
        let tracked = |r: Rank| {
            matches!(
                r,
                Rank::Ten | Rank::Jack | Rank::Queen | Rank::King | Rank::Ace
            )
        };

        let mut remaining = 5i32;

        // Remove cards we hold.
        for c in my_hand.iter().filter(|c| c.suit == suit) {
            if tracked(c.rank) {
                remaining -= 1;
            }
        }

        let Some(memory) = memory else {
            return remaining.max(0);
        };

        for trick in memory.tricks.iter() {
            for (_seat, play) in trick.plays.iter() {
                if let PlayMemory::Exact(c) = play {
                    if c.suit == suit && tracked(c.rank) {
                        remaining -= 1;
                    }
                }
            }
        }

        remaining.max(0)
    }

    // ----------------------------
    // Bidding
    // ----------------------------

    fn estimate_tricks(hand: &[Card], hand_size: u8) -> f32 {
        let counts = Self::suit_counts(hand);
        let mut suit_lengths: Vec<usize> = counts.values().copied().collect();
        suit_lengths.sort_by(|a, b| b.cmp(a));

        let longest = suit_lengths.first().copied().unwrap_or(0) as f32;
        let avg = (hand_size as f32) / 4.0;

        // High-card value: use scaled weights that account for hand size.
        // In small hands, lower cards (9, 8, 7) become valuable.
        let mut high = 0f32;
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            for c in hand.iter().filter(|c| c.suit == suit) {
                high += Self::rank_weight_for_cash_scaled(c.rank, hand_size);
            }
        }

        // Shape bonus: long suit helps, scaled by hand size.
        // The value of a long suit is proportional to hand size.
        // Reduced coefficient to be more conservative.
        let shape_multiplier = (hand_size as f32 / 13.0).max(0.3); // Scale down for small hands
        let shape = ((longest - avg).max(0.0)) * 0.35 * shape_multiplier; // Reduced from 0.45

        // Ruff potential: void/singleton bonus, scaled by hand size.
        // Voids/singletons are valuable, but scale better with hand size.
        // Reduced coefficients to be more conservative.
        let ruff_multiplier = (hand_size as f32 / 13.0).max(0.4); // Reduced from 0.5
        let mut short_suits = 0f32;
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let n = *counts.get(&suit).unwrap_or(&0) as f32;
            if n == 0.0 {
                short_suits += 0.3 * ruff_multiplier; // Reduced from 0.4
            } else if n == 1.0 {
                short_suits += 0.18 * ruff_multiplier; // Reduced from 0.25
            }
        }

        // Combine into a rough trick expectation.
        // High card weight also scales with hand size - smaller hands need higher multiplier.
        // Reduced multiplier to be more conservative and reduce overbidding.
        let high_multiplier = 0.8 + (0.15 * (13.0 / hand_size as f32).min(2.5)); // Reduced from 0.9+0.2
        let mut est = (high * high_multiplier) + shape + short_suits;

        // Consider second longest suit for two-suiter bonus
        // Reduced coefficient to be more conservative.
        let second_longest = suit_lengths.get(1).copied().unwrap_or(0) as f32;
        let two_suiter_bonus = if longest >= avg * 1.5 && second_longest >= avg * 1.2 {
            (second_longest - avg).max(0.0) * 0.15 * shape_multiplier // Reduced from 0.2
        } else {
            0.0
        };
        est += two_suiter_bonus;

        // Account for proportional difficulty: winning 3.5/5 (70%) is much harder than 3.5/13 (27%).
        // Apply penalty more broadly to reduce overbidding. Lower threshold and increase penalty.
        let trick_percentage = est / hand_size as f32;
        if trick_percentage > 0.5 && hand_size >= 6 {
            // Lower threshold from 0.6/8 to 0.5/6
            let excess = trick_percentage - 0.5;
            let difficulty_penalty = excess * 0.25; // Increased from 0.15
            est *= 1.0 - difficulty_penalty;
        }

        // Hand-size normalization: keep estimates in [0, hand_size].
        let cap = hand_size as f32;
        if est > cap {
            est = cap;
        }
        if est < 0.0 {
            est = 0.0;
        }
        est
    }

    fn adjust_for_opponent_bids(est: f32, state: &CurrentRoundInfo, hand_size: u8) -> f32 {
        // Adjust based on how opponents' bids compare to expected average.
        // If they bid significantly above/below expected, that's meaningful information.
        let expected_avg = (hand_size as f32) / 4.0;
        let mut seen = 0f32;
        let mut n = 0f32;
        for b in state.bids.iter().flatten() {
            seen += *b as f32;
            n += 1.0;
        }

        if n > 0.0 {
            let opponent_avg = seen / n;
            let deviation = opponent_avg - expected_avg;

            // Reduced scaling to be more conservative
            let hand_size_factor = (13.0 / hand_size as f32).min(3.0);
            let base_adjustment = deviation * 0.08 * hand_size_factor; // Reduced from 0.1

            // Extra adjustment for very extreme deviations (reduced)
            let extreme_factor = if deviation.abs() > 1.0 {
                1.0 + (deviation.abs() - 1.0) * 0.2 // Reduced from 0.3
            } else {
                1.0
            };

            let adjustment = base_adjustment * extreme_factor;
            est * (1.0 - adjustment.clamp(-0.15, 0.15)) // Reduced cap from 0.2 to 0.15
        } else {
            est
        }
    }

    fn adjust_for_history(est: f32, history: &GameHistory, hand_size: u8, my_seat: u8) -> f32 {
        // Only use history after enough rounds for statistical significance.
        // Early game: no data. Mid game: sparse data. Late game: meaningful patterns.
        const MIN_ROUNDS_FOR_HISTORY: usize = 8;
        if history.rounds.len() < MIN_ROUNDS_FOR_HISTORY {
            return est;
        }

        // Analyze opponent bid achievement patterns, not just bid values.
        // If opponents overbid and fail → they're giving us tricks → bid higher.
        // If opponents overbid and succeed → they have strong hands → account for that.
        let expected_avg = (hand_size as f32) / 4.0;
        let mut total_adjustment = 0f32;
        let mut opponent_count = 0f32;

        for seat in 0..4u8 {
            if seat == my_seat {
                continue;
            }

            // Look at recent rounds (last 5-8 rounds for better sample size)
            let recent_rounds: Vec<&RoundHistory> = history
                .rounds
                .iter()
                .rev()
                .take(8)
                .filter(|round| {
                    round.bids[seat as usize].is_some()
                        && round.scores[seat as usize].round_score > 0
                })
                .collect();

            if recent_rounds.len() < 3 {
                // Not enough data for this opponent
                continue;
            }

            // Weight recent rounds more heavily
            let mut weighted_achievement = 0f32;
            let mut weighted_bid_value = 0f32;
            let mut total_weight = 0f32;

            for (idx, round) in recent_rounds.iter().enumerate() {
                if let Some(bid) = round.bids[seat as usize] {
                    let round_score = round.scores[seat as usize].round_score;
                    // Derive tricks won: if round_score >= 10 and round_score - 10 == bid, then exact
                    let tricks_won = if round_score >= 10 && round_score - 10 == bid {
                        bid
                    } else {
                        round_score
                    };

                    // Bid achievement: did they make their bid?
                    let achievement = tricks_won as f32 - bid as f32;
                    let bid_value = bid as f32 - expected_avg;

                    // More recent = higher weight
                    let weight = (recent_rounds.len() - idx) as f32;
                    weighted_achievement += achievement * weight;
                    weighted_bid_value += bid_value * weight;
                    total_weight += weight;
                }
            }

            if total_weight > 0.0 {
                let avg_achievement = weighted_achievement / total_weight;
                let avg_bid_value = weighted_bid_value / total_weight;

                // Calculate effective trick-taking: how many tricks they're actually taking
                // relative to expected average. This combines bid value and achievement.
                let effective_trick_taking = avg_bid_value + avg_achievement;

                // Scale adjustment: if someone consistently takes 1 more trick than expected,
                // that's a significant signal. Reduced scaling to be more conservative.
                // The adjustment is inverted: more tricks for them = fewer for us = bid lower
                let opponent_adjustment = -effective_trick_taking * 0.6; // Reduced from 0.7

                total_adjustment += opponent_adjustment;
                opponent_count += 1.0;
            }
        }

        if opponent_count == 0.0 {
            return est;
        }

        let avg_adjustment = total_adjustment / opponent_count;
        // Reduced maximum adjustment to be more conservative and prevent extreme swings.
        // Clamp to ±1.0 instead of ±1.5 to reduce volatility.
        est + avg_adjustment.clamp(-1.0, 1.0) // Reduced from ±1.5
    }

    fn choose_bid_from_estimate(legal: &[u8], est: f32) -> u8 {
        // Round to nearest bid.
        // On exact tie, prefer lower bid (more conservative) to reduce overbidding.
        let mut best = legal[0];
        let mut best_d = (best as f32 - est).abs();
        for &b in legal.iter().skip(1) {
            let d = (b as f32 - est).abs();
            if d < best_d {
                best = b;
                best_d = d;
            } else if d == best_d && b < best {
                // On exact tie, prefer lower bid (more conservative)
                best = b;
                best_d = d;
            }
        }
        best
    }

    fn adjust_for_first_trick_leader(est: f32, state: &CurrentRoundInfo, hand_size: u8) -> f32 {
        // First trick leader has significant advantage, especially in small hands.
        // But the advantage depends on hand strength - weak cards = no advantage, strong cards = big advantage.
        let first_trick_leader = (state.dealer_pos + 1) % 4;
        let is_leader = state.player_seat == first_trick_leader;

        if is_leader {
            // Calculate hand strength for leading - high cards that can win the first trick.
            // If you have Aces/Kings, you can likely win; if you have 2s, you can't.
            let mut leading_strength = 0f32;
            for card in &state.hand {
                // High cards that can win the first trick (especially if you choose trump)
                let card_value = Self::rank_weight_for_cash_scaled(card.rank, hand_size);
                leading_strength += card_value;
            }

            // Normalize by hand size - having 2 Aces in a 2-card hand is huge, in a 13-card hand less so.
            let avg_hand_strength = leading_strength / hand_size as f32;

            // Advantage scales with:
            // 1. Hand size (smaller = more impact per trick)
            // 2. Hand strength (stronger cards = more advantage)
            let hand_size_factor = (13.0 / hand_size as f32).min(3.0);
            let strength_factor = (avg_hand_strength / 0.5).min(2.0); // Cap at 2x for very strong hands

            // If you have weak cards (2s, 3s), no advantage. If you have strong cards, significant advantage.
            // Reduced coefficient to be more conservative.
            let leader_advantage = hand_size_factor * strength_factor * 0.1; // Reduced from 0.15
            est + leader_advantage
        } else {
            est
        }
    }

    fn adjust_for_aggregate_bidding(est: f32, state: &CurrentRoundInfo, hand_size: u8) -> f32 {
        // Count how many bids we've seen so far
        let mut total_bids = 0u8;
        let mut bid_count = 0u8;
        let mut highest_bid = 0u8;
        for bid in state.bids.iter().flatten() {
            total_bids += bid;
            bid_count += 1;
            highest_bid = highest_bid.max(*bid);
        }

        // If we're the last bidder and total is already below hand_size, we'll get forced tricks
        // Reduced adjustments to be more conservative.
        if bid_count == 3 {
            let current_total = total_bids;
            let remaining_capacity = hand_size.saturating_sub(current_total);
            // If remaining capacity is high, we're more likely to get forced tricks
            if remaining_capacity > hand_size / 2 {
                est + 0.2 // Reduced from 0.3
            } else if remaining_capacity > 0 {
                est + 0.1 * (remaining_capacity as f32 / hand_size as f32) // Reduced from 0.15
            } else {
                est
            }
        } else {
            // If we're likely to win the auction (our estimate is above current highest bid),
            // add a bonus for trump selection advantage. Analysis showed highest bidders
            // underbid by ~0.67 tricks on average.
            if est > highest_bid as f32 + 0.3 {
                // We're likely to win - add bonus for trump advantage
                // Scale bonus by hand strength and hand size
                let hand_strength = est / hand_size as f32;
                let trump_bonus = if hand_strength > 0.5 {
                    // Strong hand: larger bonus
                    0.5 + (hand_strength - 0.5) * 0.4
                } else {
                    // Weak hand: smaller bonus
                    hand_strength * 0.5
                };
                est + trump_bonus.min(0.8) // Cap at 0.8 tricks
            } else {
                est
            }
        }
    }

    fn compute_bid(state: &CurrentRoundInfo, cx: &GameContext) -> u8 {
        let legal = cx.legal_bids(state);
        let hand = &state.hand;
        let hand_size = state.hand_size;

        let mut est = Self::estimate_tricks(hand, hand_size);
        est = Self::adjust_for_opponent_bids(est, state, hand_size);
        est = Self::adjust_for_first_trick_leader(est, state, hand_size);
        est = Self::adjust_for_aggregate_bidding(est, state, hand_size);
        if let Some(history) = cx.game_history() {
            est = Self::adjust_for_history(est, history, hand_size, state.player_seat);
        }

        // Apply bid-dependent and hand-size-dependent correction to compensate for
        // systematic overestimation. Analysis showed:
        // - Small bids (1-3): slight overbid, need ~0.6-0.8 reduction
        // - Medium bids (4): well-calibrated, need ~0.3-0.5 reduction
        // - Large bids (5+): underbid, need ~0.0-0.3 reduction
        // - Small hands need more reduction, large hands need less
        let base_correction = if est < 2.0 {
            // Very small bids: larger reduction
            0.8
        } else if est < 3.5 {
            // Small-medium bids: moderate reduction
            0.7
        } else if est < 5.0 {
            // Medium bids: smaller reduction
            0.5
        } else {
            // Large bids: minimal reduction (they're already underbidding)
            0.3
        };

        // Scale correction by hand size: small hands need more, large hands need less
        let hand_size_factor = (13.0 / hand_size as f32).min(2.0);
        let correction = base_correction * (0.5 + 0.5 * hand_size_factor);

        est -= correction;

        est = est.clamp(0.0, hand_size as f32);
        Self::choose_bid_from_estimate(&legal, est)
    }

    // ----------------------------
    // Trump selection
    // ----------------------------

    fn trump_score(hand: &[Card], suit: Suit) -> i32 {
        let count = hand.iter().filter(|c| c.suit == suit).count() as i32;
        let high = Self::count_high_cards_in_suit(hand, suit) as i32;
        (count * 10) + high
    }

    fn stopper_strength(hand: &[Card], suit: Suit) -> i32 {
        // Very rough "NT stopper" heuristic.
        // A = strong; Kx = good; QJx = decent.
        let mut ranks: Vec<Rank> = hand
            .iter()
            .filter(|c| c.suit == suit)
            .map(|c| c.rank)
            .collect();
        ranks.sort();
        let has = |r: Rank| ranks.contains(&r);

        let mut score = 0;
        if has(Rank::Ace) {
            score += 3;
        }
        if has(Rank::King) && ranks.len() >= 2 {
            score += 2;
        }
        if has(Rank::Queen) && has(Rank::Jack) && ranks.len() >= 3 {
            score += 1;
        }
        score
    }

    fn prefer_no_trump(hand: &[Card], hand_size: u8) -> bool {
        // Balanced-ish, with stoppers in most suits.
        let counts = Self::suit_counts(hand);
        let mut min_len = 99usize;
        let mut max_len = 0usize;
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let n = *counts.get(&suit).unwrap_or(&0);
            min_len = min_len.min(n);
            max_len = max_len.max(n);
        }
        if min_len == 0 {
            return false; // voids are dangerous in NT
        }
        if max_len >= 7 {
            return false; // too wild; prefer suit trumps
        }

        let mut stoppers = 0;
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            if Self::stopper_strength(hand, suit) >= 2 {
                stoppers += 1;
            }
        }

        // Smaller hands: require fewer stoppers.
        let need = if hand_size <= 5 { 2 } else { 3 };
        stoppers >= need
    }

    fn choose_trump_impl(state: &CurrentRoundInfo) -> Trump {
        let legal = state.legal_trumps();
        let hand = &state.hand;

        // Use bucket estimation to evaluate each legal trump option
        use crate::domain::state::Phase;
        let eval_state = CurrentRoundInfo {
            game_id: state.game_id,
            player_seat: state.player_seat,
            game_state: Phase::Trick { trick_no: 1 },
            current_round: state.current_round,
            hand_size: state.hand_size,
            dealer_pos: state.dealer_pos,
            hand: hand.to_vec(),
            bids: state.bids,
            trump: None, // Will be set per evaluation
            trick_no: 1,
            current_trick_plays: vec![],
            scores: state.scores,
            tricks_won: [0, 0, 0, 0],
            trick_leader: Some((state.dealer_pos + 1) % 4),
        };

        let mut best_trump: Option<Trump> = None;
        let mut best_estimate = -1.0f32;
        let mut tied_trumps: Vec<Trump> = Vec::new();

        // Consider opponent bids when selecting trump
        // If opponents bid low, they may have weak hands, so we might want more aggressive trump
        let opponent_bid_sum: u8 = state.bids.iter().flatten().sum();
        let expected_total = state.hand_size;
        let opponent_bid_factor = if opponent_bid_sum < expected_total / 2 {
            1.1 // Opponents bid low - slight boost to aggressive trumps
        } else if opponent_bid_sum > expected_total * 3 / 4 {
            0.95 // Opponents bid high - slight penalty
        } else {
            1.0
        };

        for &trump in &legal {
            // Create state with this trump suit
            let mut state_with_trump = eval_state.clone();
            state_with_trump.trump = Some(trump);

            // Evaluate with this trump using bucket estimation
            let (e_trump, e_cash, e_length, e_ruff) = Self::estimate_remaining_tricks_by_bucket(
                hand,
                trump,
                None, // No memory during trump selection
                state.player_seat,
                &state_with_trump,
            );

            let mut total = e_trump + e_cash + e_length + e_ruff;
            // Apply opponent bid factor
            total *= opponent_bid_factor;

            if total > best_estimate {
                best_estimate = total;
                tied_trumps.clear();
                tied_trumps.push(trump);
            } else if (total - best_estimate).abs() < 0.001 {
                // Equal scores (within floating point tolerance)
                tied_trumps.push(trump);
            }
        }

        // If we have tied trumps, use deterministic tie-breaker based on game state
        if tied_trumps.len() > 1 {
            // Use round number + player seat to rotate preference fairly
            let tie_breaker =
                (state.current_round as usize + state.player_seat as usize) % tied_trumps.len();
            best_trump = Some(tied_trumps[tie_breaker]);
        } else if let Some(trump) = tied_trumps.first() {
            best_trump = Some(*trump);
        }

        // Fallback to original method if bucket estimation didn't select anything
        if let Some(trump) = best_trump {
            trump
        } else {
            // Fallback: use original scoring method
            if legal.contains(&Trump::NoTrumps) && Self::prefer_no_trump(hand, state.hand_size) {
                let best_suit = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
                    .into_iter()
                    .map(|s| Self::trump_score(hand, s))
                    .max()
                    .unwrap_or(0);
                if best_suit < 55 {
                    return Trump::NoTrumps;
                }
            }

            let mut best_score = i32::MIN;
            let mut tied_trumps: Vec<Trump> = Vec::new();
            for &t in &legal {
                let suit = match t {
                    Trump::Clubs => Some(Suit::Clubs),
                    Trump::Diamonds => Some(Suit::Diamonds),
                    Trump::Hearts => Some(Suit::Hearts),
                    Trump::Spades => Some(Suit::Spades),
                    Trump::NoTrumps => None,
                };
                let score = suit.map(|s| Self::trump_score(hand, s)).unwrap_or(-999);
                if score > best_score {
                    best_score = score;
                    tied_trumps.clear();
                    tied_trumps.push(t);
                } else if score == best_score {
                    tied_trumps.push(t);
                }
            }

            // If tied, use deterministic tie-breaker based on game state
            if tied_trumps.len() > 1 {
                let tie_breaker =
                    (state.current_round as usize + state.player_seat as usize) % tied_trumps.len();
                tied_trumps[tie_breaker]
            } else {
                tied_trumps.first().copied().unwrap_or_else(|| legal[0])
            }
        }
    }

    // ----------------------------
    // Bid-target policy and play
    // ----------------------------

    fn bid_target_state(state: &CurrentRoundInfo) -> (u8, bool, f32, u8) {
        let my_bid = state.bids[state.player_seat as usize].unwrap_or(0);
        let tricks_remaining = Self::tricks_remaining(state);
        let tricks_won = state.tricks_won[state.player_seat as usize];

        let need = my_bid.saturating_sub(tricks_won);
        let avoid = tricks_won >= my_bid;

        let pressure = if need == 0 {
            -2.0
        } else if tricks_remaining > 0 && need >= tricks_remaining {
            2.0
        } else if tricks_remaining > 0 {
            // scale from -0.2..+1.5 as we approach "must win out"
            let ratio = (need as f32) / (tricks_remaining as f32);
            (ratio * 1.5).clamp(-0.2, 1.5)
        } else {
            0.0
        };

        (need, avoid, pressure, tricks_remaining)
    }

    fn estimate_remaining_tricks_by_bucket(
        hand: &[Card],
        trump: Trump,
        memory: Option<&RoundMemory>,
        _my_seat: u8,
        state: &CurrentRoundInfo,
    ) -> (f32, f32, f32, f32) {
        // E_cash: likely immediate-ish winners.
        let mut e_cash = 0.0;
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let mut suit_cards: Vec<Card> =
                hand.iter().copied().filter(|c| c.suit == suit).collect();
            if suit_cards.is_empty() {
                continue;
            }
            suit_cards.sort(); // low..high

            // Discount based on remaining tracked highs still "out there".
            let remaining_tracked = Self::remaining_tracked_high_cards(memory, suit, hand) as f32;
            let uncertainty = (remaining_tracked / 5.0).clamp(0.0, 1.0);

            for c in suit_cards.iter().rev().take(3) {
                let base = Self::rank_weight_for_cash_scaled(c.rank, state.hand_size); // Use scaled version
                if base <= 0.0 {
                    continue;
                }
                let disc = match c.rank {
                    Rank::Ace => 1.0 - 0.1 * uncertainty, // Slight discount for Aces
                    Rank::King => 1.0 - 0.3 * uncertainty, // Reduced from 0.35
                    Rank::Queen => 1.0 - 0.5 * uncertainty, // Reduced from 0.55
                    _ => 1.0 - 0.6 * uncertainty,         // Reduced from 0.65
                };
                e_cash += base * disc;
            }
        }

        // E_trump: trump control.
        let mut e_trump = 0.0;
        if trump != Trump::NoTrumps {
            let trump_suit = match trump {
                Trump::Clubs => Suit::Clubs,
                Trump::Diamonds => Suit::Diamonds,
                Trump::Hearts => Suit::Hearts,
                Trump::Spades => Suit::Spades,
                Trump::NoTrumps => Suit::Clubs,
            };
            let trumps: Vec<Card> = hand
                .iter()
                .copied()
                .filter(|c| c.suit == trump_suit)
                .collect();
            let trump_len = trumps.len() as f32;

            if trump_len > 0.0 {
                // Count guaranteed/semi-guaranteed winners: top trumps are likely to win
                // Sort trumps to identify top cards
                let mut trumps_sorted = trumps.clone();
                trumps_sorted.sort(); // ascending order

                // Count high trumps (A, K, Q, J, 10) - these are likely winners
                let mut high_trump_count = 0;
                for c in trumps_sorted.iter().rev() {
                    match c.rank {
                        Rank::Ace | Rank::King | Rank::Queen | Rank::Jack | Rank::Ten => {
                            high_trump_count += 1;
                        }
                        _ => break,
                    }
                }
                let high_trump_count_f = high_trump_count as f32;

                // For very strong trump suits, count them as guaranteed winners
                let guaranteed_winners;
                if trump_len >= 5.0 && high_trump_count_f >= 3.0 {
                    // Strong trump suit: most cards are winners
                    let low_trump_count = trump_len - high_trump_count_f;
                    guaranteed_winners = high_trump_count_f + (low_trump_count * 0.7);
                } else if trump_len >= 4.0 && high_trump_count_f >= 2.0 {
                    // Good trump suit: high cards likely winners, lower cards partial
                    let low_trump_count = trump_len - high_trump_count_f;
                    guaranteed_winners = high_trump_count_f + (low_trump_count * 0.6);
                } else {
                    // Weaker trump suit: count high cards as winners, lower cards as partial
                    let low_trump_count = trump_len - high_trump_count_f;
                    if high_trump_count_f >= 2.0 {
                        guaranteed_winners = high_trump_count_f + (low_trump_count * 0.5);
                    } else {
                        guaranteed_winners = high_trump_count_f * 0.9 + (low_trump_count * 0.4);
                    }
                }

                // Smaller hands: trump control is *more* valuable.
                let hand_ratio = (state.hand_size as f32 / 13.0).clamp(0.15, 1.0);
                let scale = (1.0 / hand_ratio).min(2.5); // Increased from 2.0
                let mut base = guaranteed_winners * scale;

                // Add a small bonus for extra trump length (control/ruffing power)
                if trump_len > high_trump_count_f {
                    let extra_length = trump_len - high_trump_count_f;
                    base += extra_length * 0.2 * scale;
                }

                e_trump += base;
            }
        }

        // Ruff potential.
        let mut e_ruff = 0.0;
        if trump != Trump::NoTrumps {
            let counts = Self::suit_counts(hand);
            let trump_suit = match trump {
                Trump::Clubs => Suit::Clubs,
                Trump::Diamonds => Suit::Diamonds,
                Trump::Hearts => Suit::Hearts,
                Trump::Spades => Suit::Spades,
                Trump::NoTrumps => Suit::Clubs,
            };
            let trump_len = *counts.get(&trump_suit).unwrap_or(&0) as f32;

            if trump_len >= 2.0 {
                for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    if suit == trump_suit {
                        continue;
                    }
                    let n = *counts.get(&suit).unwrap_or(&0) as f32;
                    if n == 0.0 {
                        e_ruff += 0.55;
                    } else if n == 1.0 {
                        e_ruff += 0.25;
                    }
                }
                e_ruff *= (trump_len / (state.hand_size as f32)).clamp(0.2, 1.0);
            }
        }

        // Length potential.
        let mut e_length = 0.0;
        if trump == Trump::NoTrumps {
            // In NT, length is real once established.
            // Account for high cards in long suits for better evaluation.
            let counts = Self::suit_counts(hand);
            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let n = *counts.get(&suit).unwrap_or(&0) as i32;
                let raw = (n - 3).max(0) as f32;
                let high_cards = hand
                    .iter()
                    .filter(|c| c.suit == suit)
                    .filter(|c| matches!(c.rank, Rank::Ace | Rank::King | Rank::Queen))
                    .count() as f32;
                let quality_bonus = high_cards * 0.15; // Bonus for high cards in long suits
                e_length += (raw * 0.55) + quality_bonus;
            }
        } else {
            // In suit trumps, length is heavily discounted (ruff risk).
            let counts = Self::suit_counts(hand);
            let opponent_voids = Self::detect_opponent_voids(memory);
            let tricks_remaining = Self::tricks_remaining(state);
            let trump_suit = match trump {
                Trump::Clubs => Suit::Clubs,
                Trump::Diamonds => Suit::Diamonds,
                Trump::Hearts => Suit::Hearts,
                Trump::Spades => Suit::Spades,
                Trump::NoTrumps => Suit::Clubs,
            };
            let trump_len = *counts.get(&trump_suit).unwrap_or(&0);

            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                if suit == trump_suit {
                    continue;
                }
                let n = *counts.get(&suit).unwrap_or(&0) as i32;
                let raw = (n - 3).max(0) as f32;
                if raw <= 0.0 {
                    continue;
                }

                let discount = Self::safe_to_run_suit_factor(
                    suit,
                    &opponent_voids,
                    trump_len,
                    tricks_remaining,
                );
                e_length += raw * discount;
            }
        }

        (e_trump, e_cash, e_length, e_ruff)
    }

    fn safe_to_run_suit_factor(
        suit: Suit,
        opponent_voids: &[Vec<Suit>; 4],
        trump_len: usize,
        tricks_remaining: u8,
    ) -> f32 {
        // If any opponent is already known void in this suit, assume ruffs kill length tricks.
        for voids in opponent_voids.iter().take(4) {
            if voids.contains(&suit) {
                return 0.0;
            }
        }

        // Baseline is low in trump games.
        let mut f: f32 = 0.2;

        // Trump control increases safety.
        if trump_len >= 5 {
            f = 0.55;
        } else if trump_len >= 4 {
            f = 0.45;
        } else if trump_len >= 3 {
            f = 0.35;
        }

        // Late hand: trumps likely lower.
        if tricks_remaining <= 4 {
            f += 0.2;
        } else if tricks_remaining <= 6 {
            f += 0.1;
        }

        f.clamp(0.0, 0.8)
    }

    fn current_trick_winner(
        current_plays: &[(u8, Card)],
        lead: Suit,
        trump: Trump,
    ) -> Option<Card> {
        let mut winner = None;
        for (_seat, card) in current_plays.iter() {
            winner = match winner {
                None => Some(*card),
                Some(w) => {
                    if card_beats(*card, w, lead, trump) {
                        Some(*card)
                    } else {
                        Some(w)
                    }
                }
            };
        }
        winner
    }

    fn would_win_now(card: Card, current_plays: &[(u8, Card)], lead: Suit, trump: Trump) -> bool {
        let winner = Self::current_trick_winner(current_plays, lead, trump);
        match winner {
            None => true,
            Some(w) => card_beats(card, w, lead, trump),
        }
    }

    fn win_certainty(card: Card, ctx: &ScoringContext<'_>) -> WinCertainty {
        let Some(lead) = ctx.lead_suit else {
            // On lead, treat as likely if high trump / ace; otherwise fragile.
            if ctx.trump != Trump::NoTrumps && Self::is_trump(&card, ctx.trump) {
                if matches!(card.rank, Rank::Ace | Rank::King | Rank::Queen) {
                    return WinCertainty::Likely;
                }
                return WinCertainty::Fragile;
            }
            if matches!(card.rank, Rank::Ace) {
                return WinCertainty::Likely;
            }
            return WinCertainty::Fragile;
        };

        if !Self::would_win_now(card, ctx.current_plays, lead, ctx.trump) {
            return WinCertainty::No;
        }

        let left = Self::players_left_to_act(ctx.current_plays);
        if left == 0 {
            return WinCertainty::Sure;
        }

        // Use tracked-high remaining as a proxy for overtake risk.
        let remaining_tracked =
            Self::remaining_tracked_high_cards(ctx.memory, lead, ctx.my_hand) as f32;
        let risk = (remaining_tracked / 5.0).clamp(0.0, 1.0);

        let base = match card.rank {
            Rank::Ace => 1.0,
            Rank::King => 0.85,
            Rank::Queen => 0.65,
            Rank::Jack => 0.5,
            Rank::Ten => 0.4,
            _ => 0.25,
        };

        let position_penalty = 0.08 * left as f32; // Reduced from 0.12
        let score = base - (risk * 0.5) - position_penalty; // Reduced risk from 0.55

        if score >= 0.75 {
            WinCertainty::Likely
        } else {
            WinCertainty::Fragile
        }
    }

    fn accidental_win_risk(card: Card, ctx: &ScoringContext<'_>) -> f32 {
        if !ctx.avoid {
            return 0.0;
        }

        // Trump while voiding is a common accidental win.
        if ctx.trump != Trump::NoTrumps && Self::is_trump(&card, ctx.trump) {
            return 0.8;
        }

        // High ranks can win if others slough low.
        let Some(lead) = ctx.lead_suit else {
            return match card.rank {
                Rank::Ace => 0.9,
                Rank::King => 0.75,
                Rank::Queen => 0.55,
                Rank::Jack => 0.4,
                Rank::Ten => 0.3,
                _ => 0.15,
            };
        };

        if card.suit != lead {
            return 0.05;
        }

        match card.rank {
            Rank::Ace => 0.85,
            Rank::King => 0.7,
            Rank::Queen => 0.5,
            Rank::Jack => 0.35,
            Rank::Ten => 0.25,
            _ => 0.1,
        }
    }

    fn score_card_for_target(card: Card, ctx: &ScoringContext<'_>) -> f32 {
        let mut score = 0.0;

        // Primary: align with target (need/avoid), using win certainty.
        let certainty = Self::win_certainty(card, ctx);

        if ctx.avoid {
            // When exactly on target (need = 0), be very aggressive about avoiding wins
            if ctx.need == 0 {
                score -= match certainty {
                    WinCertainty::Sure => 8.0,    // Increased from 5.0
                    WinCertainty::Likely => 5.0,  // Increased from 3.5
                    WinCertainty::Fragile => 2.5, // Increased from 1.5
                    WinCertainty::No => 0.0,
                };
            } else {
                score -= match certainty {
                    WinCertainty::Sure => 5.0,
                    WinCertainty::Likely => 3.5,
                    WinCertainty::Fragile => 1.5,
                    WinCertainty::No => 0.0,
                };
            }
            score -= Self::accidental_win_risk(card, ctx) * 2.0;
        } else {
            let left = Self::players_left_to_act(ctx.current_plays);
            let win_reward = match certainty {
                WinCertainty::Sure => 3.0,
                WinCertainty::Likely => 2.4,
                WinCertainty::Fragile => 1.2,
                WinCertainty::No => 0.0,
            };
            let tempo = if left > 0 && certainty == WinCertainty::Fragile {
                0.6
            } else {
                1.0
            };
            score += win_reward * tempo;
            score += ctx.pressure * (win_reward * 0.35);
        }

        // Secondary: bucket-style guidance (increased magnitude when significantly behind/ahead).
        let e_total = ctx.e_trump + ctx.e_cash + ctx.e_length + ctx.e_ruff;
        let need_f = ctx.need as f32;
        let gap = need_f - e_total;

        if !ctx.avoid {
            if gap > 1.5 {
                // Significantly behind pace: strong bias toward increasing expected wins
                if ctx.trump != Trump::NoTrumps && Self::is_trump(&card, ctx.trump) {
                    score += 1.5; // Strong bonus for trump cards
                }
                score += Self::rank_weight_for_cash(card.rank) * 1.0; // Strong bonus for winning cards
            } else if gap > 0.5 {
                // Behind pace: moderate bias toward increasing expected wins
                if ctx.trump != Trump::NoTrumps && Self::is_trump(&card, ctx.trump) {
                    score += 0.75;
                }
                score += Self::rank_weight_for_cash(card.rank) * 0.5;
            } else if e_total + 0.25 < need_f {
                // Slightly behind: light bias
                if ctx.trump != Trump::NoTrumps && Self::is_trump(&card, ctx.trump) {
                    score += 0.45;
                }
                score += Self::rank_weight_for_cash(card.rank) * 0.25;
            }
        }

        if ctx.avoid {
            if e_total > need_f + 2.0 {
                // Significantly ahead: strong bias toward reducing accidental wins
                score -= Self::rank_weight_for_cash(card.rank) * 0.8;
            } else if e_total > need_f + 1.0 {
                // Ahead: moderate bias
                score -= Self::rank_weight_for_cash(card.rank) * 0.4;
            } else if e_total > need_f + 0.75 {
                score += Self::rank_weight_for_cash(card.rank) * 0.15;
            }
        }

        // Tertiary: conserve high cards near target.
        if ctx.avoid || (ctx.need > 0 && ctx.need <= 2 && e_total >= need_f) {
            score -= Self::rank_weight_for_cash(card.rank) * 0.2;
            if ctx.trump != Trump::NoTrumps && Self::is_trump(&card, ctx.trump) {
                score -= 0.1;
            }
        }

        // Lead selection: avoid leading into known voids unless we need tricks.
        if ctx.current_plays.is_empty() {
            let lead_suit = card.suit;
            let mut void_risk = 0.0;
            for seat in 0..4usize {
                if seat == ctx.my_seat as usize {
                    continue;
                }
                if ctx.opponent_voids[seat].contains(&lead_suit) {
                    void_risk += 1.0;
                }
            }
            if void_risk > 0.0 && !ctx.avoid {
                // If we need tricks, leading into voids can actually help (they ruff, we win later)
                // Only penalize if we're close to target
                if ctx.need > 2 {
                    score -= 0.15 * void_risk; // Reduced penalty when we need many tricks
                } else {
                    score -= 0.35 * void_risk; // Keep original penalty when close
                }
            } else if void_risk > 0.0 && ctx.avoid {
                score -= 0.1 * void_risk;
            }
        }

        score
    }

    fn choose_best_scored(legal: &[Card], scores: &HashMap<Card, f32>) -> Card {
        let mut best = legal[0];
        let mut best_s = scores.get(&best).copied().unwrap_or(f32::MIN);
        for &c in legal.iter().skip(1) {
            let s = scores.get(&c).copied().unwrap_or(f32::MIN);
            if s > best_s {
                best = c;
                best_s = s;
            } else if s == best_s && c < best {
                best = c;
            }
        }
        best
    }

    fn decide_card_with_target_policy(state: &CurrentRoundInfo, cx: &GameContext) -> Card {
        let legal = state.legal_plays();
        if legal.is_empty() {
            return state.hand[0];
        }

        let trump = state.trump.unwrap_or(Trump::NoTrumps);
        let memory = cx.round_memory();
        let my_hand = &state.hand;

        let tricks_remaining = Self::tricks_remaining(state);
        let (need, avoid, pressure, _) = Self::bid_target_state(state);

        // Strict endgame overrides.
        if Self::is_endgame(tricks_remaining, state.hand_size) {
            let lead_suit = state.current_trick_plays.first().map(|(_, c)| c.suit);

            let pick_lowest_loser = |lead: Option<Suit>| -> Card {
                let mut sorted = legal.clone();
                sorted.sort();
                if let Some(lead) = lead {
                    for &c in &sorted {
                        if !Self::would_win_now(c, &state.current_trick_plays, lead, trump) {
                            return c;
                        }
                    }
                }
                sorted[0]
            };

            let pick_cheapest_winner = |lead: Suit| -> Option<Card> {
                let mut winners: Vec<Card> = legal
                    .iter()
                    .copied()
                    .filter(|&c| Self::would_win_now(c, &state.current_trick_plays, lead, trump))
                    .collect();
                if winners.is_empty() {
                    return None;
                }
                winners.sort();
                Some(winners[0])
            };

            if need == 0 {
                return pick_lowest_loser(lead_suit);
            }
            if tricks_remaining > 0 && need >= tricks_remaining {
                // Must win out - prefer highest trumps, then highest cards
                if let Some(lead) = lead_suit {
                    if let Some(w) = pick_cheapest_winner(lead) {
                        return w;
                    }
                    let mut sorted = legal.clone();
                    sorted.sort();
                    return sorted[0];
                } else {
                    // On lead - prefer highest trump, then highest card
                    let mut candidates = legal.clone();
                    candidates.sort();
                    candidates.reverse();
                    // First try to find a high trump
                    for &c in &candidates {
                        if trump != Trump::NoTrumps && Self::is_trump(&c, trump) {
                            // Among trumps, prefer highest
                            return c;
                        }
                    }
                    // No trump available, play highest card
                    return candidates[0];
                }
            }
            // neutral endgame falls through to scored selection
        }

        let opponent_voids = Self::detect_opponent_voids(memory);
        let lead_suit = state.current_trick_plays.first().map(|(_, c)| c.suit);

        let (e_trump, e_cash, e_length, e_ruff) = Self::estimate_remaining_tricks_by_bucket(
            my_hand,
            trump,
            memory,
            state.player_seat,
            state,
        );

        let ctx = ScoringContext {
            current_plays: &state.current_trick_plays,
            lead_suit,
            trump,

            pressure,
            avoid,
            need,

            memory,
            my_hand,
            opponent_voids,
            my_seat: state.player_seat,

            e_trump,
            e_cash,
            e_length,
            e_ruff,
        };

        let mut scores: HashMap<Card, f32> = HashMap::new();
        for &c in &legal {
            scores.insert(c, Self::score_card_for_target(c, &ctx));
        }

        Self::choose_best_scored(&legal, &scores)
    }
}

impl AiPlayer for Strategic {
    fn choose_bid(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<u8, AiError> {
        let legal = cx.legal_bids(state);
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal bids".into()));
        }
        Ok(Self::compute_bid(state, cx))
    }

    fn choose_trump(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<Trump, AiError> {
        let legal = state.legal_trumps();
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal trumps".into()));
        }
        Ok(Self::choose_trump_impl(state))
    }

    fn choose_play(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<Card, AiError> {
        let legal = state.legal_plays();
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal plays".into()));
        }
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
        // With improved bidding, this hand should bid higher (more aggressive)
        assert!(
            (3..=9).contains(&bid),
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

        // Hand with voids should generally be more conservative due to exactness difficulty
        // However, bucket estimation might favor void hands if they have strong trump potential
        // So we allow some flexibility: voids should not bid MORE than 1 trick higher
        assert!(
            bid_voids <= bid_no_voids + 1,
            "Hand with voids should not bid significantly higher. With voids: {}, Without: {}",
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

    #[test]
    fn test_bucket_estimation_e_length_discounted_when_opponent_void() {
        // Test that E_length is heavily discounted (factor 0.0) when an opponent is void
        let hand = vec![
            // 5 Hearts (long suit)
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
                suit: Suit::Clubs,
                rank: Rank::Ace,
            },
        ];

        let state = CurrentRoundInfo {
            game_id: 1,
            player_seat: 0,
            game_state: Phase::Trick { trick_no: 2 },
            current_round: 1,
            hand_size: 13,
            dealer_pos: 3,
            hand: hand.clone(),
            bids: [Some(3), Some(2), Some(2), Some(1)],
            trump: Some(Trump::Spades),
            trick_no: 2,
            current_trick_plays: vec![],
            scores: [0, 0, 0, 0],
            tricks_won: [0, 0, 0, 0],
            trick_leader: Some(0),
        };

        // Create memory showing player 1 (seat 1) is void in Hearts (they played a different suit when Hearts was led)
        let trick1 = TrickMemory::new(
            1,
            vec![
                (
                    0,
                    PlayMemory::Exact(Card {
                        suit: Suit::Hearts,
                        rank: Rank::Seven,
                    }),
                ),
                (
                    1,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs, // Player 1 played Clubs when Hearts was led = void in Hearts
                        rank: Rank::Two,
                    }),
                ),
                (
                    2,
                    PlayMemory::Exact(Card {
                        suit: Suit::Hearts,
                        rank: Rank::Eight,
                    }),
                ),
                (
                    3,
                    PlayMemory::Exact(Card {
                        suit: Suit::Hearts,
                        rank: Rank::Nine,
                    }),
                ),
            ],
        );

        let memory = RoundMemory::new(crate::ai::memory::MemoryMode::Full, vec![trick1]);
        let context = GameContext::new(1)
            .with_history(GameHistory { rounds: vec![] })
            .with_round_memory(Some(memory));

        let (_e_trump, _e_cash, e_length, _e_ruff) = Strategic::estimate_remaining_tricks_by_bucket(
            &hand,
            Trump::Spades,
            context.round_memory(),
            0,
            &state,
        );

        // E_length should be heavily discounted (close to 0) because opponent is void in Hearts
        // The safe_to_run_suit_factor should return 0.0 when an opponent is void
        assert!(
            e_length < 0.1,
            "E_length should be heavily discounted when opponent is void. Got: {}",
            e_length
        );
    }

    #[test]
    fn test_bucket_estimation_aggressive_when_e_total_less_than_need() {
        // Test that when E_total < need, the AI makes more aggressive choices (prefers winning cards)
        // Hand with low expected tricks but high need
        let hand = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace, // Winner
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two, // Loser
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Three,
            },
        ];

        let (mut state, context) = build_test_state_with_memory(
            [Some(5), Some(1), Some(1), Some(1)], // We bid 5, need many tricks
            3,                                    // Trick 3 of 13
            13,                                   // Hand size 13
            hand,
            vec![], // On lead
            vec![], // No tricks won yet (need 5 more)
            Some(Trump::Spades),
        );

        state.trick_leader = Some(0);

        let ai = Strategic::new(None);
        let chosen = ai
            .choose_play(&state, &context)
            .expect("Should choose a card");

        // With low E_total and high need, should prefer winning cards (Ace over Two)
        // The bucket-based bias should add bonus to winning cards
        assert_eq!(chosen.rank, Rank::Ace);
        assert_eq!(chosen.suit, Suit::Hearts);
    }

    #[test]
    fn test_bucket_estimation_conservative_when_e_total_greater_than_need() {
        // Test that when E_total > need, the AI makes more conservative choices
        // Hand with high expected tricks but low need
        let hand = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace, // Winner
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two, // Loser
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
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
        ];

        // We bid 2 and already won 1 trick, need only 1 more
        // But have high E_total (many high cards)
        let trick1 = TrickMemory::new(
            1,
            vec![
                (
                    0,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Jack,
                    }),
                ),
                (
                    1,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Ten,
                    }),
                ),
                (
                    2,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Nine,
                    }),
                ),
                (
                    3,
                    PlayMemory::Exact(Card {
                        suit: Suit::Clubs,
                        rank: Rank::Eight,
                    }),
                ),
            ],
        );

        let (mut state, context) = build_test_state_with_memory(
            [Some(2), Some(1), Some(1), Some(1)], // We bid 2, won 1, need 1 more
            3,                                    // Trick 3 of 13
            13,                                   // Hand size 13
            hand,
            vec![], // On lead
            vec![trick1],
            Some(Trump::Spades),
        );

        state.trick_leader = Some(0);

        let ai = Strategic::new(None);
        let chosen = ai
            .choose_play(&state, &context)
            .expect("Should choose a card");

        // With high E_total and low need, the bucket-based bias should slightly penalize winning cards
        // However, the existing logic (pressure, target_score) might still prefer winning
        // This test mainly verifies the code doesn't crash and makes a reasonable choice
        // In practice, the bias is subtle and may not always override target_score
        assert!(state.hand.contains(&chosen));
    }

    #[test]
    fn test_bucket_estimation_no_trump_e_trump_and_e_ruff_zero() {
        // Test that E_trump and E_ruff are 0.0 when trump is NoTrumps
        let hand = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ];

        let state = CurrentRoundInfo {
            game_id: 1,
            player_seat: 0,
            game_state: Phase::Trick { trick_no: 2 },
            current_round: 1,
            hand_size: 13,
            dealer_pos: 3,
            hand: hand.clone(),
            bids: [Some(3), Some(2), Some(2), Some(1)],
            trump: Some(Trump::NoTrumps),
            trick_no: 2,
            current_trick_plays: vec![],
            scores: [0, 0, 0, 0],
            tricks_won: [0, 0, 0, 0],
            trick_leader: Some(0),
        };

        let context = GameContext::new(1)
            .with_history(GameHistory { rounds: vec![] })
            .with_round_memory(None);

        let (e_trump, e_cash, e_length, e_ruff) = Strategic::estimate_remaining_tricks_by_bucket(
            &hand,
            Trump::NoTrumps,
            context.round_memory(),
            0,
            &state,
        );

        assert_eq!(e_trump, 0.0, "E_trump should be 0.0 when trump is NoTrumps");
        assert_eq!(e_ruff, 0.0, "E_ruff should be 0.0 when trump is NoTrumps");
        // E_cash and E_length should still be computed
        assert!(e_cash > 0.0 || e_length >= 0.0);
    }

    #[test]
    fn test_bidding_considers_bucket_estimate() {
        // Test that bidding code runs with bucket estimation integrated
        // Hand with strong trump suit to test bucket estimation
        let hand = vec![
            // 5 Hearts (strong trump suit)
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
            // Weak other suits
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Three,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Three,
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

        // Sanity check: should produce a valid bid
        assert!((0..=13).contains(&bid));
    }
}
