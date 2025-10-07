#![cfg(test)]

use super::*;
use crate::domain::bidding::{place_bid, Bid};
use crate::domain::cards::{Card, Rank, Suit, Trump};
use crate::domain::fixtures::CardFixtures;
use crate::domain::scoring::apply_round_scoring;
use crate::domain::tricks::{legal_moves, play_card, resolve_current_trick};
use crate::errors::domain::{DomainError, ValidationKind};

fn make_state_with_hands(hands: [Vec<Card>; 4], hand_size: u8, turn_start: PlayerId) -> GameState {
    GameState {
        phase: Phase::Bidding,
        round_no: 1,
        hand_size,
        hands,
        turn_start,
        turn: turn_start,
        leader: turn_start,
        trick_no: 0,
        scores_total: [0; 4],
        round: RoundState::new(),
    }
}

#[test]
fn bidding_legal_range_and_phase_turning() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 5, 0);

    let bids = crate::domain::bidding::legal_bids(&state, 0);
    assert_eq!(bids.first().unwrap().value(), 0);
    assert_eq!(bids.last().unwrap().value(), 5);

    assert!(place_bid(&mut state, 0, Bid(3)).is_ok());
    assert!(place_bid(&mut state, 1, Bid(4)).is_ok());
    assert!(place_bid(&mut state, 2, Bid(1)).is_ok());
    assert!(place_bid(&mut state, 3, Bid(4)).is_ok());
    // Highest is 4; tie between player 1 and 3; start is 0 so earliest in order is 1
    assert_eq!(state.phase, Phase::TrumpSelect);
    assert_eq!(state.round.winning_bidder, Some(1));
}

#[test]
fn trump_selection_only_by_winner() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 3, 0);
    for (p, b) in [(0, 0), (1, 2), (2, 2), (3, 1)] {
        assert!(place_bid(&mut state, p, Bid(b)).is_ok());
    }
    assert_eq!(state.round.winning_bidder, Some(1));
    // wrong player
    assert!(crate::domain::set_trump(&mut state, 0, Trump::Hearts).is_err());
    // correct player
    assert!(crate::domain::set_trump(&mut state, 1, Trump::Spades).is_ok());
    assert_eq!(state.phase, Phase::Trick { trick_no: 1 });
    assert_eq!(state.leader, 1);
    assert_eq!(state.turn, 1);
    assert_eq!(state.round.trump, Some(Trump::Spades));
    assert!(state.round.trick_plays.is_empty());
    assert!(state.round.trick_lead.is_none());
}

#[test]
fn legal_moves_follow_lead() {
    // Hands for a small test
    let h0 = CardFixtures::parse_hardcoded(&["AS", "KH", "2C"]);
    let h1 = CardFixtures::parse_hardcoded(&["TS", "3H", "4C"]);
    let h2 = CardFixtures::parse_hardcoded(&["QS", "5D", "6C"]);
    let h3 = CardFixtures::parse_hardcoded(&["9S", "7H", "8C"]);
    let mut state = make_state_with_hands([h0, h1, h2, h3], 3, 0);
    for p in 0..4 {
        assert!(place_bid(&mut state, p, Bid(0)).is_ok());
    }
    crate::domain::set_trump(&mut state, 0, Trump::Hearts).unwrap();
    // First to play can play any
    let lm0 = legal_moves(&state, 0);
    assert_eq!(lm0.len(), 3);
    // Play AS -> lead Spades
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    // Player 1 must follow spades if possible
    let lm1 = legal_moves(&state, 1);
    assert!(lm1.iter().all(|c| c.suit == Suit::Spades) && !lm1.is_empty());
}

#[test]
fn play_card_errors_and_trick_resolution() {
    let h0 = CardFixtures::parse_hardcoded(&["AS", "KH", "2C"]);
    let h1 = CardFixtures::parse_hardcoded(&["TS", "3H", "4C"]);
    let h2 = CardFixtures::parse_hardcoded(&["QS", "5D", "6C"]);
    let h3 = CardFixtures::parse_hardcoded(&["9S", "7H", "8C"]);
    let mut state = make_state_with_hands([h0, h1, h2, h3], 3, 0);
    for p in 0..4 {
        place_bid(&mut state, p, Bid(0)).unwrap();
    }
    crate::domain::set_trump(&mut state, 0, Trump::Hearts).unwrap();
    // Out of turn
    assert_eq!(
        play_card(
            &mut state,
            1,
            Card {
                suit: Suit::Spades,
                rank: Rank::Ten
            }
        )
        .unwrap_err(),
        DomainError::validation(ValidationKind::OutOfTurn, "Out of turn")
    );
    // Not in hand
    assert_eq!(
        play_card(
            &mut state,
            0,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace
            }
        )
        .unwrap_err(),
        DomainError::validation(ValidationKind::CardNotInHand, "Card not in hand")
    );
    // Play trick fully
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Spades,
            rank: Rank::Ten,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Spades,
            rank: Rank::Queen,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Spades,
            rank: Rank::Nine,
        },
    )
    .unwrap();
    // Highest trump is none in trick; lead spades so Ace wins -> player 0 leads next
    assert_eq!(state.leader, 0);
    assert_eq!(state.turn, 0);
    assert_eq!(state.round.tricks_won[0], 1);
}

