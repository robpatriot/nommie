// WebSocket reconnection tests

use std::time::Duration;

use backend::db::require_db;

use crate::support::auth::mint_test_token;
use crate::support::build_test_state;
use crate::support::factory::create_test_user;
use crate::support::game_setup::setup_game_with_players;
use crate::support::txn_helpers::rollback_eventually;
use crate::support::websocket::{attach_test_registry, start_test_server, wait_for_connections};
use crate::support::websocket_client::WebSocketClient;

#[tokio::test]
async fn websocket_reconnect_receives_latest_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_reconnect_snapshot";
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

    // Connect first time
    let ws_url = format!("ws://{}/ws/games/{}?token={}", addr, setup.game_id, token);
    let mut client1 = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;

    // Get initial snapshot
    let _ack1 = client1.recv_json_timeout(Duration::from_secs(5)).await?;
    let snapshot1 = client1
        .recv_json_timeout(Duration::from_secs(5))
        .await?
        .expect("should receive snapshot");

    // Disconnect
    client1.close().await?;

    wait_for_connections(&registry, 0, Duration::from_secs(1)).await?;

    // Reconnect
    let mut client2 = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;

    // Get new initial snapshot (should have same structure)
    let _ack2 = client2.recv_json_timeout(Duration::from_secs(5)).await?;
    let snapshot2 = client2
        .recv_json_timeout(Duration::from_secs(5))
        .await?
        .expect("should receive snapshot on reconnect");

    // Both snapshots should have the same structure
    assert_eq!(snapshot1["type"], "snapshot");
    assert_eq!(snapshot2["type"], "snapshot");
    assert!(snapshot1.get("data").is_some());
    assert!(snapshot2.get("data").is_some());

    // Close clients and stop server to release all references to SharedTxn
    client2.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(1)).await?;
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
async fn websocket_reconnect_after_disconnect() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_reconnect_after_disconnect";
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

    // Connect and disconnect multiple times
    for _ in 0..3 {
        let mut client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;
        let _ack = client.recv_json_timeout(Duration::from_secs(5)).await?;
        let _snapshot = client.recv_json_timeout(Duration::from_secs(5)).await?;
        client.close().await?;
        wait_for_connections(&registry, 0, Duration::from_secs(1)).await?;
    }

    // Final connection should still work
    let mut final_client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;
    let final_ack = final_client
        .recv_json_timeout(Duration::from_secs(5))
        .await?;
    assert_eq!(final_ack.expect("should receive ack")["type"], "ack");

    // Close client and stop server to release all references to SharedTxn
    final_client.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(1)).await?;
    server_handle.stop(true).await;
    if let Err(e) = server_join.await {
        eprintln!("Test server join error: {:?}", e);
    }
    tokio::task::yield_now().await;

    // Rollback transaction at end of test
    rollback_eventually(shared).await?;

    Ok(())
}
