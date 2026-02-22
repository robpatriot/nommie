// apps/backend/tests/ws/broadcast_tests.rs
// Multi-client broadcast tests (topic isolation + fan-out)

use std::time::Duration;

use backend::db::require_db;

use crate::support::auth::mint_test_token;
use crate::support::build_test_state;
use crate::support::db_memberships::attach_human_to_seat;
use crate::support::factory::create_test_user;
use crate::support::game_setup::setup_game_with_players;
use crate::support::txn_helpers::rollback_eventually;
use crate::support::websocket::{
    attach_test_registry, broadcast_snapshot, start_test_server, wait_for_connections,
};
use crate::support::websocket_client::WebSocketClient;

#[tokio::test]
async fn broadcast_reaches_all_subscribed_clients() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "broadcast_all_clients";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;

    let user1_sub = format!("{test_name}_user1");
    let user1_email = format!("{test_name}_user1@example.com");
    let user1_id = create_test_user(shared.transaction(), &user1_sub, Some("User1")).await?;

    let user2_sub = format!("{test_name}_user2");
    let user2_email = format!("{test_name}_user2@example.com");
    let user2_id = create_test_user(shared.transaction(), &user2_sub, Some("User2")).await?;

    attach_human_to_seat(shared.transaction(), setup.game_id, 0, user1_id).await?;
    attach_human_to_seat(shared.transaction(), setup.game_id, 1, user2_id).await?;

    let (state, registry) = attach_test_registry(state);
    let token1 = mint_test_token(&user1_sub, &user1_email, &security);
    let token2 = mint_test_token(&user2_sub, &user2_email, &security);

    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    let ws_url1 = format!("ws://{}/ws?token={}", addr, token1);
    let mut client1 = WebSocketClient::connect_retry(&ws_url1, Duration::from_secs(1)).await?;
    client1.hello().await?;
    let (_ack1, _state1) = client1.subscribe_game(setup.game_id).await?;

    let ws_url2 = format!("ws://{}/ws?token={}", addr, token2);
    let mut client2 = WebSocketClient::connect_retry(&ws_url2, Duration::from_secs(1)).await?;
    client2.hello().await?;
    let (_ack2, _state2) = client2.subscribe_game(setup.game_id).await?;

    wait_for_connections(&registry, 2, Duration::from_secs(1)).await?;

    broadcast_snapshot(&registry, setup.game_id, 2);

    let msg1 = client1
        .recv_json_timeout(Duration::from_secs(5))
        .await?
        .expect("client1 should receive broadcast");
    let msg2 = client2
        .recv_json_timeout(Duration::from_secs(5))
        .await?
        .expect("client2 should receive broadcast");

    assert_eq!(msg1["type"], "game_state");
    assert_eq!(msg2["type"], "game_state");

    // Sanity: new shape exists
    assert!(msg1.get("topic").is_some());
    assert!(msg1.get("version").is_some());
    assert!(msg1.get("game").is_some());
    assert!(msg1.get("viewer").is_some());

    assert!(msg2.get("topic").is_some());
    assert!(msg2.get("version").is_some());
    assert!(msg2.get("game").is_some());
    assert!(msg2.get("viewer").is_some());

    client1.close().await?;
    client2.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}

#[tokio::test]
async fn broadcast_only_sent_to_same_game_subscribers() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name1 = "broadcast_game1";
    let test_name2 = "broadcast_game2";
    let setup1 = setup_game_with_players(shared.transaction(), test_name1).await?;
    let setup2 = setup_game_with_players(shared.transaction(), test_name2).await?;

    let user1_sub = format!("{test_name1}_user");
    let user1_email = format!("{test_name1}@example.com");
    let user1_id = create_test_user(shared.transaction(), &user1_sub, Some("User1")).await?;

    let user2_sub = format!("{test_name2}_user");
    let user2_email = format!("{test_name2}@example.com");
    let user2_id = create_test_user(shared.transaction(), &user2_sub, Some("User2")).await?;

    attach_human_to_seat(shared.transaction(), setup1.game_id, 0, user1_id).await?;
    attach_human_to_seat(shared.transaction(), setup2.game_id, 0, user2_id).await?;

    let (state, registry) = attach_test_registry(state);
    let token1 = mint_test_token(&user1_sub, &user1_email, &security);
    let token2 = mint_test_token(&user2_sub, &user2_email, &security);

    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    let ws_url1 = format!("ws://{}/ws?token={}", addr, token1);
    let ws_url2 = format!("ws://{}/ws?token={}", addr, token2);

    let (client1, client2) = tokio::join!(
        async {
            let mut c = WebSocketClient::connect_retry(&ws_url1, Duration::from_secs(1)).await?;
            c.hello().await?;
            let _ = c.subscribe_game(setup1.game_id).await?;
            Ok::<_, Box<dyn std::error::Error>>(c)
        },
        async {
            let mut c = WebSocketClient::connect_retry(&ws_url2, Duration::from_secs(1)).await?;
            c.hello().await?;
            let _ = c.subscribe_game(setup2.game_id).await?;
            Ok::<_, Box<dyn std::error::Error>>(c)
        },
    );
    let mut client1 = client1?;
    let mut client2 = client2?;

    wait_for_connections(&registry, 2, Duration::from_secs(1)).await?;

    broadcast_snapshot(&registry, setup1.game_id, 2);

    let msg1 = client1
        .recv_json_timeout(Duration::from_secs(5))
        .await?
        .expect("client1 should receive broadcast");
    assert_eq!(msg1["type"], "game_state");

    let msg2_result = client2.recv_json_timeout(Duration::from_millis(200)).await;
    assert!(
        msg2_result.is_err() || msg2_result.unwrap().is_none(),
        "client2 should not receive broadcast for different game"
    );

    client1.close().await?;
    client2.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(2)).await?;
    server_handle.stop(true).await;
    let _ = server_join.await;

    rollback_eventually(shared).await?;
    Ok(())
}
