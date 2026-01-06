//! Tactician AI - A tactical approach to Nommie.
//!
//! The Tactician AI focuses on accurate bid estimation and adaptive play to maximize
//! the +10 bonus by hitting exact bids. Key principles:
//!
//! - **Bidding**: Estimate trick-taking power from hand composition (honors, length, shape)
//! - **Trump Selection**: Choose trump to maximize control in your longest/strongest suit
//! - **Card Play**: Dynamically adjust between winning and ducking based on tricks needed

use std::sync::Mutex;

use rand::prelude::*;

use super::trait_def::{AiError, AiPlayer};
use crate::domain::player_view::CurrentRoundInfo;
use crate::domain::round_memory::PlayMemory;
use crate::domain::{Card, GameContext, Rank, Suit, Trump};

/// Tactician AI - plays to hit exact bids through tactical decision-making.
///
/// The Tactician evaluates hands using a weighted scoring system that accounts for:
/// - High card strength (Aces, Kings, Queens)
/// - Suit length (potential to establish winners)
/// - Distribution (short suits become trumping opportunities)
///
/// During play, it tracks progress toward the bid and switches between
/// aggressive (winning) and passive (ducking) modes.
pub struct Tactician {
    rng: Mutex<StdRng>,
}

impl Tactician {
    pub const NAME: &'static str = "Tactician";
    pub const VERSION: &'static str = "1.4.0";

    /// Create a new Tactician AI.
    ///
    /// # Arguments
    ///
    /// * `seed` - Optional RNG seed for deterministic behavior
    pub fn new(seed: Option<u64>) -> Self {
        let rng = if let Some(s) = seed {
            StdRng::seed_from_u64(s)
        } else {
            StdRng::from_os_rng()
        };
        Self {
            rng: Mutex::new(rng),
        }
    }

    /// Evaluate hand strength for bidding purposes.
    ///
    /// Returns estimated number of tricks this hand can win.
    /// `is_declarer` indicates if we expect to win the auction and choose trump.
    fn evaluate_hand(
        &self,
        hand: &[Card],
        potential_trump: Option<Trump>,
        is_declarer: bool,
    ) -> f64 {
        if hand.is_empty() {
            return 0.0;
        }

        let hand_size = hand.len() as f64;

        // Count cards per suit
        let mut suit_counts = [0u8; 4];
        let mut suit_honors = [0.0f64; 4]; // Weighted honor count per suit

        for card in hand {
            let idx = suit_index(card.suit);
            suit_counts[idx] += 1;

            // Weight honors by their trick-taking potential
            // Adjusted weights based on calibration data
            let honor_value = match card.rank {
                Rank::Ace => 0.95,   // Nearly guaranteed trick
                Rank::King => 0.55,  // Good but can be captured
                Rank::Queen => 0.30, // Needs support
                Rank::Jack => 0.15,
                Rank::Ten => 0.08,
                _ => 0.0,
            };
            suit_honors[idx] += honor_value;
        }

        let mut estimated_tricks = 0.0;

        // Evaluate each suit
        for suit_idx in 0..4 {
            let count = suit_counts[suit_idx] as f64;
            let honors = suit_honors[suit_idx];

            if count == 0.0 {
                continue;
            }

            // Is this suit trump?
            let is_trump = potential_trump
                .and_then(|t| Suit::try_from(t).ok())
                .map(|s| suit_index(s) == suit_idx)
                .unwrap_or(false);

            // Base value from honors
            let mut suit_tricks = honors;

            // Length bonus: long suits can establish low cards as winners
            if count >= 4.0 {
                let length_bonus = if is_trump {
                    (count - 3.0) * 0.5 // Trump length very valuable
                } else {
                    (count - 3.0) * 0.25 // Side suit length
                };
                suit_tricks += length_bonus;
            }

            // Trump suit control bonus when we're declarer
            if is_trump && is_declarer {
                suit_tricks += count * 0.15; // Each trump has extra value as declarer
            }

            estimated_tricks += suit_tricks;
        }

        // Void/singleton bonus: can trump in when void (only if we have trump)
        let voids = suit_counts.iter().filter(|&&c| c == 0).count();
        let singletons = suit_counts.iter().filter(|&&c| c == 1).count();

        if potential_trump.is_some() && potential_trump != Some(Trump::NoTrumps) {
            // Ruffing potential - more valuable with more trumps
            let trump_idx = potential_trump
                .and_then(|t| Suit::try_from(t).ok())
                .map(suit_index);
            let trump_count = trump_idx.map(|i| suit_counts[i]).unwrap_or(0) as f64;

            if trump_count >= 2.0 {
                estimated_tricks += voids as f64 * 0.5;
                estimated_tricks += singletons as f64 * 0.25;
            }
        }

        // Hand size adjustment: larger hands need upward correction
        // Data shows we underbid more on larger hands
        let size_factor = if hand_size <= 5.0 {
            0.85 // Small hands: be conservative (we overbid)
        } else if hand_size <= 9.0 {
            0.95 // Medium hands: slight reduction
        } else {
            1.05 // Large hands: we underbid, so boost estimate
        };

        estimated_tricks * size_factor
    }

