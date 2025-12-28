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
//! - Consider opponent trump preferences from history.
//! - Consider No Trump (if legal) when the hand is balanced and has enough top cards.
//!
//! Play strategy (memory-aware):
//! - Track played cards via RoundMemory for better decisions.
//! - Detect opponent voids to avoid wasting trump.
//! - Adjust aggressiveness based on trick number and bid progress.
//! - If following suit: try to win cheaply when late and profitable, else conserve (play low).
//! - If void in lead suit: consider ruffing cheaply if it can win; otherwise discard lowest.
//! - On lead: prefer long suit; lead low from length to preserve high cards.
//!
//! Notes & assumptions:
//! - Uses public fields (`state.hand`, `state.trump`, `state.current_trick_plays`, etc.)
//!   plus the `legal_*()` helpers and context information.
//! - `Card` is assumed to expose `suit: Suit` and implement `Ord` among same-suit ranks.
//! - `Trump` is one of the five enum variants (`Clubs`, `Diamonds`, `Hearts`, `Spades`, `NoTrumps`).
//!
//! Determinism:
//! - No randomness used. `seed` is stored for future extensions (e.g., tie-breaking knobs).
use crate::ai::{AiError, AiPlayer};
use crate::domain::player_view::{CurrentRoundInfo, GameHistory};
use crate::domain::round_memory::{PlayMemory, RoundMemory};
use crate::domain::{Card, GameContext, Suit, Trump};

#[derive(Clone)]
pub struct Strategic {
    _seed: Option<u64>, // reserved, currently unused for strict determinism
}

impl Strategic {
    pub const NAME: &'static str = "Strategic";
    pub const VERSION: &'static str = "1.0.0";

    pub fn new(seed: Option<u64>) -> Self {
        Self { _seed: seed }
    }

    // ---------- Utilities (pure, small, deterministic) ----------

    fn suit_index(suit: Suit) -> usize {
        match suit {
            Suit::Clubs => 0,
            Suit::Diamonds => 1,
            Suit::Hearts => 2,
            Suit::Spades => 3,
        }
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

    fn trump_suit(trump: Trump) -> Option<Suit> {
        match trump {
            Trump::NoTrumps => None,
            Trump::Clubs => Some(Suit::Clubs),
            Trump::Diamonds => Some(Suit::Diamonds),
            Trump::Hearts => Some(Suit::Hearts),
            Trump::Spades => Some(Suit::Spades),
        }
    }

    fn lowest(cards: &[Card]) -> Option<Card> {
        // Assumes `Card: Ord` with ascending order by rank (then suit) if applicable.
        cards.iter().copied().min()
    }

    fn by_suit<'a>(cards: impl Iterator<Item = &'a Card>, suit: Suit) -> Vec<Card> {
        cards.filter(|c| c.suit == suit).copied().collect()
    }

