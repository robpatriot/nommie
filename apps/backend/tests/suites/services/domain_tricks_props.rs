//! Property tests for trick-taking logic (pure domain, no DB).
//!
//! Properties tested:
//! - First card of a trick establishes the led suit
//! - Players must follow suit if they can
//! - If void in led suit, any card is legal
//! - Trick winner is highest card of led suit (no trump) or highest trump
//! - Ties are impossible under standard deck ordering

include!("../../common/proptest_prelude.rs");

use backend::domain::state::Phase;
use backend::domain::tricks::{legal_moves, play_card};
use backend::domain::{Card, Rank, Suit, Trump};
use backend::errors::domain::{DomainError, ValidationKind};
use proptest::prelude::*;

proptest! {
    #![proptest_config(proptest_prelude_config())]

    /// Property: First card establishes led suit
    #[test]
    fn prop_first_card_establishes_lead(
        card in crate::support::domain_gens::card(),
    ) {
        use backend::domain::state::init_round;
        let hands = [vec![card], vec![], vec![], vec![]];
        let mut state = init_round(1, 5, hands, 0, [0; 4]);
        state.phase = Phase::Trick { trick_no: 1 };
        state.turn = 0;
        state.leader = 0;
        state.round.trump = Some(Trump::NoTrump);

        // Play the first card
        let result = play_card(&mut state, 0, card);
        prop_assert!(result.is_ok(), "First card play should succeed");

        // Led suit should be set
        prop_assert_eq!(state.round.trick_lead, Some(card.suit),
            "First card should establish led suit");
    }

    /// Property: Must follow suit when able
    #[test]
    fn prop_must_follow_suit_when_able(
        (lead_suit, lead_rank, off_suit_card) in prop::strategy::Strategy::prop_flat_map(
            crate::support::domain_gens::suit(),
            |lead| {
                let other_suits = vec![Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
                    .into_iter()
                    .filter(|s| *s != lead)
                    .collect::<Vec<_>>();
                (
                    Just(lead),
                    crate::support::domain_gens::rank(),
                    (prop::sample::select(other_suits), crate::support::domain_gens::rank())
                        .prop_map(|(s, r)| Card { suit: s, rank: r })
                )
            }
        ),
    ) {
        // off_suit_card is guaranteed to be off-suit by construction

        use backend::domain::state::init_round;

        // Player 0 has a card of the led suit AND an off-suit card
        let lead_card = Card { suit: lead_suit, rank: lead_rank };
        let hands = [vec![lead_card, off_suit_card], vec![], vec![], vec![]];
        let mut state = init_round(1, 5, hands, 0, [0; 4]);
        state.phase = Phase::Trick { trick_no: 1 };
        state.turn = 0;
        state.leader = 0;
        state.round.trump = Some(Trump::NoTrump);

        // Set up trick_lead as if someone already played
        state.round.trick_lead = Some(lead_suit);
        state.round.trick_plays.push((1, Card { suit: lead_suit, rank: Rank::Two }));

        // Try to play the off-suit card when we have lead suit
        let result = play_card(&mut state, 0, off_suit_card);

        prop_assert!(result.is_err(),
            "Playing off-suit when holding lead suit should fail");

        if let Err(DomainError::Validation(kind, _)) = result {
            prop_assert_eq!(kind, ValidationKind::MustFollowSuit,
                "Should be MustFollowSuit error");
        }
    }

    /// Property: Can play any card when void in led suit
    #[test]
    fn prop_can_play_any_when_void(
        (lead_suit, player_card) in prop::strategy::Strategy::prop_flat_map(
            crate::support::domain_gens::suit(),
            |lead| {
                let other_suits = vec![Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
                    .into_iter()
                    .filter(|s| *s != lead)
                    .collect::<Vec<_>>();
                (
                    Just(lead),
                    (prop::sample::select(other_suits), crate::support::domain_gens::rank())
                        .prop_map(|(s, r)| Card { suit: s, rank: r })
                )
            }
        ),
    ) {
        // player_card is guaranteed to NOT be of the led suit by construction

        use backend::domain::state::init_round;

        // Player 0 has only cards NOT of the led suit
        let hands = [vec![player_card], vec![], vec![], vec![]];
        let mut state = init_round(1, 5, hands, 0, [0; 4]);
        state.phase = Phase::Trick { trick_no: 1 };
        state.turn = 0;
        state.leader = 0;
        state.round.trump = Some(Trump::NoTrump);

        // Set up trick_lead as if someone already played
        state.round.trick_lead = Some(lead_suit);
        state.round.trick_plays.push((1, Card { suit: lead_suit, rank: Rank::Ace }));

        // Should be able to play any card since void in lead suit
        let legal = legal_moves(&state, 0);

        prop_assert!(legal.contains(&player_card),
            "Player void in lead suit should be able to play {player_card:?}");
    }
}

/// Test: Legal moves when holding lead suit
#[test]
fn test_legal_moves_with_lead_suit() {
    use backend::domain::state::init_round;

    // Player has: 2H, 5H, 7C
    // Lead suit is Hearts
    let hands = [
        vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Five,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Seven,
            },
        ],
        vec![],
        vec![],
        vec![],
    ];
    let mut state = init_round(1, 5, hands, 0, [0; 4]);
    state.phase = Phase::Trick { trick_no: 1 };
    state.turn = 0;
    state.round.trick_lead = Some(Suit::Hearts);

    let legal = legal_moves(&state, 0);

    // Should only allow Hearts
    assert_eq!(legal.len(), 2, "Should have 2 legal moves (both Hearts)");
    assert!(legal.contains(&Card {
        suit: Suit::Hearts,
        rank: Rank::Two
    }));
    assert!(legal.contains(&Card {
        suit: Suit::Hearts,
        rank: Rank::Five
    }));
    assert!(!legal.contains(&Card {
        suit: Suit::Clubs,
        rank: Rank::Seven
    }));
}

