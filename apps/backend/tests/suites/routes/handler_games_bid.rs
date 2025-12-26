use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::{games, round_bids};
use backend::services::game_flow::GameFlowService;
use backend::state::security_config::SecurityConfig;
use backend::{AppError, ErrorCode};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::support::game_phases::setup_game_in_bidding_phase;
use crate::support::test_state_builder;

#[tokio::test]
async fn submit_bid_success() -> Result<(), AppError> {
    let security = SecurityConfig::new("routes-bid-secret");
    let state = test_state_builder()?
        .with_security(security)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required").clone();
    let shared = SharedTxn::open(&db).await?;

    let setup = setup_game_in_bidding_phase(shared.transaction(), "routes_bid_success").await?;
    let dealer_pos = setup.dealer_pos as usize;
    let actor_seat = ((dealer_pos + 1) % 4) as u8;

    let service = GameFlowService;
    let game = games::Entity::find_by_id(setup.game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    service
        .submit_bid(
            shared.transaction(),
            setup.game_id,
            actor_seat,
            1,
            game.version,
        )
        .await?;

    let stored_bid = round_bids::Entity::find()
        .filter(round_bids::Column::RoundId.eq(setup.round_id))
        .filter(round_bids::Column::PlayerSeat.eq(actor_seat as i16))
        .one(shared.transaction())
        .await?
        .expect("bid stored");
    assert_eq!(stored_bid.bid_value, 1);

    let game = games::Entity::find_by_id(setup.game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    assert_eq!(game.state, backend::entities::games::GameState::Bidding);

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn submit_bid_out_of_turn_rejected() -> Result<(), AppError> {
    let security = SecurityConfig::new("routes-bid-secret");
    let state = test_state_builder()?
        .with_security(security)
        .build()
        .await?;

    let db = require_db(&state).expect("DB required").clone();
    let shared = SharedTxn::open(&db).await?;

    let setup = setup_game_in_bidding_phase(shared.transaction(), "routes_bid_out_of_turn").await?;
    let dealer_pos = setup.dealer_pos;
    let actor_seat = dealer_pos; // Dealer should bid last

    let service = GameFlowService;
    let game = games::Entity::find_by_id(setup.game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    let result = service
        .submit_bid(
            shared.transaction(),
            setup.game_id,
            actor_seat,
            1,
            game.version,
        )
        .await;

    let err = result.expect_err("expected out-of-turn error");
    assert_eq!(err.code(), ErrorCode::OutOfTurn);

    let stored_bids = round_bids::Entity::find()
        .filter(round_bids::Column::RoundId.eq(setup.round_id))
        .all(shared.transaction())
        .await?;
    assert!(stored_bids.is_empty());

    shared.rollback().await?;
    Ok(())
}
