use backend::entities::games::{self, GameState, GameVisibility};
use backend::entities::users::Model as User;
use backend::AppError;
use sea_orm::{ActiveModelTrait, ConnectionTrait, NotSet, Set};
use time::OffsetDateTime;

use super::test_utils::{test_seed, test_user_sub};

/// Seed a user with a specific sub value for testing purposes.
///
/// # Arguments
/// * `db` - Database connection
/// * `sub` - External identifier for the user (e.g., "test-sub-123")
/// * `_email` - Optional email for the user (currently unused)
///
/// # Returns
/// The created user model
pub async fn seed_user_with_sub(
    db: &(impl ConnectionTrait + Send),
    sub: &str,
    _email: Option<&str>,
) -> Result<User, sea_orm::DbErr> {
    let now = OffsetDateTime::now_utc();

    let user = backend::entities::users::ActiveModel {
        id: NotSet, // Let database auto-generate
        sub: Set(sub.to_string()),
        username: Set(Some("Test User".to_string())),
        is_ai: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let user = user.insert(db).await?;
    Ok(user)
}

/// Create a test user with custom sub and username
///
/// # Arguments
/// * `conn` - Database connection or transaction
/// * `sub` - External identifier for the user (e.g., "test-sub-123")
/// * `username` - Optional username for the user
///
/// # Returns
/// ID of the created user
pub async fn create_test_user(
    conn: &impl ConnectionTrait,
    sub: &str,
    username: Option<&str>,
) -> Result<i64, AppError> {
    create_test_user_with_randomization(conn, sub, username, false).await
}

/// Create a test user with optional sub randomization
///
/// # Arguments
/// * `conn` - Database connection or transaction
/// * `sub` - Base external identifier (will be randomized if `randomize_sub` is true)
/// * `username` - Optional username for the user
/// * `randomize_sub` - If true, appends random number to sub to avoid conflicts
///
/// # Returns
/// ID of the created user
pub async fn create_test_user_with_randomization(
    conn: &impl ConnectionTrait,
    sub: &str,
    username: Option<&str>,
    randomize_sub: bool,
) -> Result<i64, AppError> {
    let now = OffsetDateTime::now_utc();
    let final_sub = if randomize_sub {
        format!("{}_{}", sub, rand::random::<u32>())
    } else {
        sub.to_string()
    };

    let user = backend::entities::users::ActiveModel {
        id: NotSet,
        sub: Set(final_sub),
        username: Set(username.map(|s| s.to_string())),
        is_ai: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let inserted = user.insert(conn).await?;
    Ok(inserted.id)
}

/// Create a fresh game in Lobby state with test-specific unique seeds and user subs.
///
/// **NOTE: This function bypasses the repository layer and uses ActiveModel directly.**
/// This is intentional for:
/// - Performance: Faster setup when creating many games in tests
/// - Control: Full control over all fields including lock_version, state, etc.
/// - Complex scenarios: Setting up games in non-standard states (e.g., already in BIDDING)
///
/// For simple game creation in integration/service tests, use `repos::games::create_game()`
/// instead to test the full production code path.
///
/// # Arguments
/// * `conn` - Database connection or transaction
/// * `test_name` - Test name for unique seed generation (e.g., "game_hist_empty")
///
/// # Returns
/// ID of the created game
pub async fn create_fresh_lobby_game(
    conn: &impl ConnectionTrait,
    test_name: &str,
) -> Result<i64, AppError> {
    let user_sub = test_user_sub(&format!("{}_creator", test_name));
    let rng_seed = test_seed(test_name);

    let user_id = create_test_user(conn, &user_sub, Some("Creator")).await?;
    let now = OffsetDateTime::now_utc();

    let game = games::ActiveModel {
        id: NotSet,
        created_by: Set(Some(user_id)),
        visibility: Set(GameVisibility::Public),
        state: Set(GameState::Lobby),
        created_at: Set(now),
        updated_at: Set(now),
        started_at: Set(None),
        ended_at: Set(None),
        name: Set(Some("Test Game".to_string())),
        join_code: Set(None),
        rules_version: Set("1.0".to_string()),
        rng_seed: Set(Some(rng_seed)),
        current_round: Set(None),
        starting_dealer_pos: Set(None),
        current_trick_no: Set(0i16),
        current_round_id: Set(None),
        lock_version: Set(1), // New games start at lock_version 1
    };

    let inserted = game.insert(conn).await?;
    Ok(inserted.id)
}

/// Create a test game with a creator user.
///
/// This creates a game that's already configured (has round 1, dealer position set).
/// For a truly fresh lobby game, use `create_fresh_lobby_game()` instead.
///
/// # Arguments
/// * `conn` - Database connection or transaction
///
/// # Returns
/// ID of the created game
pub async fn create_test_game(conn: &impl ConnectionTrait) -> Result<i64, AppError> {
    create_test_game_with_options(conn, None, None).await
}

/// Create a test game with custom join code and randomization options.
///
/// This builds on `create_fresh_lobby_game()` and then configures the game
/// with round 1 and dealer position already set.
///
/// # Arguments
/// * `conn` - Database connection or transaction
/// * `join_code` - Optional join code (if None, uses "ABC123")
/// * `randomize_join_code` - If true, generates random join code
///
/// # Returns
/// ID of the created game
pub async fn create_test_game_with_options(
    conn: &impl ConnectionTrait,
    join_code: Option<String>,
    randomize_join_code: Option<bool>,
) -> Result<i64, AppError> {
    // Start with a fresh lobby game
    let game_id = create_fresh_lobby_game(conn, "test_game_with_opts").await?;

    // Update it with test-specific configuration
    use backend::entities::games::Entity as GamesEntity;
    use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};

    let game = GamesEntity::find_by_id(game_id)
        .one(conn)
        .await?
        .ok_or_else(|| {
            AppError::internal(
                backend::ErrorCode::InternalError,
                "Failed to find just-created game".to_string(),
                std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Game not found after creation",
                ),
            )
        })?;

    let final_join_code = if randomize_join_code.unwrap_or(false) {
        format!("C{}", rand::random::<u32>() % 1000000)
    } else {
        join_code.unwrap_or_else(|| "ABC123".to_string())
    };

    let mut active_game = game.into_active_model();
    active_game.join_code = Set(Some(final_join_code));
    active_game.current_round = Set(Some(1));
    active_game.starting_dealer_pos = Set(Some(0));
    active_game.lock_version = Set(1);

    active_game.update(conn).await?;

    Ok(game_id)
}
