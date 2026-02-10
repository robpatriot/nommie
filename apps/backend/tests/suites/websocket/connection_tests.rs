// apps/backend/tests/ws/connection_tests.rs
// WebSocket connection + handshake + initial subscription snapshot tests

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
async fn websocket_hello_ack_succeeds_with_valid_jwt() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_hello_ack_valid_jwt";
    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let _user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);

    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    let ws_url = format!("ws://{}/ws?token={}", addr, token);
    let mut client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;

    let hello_ack = client.hello().await?;
    assert_eq!(hello_ack["type"], "hello_ack");
    assert_eq!(hello_ack["protocol"], 1);
    assert!(hello_ack.get("user_id").is_some());

    client.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}

#[tokio::test]
async fn websocket_subscribe_ack_then_snapshot_ordering() -> Result<(), Box<dyn std::error::Error>>
{
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_subscribe_ack_then_snapshot";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;

    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    attach_human_to_seat(shared.transaction(), setup.game_id, 0, user_id).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);

    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;
    let ws_url = format!("ws://{}/ws?token={}", addr, token);

    let mut client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;
    client.hello().await?;

    let (ack, game_state) = client.subscribe_game(setup.game_id).await?;
    assert_eq!(ack["type"], "ack");
    assert_eq!(ack["command"], "subscribe");
    assert_eq!(ack["topic"]["kind"], "game");
    assert_eq!(ack["topic"]["id"], setup.game_id);

    assert_eq!(game_state["type"], "game_state");
    assert_eq!(game_state["topic"]["kind"], "game");
    assert_eq!(game_state["topic"]["id"], setup.game_id);

    // New shape: game + viewer, no "data"
    assert!(game_state.get("game").is_some());
    assert!(game_state.get("viewer").is_some());
    assert!(game_state.get("version").is_some());

    client.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}

#[tokio::test]
async fn websocket_ack_semantics_subscribe_and_unsubscribe(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_ack_semantics";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;

    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    attach_human_to_seat(shared.transaction(), setup.game_id, 0, user_id).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);

    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;
    let ws_url = format!("ws://{}/ws?token={}", addr, token);

    let mut client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;
    client.hello().await?;

    let (sub_ack, _game_state) = client.subscribe_game(setup.game_id).await?;
    assert_eq!(sub_ack["type"], "ack");
    assert_eq!(sub_ack["command"], "subscribe");
    assert_eq!(sub_ack["topic"]["kind"], "game");
    assert_eq!(sub_ack["topic"]["id"], setup.game_id);

    let unsub_ack = client.unsubscribe_game(setup.game_id).await?;
    assert_eq!(unsub_ack["type"], "ack");
    assert_eq!(unsub_ack["command"], "unsubscribe");
    assert_eq!(unsub_ack["topic"]["kind"], "game");
    assert_eq!(unsub_ack["topic"]["id"], setup.game_id);

    client.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}
