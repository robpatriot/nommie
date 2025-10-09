//! Adapter tests validating that games_sea works identically with pooled connections and transactions.
//!
//! These tests verify the "generic over C: ConnectionTrait" design by running the same
//! operations through both connection types and asserting equivalence.
//!
//! **Important:** While this file uses `mod common` (RollbackOnOk policy), some tests
//! intentionally use pooled connections via `require_db()` which auto-commit their changes.
//! These tests perform manual cleanup to leave the database unchanged.
//!
//! Pattern:
//! - Pooled path: operations commit → manual cleanup with delete_games_by_name
//! - Transaction path: operations auto-rollback via RollbackOnOk policy

mod common;
mod support;

use backend::adapters::games_sea::{self, GameCreate};
use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::with_txn;
use backend::error::AppError;
use backend::infra::state::build_state;
use rand::Rng;
use support::db_games::delete_games_by_name;
use support::games_sea_helpers::run_game_flow;

/// Test: pooled connection vs transaction equivalence
#[tokio::test]
async fn test_games_flow_pooled_vs_txn_equivalence() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    // Run flow with REAL pooled connection (commits happen)
    // Use random 6-digit number for uniqueness
    let pooled_marker = format!("T{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let db = require_db(&state)?;
    let pooled_probe = run_game_flow(db, &pooled_marker)
        .await
        .map_err(|e| AppError::internal(format!("pooled flow failed: {e}")))?;

    // Cleanup pooled path (data was committed)
    with_txn(None, &state, |txn| {
        let pooled_marker = pooled_marker.clone();
        Box::pin(async move {
            delete_games_by_name(txn, &pooled_marker).await?;
            Ok::<_, AppError>(())
        })
    })
    .await?;

    // Run flow with transaction (auto-rollback via test policy)
    let txn_probe = with_txn(None, &state, |txn| {
        Box::pin(async move {
            let probe = run_game_flow(txn, "TEST002")
                .await
                .map_err(|e| AppError::internal(format!("txn flow failed: {e}")))?;
            Ok(probe)
        })
    })
    .await?;

    // Assert equivalence (fields should match except join_code which is different by design)
    assert_eq!(
        pooled_probe.state, txn_probe.state,
        "state should be identical"
    );
    assert_eq!(
        pooled_probe.visibility, txn_probe.visibility,
        "visibility should be identical"
    );
    assert_eq!(
        pooled_probe.lock_version, txn_probe.lock_version,
        "lock_version should be identical"
    );
    assert_eq!(
        pooled_probe.name, txn_probe.name,
        "name should be identical"
    );

    Ok(())
}

/// Test: timestamp policy consistency in both pooled and txn contexts
#[tokio::test]
async fn test_games_flow_timestamp_policy_consistent() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    // Run with REAL pooled connection (commits happen)
    // Use random 6-digit number for uniqueness
    let pooled_marker = format!("T{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let db = require_db(&state)?;
    run_game_flow(db, &pooled_marker)
        .await
        .map_err(|e| AppError::internal(format!("pooled timestamp test failed: {e}")))?;

    // Cleanup pooled path
    with_txn(None, &state, |txn| {
        let pooled_marker = pooled_marker.clone();
        Box::pin(async move {
            delete_games_by_name(txn, &pooled_marker).await?;
            Ok::<_, AppError>(())
        })
    })
    .await?;

    // Run with transaction (auto-rollback)
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            run_game_flow(txn, "TEST004")
                .await
                .map_err(|e| AppError::internal(format!("txn timestamp test failed: {e}")))?;
            Ok(())
        })
    })
    .await?;

    // If we reach here, both flows passed all timestamp assertions
    Ok(())
}

/// Test: error behavior consistency (duplicate join_code constraint)
#[tokio::test]
async fn test_games_error_behavior_consistent() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    // Test with REAL pooled connection
    // Use random 6-digit number for uniqueness
    let pooled_marker = format!("D{:06}", rand::thread_rng().gen_range(0..1_000_000));
    let db = require_db(&state)?;

    // Create first game (commits)
    let dto1 = GameCreate::new(&pooled_marker).with_name("First");
    games_sea::create_game(db, dto1)
        .await
        .map_err(|e| AppError::db(format!("create failed: {e}")))?;

    // Try to create second game with same join_code (should fail)
    let dto2 = GameCreate::new(&pooled_marker).with_name("Second");
    let pooled_result = games_sea::create_game(db, dto2).await;

    // Should have failed due to unique constraint
    assert!(
        pooled_result.is_err(),
        "pooled: duplicate join_code should fail"
    );

    // Cleanup pooled path
    with_txn(None, &state, |txn| {
        let pooled_marker = pooled_marker.clone();
        Box::pin(async move {
            delete_games_by_name(txn, &pooled_marker).await?;
            Ok::<_, AppError>(())
        })
    })
    .await?;

    // Test with transaction (auto-rollback)
    let txn_result = with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create first game
            let dto1 = GameCreate::new("DUPE002").with_name("First");
            games_sea::create_game(txn, dto1).await?;

            // Try to create second game with same join_code
            let dto2 = GameCreate::new("DUPE002").with_name("Second");
            let result = games_sea::create_game(txn, dto2).await;

            Ok::<_, AppError>(result)
        })
    })
    .await?;

    // Should have failed due to unique constraint
    assert!(txn_result.is_err(), "txn: duplicate join_code should fail");

    // Both contexts should produce errors (constraint violation)
    // The exact error type should be consistent
    Ok(())
}

/// Test: not-found errors are consistent between pooled and txn
#[tokio::test]
async fn test_games_not_found_consistent() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    let non_existent_id = 999_999_999_i64;

    // Test with REAL pooled connection
    let db = require_db(&state)?;

    let pooled_result = games_sea::find_by_id(db, non_existent_id)
        .await
        .map_err(|e| AppError::db(format!("find_by_id failed: {e}")))?;

    assert!(
        pooled_result.is_none(),
        "pooled: non-existent game should return None"
    );

    // Test with transaction
    let txn_result = with_txn(None, &state, |txn| {
        Box::pin(async move {
            let result = games_sea::find_by_id(txn, non_existent_id).await?;
            Ok::<_, AppError>(result)
        })
    })
    .await?;

    assert!(
        txn_result.is_none(),
        "txn: non-existent game should return None"
    );

    // Test with non-existent join_code (pooled)
    let pooled_join_code = games_sea::find_by_join_code(db, "NOTFOUND999")
        .await
        .map_err(|e| AppError::db(format!("find_by_join_code failed: {e}")))?;

    assert!(
        pooled_join_code.is_none(),
        "pooled: non-existent join_code should return None"
    );

    // Test with transaction
    let txn_join_code = with_txn(None, &state, |txn| {
        Box::pin(async move {
            let result = games_sea::find_by_join_code(txn, "NOTFOUND888").await?;
            Ok::<_, AppError>(result)
        })
    })
    .await?;

    assert!(
        txn_join_code.is_none(),
        "txn: non-existent join_code should return None"
    );

    Ok(())
}
