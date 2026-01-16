// Tests for SharedTxn reuse behavior
//
// These tests verify that SharedTxn bypasses the transaction policy and
// that with_txn neither commits nor rolls back when a SharedTxn is used.

use actix_web::test;
use backend::db::require_db;
use backend::db::txn::with_txn;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::prelude::SharedTxn;
use sea_orm::{EntityTrait, Set};

use crate::support::build_test_state;

#[actix_web::test]
async fn test_shared_txn_reuse_bypasses_policy() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    // Get pooled DB and open a shared txn
    let db = require_db(&state).expect("DB required for this test");
    let shared = SharedTxn::open(db).await?;

    // Create a mutable request and inject the shared transaction
    let mut req = test::TestRequest::default().to_http_request();
    shared.inject(&mut req);

    // Write via with_txn using the shared transaction and confirm existence inside txn
    let inserted_id = with_txn(Some(&req), &state, |txn| {
        Box::pin(async move {
            let now = time::OffsetDateTime::now_utc();
            let game = games::ActiveModel {
                visibility: Set(GameVisibility::Private),
                state: Set(GameState::Lobby),
                rules_version: Set("nommie-1.0.0".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };

            let inserted = games::Entity::insert(game)
                .exec(txn)
                .await
                .map_err(|e| backend::AppError::from(backend::infra::db_errors::map_db_err(e)))?;

            // Confirm the row exists from within with_txn
            let found = games::Entity::find_by_id(inserted.last_insert_id)
                .one(txn)
                .await
                .map_err(|e| backend::AppError::from(backend::infra::db_errors::map_db_err(e)))?;
            assert!(found.is_some(), "insert should be visible within with_txn");

            Ok::<_, backend::AppError>(inserted.last_insert_id)
        })
    })
    .await?;

    // While SharedTxn is still active, verify the row still exists via another with_txn using the
    // same request and therefore SharedTxn
    with_txn(Some(&req), &state, |txn| {
        let id = inserted_id;
        Box::pin(async move {
            let found = games::Entity::find_by_id(id)
                .one(txn)
                .await
                .map_err(|e| backend::AppError::from(backend::infra::db_errors::map_db_err(e)))?;
            assert!(
                found.is_some(),
                "insert should persist within SharedTxn after with_txn returns"
            );
            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    // Drop the request to release the shared transaction reference
    drop(req);

    // Roll back the shared transaction explicitly to prove tests own rollback
    shared.rollback().await.unwrap();

    // Outside the transaction, verify the row is gone after rollback
    let db = require_db(&state)?;
    let found = games::Entity::find_by_id(inserted_id).one(db).await?;
    assert!(
        found.is_none(),
        "row should not persist after SharedTxn rollback"
    );

    Ok(())
}
