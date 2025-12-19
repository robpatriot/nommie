// WebSocket connection and initial snapshot tests

use backend::db::require_db;

use crate::support::auth::mint_test_token;
use crate::support::build_test_state;
use crate::support::factory::create_test_user;
use crate::support::game_setup::setup_game_with_players;
use crate::support::txn_helpers::rollback_eventually;
use crate::support::websocket::{attach_test_registry, start_test_server, wait_for_connections};
use crate::support::websocket_client::WebSocketClient;

#[tokio::test]
async fn websocket_connect_succeeds_with_valid_jwt() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    // Create game and user
    let test_name = "ws_connect_valid_jwt";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;
    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    // Update first membership to use our test user
    use backend::entities::game_players;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
    let membership = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(setup.game_id))
        .filter(game_players::Column::TurnOrder.eq(0))
        .one(shared.transaction())
        .await?
        .expect("membership should exist");

    let mut membership: game_players::ActiveModel = membership.into();
    membership.human_user_id = sea_orm::Set(Some(user_id));
    ActiveModelTrait::update(membership, shared.transaction()).await?;

    // Attach test registry
    let (state, registry) = attach_test_registry(state);

    // Create token
    let token = mint_test_token(&user_sub, &user_email, &security);

    // Start test server
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;
    // Connect WebSocket
    let ws_url = format!("ws://{}/ws/games/{}?token={}", addr, setup.game_id, token);
    let mut client =
        WebSocketClient::connect_retry(&ws_url, std::time::Duration::from_secs(1)).await?;

    // Wait for initial ack
    let msg = client
        .recv_json_timeout(std::time::Duration::from_secs(5))
        .await?
        .expect("should receive ack message");

    assert_eq!(msg["type"], "ack");
    assert_eq!(msg["message"], "connected");

    // Wait for initial snapshot
    let snapshot_msg = client
        .recv_json_timeout(std::time::Duration::from_secs(5))
        .await?
        .expect("should receive snapshot message");

    assert_eq!(snapshot_msg["type"], "snapshot");
    assert!(snapshot_msg.get("data").is_some());

    // Close client and stop server to release all references to SharedTxn
    client.close().await?;
    wait_for_connections(&registry, 0, std::time::Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    if let Err(e) = server_join.await {
        eprintln!("Test server join error: {:?}", e);
    }
    tokio::task::yield_now().await;

    // Rollback transaction at end of test
    rollback_eventually(shared).await?;

    Ok(())
}

#[tokio::test]
async fn websocket_receives_initial_ack() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_initial_ack";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;
    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    use backend::entities::game_players;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
    let membership = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(setup.game_id))
        .filter(game_players::Column::TurnOrder.eq(0))
        .one(shared.transaction())
        .await?
        .expect("membership should exist");

    let mut membership: game_players::ActiveModel = membership.into();
    membership.human_user_id = sea_orm::Set(Some(user_id));
    ActiveModelTrait::update(membership, shared.transaction()).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;
    let ws_url = format!("ws://{}/ws/games/{}?token={}", addr, setup.game_id, token);
    let mut client =
        WebSocketClient::connect_retry(&ws_url, std::time::Duration::from_secs(1)).await?;

    let msg = client
        .recv_json_timeout(std::time::Duration::from_secs(5))
        .await?
        .expect("should receive ack");

    assert_eq!(msg["type"], "ack");
    assert_eq!(msg["message"], "connected");

    // Close client and stop server to release all references to SharedTxn
    client.close().await?;
    wait_for_connections(&registry, 0, std::time::Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    if let Err(e) = server_join.await {
        eprintln!("Test server join error: {:?}", e);
    }
    tokio::task::yield_now().await;

    // Rollback transaction at end of test
    rollback_eventually(shared).await?;

    Ok(())
}

#[tokio::test]
async fn websocket_receives_initial_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_initial_snapshot";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;
    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    use backend::entities::game_players;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
    let membership = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(setup.game_id))
        .filter(game_players::Column::TurnOrder.eq(0))
        .one(shared.transaction())
        .await?
        .expect("membership should exist");

    let mut membership: game_players::ActiveModel = membership.into();
    membership.human_user_id = sea_orm::Set(Some(user_id));
    ActiveModelTrait::update(membership, shared.transaction()).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;
    let ws_url = format!("ws://{}/ws/games/{}?token={}", addr, setup.game_id, token);
    let mut client =
        WebSocketClient::connect_retry(&ws_url, std::time::Duration::from_secs(1)).await?;

    // Skip ack
    let _ack = client
        .recv_json_timeout(std::time::Duration::from_secs(5))
        .await?
        .expect("should receive ack");

    // Get snapshot
    let snapshot_msg = client
        .recv_json_timeout(std::time::Duration::from_secs(5))
        .await?
        .expect("should receive snapshot");

    assert_eq!(snapshot_msg["type"], "snapshot");
    let data = snapshot_msg.get("data").expect("snapshot should have data");
    assert!(data.get("snapshot").is_some());
    assert!(data.get("lock_version").is_some());

    // Close client and stop server to release all references to SharedTxn
    client.close().await?;
    wait_for_connections(&registry, 0, std::time::Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    if let Err(e) = server_join.await {
        eprintln!("Test server join error: {:?}", e);
    }
    tokio::task::yield_now().await;

    // Rollback transaction at end of test
    rollback_eventually(shared).await?;

    Ok(())
}
