// Session helpers for tests

use backend::auth::session::{generate_session_token, store_session, store_ws_token, SessionData};
use backend::entities::allowed_emails;
use backend::state::app_state::AppState;
use backend::AppError;
use sea_orm::sea_query::OnConflict;
use sea_orm::{ActiveValue, EntityTrait};

/// Create a test session in Redis and return the session token.
///
/// Requires REDIS_URL to be set in the environment, which will be picked up
/// by `test_state_builder` automatically.
pub async fn create_test_session(
    state: &AppState,
    user_id: i64,
    sub: &str,
    email: &str,
) -> Result<String, AppError> {
    let mut conn = state.session_redis().ok_or_else(|| {
        AppError::redis_unavailable(
            "session Redis not configured for test".to_string(),
            backend::error::Sentinel("session Redis not available"),
            None,
        )
    })?;
    let token = generate_session_token();
    let data = SessionData {
        user_id,
        sub: sub.to_string(),
        email: email.to_string(),
    };
    store_session(&mut conn, &token, &data).await?;
    Ok(token)
}

/// Create a test WebSocket token in Redis and return the token.
pub async fn create_test_ws_token(
    state: &AppState,
    user_id: i64,
    sub: &str,
    email: &str,
) -> Result<String, AppError> {
    let mut conn = state.session_redis().ok_or_else(|| {
        AppError::redis_unavailable(
            "session Redis not configured for test".to_string(),
            backend::error::Sentinel("session Redis not available"),
            None,
        )
    })?;
    let token = generate_session_token();
    let data = SessionData {
        user_id,
        sub: sub.to_string(),
        email: email.to_string(),
    };
    store_ws_token(&mut conn, &token, &data).await?;
    Ok(token)
}

/// Insert an email into the admission table for testing. Idempotent (ON CONFLICT DO NOTHING).
pub async fn seed_admission_email(conn: &sea_orm::DatabaseConnection, email: &str, is_admin: bool) {
    let now = time::OffsetDateTime::now_utc();
    let model = allowed_emails::ActiveModel {
        id: ActiveValue::NotSet,
        email: ActiveValue::Set(email.to_string()),
        is_admin: ActiveValue::Set(is_admin),
        created_at: ActiveValue::Set(now),
    };
    let _ = allowed_emails::Entity::insert(model)
        .on_conflict(
            OnConflict::columns([allowed_emails::Column::Email])
                .do_nothing()
                .to_owned(),
        )
        .exec(conn)
        .await;
}
