use backend::adapters::games_sea::{self, GameCreate, GameUpdateRound};
use backend::db::txn::with_txn;
use backend::domain::state::Phase;
use backend::error::AppError;
use backend::repos::{games, rounds, tricks};
use backend::services::game_flow::GameFlowService;
use backend::services::games::GameService;
use backend::utils::join_code::generate_join_code;

use crate::support::build_test_state;
use crate::support::game_phases::{
    setup_game_in_bidding_phase, setup_game_in_trick_play_phase,
    setup_game_in_trump_selection_phase,
};
use crate::support::trick_helpers::create_tricks_by_winner_counts;

/// Test: Load game state from database after dealing
#[tokio::test]
async fn test_load_state_after_deal() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let setup = setup_game_in_bidding_phase(txn, "state_load_bidding").await?;
            let game_service = GameService;

            let loaded_state = game_service.load_game_state(txn, setup.game_id).await?;

            assert_eq!(loaded_state.phase, Phase::Bidding);
            assert_eq!(loaded_state.round_no, 1);
            assert_eq!(loaded_state.hand_size, 13); // Round 1 has 13 cards
            assert_eq!(loaded_state.scores_total, [0, 0, 0, 0]);

            assert_eq!(loaded_state.hands[0].len(), 13);
            assert_eq!(loaded_state.hands[1].len(), 13);
            assert_eq!(loaded_state.hands[2].len(), 13);
            assert_eq!(loaded_state.hands[3].len(), 13);

            assert_eq!(loaded_state.round.bids, [None, None, None, None]);
            assert_eq!(loaded_state.round.trump, None);
            assert_eq!(loaded_state.round.tricks_won, [0, 0, 0, 0]);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Load state after bidding completes
#[tokio::test]
async fn test_load_state_after_bidding() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Round 1: dealer at seat 0, bids: 5, 4, 3, 0
            let setup =
                setup_game_in_trump_selection_phase(txn, "state_load_trump", [5, 4, 3, 0]).await?;
            let game_service = GameService;

            let loaded_state = game_service.load_game_state(txn, setup.game_id).await?;

            assert_eq!(loaded_state.phase, Phase::TrumpSelect);

            assert_eq!(loaded_state.round.bids[0], Some(5));
            assert_eq!(loaded_state.round.bids[1], Some(4));
            assert_eq!(loaded_state.round.bids[2], Some(3));
            assert_eq!(loaded_state.round.bids[3], Some(0));

            // Winning bidder should be seat 0 (highest bid)
            assert_eq!(loaded_state.round.winning_bidder, Some(0));

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Load state after trump selection
#[tokio::test]
async fn test_load_state_after_trump() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Round 1: dealer at seat 0, bids: 4, 5, 3, 0, trump: Hearts
            let setup = setup_game_in_trick_play_phase(
                txn,
                "state_load_trick1",
                [4, 5, 3, 0],
                rounds::Trump::Hearts,
            )
            .await?;
            let game_service = GameService;

            let loaded_state = game_service.load_game_state(txn, setup.game_id).await?;

            assert_eq!(loaded_state.phase, Phase::Trick { trick_no: 1 });

            assert_eq!(
                loaded_state.round.trump,
                Some(backend::domain::Trump::Hearts)
            );

            assert_eq!(loaded_state.trick_no, 1);
            assert_eq!(loaded_state.round.trick_plays.len(), 0);
            assert_eq!(loaded_state.round.trick_lead, None);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Load state with trick in progress
