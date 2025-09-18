use backend::config::db::DbProfile;
use backend::entities::games::{self, GameState, GameVisibility};
use backend::infra::state::build_state;
use sea_orm::{EntityTrait, Set};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn insert_defaults_and_fetch() {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");
    let db = state.db().expect("database should be available");

    // Insert a games row with minimal fields
    let now = time::OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        rules_version: Set("nommie-1.0.0".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let inserted_game = games::Entity::insert(game)
        .exec(db)
        .await
        .expect("should insert game successfully");

    // Assert id > 0
    assert!(inserted_game.last_insert_id > 0);

    // Fetch by id and assert it exists
    let fetched_game = games::Entity::find_by_id(inserted_game.last_insert_id)
        .one(db)
        .await
        .expect("should query successfully")
        .expect("should have game row");

    // Assert state round-trips correctly
    assert_eq!(fetched_game.state, GameState::Lobby);
    assert_eq!(fetched_game.visibility, GameVisibility::Public);
    assert_eq!(fetched_game.rules_version, "nommie-1.0.0");
    assert_eq!(fetched_game.lock_version, 0);
}

#[tokio::test]
#[serial]
async fn join_code_unique() {
    let state = build_state()
        .with_db(DbProfile::Test)
        .build()
        .await
        .expect("build test state with DB");
    let db = state.db().expect("database should be available");

    // Insert first game with join_code
    let now = time::OffsetDateTime::now_utc();
    let game1 = games::ActiveModel {
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        rules_version: Set("nommie-1.0.0".to_string()),
        join_code: Set(Some("ABC123".to_string())),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let inserted_game1 = games::Entity::insert(game1)
        .exec(db)
        .await
        .expect("should insert first game successfully");

    // Try to insert second game with same join_code
    let now2 = time::OffsetDateTime::now_utc();
    let game2 = games::ActiveModel {
        visibility: Set(GameVisibility::Private),
        state: Set(GameState::Lobby),
        rules_version: Set("nommie-1.0.0".to_string()),
        join_code: Set(Some("ABC123".to_string())), // Same join_code
        created_at: Set(now2),
        updated_at: Set(now2),
        ..Default::default()
    };

    let result = games::Entity::insert(game2).exec(db).await;

    // Assert the second insert errors (unique violation)
    assert!(
        result.is_err(),
        "Second insert with same join_code should fail with unique violation"
    );

    // Verify the first game still exists
    let fetched_game = games::Entity::find_by_id(inserted_game1.last_insert_id)
        .one(db)
        .await
        .expect("should query successfully")
        .expect("should have first game row");

    assert_eq!(fetched_game.join_code, Some("ABC123".to_string()));
}
