use actix_web::http::StatusCode;
use actix_web::{test, web, HttpMessage};
use backend::ai::{HeuristicV1, RandomPlayer};
use backend::db::require_db;
use backend::db::txn::SharedTxn;
use backend::entities::{ai_profiles, game_players, games};
use backend::middleware::jwt_extract::JwtExtract;
use backend::routes::games::configure_routes;
use backend::state::security_config::SecurityConfig;
use backend::AppError;
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

    let game = games::Entity::find_by_id(game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    let lock_version = game.lock_version;

    let req = test::TestRequest::post()
        .uri(&format!("/api/games/{game_id}/ai/add"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "registry_name": HeuristicV1::NAME,
            "lock_version": lock_version
        }))
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
        .find(|m| m.player_kind == game_players::PlayerKind::Ai)
        .expect("AI membership exists");
    assert_eq!(ai_membership.turn_order, 1);
    assert!(ai_membership.is_ready);

    let profile = ai_profiles::Entity::find_by_id(
        ai_membership
            .ai_profile_id
            .expect("ai_profile_id should be set"),
    )
    .one(shared.transaction())
    .await?
    .expect("AI profile exists");
    assert_eq!(profile.registry_name, HeuristicV1::NAME);
    assert_eq!(profile.registry_version, HeuristicV1::VERSION);

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn host_can_update_ai_seat_profile() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let security: SecurityConfig = state.security.clone();
    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(db).await?;

    let test_name = "host_can_update_ai_seat_profile";
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

    let game = games::Entity::find_by_id(game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    let mut lock_version = game.lock_version;

    // Add initial AI seat (defaults to heuristic).
    let add_req = test::TestRequest::post()
        .uri(&format!("/api/games/{game_id}/ai/add"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "registry_name": HeuristicV1::NAME,
            "lock_version": lock_version
        }))
        .to_request();
    add_req.extensions_mut().insert(shared.clone());
    let add_resp = test::call_service(&app, add_req).await;
    assert_eq!(add_resp.status(), StatusCode::NO_CONTENT);
    drop(add_resp);

    // Refresh lock_version after adding AI seat
    let game = games::Entity::find_by_id(game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    lock_version = game.lock_version;

    // Update the AI seat to RandomPlayer.
    let update_req = test::TestRequest::post()
        .uri(&format!("/api/games/{game_id}/ai/update"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "seat": 1,
            "registry_name": RandomPlayer::NAME,
            "lock_version": lock_version
        }))
        .to_request();
    update_req.extensions_mut().insert(shared.clone());
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::NO_CONTENT);
    drop(update_resp);

    let ai_membership = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(game_id))
        .filter(game_players::Column::PlayerKind.eq(game_players::PlayerKind::Ai))
        .one(shared.transaction())
        .await?
        .expect("AI membership exists");

    let profile = ai_profiles::Entity::find_by_id(
        ai_membership
            .ai_profile_id
            .expect("ai_profile_id should be set"),
    )
    .one(shared.transaction())
    .await?
    .expect("AI profile exists");

    assert_eq!(profile.registry_name, RandomPlayer::NAME);
    assert_eq!(profile.registry_version, RandomPlayer::VERSION);

    let override_record = backend::entities::ai_overrides::Entity::find()
        .filter(backend::entities::ai_overrides::Column::GamePlayerId.eq(ai_membership.id))
        .one(shared.transaction())
        .await?;
    if let Some(record) = override_record {
        if let Some(cfg) = record.config {
            assert!(
                cfg.get("seed").is_some(),
                "Random player config should include a seed"
            );
        }
    }

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

    let game = games::Entity::find_by_id(game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    let mut lock_version = game.lock_version;

    // Add an AI seat first.
    let add_req = test::TestRequest::post()
        .uri(&format!("/api/games/{game_id}/ai/add"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "lock_version": lock_version
        }))
        .to_request();
    add_req.extensions_mut().insert(shared.clone());
    let add_resp = test::call_service(&app, add_req).await;
    assert_eq!(add_resp.status(), StatusCode::NO_CONTENT);
    drop(add_resp);

    // Refresh lock_version after adding AI seat
    let game = games::Entity::find_by_id(game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    lock_version = game.lock_version;

    // Remove the AI seat.
    let remove_req = test::TestRequest::post()
        .uri(&format!("/api/games/{game_id}/ai/remove"))
        .insert_header(("Authorization", format!("Bearer {token}")))
        .set_json(json!({
            "lock_version": lock_version
        }))
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
    assert_eq!(memberships[0].human_user_id, Some(host_user_id));

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

    let game = games::Entity::find_by_id(game_id)
        .one(shared.transaction())
        .await?
        .expect("game exists");
    let lock_version = game.lock_version;

    let req = test::TestRequest::post()
        .uri(&format!("/api/games/{game_id}/ai/add"))
        .insert_header(("Authorization", format!("Bearer {non_host_token}")))
        .set_json(json!({
            "lock_version": lock_version
        }))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    drop(resp);

    shared.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn ai_registry_endpoint_lists_factories() -> Result<(), AppError> {
    let state = build_test_state().await?;
    let security: SecurityConfig = state.security.clone();
    let db = require_db(&state).expect("DB required");
    let shared = SharedTxn::open(db).await?;

    let host_sub = test_user_sub("ai_registry_host");
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

    let req = test::TestRequest::get()
        .uri("/api/games/ai/registry")
        .insert_header(("Authorization", format!("Bearer {token}")))
        .to_request();
    req.extensions_mut().insert(shared.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = test::read_body(resp).await;
    let parsed: serde_json::Value =
        serde_json::from_slice(&body).expect("registry endpoint should return JSON");
    let ais = parsed
        .get("ais")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let names: Vec<&str> = ais
        .iter()
        .filter_map(|entry| entry.get("name").and_then(|value| value.as_str()))
        .collect();

    assert!(
        names.contains(&HeuristicV1::NAME),
        "registry should include HeuristicV1"
    );
    assert!(
        names.contains(&RandomPlayer::NAME),
        "registry should include RandomPlayer"
    );

    shared.rollback().await?;
    Ok(())
}
