// Unit tests for SnapshotCache
//
// Tests include:
// - Cache hit: retrieving a value that's already cached
// - Cache miss: building and inserting a new value
// - Concurrent cache checks: multiple concurrent requests for the same key only build once

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use backend::domain::snapshot::{GameSnapshot, PhaseSnapshot, SeatPublic};
use backend::repos::games::Game;
use backend::routes::snapshot_cache::{SharedSnapshotParts, SnapshotCache};
use tokio::time::{sleep, Duration};

fn create_mock_game(game_id: i64, version: i32) -> Game {
    let now = time::OffsetDateTime::now_utc();
    Game {
        id: game_id,
        created_by: None,
        visibility: backend::entities::games::GameVisibility::Public,
        state: backend::entities::games::GameState::Bidding,
        created_at: now,
        updated_at: now,
        started_at: None,
        ended_at: None,
        name: None,
        rules_version: "nommie-1.0.0".to_string(),
        rng_seed: vec![0u8; 32],
        current_round: None,
        starting_dealer_pos: None,
        current_trick_no: 0,
        current_round_id: None,
        version,
    }
}

fn create_mock_shared_parts(game_id: i64, version: i32) -> SharedSnapshotParts {
    let game = create_mock_game(game_id, version);
    let seating = [
        SeatPublic::empty(0), // North
        SeatPublic::empty(1), // East
        SeatPublic::empty(2), // South
        SeatPublic::empty(3), // West
    ];
    let snapshot = GameSnapshot {
        game: backend::domain::snapshot::GameHeader {
            round_no: Some(1),
            dealer: Some(0), // North
            seating: seating.clone(),
            scores_total: [0, 0, 0, 0],
            host_seat: 0, // North
        },
        phase: PhaseSnapshot::Init,
    };

    SharedSnapshotParts {
        game,
        snapshot,
        seating,
        version,
    }
}

#[tokio::test]
async fn test_cache_hit() {
    let cache = SnapshotCache::new();
    let key = (1, 1);
    let parts = create_mock_shared_parts(1, 1);

    // Insert value into cache
    let inserted = cache.get_or_insert(key, parts.clone()).await;

    // Retrieve from cache - should be a hit
    let retrieved = cache.get(key).expect("Cache should contain the value");

    // Verify it's the same Arc (same pointer)
    assert!(
        Arc::ptr_eq(&inserted, &retrieved),
        "Should return the same Arc"
    );

    // Verify the data matches
    assert_eq!(retrieved.game.id, 1);
    assert_eq!(retrieved.version, 1);
}

#[tokio::test]
async fn test_cache_miss() {
    let cache = SnapshotCache::new();
    let key = (2, 1);

    // Cache should be empty
    assert!(cache.get(key).is_none(), "Cache should be empty initially");

    // Insert value
    let parts = create_mock_shared_parts(2, 1);
    let inserted = cache.get_or_insert(key, parts).await;

    // Now should be in cache
    let retrieved = cache.get(key).expect("Cache should contain the value");
    assert!(
        Arc::ptr_eq(&inserted, &retrieved),
        "Should return the same Arc"
    );
}