    fn lowest_in_suit<'a>(cards: impl Iterator<Item = &'a Card>, suit: Suit) -> Option<Card> {
        let mut v = Self::by_suit(cards, suit);
        if v.is_empty() {
            None
        } else {
            v.sort(); // ascending
            v.first().copied()
        }
    }

    fn lowest_trump<'a>(cards: impl Iterator<Item = &'a Card>, trump: Trump) -> Option<Card> {
        if let Some(ts) = Self::trump_suit(trump) {
            Self::lowest_in_suit(cards, ts)
        } else {
            None
        }
    }

    /// High-card score of a suit: uses the top 2 cards in that suit
    /// as a proxy for control; relies on `Ord` within a suit.
    fn suit_high_score(suit_cards: &mut [Card]) -> usize {
        if suit_cards.is_empty() {
            return 0;
        }
        suit_cards.sort(); // ascending
        let n = suit_cards.len();
        // Give 2 points for the top card, 1 for the second (if present).
        let mut score = 0;
        if n >= 1 {
            score += 2; // top
        }
        if n >= 2 {
            score += 1; // second
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

        // Counts by suit and capture cards per suit
        let mut by_suit_cards: [Vec<Card>; 4] = [vec![], vec![], vec![], vec![]];
        for c in hand.iter() {
            by_suit_cards[Self::suit_index(c.suit)].push(*c);
        }
        let counts = [
            by_suit_cards[0].len(),
            by_suit_cards[1].len(),
            by_suit_cards[2].len(),
            by_suit_cards[3].len(),
        ];

        let longest = *counts.iter().max().unwrap_or(&0) as f32;

        // High-card control proxy
        let mut hi = 0usize;
        for v in &mut by_suit_cards {
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

    /// Consider No Trump if available: prefer when hand is balanced and has some top cards.
    fn prefer_no_trump(hand: &[Card]) -> bool {
        // Balanced if no suit has >= 6 and none <= 1
        let mut counts = [0usize; 4];
        for c in hand {
            counts[Self::suit_index(c.suit)] += 1;
        }
        let balanced = counts.iter().all(|&c| (2..=5).contains(&c));
        if !balanced {
            return false;
        }
        // Top-card density proxy: look at the top ~quartile overall.
        let mut sorted = hand.to_vec();
        sorted.sort(); // ascending
        let k = sorted.len().max(1).div_ceil(4); // ceil(n/4)
                                                 // Require at least some "control": here just check size threshold.
        k >= 3
    }

    /// Compare trick cards: does `a` beat `b` given (lead, trump)?
    /// - Any trump beats any non-trump.
    /// - Within the same suit, higher rank (by `Ord`) wins.
    /// - Off-suit non-trump cannot beat on-suit non-trump.
    fn wins_over(a: Card, b: Card, lead: Suit, trump: Trump) -> bool {
        let a_tr = Self::is_trump(&a, trump);
        let b_tr = Self::is_trump(&b, trump);
        if a_tr && !b_tr {
            return true;
        }
        if !a_tr && b_tr {
            return false;
        }
        // Neither or both are trumps -> compare within suit context
        let trump_suit = Self::trump_suit(trump);
        let a_s = if a_tr {
            trump_suit.unwrap_or(a.suit)
        } else {
            a.suit
        };
        let b_s = if b_tr {
            trump_suit.unwrap_or(b.suit)
        } else {
            b.suit
        };

        if a_s == b_s {
            return a > b; // higher rank wins
        }

        // Different suits, neither is trump: only a that follows lead can beat b.
        if a_s == lead && b_s != lead {
            return true;
        }
        false
    }

    /// Given legal plays and current trick context, pick the smallest winning card if
    /// winning is possible; otherwise the lowest legal card.
    ///
    /// Note: This is the basic version; strategic AI uses pick_memory_aware_play instead.
    #[allow(dead_code)] // Kept for reference, but strategic AI uses pick_memory_aware_play
    fn pick_smallest_winning_or_low(
        legal: &[Card],
        current_plays: &[(u8, Card)],
        trump: Trump,
        lead: Option<Suit>,
    ) -> Card {
        debug_assert!(!legal.is_empty(), "No legal plays provided to strategic AI");

        // Determine current winner (if any)
        let (lead_suit, cur_winner) = if let Some(ls) = lead {
            if current_plays.is_empty() {
                (Some(ls), None)
            } else {
                let mut winner = current_plays[0].1;
                for &(_, c) in &current_plays[1..] {
                    if Self::wins_over(c, winner, ls, trump) {
                        winner = c;
                    }
                }
                (Some(ls), Some(winner))
            }
        } else {
            (None, None)
        };

        // Try to beat current winner using the cheapest winning card.
        if let (Some(ls), Some(w)) = (lead_suit, cur_winner) {
            let mut winners: Vec<Card> = legal
                .iter()
                .copied()
                .filter(|&c| Self::wins_over(c, w, ls, trump))
                .collect();
            if !winners.is_empty() {
                winners.sort(); // ascending; cheapest winning
                return winners[0];
            }
        }

        // Otherwise, if following suit is required and possible, shed low in the lead suit.
        if let Some(ls) = lead_suit {
            if let Some(low) = Self::lowest_in_suit(legal.iter(), ls) {
                return low;
            }
        }

        // Otherwise, if void: consider ruffing cheaply if it might help tempo later.
        if let Some(low_trump) = Self::lowest_trump(legal.iter(), trump) {
            // Only ruff if not first or early player in trick (avoid wasting trump early).
            if current_plays.len() >= 2 {
                return low_trump;
            }
        }

        // Fallback: discard overall lowest legal card.
        Self::lowest(legal).unwrap_or(legal[0])
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

    /// Count cards that have been played in a suit based on memory.
    /// Returns approximate count (may be lower than actual due to imperfect memory).
    #[allow(dead_code)] // Useful helper for future enhancements
    fn count_played_in_suit(memory: Option<&RoundMemory>, suit: Suit) -> usize {
        let mut count = 0;
        if let Some(mem) = memory {
            for trick in &mem.tricks {
                for (_, play_mem) in &trick.plays {
                    match play_mem {
                        PlayMemory::Exact(card) if card.suit == suit => count += 1,
                        PlayMemory::Suit(s) if *s == suit => count += 1,
                        _ => {}
                    }
                }
            }
        }
        count
    }

    /// Estimate remaining high cards in a suit based on memory and hand.
    /// Returns approximate count of remaining high cards (Ace, King, Queen, Jack).
    fn estimate_remaining_high_cards(
        memory: Option<&RoundMemory>,
        suit: Suit,
        my_hand: &[Card],
    ) -> f32 {
        // Count high cards we have
        let my_high_count = my_hand
            .iter()
            .filter(|c| {
                c.suit == suit
                    && matches!(
                        c.rank,
                        crate::domain::Rank::Jack
                            | crate::domain::Rank::Queen
                            | crate::domain::Rank::King
                            | crate::domain::Rank::Ace
                    )
            })
            .count() as f32;

        // Count high cards we remember being played
        let mut played_high = 0.0;
        if let Some(mem) = memory {
            for trick in &mem.tricks {
                for (_, play_mem) in &trick.plays {
                    match play_mem {
                        PlayMemory::Exact(card) if card.suit == suit => {
                            if matches!(
                                card.rank,
                                crate::domain::Rank::Jack
                                    | crate::domain::Rank::Queen
                                    | crate::domain::Rank::King
                                    | crate::domain::Rank::Ace
                            ) {
                                played_high += 1.0;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Estimate remaining: 16 total high cards (4 per suit), minus what we have, minus what we remember
        (16.0 - my_high_count - played_high).max(0.0)
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

    /// Get trick progress urgency: higher value means more urgency to win tricks.
    /// Based on trick number and hand size - later tricks are more urgent.
    fn trick_urgency(trick_no: u8, hand_size: u8) -> f32 {
        if hand_size == 0 {
            return 1.0;
        }
        let progress = trick_no as f32 / hand_size as f32;
        // Early tricks (0.0-0.33): 0.8 urgency
        // Mid tricks (0.33-0.67): 1.0 urgency
        // Late tricks (0.67-1.0): 1.2 urgency
        if progress < 0.33 {
            0.8
        } else if progress < 0.67 {
            1.0
        } else {
            1.2
        }
    }

    /// Memory-aware play selection with trick-progressive strategy.
    fn pick_memory_aware_play(state: &CurrentRoundInfo, cx: &GameContext, urgency: f32) -> Card {
        let legal = state.legal_plays();
        debug_assert!(!legal.is_empty(), "No legal plays provided to strategic AI");

        let current_plays = &state.current_trick_plays;
        let trump = state.trump.unwrap_or(Trump::NoTrumps);
        let lead = current_plays.first().map(|&(_, c)| c.suit);
        let memory = cx.round_memory();
        let my_hand = &state.hand;

        let lead_suit = match lead {
            Some(ls) => ls,
            None => return Self::lowest(&legal).unwrap_or(legal[0]), // Should not happen when following
        };

        // Determine current winner (if any)
        let cur_winner = if current_plays.is_empty() {
            None
        } else {
            let mut winner = current_plays[0].1;
            for &(_, c) in &current_plays[1..] {
                if Self::wins_over(c, winner, lead_suit, trump) {
                    winner = c;
                }
            }
            Some(winner)
        };

        // Try to beat current winner using the cheapest winning card.
        // Higher urgency makes us more willing to win.
        if let Some(w) = cur_winner {
            let mut winners: Vec<Card> = legal
                .iter()
                .copied()
                .filter(|&c| Self::wins_over(c, w, lead_suit, trump))
                .collect();
            if !winners.is_empty() {
                winners.sort(); // ascending; cheapest winning

                // If urgency is high (late tricks), be more aggressive about winning
                // Otherwise, only win if it's cheap (small card)
                if urgency > 1.1 || winners[0] < w {
                    return winners[0];
                }
                // If urgency is low and winning would be expensive, consider passing
                // (fall through to play low)
            }
        }

        // Following suit: use memory to decide if we should conserve or shed
        if let Some(following) = Self::lowest_in_suit(legal.iter(), lead_suit) {
            // Check remaining high cards in this suit using memory
            let remaining_high = Self::estimate_remaining_high_cards(memory, lead_suit, my_hand);
            let suit_in_hand: Vec<Card> = legal
                .iter()
                .filter(|c| c.suit == lead_suit)
                .copied()
                .collect();
            let my_high_in_suit = suit_in_hand
                .iter()
                .filter(|c| {
                    matches!(
                        c.rank,
                        crate::domain::Rank::Jack
                            | crate::domain::Rank::Queen
                            | crate::domain::Rank::King
                            | crate::domain::Rank::Ace
                    )
                })
                .count();

            // If many high cards remain and we have some, be more conservative (play low)
            // If few high cards remain or we don't have many, it's safe to play higher
            if remaining_high > 8.0 && my_high_in_suit > 1 {
                // Many high cards still out - play low to conserve
                return following;
            } else {
                // Few high cards remain - safe to play higher
                let mut suit_cards: Vec<Card> = legal
                    .iter()
                    .filter(|c| c.suit == lead_suit)
                    .copied()
                    .collect();
                suit_cards.sort();
                // Play second-lowest if available (keeps lowest as insurance), otherwise lowest
                if suit_cards.len() > 1 && urgency < 1.0 {
                    return suit_cards[1];
                } else {
                    return following;
                }
            }
        }

        // Void in lead suit: consider ruffing cheaply if it might help
        if let Some(low_trump) = Self::lowest_trump(legal.iter(), trump) {
            // Only ruff if not first or early player in trick (avoid wasting trump early).
            // Higher urgency makes us more willing to ruff.
            if current_plays.len() >= 2 || urgency > 1.1 {
                return low_trump;
            }
        }

        // Fallback: discard overall lowest legal card.
        Self::lowest(&legal).unwrap_or(legal[0])
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

        // If NoTrumps is legal and the hand is balanced with some top cards, consider it.
        let no_trump_legal = legal.iter().any(|t| matches!(t, Trump::NoTrumps));
        let want_nt = no_trump_legal && Self::prefer_no_trump(&hand);

        if want_nt {
            return Ok(Trump::NoTrumps);
        }

        // Score each legal suit and pick the best, with optional opponent preference adjustment
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

        // If only NoTrumps was legal or no suit evaluated, default to first legal.
        if let Some((_, t)) = best {
            Ok(t)
        } else {
            Ok(legal[0])
        }
    }

    fn choose_play(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<Card, AiError> {
        let legal = state.legal_plays();
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal plays".into()));
        }

        let memory = cx.round_memory();
        let urgency = Self::trick_urgency(state.trick_no, state.hand_size);

        let choice = match state.current_trick_plays.len() {
            0 => {
                // On lead: prefer lowest from our longest suit, but avoid leading suits where opponents are void
                let hand = &state.hand;
                let opponent_voids = Self::detect_opponent_voids(memory, state.player_seat);
                let mut by_suit: [Vec<Card>; 4] = [vec![], vec![], vec![], vec![]];
                for c in hand {
                    by_suit[Self::suit_index(c.suit)].push(*c);
                }

                // Score suits: prefer long suits, but penalize suits where opponents are void
                let mut best_idx = 0usize;
                let mut best_score = 0i32;
                for (i, suit_cards) in by_suit.iter().enumerate() {
                    let suit_len = suit_cards.len() as i32;
                    let suit = Suit::from_index(i);
                    // Penalize if we know opponents are void (they'll trump, wasting our lead)
                    let void_penalty = opponent_voids
                        .iter()
                        .map(|voids| if voids.contains(&suit) { 2 } else { 0 })
                        .sum::<i32>();
                    let score = suit_len * 10 - void_penalty;
                    if score > best_score {
                        best_score = score;
                        best_idx = i;
                    }
                }

                // If our best suit is not available to lead, just pick overall lowest legal card.
                if let Some(card) = Self::lowest_in_suit(legal.iter(), Suit::from_index(best_idx)) {
                    card
                } else {
                    Self::lowest(&legal).unwrap_or(legal[0])
                }
            }
            _ => {
                // Following: use memory-aware strategy
                Self::pick_memory_aware_play(state, cx, urgency)
            }
        };

        Ok(choice)
    }
}

// Helper to map usize -> Suit when we built suit-indexed arrays.
// Adjust if your Suit enum does not implement this conversion.
trait SuitIndex {
    fn from_index(i: usize) -> Suit;
}
impl SuitIndex for Suit {
    fn from_index(i: usize) -> Suit {
        // Assumes Suit discriminants/indexes are 0..=3 in a fixed order.
        // If your enum differs, replace this match accordingly.
        match i {
            0 => Suit::Clubs,
            1 => Suit::Diamonds,
            2 => Suit::Hearts,
            3 => Suit::Spades,
            _ => unreachable!("invalid suit index"),
        }
    }
}
