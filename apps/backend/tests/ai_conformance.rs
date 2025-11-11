use backend::ai::registry::{registered_ais, AiFactory};
use testkit::{assert_bid_legal, assert_play_legal, assert_trump_legal};

mod testkit {
    use backend::domain::player_view::CurrentRoundInfo;
    use backend::domain::{Card, GameContext, Rank, Suit, Trump};
    use backend::entities::games::GameState as DbGameState;

    #[derive(Clone)]
    pub struct Scenario {
        pub state: CurrentRoundInfo,
        pub context: GameContext,
    }

    pub fn dealer_restriction_scenario() -> Scenario {
        let hand = vec![
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Five,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Seven,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Nine,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ace,
            },
        ];

        let state = CurrentRoundInfo {
            game_id: 1,
            player_seat: 0,
            game_state: DbGameState::Bidding,
            current_round: 1,
            hand_size: 5,
            dealer_pos: 0,
            hand,
            bids: [None, Some(2), Some(1), Some(1)],
            trump: None,
            trick_no: 0,
            current_trick_plays: Vec::new(),
            scores: [0, 0, 0, 0],
            trick_leader: None,
        };

        let context = build_context(&state);
        Scenario { state, context }
    }

    pub fn must_follow_suit_scenario() -> Scenario {
        let leader_card = Card {
            suit: Suit::Hearts,
            rank: Rank::Queen,
        };

        let hand = vec![
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ten,
            },
            Card {
                suit: Suit::Clubs,
                rank: Rank::Jack,
            },
        ];

        let state = CurrentRoundInfo {
            game_id: 2,
            player_seat: 2,
            game_state: DbGameState::TrickPlay,
            current_round: 1,
            hand_size: 3,
            dealer_pos: 1,
            hand,
            bids: [Some(2), Some(1), Some(1), Some(0)],
            trump: Some(Trump::Spades),
            trick_no: 1,
            current_trick_plays: vec![(1, leader_card)],
            scores: [10, 8, 6, 4],
            trick_leader: Some(1),
        };

        let context = build_context(&state);
        Scenario { state, context }
    }

    pub fn trump_selection_scenario() -> Scenario {
        let hand = vec![
            Card {
                suit: Suit::Clubs,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Spades,
                rank: Rank::Three,
            },
        ];

        let state = CurrentRoundInfo {
            game_id: 3,
            player_seat: 0,
            game_state: DbGameState::TrumpSelection,
            current_round: 1,
            hand_size: 3,
            dealer_pos: 0,
            hand,
            bids: [Some(3), Some(1), Some(0), Some(2)],
            trump: None,
            trick_no: 0,
            current_trick_plays: Vec::new(),
            scores: [12, 9, 7, 5],
            trick_leader: None,
        };

        let context = build_context(&state);
        Scenario { state, context }
    }

    pub fn assert_bid_legal(bid: u8, state: &CurrentRoundInfo) {
        let legal = state.legal_bids();
        assert!(
            legal.contains(&bid),
            "Bid {bid} must be within legal options: {legal:?}"
        );
    }

    pub fn assert_play_legal(card: Card, state: &CurrentRoundInfo) {
        let legal = state.legal_plays();
        assert!(
            legal.contains(&card),
            "Card {:?} must be within legal plays: {:?}",
            card,
            legal
        );
    }

    pub fn assert_trump_legal(trump: Trump, state: &CurrentRoundInfo) {
        let legal = state.legal_trumps();
        assert!(
            legal.contains(&trump),
            "Trump {:?} must be within legal trumps: {:?}",
            trump,
            legal
        );
    }

    fn build_context(state: &CurrentRoundInfo) -> GameContext {
        GameContext::new(state.game_id).with_round_info(state.clone())
    }
}

#[test]
fn ai_conformance_suite() {
    let factories = registered_ais();
    assert!(
        !factories.is_empty(),
        "Registry must expose at least one AI factory"
    );

    for factory in factories {
        println!(
            "Running AI conformance checks for {} v{}",
            factory.name, factory.version
        );
        run_dealer_restriction(factory);
        run_must_follow_suit(factory);
        run_determinism(factory);
    }
}

fn run_dealer_restriction(factory: &AiFactory) {
    let scenario = testkit::dealer_restriction_scenario();
    let ai = (factory.make)(Some(101));
    let bid = ai
        .choose_bid(&scenario.state, &scenario.context)
        .expect("AI should produce a bid");
    assert_bid_legal(bid, &scenario.state);
}

fn run_must_follow_suit(factory: &AiFactory) {
    let scenario = testkit::must_follow_suit_scenario();
    let ai = (factory.make)(Some(202));
    let card = ai
        .choose_play(&scenario.state, &scenario.context)
        .expect("AI should produce a card to play");
    assert_play_legal(card, &scenario.state);
}

fn run_determinism(factory: &AiFactory) {
    let bidding = testkit::dealer_restriction_scenario();
    let bid_seed = 303;
    let bid_a = (factory.make)(Some(bid_seed))
        .choose_bid(&bidding.state, &bidding.context)
        .expect("AI should produce a bid for determinism check");
    let bid_b = (factory.make)(Some(bid_seed))
        .choose_bid(&bidding.state, &bidding.context)
        .expect("AI should reproduce the same bid with identical seed");
    assert_eq!(
        bid_a, bid_b,
        "AI bid must be deterministic for identical seeds"
    );

    let plays = testkit::must_follow_suit_scenario();
    let play_seed = 404;
    let play_a = (factory.make)(Some(play_seed))
        .choose_play(&plays.state, &plays.context)
        .expect("AI should produce a play for determinism check");
    let play_b = (factory.make)(Some(play_seed))
        .choose_play(&plays.state, &plays.context)
        .expect("AI should reproduce the same play with identical seed");
    assert_eq!(
        play_a, play_b,
        "AI play must be deterministic for identical seeds"
    );
    assert_play_legal(play_a, &plays.state);

    let trumps = testkit::trump_selection_scenario();
    let trump_seed = 505;
    let trump_a = (factory.make)(Some(trump_seed))
        .choose_trump(&trumps.state, &trumps.context)
        .expect("AI should produce a trump selection");
    let trump_b = (factory.make)(Some(trump_seed))
        .choose_trump(&trumps.state, &trumps.context)
        .expect("AI should reproduce the same trump with identical seed");
    assert_eq!(
        trump_a, trump_b,
        "AI trump selection must be deterministic for identical seeds"
    );
    assert_trump_legal(trump_a, &trumps.state);
}
