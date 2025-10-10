mod common;
mod support;

use backend::db::txn::with_txn;
use backend::domain::state::Phase;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::{games, rounds, tricks};
use backend::services::game_flow::GameFlowService;
use backend::services::games::GameService;
use ulid::Ulid;

fn short_join_code() -> String {
    format!("{}", Ulid::new()).chars().take(10).collect()
}

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
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;

            let flow_service = GameFlowService::new();
            let game_service = GameService::new();

            // Deal round 1
            flow_service.deal_round(txn, game.id).await?;

            // Load state from DB
            let loaded_state = game_service.load_game_state(txn, game.id).await?;

            // Verify state reconstruction
            assert_eq!(loaded_state.phase, Phase::Bidding);
            assert_eq!(loaded_state.round_no, 1);
            assert_eq!(loaded_state.hand_size, 13); // Round 1 has 13 cards
            assert_eq!(loaded_state.scores_total, [0, 0, 0, 0]);

            // All 4 hands should be loaded and non-empty
            assert_eq!(loaded_state.hands[0].len(), 13);
            assert_eq!(loaded_state.hands[1].len(), 13);
            assert_eq!(loaded_state.hands[2].len(), 13);
            assert_eq!(loaded_state.hands[3].len(), 13);

            // Bids should all be None
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
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;

            let flow_service = GameFlowService::new();
            let game_service = GameService::new();

            // Deal and bid
            flow_service.deal_round(txn, game.id).await?;
            flow_service.submit_bid(txn, game.id, 0, 5).await?;
            flow_service.submit_bid(txn, game.id, 1, 4).await?;
            flow_service.submit_bid(txn, game.id, 2, 3).await?;
            flow_service.submit_bid(txn, game.id, 3, 0).await?; // Dealer

            // Load state
            let loaded_state = game_service.load_game_state(txn, game.id).await?;

            // Should be in TrumpSelection phase now
            assert_eq!(loaded_state.phase, Phase::TrumpSelect);

            // Bids should be loaded
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
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;

            let flow_service = GameFlowService::new();
            let game_service = GameService::new();

            // Deal, bid, set trump
            flow_service.deal_round(txn, game.id).await?;
            flow_service.submit_bid(txn, game.id, 0, 5).await?;
            flow_service.submit_bid(txn, game.id, 1, 4).await?;
            flow_service.submit_bid(txn, game.id, 2, 3).await?;
            flow_service.submit_bid(txn, game.id, 3, 0).await?;
            flow_service
                .set_trump(txn, game.id, 0, rounds::Trump::Hearts)
                .await?;

            // Load state
            let loaded_state = game_service.load_game_state(txn, game.id).await?;

            // Should be in TrickPlay phase
            assert_eq!(loaded_state.phase, Phase::Trick { trick_no: 1 });

            // Trump should be loaded
            assert_eq!(
                loaded_state.round.trump,
                Some(backend::domain::Trump::Hearts)
            );

            // Trick state should be initialized
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
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;

            let flow_service = GameFlowService::new();
            let game_service = GameService::new();

            // Set up game in TrickPlay phase with some plays
            flow_service.deal_round(txn, game.id).await?;
            flow_service.submit_bid(txn, game.id, 0, 3).await?;
            flow_service.submit_bid(txn, game.id, 1, 3).await?;
            flow_service.submit_bid(txn, game.id, 2, 4).await?;
            flow_service.submit_bid(txn, game.id, 3, 2).await?;
            flow_service
                .set_trump(txn, game.id, 2, rounds::Trump::Spades)
                .await?;

            // Manually create a trick with 2 plays (mid-trick)
            let round = rounds::find_by_game_and_round(txn, game.id, 1)
                .await?
                .unwrap();

            use backend::repos::plays;
            let trick = tricks::create_trick(txn, round.id, 0, tricks::Suit::Hearts, 0).await?;
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

            // Load state
            let loaded_state = game_service.load_game_state(txn, game.id).await?;

            // Verify trick is partially loaded
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
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;

            let flow_service = GameFlowService::new();
            let game_service = GameService::new();

            // Complete one full round
            flow_service.deal_round(txn, game.id).await?;
            flow_service.submit_bid(txn, game.id, 0, 7).await?;
            flow_service.submit_bid(txn, game.id, 1, 3).await?;
            flow_service.submit_bid(txn, game.id, 2, 2).await?;
            flow_service.submit_bid(txn, game.id, 3, 0).await?;

            let round1 = rounds::find_by_game_and_round(txn, game.id, 1)
                .await?
                .unwrap();

            // Create tricks with winners
            for i in 0..7 {
                tricks::create_trick(txn, round1.id, i, tricks::Suit::Hearts, 0).await?;
            }
            for i in 7..10 {
                tricks::create_trick(txn, round1.id, i, tricks::Suit::Spades, 1).await?;
            }
            for i in 10..12 {
                tricks::create_trick(txn, round1.id, i, tricks::Suit::Clubs, 2).await?;
            }
            tricks::create_trick(txn, round1.id, 12, tricks::Suit::Diamonds, 3).await?;

            flow_service.score_round(txn, game.id).await?;

            // Load state - should have scores
            let loaded_state = game_service.load_game_state(txn, game.id).await?;

            // Verify cumulative scores loaded
            // P0: 7+10=17, P1: 3+10=13, P2: 2+10=12, P3: 1+0=1 (didn't meet bid 0)
            assert_eq!(loaded_state.scores_total, [17, 13, 12, 1]);

            // Verify tricks_won loaded
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

            // Should return minimal stub
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
