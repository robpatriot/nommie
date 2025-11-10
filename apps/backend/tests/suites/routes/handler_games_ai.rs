use actix_web::http::StatusCode;
use actix_web::{test, web, HttpMessage};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::{game_players, games, users};
use backend::error::AppError;
use backend::middleware::jwt_extract::JwtExtract;
use backend::routes::games::configure_routes;
use backend::state::security_config::SecurityConfig;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde_json::json;

use crate::support::app_builder::create_test_app;
use crate::support::auth::mint_test_token;
use crate::support::build_test_state;
use crate::support::db_memberships::create_test_game_player_with_ready;
use crate::support::factory::{create_fresh_lobby_game, create_test_user};
use crate::support::test_utils::test_user_sub;

#[tokio::test]
async fn host_can_add_ai_seat() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let security: SecurityConfig = state.security.clone();
    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(db).await?;

    let test_name = "host_can_add_ai_seat";
    let game_id = create_fresh_lobby_game(shared.transaction(), test_name).await?;
    let game = games::Entity::find_by_id(game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    let host_user_id = game.created_by.expect("game has creator");
    create_test_game_player_with_ready(shared.transaction(), game_id, host_user_id, 0, false)
        .await?;

    let host_sub = test_user_sub(&format!("{test_name}_creator"));
    let host_email = format!("{host_sub}@example.com");
    let token = mint_test_token(&host_sub, &host_email, &security);

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
        .uri(&format!("/api/games/{game_id}/ai/add"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({}))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    drop(resp);

    let memberships = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(game_id))
        .all(shared.transaction())
        .await?;
    assert_eq!(memberships.len(), 2);
    let ai_membership = memberships
        .into_iter()
        .find(|m| m.user_id != host_user_id)
        .expect("AI membership exists");
    assert_eq!(ai_membership.turn_order, 1);
    assert!(ai_membership.is_ready);

    let ai_user = users::Entity::find_by_id(ai_membership.user_id)
        .one(shared.transaction())
        .await?
        .expect("AI user exists");
    assert!(ai_user.is_ai);

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn host_can_remove_ai_seat() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let security: SecurityConfig = state.security.clone();
    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(db).await?;

    let test_name = "host_can_remove_ai_seat";
    let game_id = create_fresh_lobby_game(shared.transaction(), test_name).await?;
    let game = games::Entity::find_by_id(game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    let host_user_id = game.created_by.expect("game has creator");
    create_test_game_player_with_ready(shared.transaction(), game_id, host_user_id, 0, false)
        .await?;

    let host_sub = test_user_sub(&format!("{test_name}_creator"));
    let host_email = format!("{host_sub}@example.com");
    let token = mint_test_token(&host_sub, &host_email, &security);

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

    // Add an AI seat first.
    let add_req = test::TestRequest::post()
        .uri(&format!("/api/games/{game_id}/ai/add"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({}))
        .to_request();
    add_req.extensions_mut().insert(shared.clone());
    let add_resp = test::call_service(&app, add_req).await;
    assert_eq!(add_resp.status(), StatusCode::NO_CONTENT);
    drop(add_resp);

    // Remove the AI seat.
    let remove_req = test::TestRequest::post()
        .uri(&format!("/api/games/{game_id}/ai/remove"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({}))
        .to_request();
    remove_req.extensions_mut().insert(shared.clone());
    let remove_resp = test::call_service(&app, remove_req).await;
    assert_eq!(remove_resp.status(), StatusCode::NO_CONTENT);
    drop(remove_resp);

    let memberships = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(game_id))
        .all(shared.transaction())
        .await?;
    assert_eq!(memberships.len(), 1);
    assert_eq!(memberships[0].user_id, host_user_id);

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn non_host_cannot_manage_ai() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let security: SecurityConfig = state.security.clone();
    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(db).await?;

    let test_name = "non_host_cannot_manage_ai";
    let game_id = create_fresh_lobby_game(shared.transaction(), test_name).await?;
    let game = games::Entity::find_by_id(game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    let host_user_id = game.created_by.expect("game has creator");
    create_test_game_player_with_ready(shared.transaction(), game_id, host_user_id, 0, false)
        .await?;

    let host_sub = test_user_sub(&format!("{test_name}_creator"));

    // Create a second human player (non-host) in seat 1.
    let non_host_sub = format!("{host_sub}_guest");
    let non_host_id = create_test_user(shared.transaction(), &non_host_sub, Some("Guest")).await?;
    create_test_game_player_with_ready(shared.transaction(), game_id, non_host_id, 1, false)
        .await?;

    let non_host_email = format!("{non_host_sub}@example.com");
    let non_host_token = mint_test_token(&non_host_sub, &non_host_email, &security);

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
        .uri(&format!("/api/games/{game_id}/ai/add"))
        .insert_header(("Authorization", format!("Bearer {non_host_token}")))
        .set_json(json!({}))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    drop(resp);

    shared.rollback().await?;
    Ok(())
}
