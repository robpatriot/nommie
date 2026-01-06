//! Heuristic — a stronger, deterministic baseline AI for Nommie.
//!
//! Goals:
//! - Stay 100% legal using the engine’s `legal_*()` helpers.
//! - Be deterministic (no RNG), but materially stronger than a pure random/lowest-card AI.
//!
//! Bidding (simple EV-style):
//! - Estimate trick potential from suit shape and top-card density.
//! - Choose the closest legal bid to the estimate (bias slightly downward to reduce busts).
//!
//! Trump selection:
//! - Prefer the suit with the best (count, high-card) tuple.
//! - Consider No Trump (if legal) when the hand is balanced and has enough top cards.
//!
//! Play strategy (individual game, no partners):
//! - If following suit: try to win cheaply when late and profitable, else conserve (play low).
//! - If void in lead suit: consider ruffing cheaply if it can win; otherwise discard lowest.
//! - On lead: prefer long suit; lead low from length to preserve high cards.
//! - Bid-aware: avoid winning when tricks_won >= bid; prefer winning when behind bid.
//!
//! Notes & assumptions:
//! - Uses only public, common-sense fields (`state.hand`, `state.trump`, `state.current_trick_plays`)
//!   plus the `legal_*()` helpers; adjust import paths or names as needed.
//! - `Card` is assumed to expose `suit: Suit` and implement `Ord` among same-suit ranks.
//! - `Trump` is one of the five enum variants (`Clubs`, `Diamonds`, `Hearts`, `Spades`, `NoTrumps`).
//!
//! Determinism:
//! - No randomness used. `seed` is stored for future extensions (e.g., tie-breaking knobs).
use std::collections::HashMap;

use crate::ai::{AiError, AiPlayer};
use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::{card_beats, Card, GameContext, Suit, Trump};

#[derive(Clone)]
pub struct Heuristic {
    _seed: Option<u64>, // reserved, currently unused for strict determinism
}

impl Heuristic {
    pub const NAME: &'static str = "Heuristic";
    pub const VERSION: &'static str = "1.0.0";

    pub fn new(seed: Option<u64>) -> Self {
        Self { _seed: seed }
    }

    // ---------- Utilities (pure, small, deterministic) ----------

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
        if let Ok(ts) = trump.try_into() {
            Self::lowest_in_suit(cards, ts)
        } else {
            None
        }
    }

    /// Heuristic "high-card" score of a suit: uses the top 2 cards in that suit
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
        let mut by_suit_counts: HashMap<Suit, usize> = HashMap::new();
        for c in hand {
            *by_suit_counts.entry(c.suit).or_insert(0) += 1;
        }
        let balanced = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
            .iter()
            .all(|&suit| {
                let count = by_suit_counts.get(&suit).copied().unwrap_or(0);
                (2..=5).contains(&count)
            });
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

    /// Given legal plays and current trick context, pick the smallest winning card if
    /// winning is possible; otherwise the lowest legal card.
    fn pick_smallest_winning_or_low(
        legal: &[Card],
        current_plays: &[(u8, Card)],
        trump: Trump,
        lead: Option<Suit>,
    ) -> Card {
        debug_assert!(!legal.is_empty(), "No legal plays provided to heuristic");

        // Determine current winner (if any)
        let (lead_suit, cur_winner) = if let Some(ls) = lead {
            if current_plays.is_empty() {
                (Some(ls), None)
            } else {
                let mut winner = current_plays[0].1;
                for &(_, c) in &current_plays[1..] {
                    if card_beats(c, winner, ls, trump) {
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
                .filter(|&c| card_beats(c, w, ls, trump))
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
}

impl AiPlayer for Heuristic {
    fn choose_bid(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<u8, AiError> {
        let legal = cx.legal_bids(state);
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal bids".into()));
        }
        let est = Self::estimate_tricks(state);
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

        // Score each legal suit and pick the best.
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

    fn choose_play(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<Card, AiError> {
        let legal = state.legal_plays();
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal plays".into()));
        }

        let trump = state.trump.unwrap_or(Trump::NoTrumps);
        let lead = state.current_trick_plays.first().map(|&(_, c)| c.suit);

        // Get tricks won so far and bid to make bid-aware decisions
        let tricks_won_so_far = state.tricks_won[state.player_seat as usize];
        let my_bid = state.bids[state.player_seat as usize].unwrap_or(0);
        let need_more = tricks_won_so_far < my_bid;
        let should_avoid_winning = tricks_won_so_far >= my_bid;

        let choice = match state.current_trick_plays.len() {
            0 => {
                // On lead: prefer lowest from our longest suit (keeps high cards).
                let hand = &state.hand;
                let mut by_suit: HashMap<Suit, Vec<Card>> = HashMap::new();
                for c in hand {
                    by_suit.entry(c.suit).or_default().push(*c);
                }

                // Find the suit with the most cards
                let best_suit = by_suit
                    .iter()
                    .max_by_key(|(_, cards)| cards.len())
                    .map(|(suit, _)| *suit);

                // If our longest suit is available to lead, use it
                if let Some(suit) = best_suit {
                    if let Some(card) = Self::lowest_in_suit(legal.iter(), suit) {
                        return Ok(card);
                    }
                }
                // Fallback: pick overall lowest legal card
                Self::lowest(&legal).unwrap_or(legal[0])
            }
            _ => {
                // When following: use bid-aware logic
                if should_avoid_winning {
                    // Already met or exceeded bid: prefer losing (play low)
                    if let Some(ls) = lead {
                        if let Some(low) = Self::lowest_in_suit(legal.iter(), ls) {
                            // Check if this card would win the trick
                            let mut would_win = false;
                            if let Some(cur_winner) =
                                state.current_trick_plays.first().map(|&(_, c)| c)
                            {
                                if card_beats(low, cur_winner, ls, trump) {
                                    // This low card would still win - try even lower if available
                                    let mut sorted_legal = legal.to_vec();
                                    sorted_legal.sort();
                                    // Find the lowest card that wouldn't win
                                    for card in sorted_legal {
                                        if !card_beats(card, cur_winner, ls, trump) {
                                            return Ok(card);
                                        }
                                    }
                                    // All cards would win - play the lowest anyway
                                    would_win = true;
                                }
                            }
                            if !would_win {
                                return Ok(low);
                            }
                        }
                    }
                    // Fall through to lowest overall if no suit match
                    Self::lowest(&legal).unwrap_or(legal[0])
                } else if need_more {
                    // Behind bid: prefer winning if possible
                    Self::pick_smallest_winning_or_low(
                        &legal,
                        &state.current_trick_plays,
                        trump,
                        lead,
                    )
                } else {
                    // Neutral (tricks_won == bid but unlikely): use default strategy
                    Self::pick_smallest_winning_or_low(
                        &legal,
                        &state.current_trick_plays,
                        trump,
                        lead,
                    )
                }
            }
        };

        Ok(choice)
    }
}
