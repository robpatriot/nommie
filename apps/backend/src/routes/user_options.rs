use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::db::txn::with_txn;
use crate::error::AppError;
use crate::extractors::current_user::CurrentUser;
use crate::extractors::ValidatedJson;
use crate::repos::user_options::{self, AppearanceMode, UserOptions};
use crate::state::app_state::AppState;

#[derive(Debug, Serialize)]
pub struct UserOptionsResponse {
    pub appearance_mode: AppearanceMode,
    pub updated_at: String,
}

impl From<UserOptions> for UserOptionsResponse {
    fn from(value: UserOptions) -> Self {
        Self {
            appearance_mode: value.appearance_mode,
            updated_at: value.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserOptionsRequest {
    pub appearance_mode: AppearanceMode,
}

async fn get_user_options(
    req: HttpRequest,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    let user_id = current_user.id;
    let options = with_txn(Some(&req), &app_state, move |txn| {
        Box::pin(async move {
            user_options::ensure_default_for_user(txn, user_id)
                .await
                .map_err(AppError::from)
        })
    })
    .await?;

    Ok(HttpResponse::Ok().json(UserOptionsResponse::from(options)))
}

async fn update_user_options(
    req: HttpRequest,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
    body: ValidatedJson<UpdateUserOptionsRequest>,
) -> Result<HttpResponse, AppError> {
    let user_id = current_user.id;
    let mode = body.appearance_mode;

    let options = with_txn(Some(&req), &app_state, move |txn| {
        Box::pin(async move {
            user_options::set_appearance_mode(txn, user_id, mode)
                .await
                .map_err(AppError::from)
        })
    })
    .await?;

    Ok(HttpResponse::Ok().json(UserOptionsResponse::from(options)))
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/options")
            .route(web::get().to(get_user_options))
            .route(web::put().to(update_user_options)),
    );
}
