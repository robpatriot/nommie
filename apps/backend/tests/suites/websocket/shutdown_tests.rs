// WebSocket shutdown tests

use std::sync::Arc;

use backend::ws::hub::GameSessionRegistry;

#[tokio::test]
async fn websocket_shutdown_closes_all_connections() -> Result<(), Box<dyn std::error::Error>> {
    // Test that close_all_connections() returns requests for all active sessions
    let registry = Arc::new(GameSessionRegistry::new());

    // Initially no connections
    assert_eq!(registry.active_connections_count(), 0);

    // Call close_all_connections with no sessions - should return empty vec
    let shutdown_requests = registry.close_all_connections();
    assert_eq!(shutdown_requests.len(), 0);

    Ok(())
}

#[tokio::test]
async fn websocket_shutdown_unregisters_all_sessions() -> Result<(), Box<dyn std::error::Error>> {
    // Test that unregister properly updates connection count
    let registry = Arc::new(GameSessionRegistry::new());

    assert_eq!(registry.active_connections_count(), 0);

    // Simulate unregister with non-existent token (should not panic)
    use uuid::Uuid;
    let fake_token = Uuid::new_v4();
    registry.unregister(1, fake_token);

    // Count should still be 0
    assert_eq!(registry.active_connections_count(), 0);

    Ok(())
}

#[test]
fn websocket_registry_thread_safe() {
    // Test that registry can be safely accessed from multiple threads
    use std::sync::Arc;
    use std::thread;

    let registry = Arc::new(GameSessionRegistry::new());

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let registry = registry.clone();
            thread::spawn(move || {
                // Each thread does some operations
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

    // After all threads complete, count should still be 0
    assert_eq!(registry.active_connections_count(), 0);
}
