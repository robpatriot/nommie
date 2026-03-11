//! Admin user search repository.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use sea_orm::ConnectionTrait;

use crate::adapters::admin_users_sea;
use crate::errors::domain::{DomainError, InfraErrorKind, ValidationKind};

const DEFAULT_LIMIT: u32 = 20;
const MAX_LIMIT: u32 = 50;

/// Query parameters for admin user search.
#[derive(Debug, Clone)]
pub struct AdminUserSearchQuery {
    pub q: String,
    pub limit: u32,
    pub cursor: Option<String>,
}

/// A single user in search results.
#[derive(Debug, Clone)]
pub struct AdminUserSearchItem {
    pub id: i64,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: crate::entities::users::UserRole,
    #[allow(dead_code)] // used for cursor encoding, not in API response
    pub created_at: time::OffsetDateTime,
}

/// Paginated search result.
#[derive(Debug, Clone)]
pub struct AdminUserSearchResult {
    pub items: Vec<AdminUserSearchItem>,
    pub next_cursor: Option<String>,
}

/// Validate and normalize search query. Returns error if q is empty after trim.
pub fn validate_search_query(
    q: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Result<AdminUserSearchQuery, DomainError> {
    let q = q.unwrap_or_default().trim().to_string();
    if q.is_empty() {
        return Err(DomainError::validation(
            ValidationKind::InvalidSearchQuery,
            "Search query q is required and must be non-empty after trim",
        ));
    }
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    if limit == 0 || limit > MAX_LIMIT {
        return Err(DomainError::validation(
            ValidationKind::InvalidSearchQuery,
            format!("Limit must be between 1 and {}", MAX_LIMIT),
        ));
    }
    Ok(AdminUserSearchQuery { q, limit, cursor })
}

/// Decode cursor from opaque string. Returns error if invalid.
pub fn decode_cursor(cursor: &str) -> Result<admin_users_sea::AdminUserCursor, DomainError> {
    let bytes = BASE64
        .decode(cursor)
        .map_err(|_| DomainError::validation(ValidationKind::InvalidCursor, "Invalid cursor"))?;
    let s = String::from_utf8(bytes)
        .map_err(|_| DomainError::validation(ValidationKind::InvalidCursor, "Invalid cursor"))?;
    serde_json::from_str(&s)
        .map_err(|_| DomainError::validation(ValidationKind::InvalidCursor, "Invalid cursor"))
}

/// Encode cursor to opaque string.
pub fn encode_cursor(created_at: &time::OffsetDateTime, id: i64) -> Result<String, DomainError> {
    let c = admin_users_sea::AdminUserCursor {
        created_at: *created_at,
        id,
    };
    let json = serde_json::to_string(&c)
        .map_err(|e| DomainError::validation(ValidationKind::InvalidCursor, e.to_string()))?;
    Ok(BASE64.encode(json.as_bytes()))
}

pub async fn search_users_for_admin<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    query: AdminUserSearchQuery,
) -> Result<AdminUserSearchResult, DomainError> {
    let cursor = match &query.cursor {
        Some(c) => Some(decode_cursor(c)?),
        None => None,
    };

    let (rows, has_next) =
        admin_users_sea::search_users_for_admin(conn, &query.q, query.limit, cursor.as_ref())
            .await?;

    let items: Vec<AdminUserSearchItem> = rows
        .iter()
        .map(|r| AdminUserSearchItem {
            id: r.id,
            display_name: r.display_name.clone(),
            email: r.email.clone(),
            role: r.role.clone(),
            created_at: r.created_at,
        })
        .collect();

    let next_cursor = if has_next {
        let last = rows.last().ok_or_else(|| {
            DomainError::Infra(
                InfraErrorKind::DataCorruption,
                "cursor pagination invariant violated".into(),
            )
        })?;
        Some(encode_cursor(&last.created_at, last.id)?)
    } else {
        None
    };

    Ok(AdminUserSearchResult { items, next_cursor })
}
