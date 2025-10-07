//! Tests for rollback policy behavior
//!
//! This module now uses the common initialization which sets the
//! RollbackOnOk policy and does not persist writes to the database.
mod common;

use backend::config::db::DbProfile;
use backend::db::require_db;
use backend::db::txn::with_txn;
use backend::db::txn_policy::{current, TxnPolicy};
use backend::entities::games::{self, GameState, GameVisibility};
use backend::infra::state::build_state;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set};
use tracing::debug;

#[actix_web::test]
async fn test_rollback_policy() -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the rollback policy
    assert_eq!(current(), TxnPolicy::RollbackOnOk);

    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Insert a games row inside the transaction and return its id
    let inserted_id = with_txn(None, &state, |txn| {
        Box::pin(async move {
            let now = time::OffsetDateTime::now_utc();
            let game = games::ActiveModel {
                visibility: Set(GameVisibility::Public),
                state: Set(GameState::Lobby),
                rules_version: Set("nommie-1.0.0".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };

            let inserted = games::Entity::insert(game).exec(txn).await.map_err(|e| {
                backend::error::AppError::from(backend::infra::db_errors::map_db_err(e))
            })?;
            debug!(
                id = inserted.last_insert_id,
                "inserted games row inside txn"
            );
            Ok::<_, backend::error::AppError>(inserted.last_insert_id)
        })
    })
    .await?;

    // Outside the transaction, verify the row is not present (rolled back)
    let db = require_db(&state)?;
    let found = games::Entity::find_by_id(inserted_id).one(db).await?;
    debug!(
        id = inserted_id,
        "query after txn returned: {}",
        found.is_some()
    );
    assert!(
        found.is_none(),
        "row should not persist after rollback-on-ok"
    );

    Ok(())
}

#[actix_web::test]
async fn test_rollback_policy_on_error() -> Result<(), Box<dyn std::error::Error>> {
    // Verify we're using the rollback policy
    assert_eq!(current(), TxnPolicy::RollbackOnOk);

    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Insert inside the transaction, then return an error to force rollback
    let unique_code = format!("TEST{}", &uuid::Uuid::new_v4().to_string()[..6]);
    let result = with_txn(None, &state, |txn| {
        let unique_code = unique_code.clone();
        Box::pin(async move {
            let now = time::OffsetDateTime::now_utc();
            let game = games::ActiveModel {
                visibility: Set(GameVisibility::Private),
                state: Set(GameState::Lobby),
                rules_version: Set("nommie-1.0.0".to_string()),
                join_code: Set(Some(unique_code.clone())),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };

            // Perform the insert, then intentionally return an error
            let inserted = games::Entity::insert(game)
                .exec(txn)
                .await
                .map_err(|e| {
                    backend::error::AppError::from(backend::infra::db_errors::map_db_err(e))
                })?;
            debug!(id = inserted.last_insert_id, join_code = %unique_code, "inserted games row inside txn before error");

            Err::<String, _>(backend::error::AppError::Internal {
                code: backend::errors::ErrorCode::InternalError,
                detail: "test error".to_string(),
            })
        })
    })
    .await;

    // Verify the operation failed
    assert!(result.is_err());

    // Outside the transaction, verify no row with the unique join_code exists
    let db = require_db(&state)?;
    let found = games::Entity::find()
        .filter(games::Column::JoinCode.eq(Some(unique_code)))
        .one(db)
        .await?;
    debug!(
        "query after txn (error case) found row: {}",
        found.is_some()
    );
    assert!(
        found.is_none(),
        "row should not persist after rollback on error"
    );

    Ok(())
}
