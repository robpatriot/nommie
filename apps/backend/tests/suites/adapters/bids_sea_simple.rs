//! Minimal test to verify round_bids entity works with i16 types

use backend::db::txn::with_txn;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::entities::{game_rounds, round_bids};
use backend::AppError;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use crate::support::{build_test_state, test_seed};

/// Minimal test: insert and read a bid directly using SeaORM entities
#[tokio::test]
async fn test_bid_entity_direct() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // First, create a minimal game
            let now = time::OffsetDateTime::now_utc();
            let game = games::ActiveModel {
                visibility: Set(GameVisibility::Private),
                state: Set(GameState::Lobby),
                rules_version: Set("1.0".to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                rng_seed: Set(test_seed("bids_sea_simple").to_vec()),
                current_trick_no: Set(0i16),
                version: Set(1),
                ..Default::default()
            };
            let inserted_game = game.insert(txn).await?;

            // Then create a round
            let round = game_rounds::ActiveModel {
                id: sea_orm::NotSet,
                game_id: Set(inserted_game.id),
                round_no: Set(1i16),
                trump: Set(None),
                created_at: Set(now),
                completed_at: Set(None),
            };

            let inserted_round = round.insert(txn).await?;

            // Now create a bid directly using the entity
            let bid = round_bids::ActiveModel {
                id: sea_orm::NotSet,
                round_id: Set(inserted_round.id),
                player_seat: Set(0i16),
                bid_value: Set(5i16),
                bid_order: Set(0i16),
                created_at: Set(now),
            };

            let inserted_bid = bid.insert(txn).await?;

            // Verify we can read it back
            let fetched_bid = round_bids::Entity::find_by_id(inserted_bid.id)
                .one(txn)
                .await?
                .expect("bid should exist");

            // Verify the values
            assert_eq!(fetched_bid.player_seat, 0i16);
            assert_eq!(fetched_bid.bid_value, 5i16);
            assert_eq!(fetched_bid.bid_order, 0i16);

            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
