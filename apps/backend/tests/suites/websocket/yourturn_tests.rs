// apps/backend/tests/suites/websocket/yourturn_tests.rs
// WebSocket user-scoped your_turn delivery tests

use std::time::Duration;

use backend::db::require_db;
use backend::ws::session::HubEvent;

use crate::support::auth::mint_test_token;
use crate::support::build_test_state;
use crate::support::factory::create_test_user;
use crate::support::game_setup::setup_game_with_players;
use crate::support::test_utils::test_user_sub;
use crate::support::txn_helpers::rollback_eventually;
use crate::support::websocket::{attach_test_registry, start_test_server, wait_for_connections};
use crate::support::websocket_client::WebSocketClient;

#[tokio::test]
async fn websocket_yourturn_delivered_to_target_user() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_yourturn_delivered_to_target_user";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;

    let user_sub = format!("{test_name}_user");
    let user_email = format!("{test_name}@example.com");
    let user_id = create_test_user(shared.transaction(), &user_sub, Some("Test User")).await?;

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    let ws_url = format!("ws://{}/ws?token={}", addr, token);

    let mut client = WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;
    client.hello().await?;

    // Broadcast a user-scoped your_turn (no subscription required)
    let version = 123;
    registry.broadcast_to_user(
        user_id,
        HubEvent::YourTurn {
            game_id: setup.game_id,
            version,
        },
    );

    let msg = client
        .recv_type(Duration::from_secs(2), "your_turn")
        .await?;
    assert_eq!(msg["version"], version);
    assert_eq!(msg["type"], "your_turn");
    assert_eq!(msg["game_id"], setup.game_id);

    client.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}

#[tokio::test]
async fn websocket_yourturn_not_delivered_to_other_users() -> Result<(), Box<dyn std::error::Error>>
{
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_yourturn_not_delivered_to_other_users";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;

    let user_a_sub = format!("{test_name}_user_a");
    let user_a_email = format!("{test_name}_a@example.com");
    let user_a_id = create_test_user(shared.transaction(), &user_a_sub, Some("User A")).await?;

    let user_b_sub = format!("{test_name}_user_b");
    let user_b_email = format!("{test_name}_b@example.com");
    let _user_b_id = create_test_user(shared.transaction(), &user_b_sub, Some("User B")).await?;

    let (state, registry) = attach_test_registry(state);
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    // Connect user A
    let token_a = mint_test_token(&user_a_sub, &user_a_email, &security);
    let ws_url_a = format!("ws://{}/ws?token={}", addr, token_a);
    let mut client_a = WebSocketClient::connect_retry(&ws_url_a, Duration::from_secs(1)).await?;
    client_a.hello().await?;

    // Connect user B
    let token_b = mint_test_token(&user_b_sub, &user_b_email, &security);
    let ws_url_b = format!("ws://{}/ws?token={}", addr, token_b);
    let mut client_b = WebSocketClient::connect_retry(&ws_url_b, Duration::from_secs(1)).await?;
    client_b.hello().await?;

    // Broadcast to A only
    registry.broadcast_to_user(
        user_a_id,
        HubEvent::YourTurn {
            game_id: setup.game_id,
            version: 999,
        },
    );

    // A should receive it
    let msg_a = client_a
        .recv_type(Duration::from_secs(2), "your_turn")
        .await?;
    assert_eq!(msg_a["game_id"], setup.game_id);

    // B should not receive it (short timeout; success means "no message")
    let maybe_b = client_b
        .recv_json_timeout(Duration::from_millis(250))
        .await?;
    if let Some(v) = maybe_b {
        // If anything arrived, it must not be a your_turn.
        assert_ne!(v.get("type").and_then(|t| t.as_str()), Some("your_turn"));
    }

    client_a.close().await?;
    client_b.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}

#[tokio::test]
async fn websocket_yourturn_not_delivered_to_sessions_already_in_that_game(
) -> Result<(), Box<dyn std::error::Error>> {
    use serde_json::json;

    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "ws_yourturn_not_delivered_to_sessions_already_in_that_game";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;

    // Use an existing game member (player 0) created by setup_game_with_players_ex.
    let player_idx = 0usize;
    let user_id = setup.user_ids[player_idx];

    // Reconstruct the user's sub the same way setup_game_with_players_ex does.
    let user_sub = test_user_sub(&format!("{}_player_{}", test_name, player_idx));

    // Email isn't used for membership; provide a stable value for minting.
    let user_email = format!("{user_sub}@example.com");

    let (state, registry) = attach_test_registry(state);
    let token = mint_test_token(&user_sub, &user_email, &security);
    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    let ws_url = format!("ws://{}/ws?token={}", addr, token);

    // Connect TWO clients as the SAME user
    let mut client_in_game =
        WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;
    client_in_game.hello().await?;

    let mut client_not_in_game =
        WebSocketClient::connect_retry(&ws_url, Duration::from_secs(1)).await?;
    client_not_in_game.hello().await?;

    // Subscribe ONLY the first client to the game topic
    let sub_msg = json!({
        "type": "subscribe",
        "topic": { "kind": "game", "id": setup.game_id }
    });
    client_in_game.send_json(&sub_msg).await?;

    // Wait for explicit subscribe ack
    let ack = client_in_game
        .recv_type(Duration::from_secs(2), "ack")
        .await?;
    assert_eq!(ack["command"], "subscribe");
    assert_eq!(ack["topic"]["kind"], "game");
    assert_eq!(ack["topic"]["id"], setup.game_id);

    // Broadcast a user-scoped your_turn, excluding sessions already in that game
    let version = 456;
    registry.broadcast_to_user_excl_topic(
        user_id,
        HubEvent::YourTurn {
            game_id: setup.game_id,
            version,
        },
    );

    // The client NOT subscribed to the game should receive it
    let msg_ok = client_not_in_game
        .recv_type(Duration::from_secs(2), "your_turn")
        .await?;
    assert_eq!(msg_ok["version"], version);
    assert_eq!(msg_ok["type"], "your_turn");
    assert_eq!(msg_ok["game_id"], setup.game_id);

    // The client already in the game should NOT receive it
    let maybe_in_game = client_in_game
        .recv_json_timeout(Duration::from_millis(250))
        .await?;
    if let Some(v) = maybe_in_game {
        assert_ne!(v.get("type").and_then(|t| t.as_str()), Some("your_turn"));
    }

    client_in_game.close().await?;
    client_not_in_game.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}
