// Multi-client broadcast tests

use std::time::Duration;

use backend::db::require_db;

use crate::support::auth::mint_test_token;
use crate::support::build_test_state;
use crate::support::factory::create_test_user;
use crate::support::game_setup::setup_game_with_players;
use crate::support::txn_helpers::rollback_eventually;
use crate::support::websocket::{
    attach_test_registry, broadcast_snapshot, start_test_server, wait_for_connections,
};
use crate::support::websocket_client::WebSocketClient;

#[tokio::test]
async fn broadcast_reaches_all_connected_clients() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared = backend::db::txn::SharedTxn::open(db).await?;

    let test_name = "broadcast_all_clients";
    let setup = setup_game_with_players(shared.transaction(), test_name).await?;

    // Create two users and assign them to different seats
    let user1_sub = format!("{test_name}_user1");
    let user1_email = format!("{test_name}_user1@example.com");
    let user1_id = create_test_user(shared.transaction(), &user1_sub, Some("User1")).await?;

    let user2_sub = format!("{test_name}_user2");
    let user2_email = format!("{test_name}_user2@example.com");
    let user2_id = create_test_user(shared.transaction(), &user2_sub, Some("User2")).await?;

    use backend::entities::game_players;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

    // Assign user1 to seat 0
    let membership1 = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(setup.game_id))
        .filter(game_players::Column::TurnOrder.eq(0))
        .one(shared.transaction())
        .await?
        .expect("membership should exist");
    let mut membership1: game_players::ActiveModel = membership1.into();
    membership1.human_user_id = sea_orm::Set(Some(user1_id));
    ActiveModelTrait::update(membership1, shared.transaction()).await?;

    // Assign user2 to seat 1
    let membership2 = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(setup.game_id))
        .filter(game_players::Column::TurnOrder.eq(1))
        .one(shared.transaction())
        .await?
        .expect("membership should exist");
    let mut membership2: game_players::ActiveModel = membership2.into();
    membership2.human_user_id = sea_orm::Set(Some(user2_id));
    ActiveModelTrait::update(membership2, shared.transaction()).await?;

    let (state, registry) = attach_test_registry(state);
    let token1 = mint_test_token(&user1_sub, &user1_email, &security);
    let token2 = mint_test_token(&user2_sub, &user2_email, &security);

    let (server_handle, addr, server_join) = start_test_server(state, shared.clone()).await?;

    // Connect two clients
    let ws_url1 = format!("ws://{}/ws/games/{}?token={}", addr, setup.game_id, token1);
    let mut client1 = WebSocketClient::connect_retry(&ws_url1, Duration::from_secs(1)).await?;

    let ws_url2 = format!("ws://{}/ws/games/{}?token={}", addr, setup.game_id, token2);
    let mut client2 = WebSocketClient::connect_retry(&ws_url2, Duration::from_secs(1)).await?;

    // Skip initial messages (ack + snapshot)
    let _ = client1.recv_json_timeout(Duration::from_secs(5)).await?;
    let _ = client1.recv_json_timeout(Duration::from_secs(5)).await?;
    let _ = client2.recv_json_timeout(Duration::from_secs(5)).await?;
    let _ = client2.recv_json_timeout(Duration::from_secs(5)).await?;
    wait_for_connections(&registry, 2, Duration::from_secs(1)).await?;

    // Simulate game mutation triggering broadcast
    broadcast_snapshot(&registry, setup.game_id, 2);

    // Both clients should receive the broadcast
    let msg1 = client1
        .recv_json_timeout(Duration::from_secs(5))
        .await?
        .expect("client1 should receive broadcast");
    let msg2 = client2
        .recv_json_timeout(Duration::from_secs(5))
        .await?
        .expect("client2 should receive broadcast");

    assert_eq!(msg1["type"], "snapshot");
    assert_eq!(msg2["type"], "snapshot");

    // Close clients and stop server to release all references to SharedTxn
    client1.close().await?;
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
async fn broadcast_only_sent_to_same_game_clients() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let security = state.security.clone();
    let db = require_db(&state)?;
    let shared1 = backend::db::txn::SharedTxn::open(db).await?;

    // Create two games
    let test_name1 = "broadcast_game1";
    let test_name2 = "broadcast_game2";
    let setup1 = setup_game_with_players(shared1.transaction(), test_name1).await?;
    let setup2 = setup_game_with_players(shared1.transaction(), test_name2).await?;

    // Create users
    let user1_sub = format!("{test_name1}_user");
    let user1_email = format!("{test_name1}@example.com");
    let user1_id = create_test_user(shared1.transaction(), &user1_sub, Some("User1")).await?;

    let user2_sub = format!("{test_name2}_user");
    let user2_email = format!("{test_name2}@example.com");
    let user2_id = create_test_user(shared1.transaction(), &user2_sub, Some("User2")).await?;

    use backend::entities::game_players;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

    // Assign users to their respective games
    let membership1 = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(setup1.game_id))
        .filter(game_players::Column::TurnOrder.eq(0))
        .one(shared1.transaction())
        .await?
        .expect("membership should exist");
    let mut membership1: game_players::ActiveModel = membership1.into();
    membership1.human_user_id = sea_orm::Set(Some(user1_id));
    ActiveModelTrait::update(membership1, shared1.transaction()).await?;

    let membership2 = game_players::Entity::find()
        .filter(game_players::Column::GameId.eq(setup2.game_id))
        .filter(game_players::Column::TurnOrder.eq(0))
        .one(shared1.transaction())
        .await?
        .expect("membership should exist");
    let mut membership2: game_players::ActiveModel = membership2.into();
    membership2.human_user_id = sea_orm::Set(Some(user2_id));
    ActiveModelTrait::update(membership2, shared1.transaction()).await?;

    let (state, registry) = attach_test_registry(state);
    let token1 = mint_test_token(&user1_sub, &user1_email, &security);
    let token2 = mint_test_token(&user2_sub, &user2_email, &security);

    let (server_handle, addr, server_join) = start_test_server(state, shared1.clone()).await?;

    // Connect clients to different games
    let ws_url1 = format!("ws://{}/ws/games/{}?token={}", addr, setup1.game_id, token1);
    let mut client1 = WebSocketClient::connect_retry(&ws_url1, Duration::from_secs(1)).await?;

    let ws_url2 = format!("ws://{}/ws/games/{}?token={}", addr, setup2.game_id, token2);
    let mut client2 = WebSocketClient::connect_retry(&ws_url2, Duration::from_secs(1)).await?;

    // Skip initial messages
    let _ = client1.recv_json_timeout(Duration::from_secs(5)).await?;
    let _ = client1.recv_json_timeout(Duration::from_secs(5)).await?;
    let _ = client2.recv_json_timeout(Duration::from_secs(5)).await?;
    let _ = client2.recv_json_timeout(Duration::from_secs(5)).await?;
    wait_for_connections(&registry, 2, Duration::from_secs(1)).await?;

    // Broadcast to game1 only
    broadcast_snapshot(&registry, setup1.game_id, 2);

    // Only client1 should receive the broadcast
    let msg1 = client1
        .recv_json_timeout(Duration::from_secs(5))
        .await?
        .expect("client1 should receive broadcast");

    assert_eq!(msg1["type"], "snapshot");

    // client2 should NOT receive anything (use short timeout)
    let msg2_result = client2.recv_json_timeout(Duration::from_millis(500)).await;
    assert!(
        msg2_result.is_err() || msg2_result.unwrap().is_none(),
        "client2 should not receive broadcast for different game"
    );

    // Close clients and stop server to release all references to SharedTxn
    client1.close().await?;
    client2.close().await?;
    wait_for_connections(&registry, 0, Duration::from_secs(1)).await?;
    server_handle.stop(true).await;
    if let Err(e) = server_join.await {
        eprintln!("Test server join error: {:?}", e);
    }
    tokio::task::yield_now().await;

    // Rollback transaction at end of test
    rollback_eventually(shared1).await?;

    Ok(())
}
