use backend::db::txn::with_txn;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::AppError;
use sea_orm::{EntityTrait, Set};

use crate::support::build_test_state;

#[tokio::test]
async fn insert_defaults_and_fetch() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Insert a games row with minimal fields
            let now = time::OffsetDateTime::now_utc();
            let game = games::ActiveModel {
                visibility: Set(GameVisibility::Public),
                state: Set(GameState::Lobby),
                rules_version: Set("nommie-1.0.0".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };

            let inserted_game = games::Entity::insert(game).exec(txn).await.map_err(|e| {
                backend::AppError::from(backend::infra::db_errors::map_db_err(e))
            })?;

            // Assert id > 0
            assert!(inserted_game.last_insert_id > 0);

            // Fetch by id and assert it exists
            let fetched_game = games::Entity::find_by_id(inserted_game.last_insert_id)
                .one(txn)
                .await
                .map_err(|e| {
                    backend::AppError::from(backend::infra::db_errors::map_db_err(e))
                })?
                .expect("should have game row");

            // Assert state round-trips correctly
            assert_eq!(fetched_game.state, GameState::Lobby);
            assert_eq!(fetched_game.visibility, GameVisibility::Public);
            assert_eq!(fetched_game.rules_version, "nommie-1.0.0");
            assert_eq!(fetched_game.lock_version, 0);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
