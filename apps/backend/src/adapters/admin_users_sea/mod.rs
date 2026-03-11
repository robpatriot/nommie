//! SeaORM adapter for admin user search.
//!
//! Data sources (verified against schema/codebase):
//! - Email: user_auth_identities.email for provider "google" (only OAuth provider in use)
//! - Display name: users.username, fallback to identity email (matches CurrentUser pattern)
//! - is_ai filter: exclude AI users (system-managed, not role-manageable)

use sea_orm::{ConnectionTrait, Statement};

use crate::entities::users::UserRole;

/// Result row from admin user search.
#[derive(Debug, Clone)]
pub struct AdminUserSearchRow {
    pub id: i64,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: UserRole,
    pub created_at: time::OffsetDateTime,
}

/// Cursor for keyset pagination. Encodes (created_at, id) for "next page" positioning.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdminUserCursor {
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: time::OffsetDateTime,
    pub id: i64,
}

/// OAuth provider used for identity/email. Verified: only provider in auth flow (routes/auth, services/users).
const PROVIDER_GOOGLE: &str = "google";

/// Search users for admin role management.
/// Joins users with user_auth_identities (provider=google) for email.
/// Filters is_ai=false. Filters by q (ILIKE on email, username).
/// Order: created_at DESC, id DESC.
/// Returns limit+1 rows; if extra row, has_next=true and last row is excluded from items.
pub async fn search_users_for_admin<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    q: &str,
    limit: u32,
    cursor: Option<&AdminUserCursor>,
) -> Result<(Vec<AdminUserSearchRow>, bool), sea_orm::DbErr> {
    let search_pattern = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
    let limit_plus_one = (limit as i64) + 1;
    let db = conn.get_database_backend();

    let (sql, values): (String, Vec<sea_orm::Value>) = match cursor {
        Some(c) => (
            format!(
                r#"
                SELECT u.id, u.username, i.email, u.role::text, u.created_at
                FROM users u
                LEFT JOIN user_auth_identities i ON u.id = i.user_id AND i.provider = '{}'
                WHERE u.is_ai = false
                AND (i.email ILIKE $1 OR u.username ILIKE $1)
                AND (
                    u.created_at < $2::timestamptz
                    OR (u.created_at = $2::timestamptz AND u.id < $3::bigint)
                )
                ORDER BY u.created_at DESC, u.id DESC
                LIMIT {}
                "#,
                PROVIDER_GOOGLE, limit_plus_one
            ),
            vec![
                search_pattern.clone().into(),
                sea_orm::Value::from(c.created_at),
                c.id.into(),
            ],
        ),
        None => (
            format!(
                r#"
                SELECT u.id, u.username, i.email, u.role::text, u.created_at
                FROM users u
                LEFT JOIN user_auth_identities i ON u.id = i.user_id AND i.provider = '{}'
                WHERE u.is_ai = false
                AND (i.email ILIKE $1 OR u.username ILIKE $1)
                ORDER BY u.created_at DESC, u.id DESC
                LIMIT {}
                "#,
                PROVIDER_GOOGLE, limit_plus_one
            ),
            vec![search_pattern.into()],
        ),
    };

    let stmt = Statement::from_sql_and_values(db, sql, values);
    let rows = conn.query_all(stmt).await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let id: i64 = row.try_get("", "id")?;
        let username: Option<String> = row.try_get("", "username")?;
        let email: Option<String> = row.try_get("", "email")?;
        let role_str: String = row.try_get("", "role")?;
        let role = match role_str.as_str() {
            "admin" => UserRole::Admin,
            _ => UserRole::User,
        };
        // Display name: username preferred, fallback to email (matches CurrentUser / identity resolution)
        let display_name = username.or(email.clone());
        let created_at: time::OffsetDateTime = row.try_get("", "created_at")?;
        items.push(AdminUserSearchRow {
            id,
            display_name,
            email,
            role,
            created_at,
        });
    }

    let has_next = items.len() > limit as usize;
    if has_next {
        items.truncate(limit as usize);
    }

    Ok((items, has_next))
}
