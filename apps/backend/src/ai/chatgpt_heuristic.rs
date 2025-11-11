//! HeuristicV1 — a stronger, deterministic baseline AI for Nommie.
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
//!
//! Notes & assumptions:
//! - Uses only public, common-sense fields (`state.hand`, `state.trump`, `state.current_trick_plays`)
//!   plus the `legal_*()` helpers; adjust import paths or names as needed.
//! - `Card` is assumed to expose `suit: Suit` and implement `Ord` among same-suit ranks.
//! - `Trump` is one of the five enum variants (`Clubs`, `Diamonds`, `Hearts`, `Spades`, `NoTrump`).
//!
//! Determinism:
//! - No randomness used. `seed` is stored for future extensions (e.g., tie-breaking knobs).
use crate::ai::{AiError, AiPlayer};
use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::{Card, GameContext, Suit, Trump};

#[derive(Clone)]
pub struct HeuristicV1 {
    _seed: Option<u64>, // reserved, currently unused for strict determinism
}

impl HeuristicV1 {
    pub const NAME: &'static str = "HeuristicV1";
    pub const VERSION: &'static str = "1.0.0";

    pub const fn name() -> &'static str {
        Self::NAME
    }

    pub const fn version() -> &'static str {
        Self::VERSION
    }

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
            Trump::NoTrump => None,
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
    fn pick_smallest_winning_or_low(
        legal: &[Card],
        current_plays: &[(i16, Card)],
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
}

impl AiPlayer for HeuristicV1 {
    fn choose_bid(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<u8, AiError> {
        let legal = state.legal_bids();
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

        // If NoTrump is legal and the hand is balanced with some top cards, consider it.
        let no_trump_legal = legal.iter().any(|t| matches!(t, Trump::NoTrump));
        let want_nt = no_trump_legal && Self::prefer_no_trump(&hand);

        if want_nt {
            return Ok(Trump::NoTrump);
        }

        // Score each legal suit and pick the best.
        let mut best: Option<(i32, Trump)> = None;
        for &t in &legal {
            let score = match t {
                Trump::Clubs => Self::trump_score_for_suit(&hand, Suit::Clubs),
                Trump::Diamonds => Self::trump_score_for_suit(&hand, Suit::Diamonds),
                Trump::Hearts => Self::trump_score_for_suit(&hand, Suit::Hearts),
                Trump::Spades => Self::trump_score_for_suit(&hand, Suit::Spades),
                Trump::NoTrump => continue,
            };
            match best {
                None => best = Some((score, t)),
                Some((bs, _)) if score > bs => best = Some((score, t)),
                _ => {}
            }
        }

        // If only NoTrump was legal or no suit evaluated, default to first legal.
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

        let trump = state.trump.unwrap_or(Trump::NoTrump);
        let lead = state.current_trick_plays.first().map(|&(_, c)| c.suit);

        let choice = match state.current_trick_plays.len() {
            0 => {
                // On lead: prefer lowest from our longest suit (keeps high cards).
                let hand = &state.hand;
                let mut by_suit: [Vec<Card>; 4] = [vec![], vec![], vec![], vec![]];
                for c in hand {
                    by_suit[Self::suit_index(c.suit)].push(*c);
                }
                let mut best_idx = 0usize;
                for i in 1..4 {
                    if by_suit[i].len() > by_suit[best_idx].len() {
                        best_idx = i;
                    }
                }
                // If our longest suit is not available to lead (e.g., all filtered out by
                // legality for some edge rule), just pick overall lowest legal card.
                if let Some(card) = Self::lowest_in_suit(legal.iter(), Suit::from_index(best_idx)) {
                    card
                } else {
                    Self::lowest(&legal).unwrap_or(legal[0])
                }
            }
            _ => {
                Self::pick_smallest_winning_or_low(&legal, &state.current_trick_plays, trump, lead)
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
