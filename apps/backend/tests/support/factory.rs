use backend::entities::games::{self, GameState, GameVisibility};
use backend::entities::users::Model as User;
use backend::error::AppError;
use sea_orm::{ActiveModelTrait, ConnectionTrait, NotSet, Set};
use time::OffsetDateTime;

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

/// Create a test game with a creator user
///
/// # Arguments
/// * `conn` - Database connection or transaction
///
/// # Returns
/// ID of the created game
pub async fn create_test_game(conn: &impl ConnectionTrait) -> Result<i64, AppError> {
    create_test_game_with_options(conn, None, None).await
}

/// Create a test game with custom join code and randomization options
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
    // Create a user first to use as created_by
    let user_id = create_test_user(conn, "creator", Some("Creator")).await?;

    let now = OffsetDateTime::now_utc();
    let final_join_code = if randomize_join_code.unwrap_or(false) {
        format!("C{}", rand::random::<u32>() % 1000000)
    } else {
        join_code.unwrap_or_else(|| "ABC123".to_string())
    };

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
        join_code: Set(Some(final_join_code)),
        rules_version: Set("1.0".to_string()),
        rng_seed: Set(Some(12345)),
        current_round: Set(Some(1)),
        hand_size: Set(Some(13)),
        dealer_pos: Set(Some(0)),
        lock_version: Set(1),
    };

    let inserted = game.insert(conn).await?;
    Ok(inserted.id)
}