#[test]
fn resolve_trick_multiple_cases() {
    // Create a RoundState with a full trick
    let mut r = RoundState::new();
    r.trump = Some(Trump::Hearts);
    r.trick_lead = Some(Suit::Clubs);
    r.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ten,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Hearts,
                rank: Rank::King,
            },
        ),
    ];
    // With trump hearts, player 3 wins (KH > 2H)
    assert_eq!(resolve_current_trick(&r), Some(3));

    // No trump played: highest of lead
    let mut r2 = RoundState::new();
    r2.trump = Some(Trump::Spades);
    r2.trick_lead = Some(Suit::Diamonds);
    r2.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Nine,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ace,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Queen,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
        ),
    ];
    assert_eq!(resolve_current_trick(&r2), Some(2));
}

#[test]
fn scoring_bonus_only_on_exact_bid() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 3, 0);
    // Fake some tallies - sum must equal hand_size
    state.round.tricks_won = [2, 1, 0, 0]; // Sum = 3, matches hand_size = 3
    state.round.bids = [Some(2), Some(0), Some(1), Some(0)];
    state.phase = Phase::Scoring;
    apply_round_scoring(&mut state);
    assert_eq!(state.scores_total, [12, 1, 0, 10]);
    assert_eq!(state.phase, Phase::Complete);
}

#[test]
fn happy_path_round_small() {
    // Hand size 3; deterministic hands
    let h0 = CardFixtures::parse_hardcoded(&["AS", "KH", "2C"]);
    let h1 = CardFixtures::parse_hardcoded(&["TS", "3H", "4C"]);
    let h2 = CardFixtures::parse_hardcoded(&["QS", "5D", "6C"]);
    let h3 = CardFixtures::parse_hardcoded(&["9S", "7H", "8C"]);
    let mut state = make_state_with_hands([h0, h1, h2, h3], 3, 0);
    // Bidding: p1 wins with 2 against ties by order
    place_bid(&mut state, 0, Bid(1)).unwrap();
    place_bid(&mut state, 1, Bid(2)).unwrap();
    place_bid(&mut state, 2, Bid(2)).unwrap();
    place_bid(&mut state, 3, Bid(1)).unwrap();
    assert_eq!(state.round.winning_bidder, Some(1));
    crate::domain::set_trump(&mut state, 1, Trump::Hearts).unwrap();
    // Trick 1: lead 1
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Spades,
            rank: Rank::Ten,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Spades,
            rank: Rank::Queen,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Spades,
            rank: Rank::Nine,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    assert_eq!(state.leader, 0);
    // Trick 2: lead 0
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Hearts,
            rank: Rank::King,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Three,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Five,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Seven,
        },
    )
    .unwrap();
    assert_eq!(state.leader, 0);
    // Trick 3: lead 0
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Two,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Four,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Six,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Eight,
        },
    )
    .unwrap();
    assert_eq!(state.phase, Phase::Scoring);
    apply_round_scoring(&mut state);
    // Player 0 bid 1, won 2 tricks: 2 points (no bonus)
    // Player 1 bid 2, won 0 tricks: 0 points (no bonus)
    // Player 2 bid 2, won 0 tricks: 0 points
    // Player 3 bid 1, won 1 trick: 1 point (exact bid = 10 point bonus)
    assert_eq!(state.scores_total, [2, 0, 0, 11]);
}

#[test]
fn trump_conversions() {
    // From<Suit> for Trump
    assert_eq!(Trump::from(Suit::Clubs), Trump::Clubs);
    assert_eq!(Trump::from(Suit::Diamonds), Trump::Diamonds);
    assert_eq!(Trump::from(Suit::Hearts), Trump::Hearts);
    assert_eq!(Trump::from(Suit::Spades), Trump::Spades);

    // TryFrom<Trump> for Suit - success cases
    use std::convert::TryInto;
    assert_eq!(Trump::Clubs.try_into(), Ok(Suit::Clubs));
    assert_eq!(Trump::Diamonds.try_into(), Ok(Suit::Diamonds));
    assert_eq!(Trump::Hearts.try_into(), Ok(Suit::Hearts));
    assert_eq!(Trump::Spades.try_into(), Ok(Suit::Spades));

    // TryFrom<Trump> for Suit - NoTrump fails
    let result: Result<Suit, _> = Trump::NoTrump.try_into();
    assert_eq!(
        result,
        Err(DomainError::validation(
            ValidationKind::InvalidTrumpConversion,
            "Cannot convert NoTrump to Suit"
        ))
    );
}

