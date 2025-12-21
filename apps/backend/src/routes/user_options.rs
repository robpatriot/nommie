use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::db::txn::with_txn;
use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::extractors::current_user::CurrentUser;
use crate::extractors::ValidatedJson;
use crate::repos::user_options::{
    self, AppearanceMode, UpdateUserOptions, UserLocale, UserOptions,
};
use crate::state::app_state::AppState;

#[derive(Debug, Serialize)]
pub struct UserOptionsResponse {
    pub appearance_mode: AppearanceMode,
    pub require_card_confirmation: bool,
    pub locale: Option<UserLocale>,
    pub updated_at: String,
}

impl From<UserOptions> for UserOptionsResponse {
    fn from(value: UserOptions) -> Self {
        Self {
            appearance_mode: value.appearance_mode,
            require_card_confirmation: value.require_card_confirmation,
            locale: value.locale,
            updated_at: value.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserOptionsRequest {
    #[serde(default)]
    pub appearance_mode: Option<AppearanceMode>,
    #[serde(default)]
    pub require_card_confirmation: Option<bool>,
    #[serde(default)]
    pub locale: Option<UserLocale>,
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
    let payload = body.into_inner();

    if let Some(locale) = payload.locale {
        tracing::info!(
            user_id = user_id,
            locale = locale.as_str(),
            "user_options.locale_updated"
        );
    }

    if payload.appearance_mode.is_none()
        && payload.require_card_confirmation.is_none()
        && payload.locale.is_none()
    {
        return Err(AppError::Validation {
            code: ErrorCode::ValidationError,
            detail: "At least one option must be provided".to_string(),
            status: StatusCode::BAD_REQUEST,
        });
    }

    let update_request = UpdateUserOptions {
        appearance_mode: payload.appearance_mode,
        require_card_confirmation: payload.require_card_confirmation,
        locale: payload.locale,
    };

    let options = with_txn(Some(&req), &app_state, move |txn| {
        Box::pin(async move {
            user_options::update_options(txn, user_id, update_request)
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
