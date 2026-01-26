// apps/backend/tests/ws/shutdown_tests.rs
// WebSocket shutdown tests

use std::sync::Arc;

use backend::ws::hub::WsRegistry;

#[tokio::test]
async fn websocket_shutdown_closes_all_connections_when_none(
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(WsRegistry::new());

    assert_eq!(registry.active_connections_count(), 0);

    let shutdown_requests = registry.close_all_connections();
    assert_eq!(shutdown_requests.len(), 0);

    Ok(())
}

#[tokio::test]
async fn websocket_shutdown_unregister_nonexistent_conn_is_safe(
) -> Result<(), Box<dyn std::error::Error>> {
    let registry = Arc::new(WsRegistry::new());

    assert_eq!(registry.active_connections_count(), 0);

    use uuid::Uuid;
    let fake_conn_id = Uuid::new_v4();
    registry.unregister_connection(fake_conn_id);

    assert_eq!(registry.active_connections_count(), 0);
    Ok(())
}

#[test]
fn websocket_registry_thread_safe() {
    use std::thread;

    let registry = Arc::new(WsRegistry::new());

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let registry = registry.clone();
            thread::spawn(move || {
                for _ in 0..100 {
                    let count = registry.active_connections_count();
                    assert_eq!(count, 0);
                }
                i
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(registry.active_connections_count(), 0);
}