#[test]
fn trump_serde() {
    // Test SCREAMING_SNAKE_CASE serialization
    assert_eq!(serde_json::to_string(&Trump::Clubs).unwrap(), "\"CLUBS\"");
    assert_eq!(
        serde_json::to_string(&Trump::Diamonds).unwrap(),
        "\"DIAMONDS\""
    );
    assert_eq!(serde_json::to_string(&Trump::Hearts).unwrap(), "\"HEARTS\"");
    assert_eq!(serde_json::to_string(&Trump::Spades).unwrap(), "\"SPADES\"");
    assert_eq!(
        serde_json::to_string(&Trump::NoTrump).unwrap(),
        "\"NO_TRUMP\""
    );

    // Test deserialization
    assert_eq!(
        serde_json::from_str::<Trump>("\"CLUBS\"").unwrap(),
        Trump::Clubs
    );
    assert_eq!(
        serde_json::from_str::<Trump>("\"DIAMONDS\"").unwrap(),
        Trump::Diamonds
    );
    assert_eq!(
        serde_json::from_str::<Trump>("\"HEARTS\"").unwrap(),
        Trump::Hearts
    );
    assert_eq!(
        serde_json::from_str::<Trump>("\"SPADES\"").unwrap(),
        Trump::Spades
    );
    assert_eq!(
        serde_json::from_str::<Trump>("\"NO_TRUMP\"").unwrap(),
        Trump::NoTrump
    );
}

#[test]
fn trump_selection_allows_no_trump() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 3, 0);
    for (p, b) in [(0, 0), (1, 2), (2, 2), (3, 1)] {
        assert!(place_bid(&mut state, p, Bid(b)).is_ok());
    }
    assert_eq!(state.round.winning_bidder, Some(1));

    // Winning bidder can select NoTrump
    assert!(crate::domain::set_trump(&mut state, 1, Trump::NoTrump).is_ok());
    assert_eq!(state.phase, Phase::Trick { trick_no: 1 });
    assert_eq!(state.round.trump, Some(Trump::NoTrump));
}

#[test]
fn trick_resolution_no_trump() {
    // With NoTrump, only the lead suit matters
    let mut r = RoundState::new();
    r.trump = Some(Trump::NoTrump);
    r.trick_lead = Some(Suit::Diamonds);
    r.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Nine,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ), // Highest card, but wrong suit
        (
            2,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Queen,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
        ), // Another high card, wrong suit
    ];
    // Player 2 wins with QD (highest of lead suit)
    assert_eq!(resolve_current_trick(&r), Some(2));

    // Another case: all follow lead suit
    let mut r2 = RoundState::new();
    r2.trump = Some(Trump::NoTrump);
    r2.trick_lead = Some(Suit::Clubs);
    r2.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ten,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Clubs,
                rank: Rank::King,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ace,
            },
        ),
    ];
    // Player 3 wins with AC (highest of lead suit)
    assert_eq!(resolve_current_trick(&r2), Some(3));
}

#[test]
fn resolve_trick_trump_arrives_late_and_wins() {
    // "Trump arrives late and wins": lead=Diamonds, trump=Spades; plays:
    // 9♦, K♦, 2♠, A♦ → player who played 2♠ wins
    let mut r = RoundState::new();
    r.trump = Some(Trump::Spades);
    r.trick_lead = Some(Suit::Diamonds);
    r.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Nine,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::King,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Spades,
                rank: Rank::Two,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
        ),
    ];

    // Verify trick_lead invariant
    assert_eq!(r.trick_plays[0].1.suit, r.trick_lead.unwrap());

    // Player 2 wins with 2♠ (trump beats lead)
    assert_eq!(resolve_current_trick(&r), Some(2));
}

#[test]
fn resolve_trick_multiple_trumps_highest_trump_wins() {
    // "Multiple trumps: highest trump wins": lead=Hearts, trump=Spades; plays
    // include Q♠, 3♠, A♠ → A♠ wins
    let mut r = RoundState::new();
    r.trump = Some(Trump::Spades);
    r.trick_lead = Some(Suit::Hearts);
    r.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Spades,
                rank: Rank::Queen,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Spades,
                rank: Rank::Three,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ),
    ];

    // Verify trick_lead invariant
    assert_eq!(r.trick_plays[0].1.suit, r.trick_lead.unwrap());

    // Player 3 wins with A♠ (highest trump)
    assert_eq!(resolve_current_trick(&r), Some(3));
}

