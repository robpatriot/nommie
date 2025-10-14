use backend::db::txn::with_txn;
use backend::domain::state::Phase;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::{games, rounds, tricks};
use backend::services::game_flow::GameFlowService;
use backend::services::games::GameService;

use crate::support::game_phases::{
    setup_game_in_bidding_phase, setup_game_in_trick_play_phase,
    setup_game_in_trump_selection_phase,
};
use crate::support::test_utils::short_join_code;
use crate::support::trick_helpers::create_tricks_by_winner_counts;

/// Test: Load game state from database after dealing
#[tokio::test]
async fn test_load_state_after_deal() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let setup = setup_game_in_bidding_phase(txn, 12345).await?;
            let game_service = GameService::new();

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
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Round 1: dealer at seat 0, bids: 5, 4, 3, 0
            let setup = setup_game_in_trump_selection_phase(txn, 12346, [5, 4, 3, 0]).await?;
            let game_service = GameService::new();

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
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Round 1: dealer at seat 0, bids: 4, 5, 3, 0, trump: Hearts
            let setup =
                setup_game_in_trick_play_phase(txn, 12347, [4, 5, 3, 0], rounds::Trump::Hearts)
                    .await?;
            let game_service = GameService::new();

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
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Round 1: dealer at seat 0, bids: 3, 3, 4, 2, trump: Spades
            let setup =
                setup_game_in_trick_play_phase(txn, 12348, [3, 3, 4, 2], rounds::Trump::Spades)
                    .await?;
            let game_service = GameService::new();

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

/// Test: Load state with cumulative scores
#[tokio::test]
async fn test_load_state_with_scores() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let setup = setup_game_in_bidding_phase(txn, 12349).await?;
            let flow_service = GameFlowService::new();
            let game_service = GameService::new();

            // Complete one full round (Round 1: dealer at seat 0, bidding starts at seat 1)
            flow_service.submit_bid(txn, setup.game_id, 1, 3).await?;
            flow_service.submit_bid(txn, setup.game_id, 2, 2).await?;
            flow_service.submit_bid(txn, setup.game_id, 3, 0).await?;
            flow_service.submit_bid(txn, setup.game_id, 0, 7).await?; // Dealer

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
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;

            let game_service = GameService::new();

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
