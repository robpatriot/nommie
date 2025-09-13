use backend::entities::users::Model as User;
use sea_orm::{ActiveModelTrait, DatabaseConnection, NotSet, Set};
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
    db: &DatabaseConnection,
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