#[test]
fn resolve_trick_notrump_only_lead_matters() {
    // "NoTrump: only lead matters even if off-suit ranks are higher": lead=Clubs,
    // trump=NO_TRUMP; only one clubs card vs three off-suits; the lone clubs card wins
    let mut r = RoundState::new();
    r.trump = Some(Trump::NoTrump);
    r.trick_lead = Some(Suit::Clubs);
    r.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::King,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Hearts,
                rank: Rank::Queen,
            },
        ),
    ];

    // Verify trick_lead invariant
    assert_eq!(r.trick_plays[0].1.suit, r.trick_lead.unwrap());

    // Player 0 wins with 2♣ (only clubs card, lead suit)
    assert_eq!(resolve_current_trick(&r), Some(0));
}

#[test]
fn scoring_exact_bid_bonus_applied_once() {
    // "Exact-bid bonus applied once": bids [3,2,4,1], tricks [3,2,7,1] → totals [13,12,7,11]
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 13, 0);
    state.round.bids = [Some(3), Some(2), Some(4), Some(1)];
    state.round.tricks_won = [3, 2, 7, 1];
    state.phase = Phase::Scoring;

    // Verify sum-of-tricks invariant before scoring
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_eq!(tricks_sum, state.hand_size);

    apply_round_scoring(&mut state);

    // Expected: [3+10, 2+10, 7+0, 1+10] = [13, 12, 7, 11]
    assert_eq!(state.scores_total, [13, 12, 7, 11]);
    assert_eq!(state.phase, Phase::Complete);
}

#[test]
fn scoring_notrump_does_not_affect_scoring_math() {
    // "NoTrump does not affect scoring math": bids [0,5,8,0], tricks [0,5,8,0] → totals [10,15,18,10]
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 13, 0);
    state.round.bids = [Some(0), Some(5), Some(8), Some(0)];
    state.round.tricks_won = [0, 5, 8, 0];
    state.round.trump = Some(Trump::NoTrump); // NoTrump setting
    state.phase = Phase::Scoring;

    // Verify sum-of-tricks invariant before scoring
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_eq!(tricks_sum, state.hand_size);

    apply_round_scoring(&mut state);

    // Expected: [0+10, 5+10, 8+10, 0+10] = [10, 15, 18, 10]
    assert_eq!(state.scores_total, [10, 15, 18, 10]);
    assert_eq!(state.phase, Phase::Complete);
}

#[test]
fn scoring_idempotence_scoring_applies_once_only() {
    // "Idempotence: scoring applies once only": call apply_round_scoring twice;
    // the second call (after phase is Complete) must leave totals unchanged
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 5, 0);
    state.round.bids = [Some(2), Some(1), Some(2), Some(0)];
    state.round.tricks_won = [2, 1, 2, 0];
    state.phase = Phase::Scoring;

    // Verify sum-of-tricks invariant before scoring
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_eq!(tricks_sum, state.hand_size);

    // First scoring call
    apply_round_scoring(&mut state);
    let scores_after_first = state.scores_total;
    assert_eq!(state.phase, Phase::Complete);

    // Second scoring call should be no-op
    apply_round_scoring(&mut state);
    assert_eq!(state.scores_total, scores_after_first);
    assert_eq!(state.phase, Phase::Complete);
}

#[test]
fn scoring_sum_of_tricks_invariant_violation_release_variant() {
    // Construct a state with hand_size = N and intentionally make
    // tricks_won sum != N. Assert on the sum inside the test **before**
    // invoking scoring. This documents the invariant.
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 5, 0);
    state.round.bids = [Some(2), Some(1), Some(2), Some(0)];
    state.round.tricks_won = [3, 1, 2, 0]; // Sum = 6, but hand_size = 5
    state.phase = Phase::Scoring;

    // Verify sum-of-tricks invariant violation before scoring
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_ne!(
        tricks_sum, state.hand_size,
        "Test intentionally violates sum-of-tricks invariant"
    );

    // In non-debug (release) builds, scoring should not panic and produce deterministic totals
    #[cfg(not(debug_assertions))]
    {
        apply_round_scoring(&mut state);
        assert_eq!(state.scores_total, [13, 11, 12, 10]);
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Sum of tricks")]
fn scoring_sum_of_tricks_invariant_violation_debug_variant() {
    // Construct the same invalid state as the release variant
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 5, 0);
    state.round.bids = [Some(2), Some(1), Some(2), Some(0)];
    state.round.tricks_won = [3, 1, 2, 0]; // Sum = 6, but hand_size = 5
    state.phase = Phase::Scoring;

    // Sanity-check the invariant is indeed violated
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_ne!(tricks_sum, state.hand_size);

    // In debug builds, this should panic due to the internal debug assertion
    apply_round_scoring(&mut state);
}
