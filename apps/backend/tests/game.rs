mod common;

use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::error::AppError;
use backend::errors::ErrorCode;
use backend::infra::state::build_state;
use sea_orm::{EntityTrait, Set};
use serial_test::serial;

use crate::common::with_savepoint;

#[tokio::test]
#[serial]
async fn insert_defaults_and_fetch() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

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

            let inserted_game = games::Entity::insert(game).exec(txn).await?;

            // Assert id > 0
            assert!(inserted_game.last_insert_id > 0);

            // Fetch by id and assert it exists
            let fetched_game = games::Entity::find_by_id(inserted_game.last_insert_id)
                .one(txn)
                .await?
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

#[tokio::test]
#[serial]
async fn join_code_unique() -> Result<(), AppError> {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");

    // Single transaction: insert first game, then try to insert second with same join_code
    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Insert first game with join_code
            let now = time::OffsetDateTime::now_utc();
            let game1 = games::ActiveModel {
                visibility: Set(GameVisibility::Public),
                state: Set(GameState::Lobby),
                rules_version: Set("nommie-1.0.0".to_string()),
                join_code: Set(Some("ABC123".to_string())),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            };

            let inserted_game1 = games::Entity::insert(game1).exec(txn).await?;

            // Verify first game was created
            assert!(inserted_game1.last_insert_id > 0);

            // Try to insert second game with same join_code using savepoint
            let result = with_savepoint(txn, |sp| async move {
                let now2 = time::OffsetDateTime::now_utc();
                let game2 = games::ActiveModel {
                    visibility: Set(GameVisibility::Private),
                    state: Set(GameState::Lobby),
                    rules_version: Set("nommie-1.0.0".to_string()),
                    join_code: Set(Some("ABC123".to_string())), // Same join_code
                    created_at: Set(now2),
                    updated_at: Set(now2),
                    ..Default::default()
                };

                games::Entity::insert(game2)
                    .exec(&sp)
                    .await
                    .map_err(AppError::from) // Route through error mapping
            })
            .await;

            // Assert the second insert errors with UniqueViolation
            match result {
                Err(err) => {
                    assert_eq!(err.code(), ErrorCode::UniqueViolation);
                }
                Ok(_) => panic!("Expected Conflict error with UNIQUE_VIOLATION code"),
            }

            // Verify the first game still exists
            let fetched_game = games::Entity::find_by_id(inserted_game1.last_insert_id)
                .one(txn)
                .await?
                .expect("should have first game row");

            assert_eq!(fetched_game.join_code, Some("ABC123".to_string()));

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
