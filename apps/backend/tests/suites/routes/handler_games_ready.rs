use actix_web::http::StatusCode;
use actix_web::{test, web, HttpMessage};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::game_players;
use backend::entities::games::{self, GameState};
use backend::error::AppError;
use backend::middleware::jwt_extract::JwtExtract;
use backend::routes::games::configure_routes;
use backend::state::security_config::SecurityConfig;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use crate::support::app_builder::create_test_app;
use crate::support::auth::mint_test_token;
use crate::support::build_test_state;
use crate::support::db_memberships::create_test_game_player_with_ready;
use crate::support::factory::{create_fresh_lobby_game, create_test_user};

#[tokio::test]
async fn mark_ready_sets_membership_flag() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let security: SecurityConfig = state.security.clone();
    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(db).await?;

    let game_id = create_fresh_lobby_game(shared.transaction(), "ready_endpoint_basic").await?;
    let user_sub = "ready-user-basic";
    let user_email = "ready-basic@example.com";
    let user_id =
        create_test_user(shared.transaction(), user_sub, Some("Ready Basic User")).await?;

    create_test_game_player_with_ready(shared.transaction(), game_id, user_id, 0, false).await?;

    let token = mint_test_token(user_sub, user_email, &security);

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::post()
        .uri(&format!("/api/games/{game_id}/ready"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    drop(resp);

    let membership = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(game_id))
        .filter(game_players::Column::UserId.eq(user_id))
        .one(shared.transaction())
        .await?
        .expect("membership should exist");
    assert!(membership.is_ready);

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn mark_ready_auto_starts_when_all_ready() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let security: SecurityConfig = state.security.clone();
    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(db).await?;

    let game_id = create_fresh_lobby_game(shared.transaction(), "ready_endpoint_autostart").await?;

    // Create four players, with three already ready (simulating humans/AI waiting on host).
    let actor_sub = "ready-host";
    let actor_email = "ready-host@example.com";
    let actor_id = create_test_user(shared.transaction(), actor_sub, Some("Host Player")).await?;
    create_test_game_player_with_ready(shared.transaction(), game_id, actor_id, 0, false).await?;

    for seat in 1..4 {
        let sub = format!("player-{seat}-ready");
        let user_id =
            create_test_user(shared.transaction(), &sub, Some(&format!("P{seat}"))).await?;
        create_test_game_player_with_ready(shared.transaction(), game_id, user_id, seat, true)
            .await?;
    }

    let token = mint_test_token(actor_sub, actor_email, &security);

    let app = create_test_app(state)
        .with_routes(|cfg| {
            cfg.service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(configure_routes),
            );
        })
        .build()
        .await?;

    let req = test::TestRequest::post()
        .uri(&format!("/api/games/{game_id}/ready"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Membership for the acting user should now be ready.
    let actor_membership = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(game_id))
        .filter(game_players::Column::UserId.eq(actor_id))
        .one(shared.transaction())
        .await?
        .expect("actor membership should exist");
    assert!(actor_membership.is_ready);

    // Game should have transitioned into bidding (round dealt).
    let game = games::Entity::find_by_id(game_id)
        .one(shared.transaction())
        .await?
        .expect("game should exist");
    assert_eq!(game.state, GameState::Bidding);
    assert_eq!(game.current_round, Some(1));

    drop(resp);

    shared.rollback().await?;
    Ok(())
}