/// Test: Legal moves when void in lead suit
#[test]
fn test_legal_moves_when_void() {
    use backend::domain::state::init_round;

    // Player has: 7C, KD, AS
    // Lead suit is Hearts (player is void)
    let hands = [
        vec![
            Card {
                suit: Suit::Clubs,
                rank: Rank::Seven,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ],
        vec![],
        vec![],
        vec![],
    ];
    let mut state = init_round(1, 5, hands, 0, [0; 4]);
    state.phase = Phase::Trick { trick_no: 1 };
    state.turn = 0;
    state.round.trick_lead = Some(Suit::Hearts);

    let legal = legal_moves(&state, 0);

    // Should allow all cards
    assert_eq!(legal.len(), 3, "Should have 3 legal moves (all cards)");
    assert!(legal.contains(&Card {
        suit: Suit::Clubs,
        rank: Rank::Seven
    }));
    assert!(legal.contains(&Card {
        suit: Suit::Diamonds,
        rank: Rank::King
    }));
    assert!(legal.contains(&Card {
        suit: Suit::Spades,
        rank: Rank::Ace
    }));
}

/// Test: Legal moves on first play of trick
#[test]
fn test_legal_moves_first_play() {
    use backend::domain::state::init_round;

    // Player has: 2H, 5H, 7C
    // No lead suit yet (first to play)
    let hands = [
        vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Five,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Seven,
            },
        ],
        vec![],
        vec![],
        vec![],
    ];
    let mut state = init_round(1, 5, hands, 0, [0; 4]);
    state.phase = Phase::Trick { trick_no: 1 };
    state.turn = 0;
    state.leader = 0;
    state.round.trick_lead = None;

    let legal = legal_moves(&state, 0);

    // Should allow all cards (no lead suit yet)
    assert_eq!(legal.len(), 3, "Should have 3 legal moves (no lead suit)");
}

/// Test: Cannot play card not in hand
#[test]
fn test_cannot_play_card_not_in_hand() {
    use backend::domain::state::init_round;

    // Player has: 2H
    let hands = [
        vec![Card {
            suit: Suit::Hearts,
            rank: Rank::Two,
        }],
        vec![],
        vec![],
        vec![],
    ];
    let mut state = init_round(1, 5, hands, 0, [0; 4]);
    state.phase = Phase::Trick { trick_no: 1 };
    state.turn = 0;
    state.leader = 0;
    state.round.trump = Some(Trump::NoTrump);

    // Try to play a card not in hand
    let not_in_hand = Card {
        suit: Suit::Spades,
        rank: Rank::Ace,
    };
    let result = play_card(&mut state, 0, not_in_hand);

    assert!(result.is_err(), "Playing card not in hand should fail");

    match result {
        Err(DomainError::Validation(ValidationKind::CardNotInHand, _)) => {
            // Expected
        }
        _ => panic!("Expected CardNotInHand validation error"),
    }
}

/// Test: Cannot play out of turn
#[test]
fn test_cannot_play_out_of_turn() {
    use backend::domain::state::init_round;

    // Player 1 has a card
    let hands = [
        vec![],
        vec![Card {
            suit: Suit::Hearts,
            rank: Rank::Two,
        }],
        vec![],
        vec![],
    ];
    let mut state = init_round(1, 5, hands, 0, [0; 4]);
    state.phase = Phase::Trick { trick_no: 1 };
    state.turn = 0; // Player 0's turn
    state.leader = 0;
    state.round.trump = Some(Trump::NoTrump);

    // Player 1 tries to play when it's player 0's turn
    let result = play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Two,
        },
    );

    assert!(result.is_err(), "Playing out of turn should fail");

    match result {
        Err(DomainError::Validation(ValidationKind::OutOfTurn, _)) => {
            // Expected
        }
        _ => panic!("Expected OutOfTurn validation error"),
    }
}

/// Test: Cannot play in wrong phase
#[test]
fn test_cannot_play_in_wrong_phase() {
    use backend::domain::state::init_round;

    let hands = [
        vec![Card {
            suit: Suit::Hearts,
            rank: Rank::Two,
        }],
        vec![],
        vec![],
        vec![],
    ];
    let mut state = init_round(1, 5, hands, 0, [0; 4]);
    state.phase = Phase::Bidding; // Wrong phase
    state.turn = 0;

    let result = play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Two,
        },
    );

    assert!(result.is_err(), "Playing in wrong phase should fail");

    match result {
        Err(DomainError::Validation(ValidationKind::PhaseMismatch, _)) => {
            // Expected
        }
        _ => panic!("Expected PhaseMismatch validation error"),
    }
}