    /// Calculate suit strength for trump selection.
    fn suit_strength(&self, hand: &[Card], suit: Suit) -> f64 {
        let mut count = 0.0;
        let mut honor_power = 0.0;

        for card in hand {
            if card.suit == suit {
                count += 1.0;
                honor_power += match card.rank {
                    Rank::Ace => 3.0,
                    Rank::King => 2.5,
                    Rank::Queen => 1.5,
                    Rank::Jack => 1.0,
                    Rank::Ten => 0.5,
                    _ => 0.1,
                };
            }
        }

        // Combined score: length matters a lot, honors boost value
        count * 2.0 + honor_power
    }

    /// Determine play urgency based on bid progress.
    /// Returns (should_try_to_win, urgency) where urgency is 0.0-1.0
    fn play_urgency(&self, state: &CurrentRoundInfo) -> (bool, f64) {
        let my_bid = state.bids[state.player_seat as usize].unwrap_or(0);
        let my_tricks = state.tricks_won[state.player_seat as usize];
        let tricks_remaining = state.hand_size - state.trick_no + 1;
        let tricks_needed = my_bid.saturating_sub(my_tricks);
        let tricks_over = my_tricks.saturating_sub(my_bid);

        if tricks_needed == 0 && tricks_over == 0 {
            // Exactly at bid - definitely duck, high urgency to stay there
            (false, 1.0)
        } else if tricks_needed == 0 {
            // Over bid - duck with high urgency
            (false, 0.9)
        } else if tricks_needed >= tricks_remaining {
            // Must win all remaining - maximum urgency
            (true, 1.0)
        } else if tricks_needed == tricks_remaining - 1 {
            // Need almost all remaining
            (true, 0.9)
        } else {
            // Some flexibility
            let ratio = tricks_needed as f64 / tricks_remaining as f64;
            if ratio > 0.5 {
                (true, ratio)
            } else if ratio > 0.3 {
                // Marginal - could go either way
                (true, ratio)
            } else {
                // Comfortable margin - can afford to duck
                (false, 1.0 - ratio)
            }
        }
    }

    /// Get our position in the current trick (0 = leading, 3 = last)
    fn trick_position(&self, state: &CurrentRoundInfo) -> u8 {
        state.current_trick_plays.len() as u8
    }

