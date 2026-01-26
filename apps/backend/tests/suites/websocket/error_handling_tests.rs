// apps/backend/tests/ws/error_handling_tests.rs
// Error handling tests for WebSocket protocol

use std::time::Duration;

use backend::db::require_db;
use serde_json::json;

use crate::support::auth::mint_test_token;
use crate::support::build_test_state;
use crate::support::factory::create_test_user;
use crate::support::game_setup::setup_game_with_players;
use crate::support::txn_helpers::rollback_eventually;
use crate::support::websocket::{attach_test_registry, start_test_server, wait_for_connections};
use crate::support::websocket_client::WebSocketClient;

#[tokio::test]
async fn websocket_bad_protocol_sends_error_and_closes() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_bad_protocol";
    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let _user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    let ws_url = format!("ws://{}/ws?token={}", addr, token);
    let mut client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;

    client
        .send_json(&json!({ "type": "hello", "protocol": 999 }))
        .await?;

    // For error-handling tests, we must allow receiving "error" without the client failing fast.
    let err = client
        .recv_type_allow_error(Duration::from_secs(5), "error")
        .await?;
    assert_eq!(err["code"], "bad_protocol");
    assert!(err.get("message").is_some());

    // The server should close after sending the error.
    let after = client
        .recv_json_allow_error(Duration::from_millis(500))
        .await?;
    assert!(after.is_none(), "expected socket to close after error");

    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}

#[tokio::test]
async fn websocket_malformed_json_sends_bad_request_and_closes(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_bad_json";
    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let _user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    let ws_url = format!("ws://{}/ws?token={}", addr, token);
    let mut client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;

    client.send("{").await?;

    let err = client
        .recv_type_allow_error(Duration::from_secs(5), "error")
        .await?;
    assert_eq!(err["code"], "bad_request");
    assert!(err.get("message").is_some());

    let after = client
        .recv_json_allow_error(Duration::from_millis(500))
        .await?;
    assert!(after.is_none(), "expected socket to close after error");

    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}

#[tokio::test]
async fn websocket_subscribe_before_hello_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_subscribe_before_hello";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;

    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let _user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    let ws_url = format!("ws://{}/ws?token={}", addr, token);
    let mut client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;

    client
        .send_json(
            &json!({ "type": "subscribe", "topic": { "kind": "game", "id": setup.game_id } }),
        )
        .await?;

    let err = client
        .recv_type_allow_error(Duration::from_secs(5), "error")
        .await?;
    assert_eq!(err["code"], "bad_request");
    assert!(err.get("message").is_some());

    let after = client
        .recv_json_allow_error(Duration::from_millis(500))
        .await?;
    assert!(after.is_none(), "expected socket to close after error");

    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}

#[tokio::test]
async fn websocket_unauthorized_subscription_is_forbidden() -> Result<(), Box<dyn std::error::Error>>
{
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_forbidden_subscription";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;

    let user_sub = format!("{test_name}_outsider");
    let user_email = format!("{test_name}_outsider@example.com");
    let _user_id = create_test_user(shared.transaction(), &user_sub, Some("Outsider")).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    let ws_url = format!("ws://{}/ws?token={}", addr, token);
    let mut client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;

    client.hello().await?;

    client
        .send_json(
            &json!({ "type": "subscribe", "topic": { "kind": "game", "id": setup.game_id } }),
        )
        .await?;

    // Forbidden should be a protocol-visible error that does NOT close the socket.
    let err = client
        .recv_type_allow_error(Duration::from_secs(5), "error")
        .await?;
    assert_eq!(err["code"], "forbidden");
    assert_eq!(err["message"], "Not a member of this game");

    // Ensure the socket is still open (no close immediately after forbidden).
    // We expect no immediate follow-up message.
    let after = client
        .recv_json_allow_error(Duration::from_millis(250))
        .await?;
    assert!(after.is_none(), "expected no immediate follow-up message");

    client.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}