/// Test: Trick winner with no trump (highest card of led suit wins)
#[test]
fn test_trick_winner_no_trump() {
    use backend::domain::state::init_round;

    // Set up 4 players with one card each
    let hands = [
        vec![Card {
            suit: Suit::Hearts,
            rank: Rank::Five,
        }],
        vec![Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        }], // Highest
        vec![Card {
            suit: Suit::Hearts,
            rank: Rank::Three,
        }],
        vec![Card {
            suit: Suit::Clubs,
            rank: Rank::King,
        }], // Off-suit
    ];
    let mut state = init_round(1, 4, hands, 0, [0; 4]);
    state.phase = Phase::Trick { trick_no: 1 };
    state.turn = 0;
    state.leader = 0;
    state.round.trump = Some(Trump::NoTrump);

    // Play all 4 cards
    assert!(play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Five
        }
    )
    .is_ok());
    assert!(play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ace
        }
    )
    .is_ok());
    assert!(play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Three
        }
    )
    .is_ok());
    assert!(play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Clubs,
            rank: Rank::King
        }
    )
    .is_ok());

    // Player 1 (Ace of Hearts) should have won the trick
    assert_eq!(
        state.round.tricks_won[1], 1,
        "Player 1 should have won the trick"
    );

    // Next leader should be player 1
    assert_eq!(state.leader, 1, "Player 1 should be next leader");
}

/// Test: Trick winner with trump (highest trump wins)
#[test]
fn test_trick_winner_with_trump() {
    use backend::domain::state::init_round;

    // Set up 4 players with one card each
    let hands = [
        vec![Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        }], // Led, but not trump
        vec![Card {
            suit: Suit::Hearts,
            rank: Rank::King,
        }],
        vec![Card {
            suit: Suit::Spades,
            rank: Rank::Two,
        }], // Trump! (lowest trump)
        vec![Card {
            suit: Suit::Spades,
            rank: Rank::Five,
        }], // Trump! (higher)
    ];
    let mut state = init_round(1, 4, hands, 0, [0; 4]);
    state.phase = Phase::Trick { trick_no: 1 };
    state.turn = 0;
    state.leader = 0;
    state.round.trump = Some(Trump::Spades); // Spades is trump

    // Play all 4 cards
    assert!(play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ace
        }
    )
    .is_ok());
    assert!(play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Hearts,
            rank: Rank::King
        }
    )
    .is_ok());
    assert!(play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Spades,
            rank: Rank::Two
        }
    )
    .is_ok());
    assert!(play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Spades,
            rank: Rank::Five
        }
    )
    .is_ok());

    // Player 3 (5 of Spades, highest trump) should have won
    assert_eq!(
        state.round.tricks_won[3], 1,
        "Player 3 should have won with highest trump"
    );

    // Next leader should be player 3
    assert_eq!(state.leader, 3, "Player 3 should be next leader");
}

/// Test: Complete trick advances to next trick or scoring
#[test]
fn test_complete_trick_advances_phase() {
    use backend::domain::state::init_round;

    // Test case 1: Not last trick
    // All players have 2 cards
    let hands = [
        vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            },
        ],
        vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            },
        ],
        vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            },
        ],
        vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            },
        ],
    ];
    let mut state = init_round(1, 2, hands, 0, [0; 4]);
    state.phase = Phase::Trick { trick_no: 1 };
    state.trick_no = 1;
    state.turn = 0;
    state.leader = 0;
    state.round.trump = Some(Trump::NoTrump);

    // Play first trick (all Hearts)
    assert!(play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Two
        }
    )
    .is_ok());
    assert!(play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Two
        }
    )
    .is_ok());
    assert!(play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Two
        }
    )
    .is_ok());
    assert!(play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Two
        }
    )
    .is_ok());

    // Should advance to trick 2
    assert_eq!(state.trick_no, 2, "Should be on trick 2");
    assert_eq!(
        state.phase,
        Phase::Trick { trick_no: 2 },
        "Should be in Trick phase for trick 2"
    );

    // Test case 2: Last trick
    // Play second trick (all Clubs)
    let winner = state.leader;
    assert!(play_card(
        &mut state,
        winner,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Two
        }
    )
    .is_ok());
    let next = (winner + 1) % 4;
    assert!(play_card(
        &mut state,
        next,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Two
        }
    )
    .is_ok());
    let next = (next + 1) % 4;
    assert!(play_card(
        &mut state,
        next,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Two
        }
    )
    .is_ok());
    let next = (next + 1) % 4;
    assert!(play_card(
        &mut state,
        next,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Two
        }
    )
    .is_ok());

    // Should advance to Scoring phase
    assert_eq!(
        state.phase,
        Phase::Scoring,
        "Should be in Scoring phase after last trick"
    );
}
