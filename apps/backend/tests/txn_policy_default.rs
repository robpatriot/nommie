//! Tests for default commit policy behavior
//!
//! This test binary runs without mod common, so it uses the OnceLock default
//! of CommitOnOk policy. It verifies that the default policy works correctly.

// Initialize logging directly (no mod common)
mod support;
#[ctor::ctor]
fn init_logging() {
    backend_test_support::logging::init();
}

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::db::txn_policy::{current, TxnPolicy};
use backend::infra::state::build_state;
use support::db_games::{count_games_by_name_pool, delete_games_by_name, insert_game_stub};
use ulid::Ulid;

#[test]
fn test_policy_is_commit_on_ok() {
    // This test binary does not import mod common, so it should see the
    // OnceLock default of CommitOnOk policy
    assert_eq!(current(), TxnPolicy::CommitOnOk);
}

#[actix_web::test]
async fn test_default_commit_policy_persists_then_cleans_up(
) -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the default policy
    assert_eq!(current(), TxnPolicy::CommitOnOk);

    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Use unique marker for name
    let name = Ulid::new().to_string();

    // Reads use fresh pool to check visibility post-commit
    let before = count_games_by_name_pool(&state, &name).await?;

    // Writes go through with_txn for policy parity and guardrails
    with_txn(None, &state, |txn| {
        let name = name.clone();
        Box::pin(async move {
            insert_game_stub(txn, &name).await?;
            Ok::<_, backend::error::AppError>(())
        })
    })
    .await?;

    assert_eq!(count_games_by_name_pool(&state, &name).await?, before + 1);

    // Cleanup via with_txn to leave DB unchanged for other tests
    with_txn(None, &state, |txn| {
        let name = name.clone();
        Box::pin(async move {
            delete_games_by_name(txn, &name).await?;
            Ok::<_, backend::error::AppError>(())
        })
    })
    .await?;

    assert_eq!(count_games_by_name_pool(&state, &name).await?, before);

    Ok(())
}

#[actix_web::test]
async fn test_default_commit_policy_on_error() -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the default policy
    assert_eq!(current(), TxnPolicy::CommitOnOk);

    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Use unique marker for name
    let name = Ulid::new().to_string();
    let before = count_games_by_name_pool(&state, &name).await?;

    // Insert then return error; should rollback (no commit)
    let result = with_txn(None, &state, |txn| {
        let name = name.clone();
        Box::pin(async move {
            insert_game_stub(txn, &name).await?;
            Err::<(), _>(backend::error::AppError::Internal {
                code: backend::errors::ErrorCode::InternalError,
                detail: "test error".to_string(),
            })
        })
    })
    .await;

    assert!(result.is_err());
    assert_eq!(count_games_by_name_pool(&state, &name).await?, before);

    Ok(())
}
