use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::infra::state::build_state;
use backend::repos::{bids, games, rounds};

use crate::support::test_utils::short_join_code;

/// Test: create_bid and find_all_by_round
#[tokio::test]
async fn test_create_bid_and_find_all() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            let bid1 = bids::create_bid(txn, round.id, 0, 5, 0).await?;
            let _bid2 = bids::create_bid(txn, round.id, 1, 7, 1).await?;
            let _bid3 = bids::create_bid(txn, round.id, 2, 3, 2).await?;

            assert!(bid1.id > 0);
            assert_eq!(bid1.bid_value, 5);
            assert_eq!(bid1.bid_order, 0);

            let all_bids = bids::find_all_by_round(txn, round.id).await?;
            assert_eq!(all_bids.len(), 3);

            // Should be ordered by bid_order
            assert_eq!(all_bids[0].bid_order, 0);
            assert_eq!(all_bids[1].bid_order, 1);
            assert_eq!(all_bids[2].bid_order, 2);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: count_bids_by_round
#[tokio::test]
async fn test_count_bids() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            let count = bids::count_bids_by_round(txn, round.id).await?;
            assert_eq!(count, 0);

            bids::create_bid(txn, round.id, 0, 5, 0).await?;
            bids::create_bid(txn, round.id, 1, 7, 1).await?;

            let count = bids::count_bids_by_round(txn, round.id).await?;
            assert_eq!(count, 2);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_winning_bid returns highest bid
#[tokio::test]
async fn test_find_winning_bid_highest() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            bids::create_bid(txn, round.id, 0, 5, 0).await?;
            bids::create_bid(txn, round.id, 1, 9, 1).await?; // Highest
            bids::create_bid(txn, round.id, 2, 3, 2).await?;

            let winner = bids::find_winning_bid(txn, round.id).await?;
            assert!(winner.is_some());
            let winner = winner.unwrap();
            assert_eq!(winner.bid_value, 9);
            assert_eq!(winner.player_seat, 1);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_winning_bid tie-breaker by bid_order
#[tokio::test]
async fn test_find_winning_bid_tiebreaker() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            bids::create_bid(txn, round.id, 0, 7, 0).await?; // First - should win tie
            bids::create_bid(txn, round.id, 1, 7, 1).await?; // Second
            bids::create_bid(txn, round.id, 2, 5, 2).await?;

            let winner = bids::find_winning_bid(txn, round.id).await?;
            assert!(winner.is_some());
            let winner = winner.unwrap();
            assert_eq!(winner.bid_value, 7);
            assert_eq!(winner.player_seat, 0);
            assert_eq!(winner.bid_order, 0);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: find_winning_bid returns None when no bids
#[tokio::test]
async fn test_find_winning_bid_none() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            let winner = bids::find_winning_bid(txn, round.id).await?;
            assert!(winner.is_none());

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: unique constraint on (round_id, player_seat)
#[tokio::test]
async fn test_unique_constraint_round_seat() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            bids::create_bid(txn, round.id, 0, 5, 0).await?;

            let result = bids::create_bid(txn, round.id, 0, 7, 1).await;

            assert!(result.is_err(), "Duplicate bid should fail");

            match result.unwrap_err() {
                backend::errors::domain::DomainError::Conflict(_, _) => {}
                e => panic!("Expected Conflict error, got {e:?}"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}

/// Test: unique constraint on (round_id, bid_order)
#[tokio::test]
async fn test_unique_constraint_round_order() -> Result<(), AppError> {
    let state = build_state()
        .with_db(backend::config::db::DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let join_code = short_join_code();
            let game = games::create_game(txn, &join_code).await?;
            let round = rounds::create_round(txn, game.id, 1, 13, 0).await?;

            bids::create_bid(txn, round.id, 0, 5, 0).await?;

            let result = bids::create_bid(txn, round.id, 1, 7, 0).await;

            assert!(result.is_err(), "Duplicate bid_order should fail");

            match result.unwrap_err() {
                backend::errors::domain::DomainError::Conflict(_, _) => {}
                e => panic!("Expected Conflict error, got {e:?}"),
            }

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
