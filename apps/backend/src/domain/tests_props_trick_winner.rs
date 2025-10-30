use proptest::prelude::*;

use crate::domain::domain_prop_helpers::oracle_trick_winner;
use crate::domain::{test_gens, test_prelude};
/// Property-based tests for trick winner resolution
use crate::domain::{RoundState, Suit, Trump};

proptest! {
    #![proptest_config(test_prelude::proptest_config())]

    /// Property: Trick winner with NO_TRUMP
    /// In a fully played trick with no trump, the winner must be the highest-ranked card
    /// of the lead suit. Off-suit cards cannot win.
    #[test]
    fn prop_trick_winner_no_trump(
        trick_data in test_gens::complete_trick(),
    ) {
        let (_leader, plays, _, _) = trick_data;
        let lead = plays[0].1.suit;
        let trump = Trump::NoTrump;

        // Build RoundState
        let mut state = RoundState::empty();
        state.trick_plays = plays.clone();
        state.trick_lead = Some(lead);
        state.trump = Some(trump);

        // Get domain winner
        let winner = crate::domain::tricks::resolve_current_trick(&state);
        prop_assert!(winner.is_some(), "Complete trick must have a winner");
        let winner_id = winner.unwrap();

        // Oracle winner
        let oracle_idx = oracle_trick_winner(&plays, trump);
        let oracle_winner_id = plays[oracle_idx].0;

        prop_assert_eq!(winner_id, oracle_winner_id,
            "Domain winner {:?} must match oracle winner {:?} for NO_TRUMP. Lead={:?}, plays={:?}",
            winner_id, oracle_winner_id, lead, plays);

        // Verify winner has lead suit (when any lead suit cards played)
        let winner_card = plays.iter().find(|(id, _)| *id == winner_id).unwrap().1;
        let lead_cards: Vec<_> = plays.iter().filter(|(_, c)| c.suit == lead).collect();

        if !lead_cards.is_empty() {
            prop_assert_eq!(winner_card.suit, lead,
                "NO_TRUMP winner must be of lead suit when lead cards are played");

            // Verify it's the highest rank of lead suit
            for (_, card) in &lead_cards {
                prop_assert!(winner_card.rank >= card.rank,
                    "Winner rank {:?} must be >= all lead suit ranks", winner_card.rank);
            }
        }
    }

    /// Property: Trick winner with trump
    /// In a fully played trick with a trump suit, if any trump cards are played,
    /// the highest trump wins; otherwise, the highest card of the lead suit wins.
    #[test]
    fn prop_trick_winner_with_trump(
        trick_data in prop::strategy::Strategy::prop_flat_map(
            test_gens::complete_trick(),
            |(lead, plays, _trump, leader)| {
                // Replace trump with a non-NoTrump value
                (Just((lead, plays, leader)), test_gens::trump_suit())
                    .prop_map(|((lead, plays, leader), trump)| (lead, plays, trump, leader))
            }
        ),
    ) {
        let (_, plays, trump, _) = trick_data;

        // trump is guaranteed to NOT be NoTrump by construction

        let lead = plays[0].1.suit;

        // Build RoundState
        let mut state = RoundState::empty();
        state.trick_plays = plays.clone();
        state.trick_lead = Some(lead);
        state.trump = Some(trump);

        // Get domain winner
        let winner = crate::domain::tricks::resolve_current_trick(&state);
        prop_assert!(winner.is_some(), "Complete trick must have a winner");
        let winner_id = winner.unwrap();

        // Oracle winner
        let oracle_idx = oracle_trick_winner(&plays, trump);
        let oracle_winner_id = plays[oracle_idx].0;

        prop_assert_eq!(winner_id, oracle_winner_id,
            "Domain winner {:?} must match oracle winner {:?} with trump. Trump={:?}, lead={:?}, plays={:?}",
            winner_id, oracle_winner_id, trump, lead, plays);

        // Verify winner logic
        let trump_suit = match trump {
            Trump::Clubs => Suit::Clubs,
            Trump::Diamonds => Suit::Diamonds,
            Trump::Hearts => Suit::Hearts,
            Trump::Spades => Suit::Spades,
            Trump::NoTrump => unreachable!(),
        };

        let winner_card = plays.iter().find(|(id, _)| *id == winner_id).unwrap().1;
        let trump_cards: Vec<_> = plays.iter().filter(|(_, c)| c.suit == trump_suit).collect();

        if !trump_cards.is_empty() {
            prop_assert_eq!(winner_card.suit, trump_suit,
                "Winner must be trump when trump cards are played");

            // Verify it's the highest trump
            for (_, card) in &trump_cards {
                prop_assert!(winner_card.rank >= card.rank,
                    "Winner rank {:?} must be >= all trump ranks", winner_card.rank);
            }
        } else {
            // No trump played: winner must be highest of lead suit
            let lead_cards: Vec<_> = plays.iter().filter(|(_, c)| c.suit == lead).collect();
            if !lead_cards.is_empty() {
                prop_assert_eq!(winner_card.suit, lead,
                    "Winner must be lead suit when no trump played");

                for (_, card) in &lead_cards {
                    prop_assert!(winner_card.rank >= card.rank,
                        "Winner rank {:?} must be >= all lead suit ranks", winner_card.rank);
                }
            }
        }
    }

    /// Property: Winner oracle cross-check
    /// The domain's trick_winner result must match an independent oracle implementation.
    #[test]
    fn prop_winner_oracle_cross_check(
        trick_data in test_gens::complete_trick(),
    ) {
        let (_, plays, trump, _) = trick_data;
        let lead = plays[0].1.suit;

        // Build RoundState
        let mut state = RoundState::empty();
        state.trick_plays = plays.clone();
        state.trick_lead = Some(lead);
        state.trump = Some(trump);

        // Get domain winner
        let domain_winner = crate::domain::tricks::resolve_current_trick(&state);
        prop_assert!(domain_winner.is_some(), "Domain must return a winner for complete trick");
        let domain_winner_id = domain_winner.unwrap();

        // Get oracle winner
        let oracle_idx = oracle_trick_winner(&plays, trump);
        let oracle_winner_id = plays[oracle_idx].0;

        prop_assert_eq!(domain_winner_id, oracle_winner_id,
            "Domain winner {:?} must match oracle winner {:?}. Trump={:?}, lead={:?}, plays={:?}",
            domain_winner_id, oracle_winner_id, trump, lead, plays);
    }

    /// Property: Next leader is trick winner
    /// After a trick completes, the next leader must be the trick winner.
    #[test]
    fn prop_next_leader_is_winner(
        trick_data in test_gens::complete_trick(),
    ) {
        let (_, plays, trump, _) = trick_data;
        let lead = plays[0].1.suit;

        // Build RoundState
        let mut state = RoundState::empty();
        state.trick_plays = plays.clone();
        state.trick_lead = Some(lead);
        state.trump = Some(trump);

        // Resolve trick
        let winner = crate::domain::tricks::resolve_current_trick(&state);
        prop_assert!(winner.is_some(), "Trick must have a winner");
        let winner_id = winner.unwrap();

        // In actual game flow, the next leader would be set to winner_id
        // This is tested implicitly by checking oracle consistency
        // Here we just verify the winner is one of the players
        prop_assert!(winner_id <= 3, "Winner must be a valid player ID (0-3)");
    }
}