#[tokio::test]
async fn test_load_state_mid_trick() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Round 1: dealer at seat 0, bids: 3, 3, 4, 2, trump: Spades
            let setup = setup_game_in_trick_play_phase(
                txn,
                "state_load_trick2",
                [3, 3, 4, 2],
                rounds::Trump::Spades,
            )
            .await?;
            let game_service = GameService;

            // Manually create trick with 2 plays to test mid-trick state loading
            let round = rounds::find_by_id(txn, setup.round_id).await?.unwrap();

            use backend::repos::plays;
            let trick = tricks::create_trick(txn, round.id, 1, tricks::Suit::Hearts, 0).await?;
            plays::create_play(
                txn,
                trick.id,
                0,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "ACE".into(),
                },
                0,
            )
            .await?;
            plays::create_play(
                txn,
                trick.id,
                1,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "KING".into(),
                },
                1,
            )
            .await?;

            let loaded_state = game_service.load_game_state(txn, setup.game_id).await?;

            assert_eq!(loaded_state.round.trick_plays.len(), 2);
            assert_eq!(
                loaded_state.round.trick_lead,
                Some(backend::domain::Suit::Hearts)
            );

            // First play should be Ace of Hearts by player 0
            assert_eq!(loaded_state.round.trick_plays[0].0, 0);
            assert_eq!(
                loaded_state.round.trick_plays[0].1.suit,
                backend::domain::Suit::Hearts
            );
            assert_eq!(
                loaded_state.round.trick_plays[0].1.rank,
                backend::domain::Rank::Ace
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: After the first trick completes and the second trick begins, the loader
/// should report the leader as the first player to act in the new trick and the
/// turn should advance to the next seat.
#[tokio::test]
async fn test_load_state_second_trick_turn_advances() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Round 1 setup: dealer at seat 0, trump Hearts
            let setup = setup_game_in_trick_play_phase(
                txn,
                "state_load_second_trick_turn",
                [4, 5, 3, 0],
                rounds::Trump::Hearts,
            )
            .await?;
            let game_service = GameService;

            // Create completed trick #1 with winner at seat 3
            let round = rounds::find_by_id(txn, setup.round_id)
                .await?
                .expect("round should exist");

            tricks::create_trick(txn, round.id, 1, tricks::Suit::Spades, 3).await?;

            // Advance DB state to trick #2
            let game = games_sea::require_game(txn, setup.game_id).await?;
            let update_round =
                GameUpdateRound::new(setup.game_id, game.lock_version).with_current_trick_no(2);
            games_sea::update_round(txn, update_round).await?;

            // Create trick #2 with first play by seat 3 (winner of trick #1)
            let trick_two =
                tricks::create_trick(txn, round.id, 2, tricks::Suit::Hearts, -1).await?;

            use backend::repos::plays;
            plays::create_play(
                txn,
                trick_two.id,
                3,
                plays::Card {
                    suit: "HEARTS".into(),
                    rank: "TEN".into(),
                },
                0,
            )
            .await?;

            let loaded_state = game_service.load_game_state(txn, setup.game_id).await?;

            assert_eq!(loaded_state.phase, Phase::Trick { trick_no: 2 });
            assert_eq!(loaded_state.round.trick_plays.len(), 1);
            assert_eq!(loaded_state.round.trick_plays[0].0, 3);
            assert_eq!(
                loaded_state.leader, 3,
                "Second trick leader should be seat 3"
            );
            assert_eq!(
                loaded_state.turn, 0,
                "Turn should advance to seat 0 immediately after seat 3 plays"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Played cards should be removed from hands when loading state.
#[tokio::test]
async fn test_load_state_removes_played_cards_from_hands() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let setup = setup_game_in_trick_play_phase(
                txn,
                "state_load_hand_trim",
                [4, 5, 3, 0],
                rounds::Trump::Hearts,
            )
            .await?;
            let game_service = GameService;
            let flow_service = GameFlowService;

            let initial_state = game_service.load_game_state(txn, setup.game_id).await?;
            let acting_seat = initial_state.turn;
            let card_to_play = initial_state.hands[acting_seat as usize][0];

            flow_service
                .play_card(txn, setup.game_id, acting_seat as i16, card_to_play, None)
                .await?;

            let updated_state = game_service.load_game_state(txn, setup.game_id).await?;
            assert!(
                updated_state
                    .round
                    .trick_plays
                    .iter()
                    .any(|(seat, card)| { *seat == acting_seat && *card == card_to_play }),
                "Played card should appear in trick plays"
            );
            assert!(
                !updated_state.hands[acting_seat as usize].contains(&card_to_play),
                "Played card should be removed from the player's hand"
            );

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Load state with cumulative scores
#[tokio::test]
async fn test_load_state_with_scores() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let setup = setup_game_in_bidding_phase(txn, "state_load_current").await?;
            let flow_service = GameFlowService;
            let game_service = GameService;

            // Complete one full round (Round 1: dealer at seat 0, bidding starts at seat 1)
            flow_service
                .submit_bid(txn, setup.game_id, 1, 3, None)
                .await?;
            flow_service
                .submit_bid(txn, setup.game_id, 2, 2, None)
                .await?;
            flow_service
                .submit_bid(txn, setup.game_id, 3, 0, None)
                .await?;
            flow_service
                .submit_bid(txn, setup.game_id, 0, 7, None)
                .await?; // Dealer

            // Tricks: P0 wins 7, P1 wins 3, P2 wins 2, P3 wins 1
            create_tricks_by_winner_counts(txn, setup.round_id, [7, 3, 2, 1]).await?;

            flow_service.score_round(txn, setup.game_id).await?;

            let loaded_state = game_service.load_game_state(txn, setup.game_id).await?;

            // P0: 7+10=17, P1: 3+10=13, P2: 2+10=12, P3: 1+0=1 (didn't meet bid 0)
            assert_eq!(loaded_state.scores_total, [17, 13, 12, 1]);

            assert_eq!(loaded_state.round.tricks_won, [7, 3, 2, 1]);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: Load state for unstarted game (Lobby)
#[tokio::test]
async fn test_load_state_lobby() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = generate_join_code();
            let game = games::create_game(txn, GameCreate::new(&join_code)).await?;

            let game_service = GameService;

            // Load state without dealing any rounds
            let loaded_state = game_service.load_game_state(txn, game.id).await?;

            // Should return empty initial state (no rounds exist yet)
            assert_eq!(loaded_state.phase, Phase::Init);
            assert_eq!(loaded_state.round_no, 0);
            assert_eq!(loaded_state.hand_size, 0);
            assert_eq!(loaded_state.hands[0].len(), 0);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
