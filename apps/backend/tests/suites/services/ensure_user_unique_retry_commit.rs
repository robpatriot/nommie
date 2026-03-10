use std::sync::Arc;

use backend::auth::google::VerifiedGoogleClaims;
use backend::db::require_db;
use backend::db::txn::with_txn;
use backend::db::txn_policy::{current, TxnPolicy};
use backend::entities::{allowed_emails, user_auth_identities, users};
use backend::services::users::UserService;
use backend::AppError;
use backend_test_support::unique_helpers::{unique_email, unique_str};
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use tokio::sync::Barrier;
use tokio::task::LocalSet;

use crate::support::build_test_state;

const PROVIDER_GOOGLE: &str = "google";

#[tokio::test]
async fn ensure_user_concurrent_calls_same_email_succeed_and_no_orphans() -> Result<(), AppError> {
    assert_eq!(current(), TxnPolicy::CommitOnOk);

    let state = Arc::new(build_test_state().await?);
    let email = unique_email("ensure-user-race");
    let google_sub = unique_str("google-sub");

    let db = require_db(state.as_ref())?;
    let now = time::OffsetDateTime::now_utc();
    let model = allowed_emails::ActiveModel {
        id: ActiveValue::NotSet,
        email: ActiveValue::Set(email.to_lowercase()),
        is_admin: ActiveValue::Set(false),
        created_at: ActiveValue::Set(now),
    };
    allowed_emails::Entity::insert(model)
        .on_conflict(
            sea_orm::sea_query::OnConflict::columns([allowed_emails::Column::Email])
                .do_nothing()
                .to_owned(),
        )
        .exec(&db)
        .await
        .map_err(|e| AppError::from(backend::infra::db_errors::map_db_err(e)))?;

    let barrier = Arc::new(Barrier::new(2));
    let local = LocalSet::new();

    let (a, b) = local
        .run_until(async {
            let b1 = barrier.clone();
            let b2 = barrier.clone();
            let s1 = state.clone();
            let s2 = state.clone();
            let email1 = email.clone();
            let email2 = email.clone();
            let sub1 = google_sub.clone();
            let sub2 = google_sub.clone();

            let t1 = tokio::task::spawn_local(async move {
                b1.wait().await;
                let admission_mode = s1.config.admission_mode;
                with_txn(None, s1.as_ref(), |txn| {
                    let email1 = email1.clone();
                    let sub1 = sub1.clone();
                    Box::pin(async move {
                        let service = UserService;
                        let claims = VerifiedGoogleClaims {
                            sub: sub1,
                            email: email1,
                            name: Some("Race".to_string()),
                        };
                        let user = service
                            .ensure_user(txn, &claims, admission_mode)
                            .await
                            .map_err(AppError::from)?;
                        Ok::<_, AppError>(user.id)
                    })
                })
                .await
            });

            let t2 = tokio::task::spawn_local(async move {
                b2.wait().await;
                let admission_mode = s2.config.admission_mode;
                let first = with_txn(None, s2.as_ref(), |txn| {
                    let email2 = email2.clone();
                    let sub2 = sub2.clone();
                    Box::pin(async move {
                        let service = UserService;
                        let claims = VerifiedGoogleClaims {
                            sub: sub2,
                            email: email2,
                            name: Some("Race".to_string()),
                        };
                        let user = service
                            .ensure_user(txn, &claims, admission_mode)
                            .await
                            .map_err(AppError::from)?;
                        Ok::<_, AppError>(user.id)
                    })
                })
                .await;
                match first {
                    Ok(id) => Ok(id),
                    Err(e) if e.code() == backend::ErrorCode::UniqueEmail => {
                        // Retry with fresh transaction (winner will have committed)
                        with_txn(None, s2.as_ref(), |txn| {
                            Box::pin(async move {
                                let service = UserService;
                                let claims = VerifiedGoogleClaims {
                                    sub: sub2,
                                    email: email2,
                                    name: Some("Race".to_string()),
                                };
                                let user = service
                                    .ensure_user(txn, &claims, admission_mode)
                                    .await
                                    .map_err(AppError::from)?;
                                Ok::<_, AppError>(user.id)
                            })
                        })
                        .await
                    }
                    Err(e) => Err(e),
                }
            });

            let a = t1.await.map_err(|e| {
                AppError::internal(
                    backend::ErrorCode::InternalError,
                    "task join failed",
                    std::io::Error::other(e.to_string()),
                )
            })??;

            let b = t2.await.map_err(|e| {
                AppError::internal(
                    backend::ErrorCode::InternalError,
                    "task join failed",
                    std::io::Error::other(e.to_string()),
                )
            })??;

            Ok::<_, AppError>((a, b))
        })
        .await?;

    assert_eq!(
        a, b,
        "both concurrent calls should resolve to the same user"
    );

    let normalized_email = email.to_lowercase();
    let conn = state.db().expect("db must exist");

    let identity_count = user_auth_identities::Entity::find()
        .filter(user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
        .filter(user_auth_identities::Column::Email.eq(&normalized_email))
        .count(&conn)
        .await?;
    assert_eq!(identity_count, 1, "should have exactly one identity row");

    let identity = user_auth_identities::Entity::find()
        .filter(user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
        .filter(user_auth_identities::Column::ProviderUserId.eq(&google_sub))
        .one(&conn)
        .await?
        .expect("identity should exist");
    let user_count = users::Entity::find()
        .filter(users::Column::Id.eq(identity.user_id))
        .count(&conn)
        .await?;
    assert_eq!(user_count, 1, "should not commit an orphan user row");

    with_txn(None, state.as_ref(), |txn| {
        let normalized_email = normalized_email.clone();
        Box::pin(async move {
            let identity = user_auth_identities::Entity::find()
                .filter(user_auth_identities::Column::Provider.eq(PROVIDER_GOOGLE))
                .filter(user_auth_identities::Column::Email.eq(&normalized_email))
                .one(txn)
                .await?
                .expect("identity should exist");
            users::Entity::delete_by_id(identity.user_id)
                .exec(txn)
                .await?;
            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
