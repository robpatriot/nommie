use actix_web::{web, HttpResponse};

use crate::entities::users::UserRole;
use crate::error::AppError;
use crate::extractors::current_user::CurrentUser;

#[derive(Debug, serde::Serialize)]
pub struct MeResponse {
    pub id: i64,
    pub username: Option<String>,
    pub role: UserRole,
}

async fn get_me(current_user: CurrentUser) -> Result<HttpResponse, AppError> {
    Ok(HttpResponse::Ok().json(MeResponse {
        id: current_user.id,
        username: current_user.username,
        role: current_user.role,
    }))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::resource("/me").route(web::get().to(get_me)));
}