    /// Check if a suit appears to be void for an opponent based on memory.
    fn detect_void(&self, seat: u8, suit: Suit, context: &GameContext) -> bool {
        if let Some(memory) = context.round_memory() {
            for trick in &memory.tricks {
                // Find the lead suit for this trick
                if let Some((_, first_play)) = trick.plays.first() {
                    let lead_suit = match first_play {
                        PlayMemory::Exact(card) => Some(card.suit),
                        PlayMemory::Suit(s) => Some(*s),
                        _ => None,
                    };

                    // If this trick was led with our target suit
                    if lead_suit == Some(suit) {
                        // Check if the target seat played something else
                        for (play_seat, play_mem) in &trick.plays {
                            if *play_seat == seat {
                                match play_mem {
                                    PlayMemory::Exact(card) if card.suit != suit => return true,
                                    PlayMemory::Suit(s) if *s != suit => return true,
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Count how many opponents might be void in a suit.
    fn count_likely_voids(
        &self,
        suit: Suit,
        state: &CurrentRoundInfo,
        context: &GameContext,
    ) -> u8 {
        let mut voids = 0;
        for seat in 0..4 {
            if seat != state.player_seat && self.detect_void(seat, suit, context) {
                voids += 1;
            }
        }
        voids
    }

    /// Check if a card can beat all cards currently in the trick.
    fn can_beat_current_trick(
        &self,
        card: &Card,
        state: &CurrentRoundInfo,
        trump: Option<Trump>,
    ) -> bool {
        if state.current_trick_plays.is_empty() {
            return true; // Leading, so yes
        }

        let lead_suit = state.current_trick_plays[0].1.suit;
        let trump_suit = trump.and_then(|t| Suit::try_from(t).ok());

        // Find current winning card in trick
        let mut winning_card = state.current_trick_plays[0].1;
        for (_, played) in state.current_trick_plays.iter().skip(1) {
            if beats(&winning_card, played, lead_suit, trump_suit) {
                winning_card = *played;
            }
        }

        // Check if our card beats the current winner
        beats(&winning_card, card, lead_suit, trump_suit)
    }

    /// Analyze played cards from memory to inform decisions.
    fn count_remaining_higher(
        &self,
        suit: Suit,
        rank: Rank,
        hand: &[Card],
        context: &GameContext,
    ) -> u8 {
        // Count how many cards of this suit higher than rank are still out
        let all_higher: Vec<Rank> = [
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
        ]
        .into_iter()
        .filter(|&r| r > rank)
        .collect();

        let mut remaining = all_higher.len() as u8;

        // Subtract cards in our hand
        for card in hand {
            if card.suit == suit && card.rank > rank {
                remaining = remaining.saturating_sub(1);
            }
        }

        // Subtract cards we remember being played
        if let Some(memory) = context.round_memory() {
            for trick in &memory.tricks {
                for (_, play_mem) in &trick.plays {
                    if let PlayMemory::Exact(card) = play_mem {
                        if card.suit == suit && card.rank > rank {
                            remaining = remaining.saturating_sub(1);
                        }
                    }
                }
            }
        }

        remaining
    }
}

/// Get suit index for array indexing (Clubs=0, Diamonds=1, Hearts=2, Spades=3).
fn suit_index(suit: Suit) -> usize {
    match suit {
        Suit::Clubs => 0,
        Suit::Diamonds => 1,
        Suit::Hearts => 2,
        Suit::Spades => 3,
    }
}

/// Check if card2 beats card1 given lead suit and trump.
fn beats(card1: &Card, card2: &Card, lead_suit: Suit, trump_suit: Option<Suit>) -> bool {
    let c1_trump = trump_suit.map(|t| card1.suit == t).unwrap_or(false);
    let c2_trump = trump_suit.map(|t| card2.suit == t).unwrap_or(false);

    match (c1_trump, c2_trump) {
        (true, true) => card2.rank > card1.rank, // Both trump: higher wins
        (true, false) => false,                  // Only c1 is trump: c2 can't beat
        (false, true) => true,                   // Only c2 is trump: c2 wins
        (false, false) => {
            // Neither trump: must follow lead suit, higher wins
            if card2.suit == lead_suit && card1.suit == lead_suit {
                card2.rank > card1.rank
            } else if card2.suit == lead_suit {
                true // c2 follows, c1 doesn't
            } else {
                false // c2 doesn't follow lead suit
            }
        }
    }
}

impl AiPlayer for Tactician {
    fn choose_bid(&self, state: &CurrentRoundInfo, context: &GameContext) -> Result<u8, AiError> {
        let legal_bids = context.legal_bids(state);
        if legal_bids.is_empty() {
            return Err(AiError::InvalidMove("No legal bids available".into()));
        }

        // Calculate current high bid from other players
        let current_high_bid = state.bids.iter().filter_map(|&b| b).max().unwrap_or(0);

        // Estimate tricks if we become declarer (choose trump)
        let declarer_estimate = [
            self.evaluate_hand(&state.hand, Some(Trump::Clubs), true),
            self.evaluate_hand(&state.hand, Some(Trump::Diamonds), true),
            self.evaluate_hand(&state.hand, Some(Trump::Hearts), true),
            self.evaluate_hand(&state.hand, Some(Trump::Spades), true),
        ]
        .into_iter()
        .fold(0.0f64, f64::max);

        // Estimate tricks if we don't win auction (defender)
        // Use a neutral trump assumption - we'll be playing against opponent's trump
        let defender_estimate =
            self.evaluate_hand(&state.hand, Some(Trump::NoTrumps), false) * 0.75;

        // Determine likelihood we'll win the auction based on our potential bid
        let bid_count = state.bids.iter().filter(|b| b.is_some()).count();

        // If our declarer estimate would be the high bid, weight toward declarer
        let would_be_high = declarer_estimate.round() as u8 > current_high_bid;

        let estimated_tricks = if would_be_high && declarer_estimate >= 2.0 {
            // We'd likely win auction - use declarer estimate
            // Add bonus for trump advantage
            declarer_estimate + 0.8
        } else {
            // Weight between declarer and defender based on position
            // First bidder (bid_count=0) has a unique position:
            // - Information disadvantage (bids first, leads first)
            // - But can set the pace for the auction
            // Give first bidder slightly more declarer weight to encourage winning auction
            let declarer_weight = match bid_count {
                0 => 0.45, // First bidder: more aggressive to win auction
                1 => 0.35, // Second: uncertain
                2 => 0.3,  // Third: more info
                _ => 0.25, // Dealer (last): most defensive
            };
            declarer_estimate * declarer_weight + defender_estimate * (1.0 - declarer_weight)
        };

        // Round to nearest integer for target bid
        let target_bid = estimated_tricks.round() as u8;

        // Find closest legal bid to target
        let min_distance = legal_bids
            .iter()
            .map(|&b| (b as i16 - target_bid as i16).abs())
            .min()
            .unwrap_or(0);

        let candidates: Vec<_> = legal_bids
            .iter()
            .filter(|&&b| (b as i16 - target_bid as i16).abs() == min_distance)
            .copied()
            .collect();

        // If multiple equally close, use random tiebreaker
        let chosen = if candidates.len() > 1 {
            let mut rng = self
                .rng
                .lock()
                .map_err(|e| AiError::Internal(format!("RNG lock poisoned: {e}")))?;
            *candidates.choose(&mut *rng).unwrap_or(&candidates[0])
        } else {
            candidates.first().copied().unwrap_or(legal_bids[0])
        };

        Ok(chosen)
    }

    fn choose_trump(
        &self,
        state: &CurrentRoundInfo,
        _context: &GameContext,
    ) -> Result<Trump, AiError> {
        let legal_trumps = state.legal_trumps();
        if legal_trumps.is_empty() {
            return Err(AiError::InvalidMove("No legal trumps available".into()));
        }

        // Calculate strength for each suit
        let suit_scores = [
            (Trump::Clubs, self.suit_strength(&state.hand, Suit::Clubs)),
            (
                Trump::Diamonds,
                self.suit_strength(&state.hand, Suit::Diamonds),
            ),
            (Trump::Hearts, self.suit_strength(&state.hand, Suit::Hearts)),
            (Trump::Spades, self.suit_strength(&state.hand, Suit::Spades)),
        ];

        // Find best suit
        let (best_trump, best_score) = suit_scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .copied()
            .unwrap_or((Trump::Spades, 0.0));

        // Consider NoTrumps if we have decent stoppers in all suits
        // but no particularly strong trump suit
        let min_suit_score = suit_scores.iter().map(|(_, s)| *s).fold(f64::MAX, f64::min);

        // NoTrumps good if: balanced hand (min suit not too weak) and best suit not overwhelming
        if min_suit_score >= 2.0 && best_score < 8.0 {
            // Check for stoppers in each suit (at least one high card)
            let has_stoppers = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
                .iter()
                .all(|&suit| {
                    state.hand.iter().any(|c| {
                        c.suit == suit
                            && matches!(c.rank, Rank::Ace | Rank::King | Rank::Queen | Rank::Jack)
                    })
                });

            if has_stoppers {
                return Ok(Trump::NoTrumps);
            }
        }

        Ok(best_trump)
    }

    fn choose_play(
        &self,
        state: &CurrentRoundInfo,
        context: &GameContext,
    ) -> Result<Card, AiError> {
        let legal_plays = state.legal_plays();
        if legal_plays.is_empty() {
            return Err(AiError::InvalidMove("No legal plays available".into()));
        }

        // Single legal play - no choice
        if legal_plays.len() == 1 {
            return Ok(legal_plays[0]);
        }

        let trump_suit = state.trump.and_then(|t| Suit::try_from(t).ok());
        let (should_win, urgency) = self.play_urgency(state);
        let position = self.trick_position(state);

        // Leading the trick
        if state.current_trick_plays.is_empty() {
            return self.choose_lead(state, context, &legal_plays, should_win, urgency);
        }

        // Following in the trick
        self.choose_follow(
            state,
            context,
            &legal_plays,
            should_win,
            urgency,
            trump_suit,
            position,
        )
    }
}

impl Tactician {
    /// Estimate probability an opponent is void in a suit.
    /// Uses memory if available, otherwise uses probabilistic reasoning.
    fn estimate_void_probability(
        &self,
        suit: Suit,
        state: &CurrentRoundInfo,
        context: &GameContext,
    ) -> f64 {
        // Check known voids from memory first
        let known_voids = self.count_likely_voids(suit, state, context);
        if known_voids > 0 {
            return known_voids as f64 * 0.9; // High confidence if we observed it
        }

        // Probabilistic estimate based on card distribution
        // Total cards in this suit in deck: 13
        // Cards we hold in this suit
        let our_count = state.hand.iter().filter(|c| c.suit == suit).count();

        // Cards played so far (from memory)
        let played_count = if let Some(memory) = context.round_memory() {
            memory
                .tricks
                .iter()
                .flat_map(|t| t.plays.iter())
                .filter(|(_, pm)| {
                    matches!(pm, PlayMemory::Exact(c) if c.suit == suit)
                        || matches!(pm, PlayMemory::Suit(s) if *s == suit)
                })
                .count()
        } else {
            0
        };

        // Remaining cards of this suit distributed among 3 opponents
        let remaining = 13usize.saturating_sub(our_count + played_count);
        let cards_per_opponent = state.hand_size as usize; // Each opponent has this many cards

        if remaining == 0 {
            return 0.9; // All cards accounted for, likely void
        }

        // Probability at least one opponent is void
        // Approximation: if remaining < 3*hand_size, some opponents might be void
        // More remaining cards = lower void probability
        if remaining >= 3 * cards_per_opponent {
            0.0 // Plenty of cards, no one void
        } else if remaining <= 3 {
            0.5 + (3.0 - remaining as f64) * 0.15 // Few cards, likely voids
        } else {
            // Linear interpolation
            let ratio = remaining as f64 / (3.0 * cards_per_opponent as f64);
            (1.0 - ratio) * 0.4
        }
    }

    /// Choose a card to lead with.
    fn choose_lead(
        &self,
        state: &CurrentRoundInfo,
        context: &GameContext,
        legal_plays: &[Card],
        should_win: bool,
        urgency: f64,
    ) -> Result<Card, AiError> {
        let trump_suit = state.trump.and_then(|t| Suit::try_from(t).ok());
        let is_first_trick = state.trick_no == 1;

        // Count suits in hand for strategy
        let mut suit_counts = [0u8; 4];
        for card in &state.hand {
            suit_counts[suit_index(card.suit)] += 1;
        }

        if should_win {
            // Lead from strength to win tricks

            // Special handling for first trick: we have no memory yet
            if is_first_trick {
                return self.choose_first_trick_lead(
                    state,
                    legal_plays,
                    &suit_counts,
                    trump_suit,
                    urgency,
                );
            }

            // High urgency: lead guaranteed winners (aces in safe suits)
            if urgency > 0.7 {
                // Find aces in suits where opponents aren't void
                for &card in legal_plays {
                    if card.rank == Rank::Ace {
                        let is_trump = trump_suit.map(|t| card.suit == t).unwrap_or(false);
                        let void_prob = self.estimate_void_probability(card.suit, state, context);

                        // Non-trump ace with low void probability is safe
                        if !is_trump && void_prob < 0.3 {
                            return Ok(card);
                        }
                    }
                }

                // If we have trump ace and good length, draw trump
                if let Some(ts) = trump_suit {
                    let trump_idx = suit_index(ts);
                    let our_trumps = suit_counts[trump_idx];

                    for &card in legal_plays {
                        if card.suit == ts && card.rank == Rank::Ace && our_trumps >= 4 {
                            return Ok(card);
                        }
                    }
                }
            }

            // Find best lead considering remaining cards and void risk
            let mut best_card = legal_plays[0];
            let mut best_score = -100.0;

            for &card in legal_plays {
                let remaining_higher =
                    self.count_remaining_higher(card.suit, card.rank, &state.hand, context);
                let is_trump = trump_suit.map(|t| card.suit == t).unwrap_or(false);
                let void_prob = self.estimate_void_probability(card.suit, state, context);

                // Base score: fewer higher cards = better chance to win
                let mut score = 12.0 - remaining_higher as f64 * 2.0;

                // Penalty for leading into likely voids (will get trumped)
                if !is_trump {
                    score -= void_prob * 6.0;
                }

                // Trump leads good when we have length
                if is_trump {
                    let our_trumps = suit_counts[suit_index(card.suit)];
                    if our_trumps >= 4 {
                        score += 3.0; // Drawing trump with good length
                    } else if urgency < 0.6 {
                        score -= 2.0; // Conserve trump when not urgent
                    }
                }

                // Bonus for leading from long suits (can establish)
                let suit_length = suit_counts[suit_index(card.suit)];
                if suit_length >= 4 && card.rank >= Rank::King {
                    score += 1.5;
                }

                if score > best_score {
                    best_score = score;
                    best_card = card;
                }
            }

            Ok(best_card)
        } else {
            // Lead low to duck - give away tricks
            self.choose_ducking_lead(
                state,
                context,
                legal_plays,
                &suit_counts,
                trump_suit,
                is_first_trick,
            )
        }
    }

    /// Choose a lead for the very first trick of a round (no memory available).
    fn choose_first_trick_lead(
        &self,
        state: &CurrentRoundInfo,
        legal_plays: &[Card],
        suit_counts: &[u8; 4],
        trump_suit: Option<Suit>,
        urgency: f64,
    ) -> Result<Card, AiError> {
        // On first trick, we have no information about opponent voids.
        // Key insight: Be PATIENT on trick 1 - save high cards for later
        // when we have information. Use trick 1 to probe/gather info.

        let tricks_remaining = state.hand_size;
        let my_bid = state.bids[state.player_seat as usize].unwrap_or(0);

        // Calculate how "desperate" we are - if we need many tricks,
        // we can't afford to be too patient
        let patience_allowed = if my_bid == 0 {
            1.0 // Bidding zero - be very patient (ducking anyway)
        } else if my_bid as f64 / tricks_remaining as f64 > 0.6 {
            0.2 // Need many tricks - must be aggressive
        } else if my_bid as f64 / tricks_remaining as f64 > 0.4 {
            0.5 // Moderate need
        } else {
            0.8 // Comfortable - can be patient
        };

        let mut best_card = legal_plays[0];
        let mut best_score = -100.0;

        for &card in legal_plays {
            let is_trump = trump_suit.map(|t| card.suit == t).unwrap_or(false);
            let suit_length = suit_counts[suit_index(card.suit)];
            let our_cards_in_suit = suit_length as usize;

            // Remaining cards of this suit (13 total per suit)
            let remaining_in_suit = 13 - our_cards_in_suit;

            // Estimate void probability
            let void_prob = if remaining_in_suit >= 9 {
                0.05
            } else if remaining_in_suit >= 6 {
                0.15
            } else if remaining_in_suit >= 3 {
                0.35
            } else {
                0.6
            };

            let mut score = 0.0;

            if is_trump {
                // Trump leads on first trick
                if suit_length >= 5 && card.rank >= Rank::Queen {
                    // Strong trump - good to draw trump
                    score += 3.0;
                } else if card.rank == Rank::Ace && suit_length >= 4 {
                    // Trump ace with good support
                    score += 4.0;
                } else if patience_allowed > 0.5 {
                    // Patient mode: avoid trump leads on trick 1
                    score -= 4.0;
                } else {
                    score -= 1.0;
                }
            } else {
                // Side suit leads - this is where patience matters most

                if card.rank == Rank::Ace {
                    // Aces are risky on trick 1 due to unknown voids
                    if patience_allowed > 0.6 {
                        // Patient: hold the ace for later when we know voids
                        score -= 2.0;
                    } else if void_prob < 0.2 {
                        score += 4.0; // Low void risk, urgency forces us
                    } else {
                        score += 1.0; // Moderate
                    }
                } else if card.rank == Rank::King {
                    let have_ace = legal_plays
                        .iter()
                        .any(|c| c.suit == card.suit && c.rank == Rank::Ace);
                    if have_ace {
                        // K with A backup - lead K to probe, save A
                        score += 3.0;
                    } else if patience_allowed > 0.5 {
                        // Unguarded K is risky on trick 1
                        score -= 1.0;
                    } else {
                        score += 1.0;
                    }
                } else if card.rank >= Rank::Jack {
                    // Q/J - moderate cards, okay for probing
                    if suit_length >= 3 {
                        score += 2.0; // Some support
                    } else {
                        score += 0.5;
                    }
                } else {
                    // Low cards - excellent for trick 1 probing!
                    // Low leads from long suits are ideal for gathering info
                    if suit_length >= 4 {
                        score += 4.0; // Lead low from long suit - establishes and probes
                        if card.rank <= Rank::Five {
                            score += 1.5; // Extra bonus for very low cards
                        }
                    } else if suit_length >= 3 {
                        score += 2.0;
                    } else {
                        // Short suit low lead - creates potential void
                        score += 1.0;
                    }
                }

                // Void risk penalty (less severe for low cards since we're probing)
                if card.rank >= Rank::Jack {
                    score -= void_prob * 3.0;
                } else {
                    score -= void_prob * 1.0;
                }
            }

            // Urgency override - when desperate, lead strength
            if urgency > 0.85 {
                score += (card.rank as i16 as f64) * 0.25;
            }

            if score > best_score {
                best_score = score;
                best_card = card;
            }
        }

        Ok(best_card)
    }

    /// Choose a low lead when trying to duck.
    fn choose_ducking_lead(
        &self,
        state: &CurrentRoundInfo,
        context: &GameContext,
        legal_plays: &[Card],
        suit_counts: &[u8; 4],
        trump_suit: Option<Suit>,
        is_first_trick: bool,
    ) -> Result<Card, AiError> {
        let mut best_card = legal_plays[0];
        let mut best_score = -100.0;

        for &card in legal_plays {
            let idx = suit_index(card.suit);
            let is_trump = trump_suit.map(|t| card.suit == t).unwrap_or(false);
            let suit_length = suit_counts[idx];

            let mut score = 0.0;

            // Lower rank is better for ducking
            score -= card.rank as i16 as f64;

            // Prefer short suits to create voids (future trumping potential)
            if suit_length == 1 {
                score += 4.0; // Creating a void!
            } else if suit_length == 2 {
                score += 2.0;
            }

            // Strongly avoid leading trump when ducking
            if is_trump {
                score -= 15.0;
            }

            // Don't waste high cards - lead low from long suits
            if suit_length >= 3 && card.rank <= Rank::Five {
                score += 2.5;
            }

            // On first trick, slightly prefer leading from suits where
            // opponents likely have cards (so they can win the trick)
            if is_first_trick && !is_trump {
                let our_cards = suit_length as usize;
                let remaining = 13 - our_cards;
                if remaining >= 9 {
                    score += 1.0; // Opponents likely have this suit
                }
            }

            // Use memory to avoid suits opponents are void in (they'll discard low)
            if !is_first_trick {
                let void_prob = self.estimate_void_probability(card.suit, state, context);
                if void_prob > 0.5 && !is_trump {
                    score -= 2.0; // Opponents void - they'll discard, we might win anyway
                }
            }

            if score > best_score {
                best_score = score;
                best_card = card;
            }
        }

        Ok(best_card)
    }

    /// Choose a card when following in a trick.
    #[allow(clippy::too_many_arguments)]
    fn choose_follow(
        &self,
        state: &CurrentRoundInfo,
        context: &GameContext,
        legal_plays: &[Card],
        should_win: bool,
        urgency: f64,
        trump_suit: Option<Suit>,
        position: u8,
    ) -> Result<Card, AiError> {
        let lead_card = state.current_trick_plays[0].1;
        let lead_suit = lead_card.suit;
        let is_last = position == 3; // We're playing last

        // Can we follow suit?
        let following_suit = legal_plays.iter().any(|c| c.suit == lead_suit);

        if should_win {
            // Try to win the trick
            let mut winning_plays: Vec<_> = legal_plays
                .iter()
                .filter(|&c| self.can_beat_current_trick(c, state, state.trump))
                .copied()
                .collect();

            if !winning_plays.is_empty() {
                // Sort by a smart ordering
                winning_plays.sort_by(|a, b| {
                    let a_trump = trump_suit.map(|t| a.suit == t).unwrap_or(false);
                    let b_trump = trump_suit.map(|t| b.suit == t).unwrap_or(false);

                    // Prefer non-trump over trump
                    match (a_trump, b_trump) {
                        (false, true) => std::cmp::Ordering::Less,
                        (true, false) => std::cmp::Ordering::Greater,
                        _ => a.rank.cmp(&b.rank), // Then by rank (lower = better)
                    }
                });

                // If we're last, use exactly the cheapest winner
                if is_last {
                    return Ok(winning_plays[0]);
                }

                // If not last and high urgency, might need to overcommit
                // to ensure win against players after us
                if urgency > 0.8 && winning_plays.len() > 1 {
                    // Check if someone after us could beat our lowest winner
                    let players_after = 3 - position;
                    if players_after > 0 {
                        // Use a slightly higher card for safety
                        let idx = (winning_plays.len() / 3).min(winning_plays.len() - 1);
                        return Ok(winning_plays[idx]);
                    }
                }

                return Ok(winning_plays[0]);
            }

            // Can't win - play lowest card to minimize loss
            return self.play_lowest(legal_plays, trump_suit);
        }

        // Trying to duck
        if following_suit {
            // Must follow suit - find cards that don't beat current winner
            let mut safe_plays: Vec<_> = legal_plays
                .iter()
                .filter(|&c| !self.can_beat_current_trick(c, state, state.trump))
                .copied()
                .collect();

            if !safe_plays.is_empty() {
                // Play highest safe card (save low cards for later ducking)
                safe_plays.sort_by_key(|c| std::cmp::Reverse(c.rank));
                return Ok(safe_plays[0]);
            }

            // All our cards win - play lowest to minimize winning margin
            return self.play_lowest(legal_plays, trump_suit);
        }

        // Can't follow suit - choose discard
        self.choose_discard(state, context, legal_plays, should_win, urgency, trump_suit)
    }

    /// Choose a card to discard when void in lead suit.
    fn choose_discard(
        &self,
        state: &CurrentRoundInfo,
        _context: &GameContext,
        legal_plays: &[Card],
        should_win: bool,
        urgency: f64,
        trump_suit: Option<Suit>,
    ) -> Result<Card, AiError> {
        let trumps: Vec<_> = legal_plays
            .iter()
            .filter(|c| trump_suit.map(|t| c.suit == t).unwrap_or(false))
            .copied()
            .collect();

        let non_trumps: Vec<_> = legal_plays
            .iter()
            .filter(|c| trump_suit.map(|t| c.suit != t).unwrap_or(true))
            .copied()
            .collect();

        if should_win {
            // Want to win - consider trumping
            if !trumps.is_empty() {
                // Check if we need to trump
                let current_winner_is_trump = state
                    .current_trick_plays
                    .iter()
                    .any(|(_, c)| trump_suit.map(|t| c.suit == t).unwrap_or(false));

                if current_winner_is_trump {
                    // Need to overtrump - find cheapest overtrump
                    let mut valid_trumps: Vec<_> = trumps
                        .iter()
                        .filter(|&c| self.can_beat_current_trick(c, state, state.trump))
                        .copied()
                        .collect();

                    if !valid_trumps.is_empty() {
                        valid_trumps.sort_by_key(|c| c.rank);
                        return Ok(valid_trumps[0]);
                    }
                } else {
                    // Can trump in with lowest trump
                    let lowest_trump = trumps.iter().min_by_key(|c| c.rank).copied();
                    if let Some(t) = lowest_trump {
                        // Only trump if urgency warrants it
                        if urgency > 0.4 {
                            return Ok(t);
                        }
                    }
                }
            }

            // Can't trump effectively - discard lowest from short suit
            return self.smart_discard(state, &non_trumps, &trumps);
        }

        // Don't want to win - avoid trumping
        if !non_trumps.is_empty() {
            return self.smart_discard(state, &non_trumps, &[]);
        }

        // Only have trump - play lowest
        self.play_lowest(legal_plays, trump_suit)
    }

    /// Smart discard considering suit lengths and future needs.
    fn smart_discard(
        &self,
        state: &CurrentRoundInfo,
        non_trumps: &[Card],
        trumps: &[Card],
    ) -> Result<Card, AiError> {
        if non_trumps.is_empty() {
            // Only trumps available
            return trumps
                .iter()
                .min_by_key(|c| c.rank)
                .copied()
                .ok_or_else(|| AiError::Internal("No cards to discard".into()));
        }

        // Count suit lengths in remaining hand
        let mut suit_counts = [0u8; 4];
        for card in &state.hand {
            suit_counts[suit_index(card.suit)] += 1;
        }

        // Find best discard
        let mut best_card = non_trumps[0];
        let mut best_score = -100.0;

        for &card in non_trumps {
            let idx = suit_index(card.suit);
            let suit_length = suit_counts[idx];

            let mut score = 0.0;

            // Prefer discarding from short suits (creating voids)
            if suit_length == 1 {
                score += 5.0; // Creating void
            } else if suit_length == 2 {
                score += 3.0;
            }

            // Prefer discarding low cards
            score -= (card.rank as i16 as f64) * 0.3;

            // Avoid discarding aces/kings unless necessary
            if matches!(card.rank, Rank::Ace | Rank::King) {
                score -= 8.0;
            }

            if score > best_score {
                best_score = score;
                best_card = card;
            }
        }

        Ok(best_card)
    }

    /// Play the lowest card, preferring non-trump.
    fn play_lowest(&self, legal_plays: &[Card], trump_suit: Option<Suit>) -> Result<Card, AiError> {
        // Prefer lowest non-trump
        let non_trumps: Vec<_> = legal_plays
            .iter()
            .filter(|c| trump_suit.map(|t| c.suit != t).unwrap_or(true))
            .collect();

        if !non_trumps.is_empty() {
            return non_trumps
                .iter()
                .min_by_key(|c| c.rank)
                .copied()
                .copied()
                .ok_or_else(|| AiError::Internal("No non-trump cards".into()));
        }

        // Only trumps
        legal_plays
            .iter()
            .min_by_key(|c| c.rank)
            .copied()
            .ok_or_else(|| AiError::Internal("No cards to play".into()))
    }
}
