//! Admin API routes.

use actix_web::{web, HttpRequest, HttpResponse};

use crate::db::require_db;
use crate::db::txn::with_txn;
use crate::error::AppError;
use crate::extractors::admin_principal::AdminPrincipal;
use crate::extractors::ValidatedJson;
use crate::repos::admin_users;
use crate::services::admin::{AdminService, RoleMutationRequest};
use crate::state::app_state::AppState;

#[derive(Debug, serde::Deserialize)]
pub struct AdminUserSearchQuery {
    pub q: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct AdminUserSummary {
    pub id: i64,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: crate::entities::users::UserRole,
}

#[derive(Debug, serde::Serialize)]
pub struct AdminUserSearchResponse {
    pub items: Vec<AdminUserSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct RoleMutationResponse {
    pub user: AdminUserSummary,
    pub changed: bool,
}

async fn search_users(
    admin: AdminPrincipal,
    query: web::Query<AdminUserSearchQuery>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let search_query =
        admin_users::validate_search_query(query.q.clone(), query.limit, query.cursor.clone())?;
    let db = require_db(state.as_ref())?;
    let result = AdminService.search_users(&db, &admin, search_query).await?;

    let items: Vec<AdminUserSummary> = result
        .items
        .into_iter()
        .map(|i| AdminUserSummary {
            id: i.id,
            display_name: i.display_name,
            email: i.email,
            role: i.role,
        })
        .collect();

    Ok(HttpResponse::Ok().json(AdminUserSearchResponse {
        items,
        next_cursor: result.next_cursor,
    }))
}

async fn grant_admin(
    admin: AdminPrincipal,
    path: web::Path<i64>,
    body: ValidatedJson<RoleMutationRequest>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    with_txn(Some(&req), state.as_ref(), |txn| {
        Box::pin(async move {
            let response = AdminService
                .grant_admin(txn, &admin, user_id, &body)
                .await?;
            Ok(HttpResponse::Ok().json(RoleMutationResponse {
                user: AdminUserSummary {
                    id: response.user.id,
                    display_name: response.user.display_name,
                    email: response.user.email,
                    role: response.user.role,
                },
                changed: response.changed,
            }))
        })
    })
    .await
}

async fn revoke_admin(
    admin: AdminPrincipal,
    path: web::Path<i64>,
    body: ValidatedJson<RoleMutationRequest>,
    req: HttpRequest,
    state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let user_id = path.into_inner();
    with_txn(Some(&req), state.as_ref(), |txn| {
        Box::pin(async move {
            let response = AdminService
                .revoke_admin(txn, &admin, user_id, &body)
                .await?;
            Ok(HttpResponse::Ok().json(RoleMutationResponse {
                user: AdminUserSummary {
                    id: response.user.id,
                    display_name: response.user.display_name,
                    email: response.user.email,
                    role: response.user.role,
                },
                changed: response.changed,
            }))
        })
    })
    .await
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/users/search").route(web::get().to(search_users)))
        .service(web::resource("/users/{user_id}/grant-admin").route(web::post().to(grant_admin)))
        .service(
            web::resource("/users/{user_id}/revoke-admin").route(web::post().to(revoke_admin)),
        );
}
