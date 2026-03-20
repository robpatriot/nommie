use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

const SESSION_TTL: u64 = 86400;
const WS_TOKEN_TTL: u64 = 90;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub user_id: i64,
    pub sub: String,
    pub email: String,
}

pub fn generate_session_token() -> String {
    Uuid::new_v4().simple().to_string()
}

pub async fn store_session(
    conn: &mut ConnectionManager,
    token: &str,
    data: &SessionData,
) -> Result<(), AppError> {
    let key = format!("session:{token}");
    let value = serde_json::to_string(data).map_err(|e| {
        AppError::internal(
            crate::errors::ErrorCode::InternalError,
            "session serialization failed",
            e,
        )
    })?;
    conn.set_ex::<_, _, ()>(&key, value, SESSION_TTL)
        .await
        .map_err(|e| {
            AppError::redis_unavailable(
                format!("failed to store session: {e}"),
                crate::error::Sentinel("redis set_ex failed"),
                None,
            )
        })
}

pub async fn get_and_slide_session(
    conn: &mut ConnectionManager,
    token: &str,
) -> Result<Option<SessionData>, AppError> {
    let key = format!("session:{token}");
    let value: Option<String> = conn.get(&key).await.map_err(|e| {
        AppError::redis_unavailable(
            format!("failed to get session: {e}"),
            crate::error::Sentinel("redis get failed"),
            None,
        )
    })?;
    if let Some(ref v) = value {
        conn.expire::<_, ()>(&key, SESSION_TTL as i64)
            .await
            .map_err(|e| {
                AppError::redis_unavailable(
                    format!("failed to slide session TTL: {e}"),
                    crate::error::Sentinel("redis expire failed"),
                    None,
                )
            })?;
        let data: SessionData = serde_json::from_str(v).map_err(|e| {
            AppError::internal(
                crate::errors::ErrorCode::InternalError,
                "session deserialization failed",
                e,
            )
        })?;
        Ok(Some(data))
    } else {
        Ok(None)
    }
}

pub async fn delete_session(conn: &mut ConnectionManager, token: &str) {
    let key = format!("session:{token}");
    let _: Result<(), _> = conn.del(&key).await;
}

pub async fn store_ws_token(
    conn: &mut ConnectionManager,
    token: &str,
    data: &SessionData,
) -> Result<(), AppError> {
    let key = format!("ws_token:{token}");
    let value = serde_json::to_string(data).map_err(|e| {
        AppError::internal(
            crate::errors::ErrorCode::InternalError,
            "ws_token serialization failed",
            e,
        )
    })?;
    conn.set_ex::<_, _, ()>(&key, value, WS_TOKEN_TTL)
        .await
        .map_err(|e| {
            AppError::redis_unavailable(
                format!("failed to store ws_token: {e}"),
                crate::error::Sentinel("redis set_ex failed"),
                None,
            )
        })
}

pub async fn get_ws_token(
    conn: &mut ConnectionManager,
    token: &str,
) -> Result<Option<SessionData>, AppError> {
    let key = format!("ws_token:{token}");
    let value: Option<String> = conn.get(&key).await.map_err(|e| {
        AppError::redis_unavailable(
            format!("failed to get ws_token: {e}"),
            crate::error::Sentinel("redis get failed"),
            None,
        )
    })?;
    match value {
        Some(v) => {
            let data: SessionData = serde_json::from_str(&v).map_err(|e| {
                AppError::internal(
                    crate::errors::ErrorCode::InternalError,
                    "ws_token deserialization failed",
                    e,
                )
            })?;
            Ok(Some(data))
        }
        None => Ok(None),
    }
}