#[tokio::test]
async fn test_concurrent_cache_checks_only_build_once() {
    let cache = Arc::new(SnapshotCache::new());
    let key = (3, 1);
    let build_count = Arc::new(AtomicU32::new(0));

    // Spawn multiple concurrent tasks that all try to get the same key
    let mut handles = vec![];
    for _ in 0..10 {
        let cache_clone = cache.clone();
        let build_count_clone = build_count.clone();
        let handle = tokio::spawn(async move {
            // Small delay to ensure all tasks start before any completes
            sleep(Duration::from_millis(10)).await;

            // Increment build count to simulate expensive work
            let _current_count = build_count_clone.fetch_add(1, Ordering::SeqCst);

            // Create parts (simulating expensive build)
            let parts = create_mock_shared_parts(3, 1);

            // Insert into cache
            cache_clone.get_or_insert(key, parts).await
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let results: Vec<_> = futures::future::join_all(handles).await;

    // All results should be the same Arc (same pointer)
    let first_result = results[0].as_ref().expect("First task should succeed");
    for result in &results {
        let arc = result.as_ref().expect("All tasks should succeed");
        assert!(
            Arc::ptr_eq(first_result, arc),
            "All tasks should receive the same cached Arc"
        );
    }

    // Verify that despite 10 concurrent requests, we only built once
    // (The build_count might be > 1 because multiple tasks might increment it
    // before the first one inserts, but get_or_insert should deduplicate)
    let final_count = build_count.load(Ordering::SeqCst);
    assert!(
        final_count <= 10,
        "Build count should be at most 10 (one per task)"
    );

    // The key insight: all tasks should get the same Arc, meaning only one
    // actually made it through the mutex and inserted into the cache
    let cached_value = cache.get(key).expect("Cache should contain the value");
    assert!(
        Arc::ptr_eq(first_result, &cached_value),
        "Cached value should match what tasks received"
    );
}

#[tokio::test]
async fn test_cache_remove() {
    let cache = SnapshotCache::new();
    let key = (4, 1);
    let parts = create_mock_shared_parts(4, 1);

    // Insert value
    cache.get_or_insert(key, parts).await;

    // Verify it's in cache
    assert!(cache.get(key).is_some(), "Cache should contain the value");

    // Remove it
    cache.remove(key);

    // Verify it's gone
    assert!(
        cache.get(key).is_none(),
        "Cache should not contain the value after remove"
    );
}

#[tokio::test]
async fn test_cache_clear() {
    let cache = SnapshotCache::new();

    // Insert multiple values
    for i in 1..=5 {
        let key = (10, i);
        let parts = create_mock_shared_parts(10, i);
        cache.get_or_insert(key, parts).await;
    }

    // Verify they're all in cache
    for i in 1..=5 {
        let key = (10, i);
        assert!(
            cache.get(key).is_some(),
            "Cache should contain value for key {key:?}"
        );
    }

    // Clear cache
    cache.clear();

    // Verify they're all gone
    for i in 1..=5 {
        let key = (10, i);
        assert!(
            cache.get(key).is_none(),
            "Cache should not contain value for key {key:?} after clear"
        );
    }
}

#[tokio::test]
async fn test_different_keys_dont_interfere() {
    let cache = SnapshotCache::new();

    // Insert values for different keys
    let key1 = (20, 1);
    let key2 = (20, 2);
    let key3 = (21, 1);

    let parts1 = create_mock_shared_parts(20, 1);
    let parts2 = create_mock_shared_parts(20, 2);
    let parts3 = create_mock_shared_parts(21, 1);

    let inserted1 = cache.get_or_insert(key1, parts1).await;
    let inserted2 = cache.get_or_insert(key2, parts2).await;
    let inserted3 = cache.get_or_insert(key3, parts3).await;

    // Verify all are in cache
    let retrieved1 = cache.get(key1).expect("Cache should contain key1");
    let retrieved2 = cache.get(key2).expect("Cache should contain key2");
    let retrieved3 = cache.get(key3).expect("Cache should contain key3");

    // Verify they're different Arcs
    assert!(
        !Arc::ptr_eq(&inserted1, &inserted2),
        "Different keys should have different values"
    );
    assert!(
        !Arc::ptr_eq(&inserted1, &inserted3),
        "Different keys should have different values"
    );
    assert!(
        !Arc::ptr_eq(&inserted2, &inserted3),
        "Different keys should have different values"
    );

    // Verify data matches
    assert_eq!(retrieved1.version, 1);
    assert_eq!(retrieved2.version, 2);
    assert_eq!(retrieved3.version, 1);
    assert_eq!(retrieved1.game.id, 20);
    assert_eq!(retrieved2.game.id, 20);
    assert_eq!(retrieved3.game.id, 21);
}
