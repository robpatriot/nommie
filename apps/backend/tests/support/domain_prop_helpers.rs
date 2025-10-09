//! Helper functions for domain property-based tests

use backend::domain::{hand_has_suit, Card, PlayerId, Rank, RoundState, Suit, Trump};

/// Independent oracle for trick winner to cross-check domain logic.
/// Returns the index (0-3) of the winning play.
/// Assumes `plays` are in seat order and length == 4.
pub fn oracle_trick_winner(plays: &[(PlayerId, Card)], trump: Trump) -> usize {
    assert_eq!(plays.len(), 4, "Oracle requires exactly 4 plays");

    // Derive lead suit from the first play to avoid parameter mismatch.
    let lead = plays[0].1.suit;

    // Map trump enum to an optional suit.
    let trump_suit: Option<Suit> = match trump {
        Trump::Clubs => Some(Suit::Clubs),
        Trump::Diamonds => Some(Suit::Diamonds),
        Trump::Hearts => Some(Suit::Hearts),
        Trump::Spades => Some(Suit::Spades),
        Trump::NoTrump => None,
    };

    // Independent rank ordering (highest first). Adjust to your game's rank order.
    fn rank_score(r: Rank) -> u8 {
        match r {
            Rank::Ace => 13,
            Rank::King => 12,
            Rank::Queen => 11,
            Rank::Jack => 10,
            Rank::Ten => 9,
            Rank::Nine => 8,
            Rank::Eight => 7,
            Rank::Seven => 6,
            Rank::Six => 5,
            Rank::Five => 4,
            Rank::Four => 3,
            Rank::Three => 2,
            Rank::Two => 1,
        }
    }

    // Compute key = (is_trump, is_lead, rank_score) and take the max.
    let mut best_idx = 0;
    let mut best_key = {
        let c = plays[0].1;
        (
            (trump_suit == Some(c.suit)) as u8,
            (c.suit == lead) as u8,
            rank_score(c.rank),
        )
    };

    for (i, &(_, c)) in plays.iter().enumerate().skip(1) {
        let key = (
            (trump_suit == Some(c.suit)) as u8,
            (c.suit == lead) as u8,
            rank_score(c.rank),
        );
        if key > best_key {
            best_key = key;
            best_idx = i;
        }
    }

    best_idx
}

/// Compute legal moves for a given hand and optional lead suit.
/// This is a test helper that mirrors domain::tricks::legal_moves but works without GameState.
pub fn legal_moves_helper(hand: &[Card], lead: Option<Suit>) -> Vec<Card> {
    if hand.is_empty() {
        return Vec::new();
    }

    if let Some(lead_suit) = lead {
        if hand_has_suit(hand, lead_suit) {
            let mut v: Vec<Card> = hand
                .iter()
                .copied()
                .filter(|c| c.suit == lead_suit)
                .collect();
            v.sort();
            return v;
        }
    }

    let mut any = hand.to_vec();
    any.sort();
    any
}

/// Helper to build a RoundState with trick data for testing
pub fn build_trick_round_state(
    plays: Vec<(PlayerId, Card)>,
    trump: Trump,
    lead: Suit,
) -> RoundState {
    let mut state = RoundState::new();
    state.trick_plays = plays;
    state.trick_lead = Some(lead);
    state.trump = Some(trump);
    state
}
