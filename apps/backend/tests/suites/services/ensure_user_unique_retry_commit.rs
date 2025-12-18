use std::sync::Arc;

use backend::db::txn::with_txn;
use backend::db::txn_policy::{current, TxnPolicy};
use backend::entities::{user_credentials, users};
use backend::services::users::UserService;
use backend::AppError;
use backend_test_support::unique_helpers::{unique_email, unique_str};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use tokio::sync::Barrier;
use tokio::task::LocalSet;

use crate::support::build_test_state;

#[tokio::test]
async fn ensure_user_concurrent_calls_same_email_succeed_and_no_orphans() -> Result<(), AppError> {
    assert_eq!(current(), TxnPolicy::CommitOnOk);

    let state = Arc::new(build_test_state().await?);
    let email = unique_email("ensure-user-race");
    let google_sub = unique_str("google-sub");

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
                with_txn(None, s1.as_ref(), |txn| {
                    let email1 = email1.clone();
                    let sub1 = sub1.clone();
                    Box::pin(async move {
                        let service = UserService;
                        let user = service
                            .ensure_user(txn, &email1, Some("Race"), &sub1, None)
                            .await
                            .map_err(AppError::from)?;
                        Ok::<_, AppError>(user.id)
                    })
                })
                .await
            });

            let t2 = tokio::task::spawn_local(async move {
                b2.wait().await;
                with_txn(None, s2.as_ref(), |txn| {
                    let email2 = email2.clone();
                    let sub2 = sub2.clone();
                    Box::pin(async move {
                        let service = UserService;
                        let user = service
                            .ensure_user(txn, &email2, Some("Race"), &sub2, None)
                            .await
                            .map_err(AppError::from)?;
                        Ok::<_, AppError>(user.id)
                    })
                })
                .await
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

    let credential_count = user_credentials::Entity::find()
        .filter(user_credentials::Column::Email.eq(&normalized_email))
        .count(conn)
        .await?;
    assert_eq!(
        credential_count, 1,
        "should have exactly one credential row"
    );

    let user_count = users::Entity::find()
        .filter(users::Column::Sub.eq(&google_sub))
        .count(conn)
        .await?;
    assert_eq!(user_count, 1, "should not commit an orphan user row");

    with_txn(None, state.as_ref(), |txn| {
        let normalized_email = normalized_email.clone();
        Box::pin(async move {
            let creds = user_credentials::Entity::find()
                .filter(user_credentials::Column::Email.eq(&normalized_email))
                .one(txn)
                .await?
                .expect("credential should exist");
            users::Entity::delete_by_id(creds.user_id).exec(txn).await?;
            Ok::<_, AppError>(())
        })
    })
    .await?;

    Ok(())
}
