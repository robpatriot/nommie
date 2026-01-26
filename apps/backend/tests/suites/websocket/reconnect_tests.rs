// apps/backend/tests/ws/reconnect_tests.rs
// WebSocket reconnection tests (handshake + subscribe + snapshot on reconnect)

use std::time::Duration;

use backend::db::require_db;

use crate::support::auth::mint_test_token;
use crate::support::build_test_state;
use crate::support::db_memberships::attach_human_to_seat;
use crate::support::factory::create_test_user;
use crate::support::game_setup::setup_game_with_players;
use crate::support::txn_helpers::rollback_eventually;
use crate::support::websocket::{attach_test_registry, start_test_server, wait_for_connections};
use crate::support::websocket_client::WebSocketClient;

#[tokio::test]
async fn websocket_reconnect_receives_snapshot_after_resubscribe(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_reconnect_snapshot";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;
    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    attach_human_to_seat(shared.transaction(), setup.game_id, 0, user_id).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    let ws_url = format!("ws://{}/ws?token={}", addr, token);

    let mut client1 = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;
    client1.hello().await?;
    let (_ack1, state1) = client1.subscribe_game(setup.game_id).await?;
    assert_eq!(state1["type"], "game_state");

    client1.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;

    let mut client2 = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;
    client2.hello().await?;
    let (_ack2, state2) = client2.subscribe_game(setup.game_id).await?;
    assert_eq!(state2["type"], "game_state");

    // New shape: game + viewer, no "data"
    assert!(state1.get("game").is_some());
    assert!(state1.get("viewer").is_some());
    assert!(state2.get("game").is_some());
    assert!(state2.get("viewer").is_some());

    client2.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}

#[tokio::test]
async fn websocket_reconnect_after_multiple_disconnects() -> Result<(), Box<dyn std::error::Error>>
{
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_reconnect_after_disconnect";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;
    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    attach_human_to_seat(shared.transaction(), setup.game_id, 0, user_id).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    let ws_url = format!("ws://{}/ws?token={}", addr, token);

    for _ in 0..3 {
        let mut client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;
        client.hello().await?;
        let (_ack, game_state) = client.subscribe_game(setup.game_id).await?;
        assert_eq!(game_state["type"], "game_state");
        client.close().await?;
        wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    }

    let mut final_client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;
    let hello_ack = final_client.hello().await?;
    assert_eq!(hello_ack["type"], "hello_ack");

    let (ack, game_state) = final_client.subscribe_game(setup.game_id).await?;
    assert_eq!(ack["type"], "ack");
    assert_eq!(game_state["type"], "game_state");

    final_client.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}
