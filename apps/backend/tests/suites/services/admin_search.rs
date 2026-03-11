//! Admin user search tests.

use backend::db::require_db;
use backend::entities::users::{self as users_entity};
use backend::errors::domain::{DomainError, ValidationKind};
use backend::repos::admin_users;
use backend_test_support::unique_helpers::{unique_email, unique_str};
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use time::OffsetDateTime;

use crate::support::build_test_state;
use crate::support::factory::seed_user_with_sub;

const PROVIDER_GOOGLE: &str = "google";

/// Seed a user with explicit created_at for deterministic ordering tests.
async fn seed_user_with_created_at(
    conn: &impl ConnectionTrait,
    sub: &str,
    _email: &str,
    created_at: OffsetDateTime,
) -> Result<users_entity::Model, sea_orm::DbErr> {
    let user = users_entity::ActiveModel {
        id: sea_orm::NotSet,
        username: Set(Some(sub.to_string())),
        is_ai: Set(false),
        role: Set(users_entity::UserRole::User),
        created_at: Set(created_at),
        updated_at: Set(created_at),
    };
    let user = user.insert(conn).await?;
    Ok(user)
}

#[tokio::test]
async fn search_requires_non_empty_q() -> Result<(), Box<dyn std::error::Error>> {
    let err = admin_users::validate_search_query(None, Some(20), None);
    assert!(err.is_err());
    let domain_err = err.unwrap_err();
    match &domain_err {
        DomainError::Validation(ValidationKind::InvalidSearchQuery, _) => {}
        _ => panic!("expected InvalidSearchQuery, got {:?}", domain_err),
    }

    let err = admin_users::validate_search_query(Some("  ".to_string()), Some(20), None);
    assert!(err.is_err());

    Ok(())
}

#[tokio::test]
async fn search_with_q_filters_by_username() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    let unique_part = unique_str("search-filter");
    let username = format!("alice_{}", unique_part);
    let email = unique_email("search-filter");

    backend::db::txn::with_txn(None, &state, |txn| {
        Box::pin(async move {
            let user = seed_user_with_sub(txn, &username, Some(&email)).await?;
            backend::repos::auth_identities::create_identity(
                txn,
                user.id,
                PROVIDER_GOOGLE,
                &unique_str("google-sub"),
                &email.to_lowercase(),
            )
            .await?;

            let query =
                admin_users::validate_search_query(Some(unique_part.clone()), Some(20), None)
                    .map_err(backend::AppError::from)?;
            let result = admin_users::search_users_for_admin(txn, query)
                .await
                .map_err(backend::AppError::from)?;

            assert!(!result.items.is_empty());
            assert!(result.items.iter().any(|i| i.id == user.id));

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn search_limit_validation() -> Result<(), Box<dyn std::error::Error>> {
    let err = admin_users::validate_search_query(Some("x".into()), Some(0), None);
    assert!(err.is_err());

    let err = admin_users::validate_search_query(Some("x".into()), Some(51), None);
    assert!(err.is_err());

    let ok = admin_users::validate_search_query(Some("x".into()), Some(1), None);
    assert!(ok.is_ok());
    assert_eq!(ok.unwrap().limit, 1);

    let ok = admin_users::validate_search_query(Some("x".into()), Some(50), None);
    assert!(ok.is_ok());
    assert_eq!(ok.unwrap().limit, 50);

    let ok = admin_users::validate_search_query(Some("x".into()), None, None);
    assert!(ok.is_ok());
    assert_eq!(ok.unwrap().limit, 20);

    Ok(())
}

#[tokio::test]
async fn search_invalid_cursor_returns_error() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required");

    let query = admin_users::AdminUserSearchQuery {
        q: "test".to_string(),
        limit: 20,
        cursor: Some("not-valid-base64!!!".to_string()),
    };

    let err = admin_users::search_users_for_admin(&db, query).await;
    assert!(err.is_err());
    let domain_err = err.unwrap_err();
    match &domain_err {
        DomainError::Validation(ValidationKind::InvalidCursor, _) => {}
        _ => panic!("expected InvalidCursor, got {:?}", domain_err),
    }

    Ok(())
}

#[tokio::test]
async fn search_invalid_cursor_malformed_timestamp_returns_error(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;
    let db = require_db(&state).expect("DB required");

    // Valid base64 but JSON with invalid RFC3339 timestamp (typed cursor requires proper format)
    let bad_cursor = "eyJjcmVhdGVkX2F0Ijoibm90LWEtZGF0ZSIsImlkIjoxfQ=="; // {"created_at":"not-a-date","id":1}

    let query = admin_users::AdminUserSearchQuery {
        q: "test".to_string(),
        limit: 20,
        cursor: Some(bad_cursor.to_string()),
    };

    let err = admin_users::search_users_for_admin(&db, query).await;
    assert!(err.is_err());
    let domain_err = err.unwrap_err();
    match &domain_err {
        DomainError::Validation(ValidationKind::InvalidCursor, _) => {}
        _ => panic!("expected InvalidCursor, got {:?}", domain_err),
    }

    Ok(())
}

#[tokio::test]
async fn search_ordering_deterministic_created_at_desc_id_desc(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    let base = unique_str("order");
    let search_term = format!("user_{}", base);
    let now = OffsetDateTime::now_utc();

    backend::db::txn::with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create 3 users with explicit created_at: oldest first (so they get lower ids)
            let t1 = now - time::Duration::seconds(3);
            let t2 = now - time::Duration::seconds(2);
            let t3 = now - time::Duration::seconds(1);

            let u1 = seed_user_with_created_at(
                txn,
                &format!("{}_1", search_term),
                &unique_email("order1"),
                t1,
            )
            .await?;
            backend::repos::auth_identities::create_identity(
                txn,
                u1.id,
                PROVIDER_GOOGLE,
                &unique_str("sub1"),
                &unique_email("order1").to_lowercase(),
            )
            .await?;

            let u2 = seed_user_with_created_at(
                txn,
                &format!("{}_2", search_term),
                &unique_email("order2"),
                t2,
            )
            .await?;
            backend::repos::auth_identities::create_identity(
                txn,
                u2.id,
                PROVIDER_GOOGLE,
                &unique_str("sub2"),
                &unique_email("order2").to_lowercase(),
            )
            .await?;

            let u3 = seed_user_with_created_at(
                txn,
                &format!("{}_3", search_term),
                &unique_email("order3"),
                t3,
            )
            .await?;
            backend::repos::auth_identities::create_identity(
                txn,
                u3.id,
                PROVIDER_GOOGLE,
                &unique_str("sub3"),
                &unique_email("order3").to_lowercase(),
            )
            .await?;

            let query =
                admin_users::validate_search_query(Some(search_term.clone()), Some(20), None)
                    .map_err(backend::AppError::from)?;
            let result = admin_users::search_users_for_admin(txn, query)
                .await
                .map_err(backend::AppError::from)?;

            assert_eq!(result.items.len(), 3);
            // ORDER BY created_at DESC, id DESC: newest first, then by id desc for ties
            assert_eq!(result.items[0].id, u3.id);
            assert_eq!(result.items[1].id, u2.id);
            assert_eq!(result.items[2].id, u1.id);

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn search_returns_next_cursor_when_more_results() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    let base = unique_str("cursor");
    let search_term = format!("page_{}", base);
    let now = OffsetDateTime::now_utc();

    backend::db::txn::with_txn(None, &state, |txn| {
        Box::pin(async move {
            for i in 1..=4 {
                let user = seed_user_with_created_at(
                    txn,
                    &format!("{}_u{}", search_term, i),
                    &unique_email(&format!("cursor{}", i)),
                    now - time::Duration::seconds(5 - i),
                )
                .await?;
                backend::repos::auth_identities::create_identity(
                    txn,
                    user.id,
                    PROVIDER_GOOGLE,
                    &unique_str(&format!("sub{}", i)),
                    &unique_email(&format!("cursor{}", i)).to_lowercase(),
                )
                .await?;
            }

            let query =
                admin_users::validate_search_query(Some(search_term.clone()), Some(2), None)
                    .map_err(backend::AppError::from)?;
            let result = admin_users::search_users_for_admin(txn, query)
                .await
                .map_err(backend::AppError::from)?;

            assert_eq!(result.items.len(), 2);
            assert!(result.next_cursor.is_some());

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn search_multi_page_pagination_correctness() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    let base = unique_str("multipage");
    let search_term = format!("mp_{}", base);
    let now = OffsetDateTime::now_utc();

    backend::db::txn::with_txn(None, &state, |txn| {
        Box::pin(async move {
            for i in 1..=5 {
                let user = seed_user_with_created_at(
                    txn,
                    &format!("{}_u{}", search_term, i),
                    &unique_email(&format!("mp{}", i)),
                    now - time::Duration::seconds(6 - i),
                )
                .await?;
                backend::repos::auth_identities::create_identity(
                    txn,
                    user.id,
                    PROVIDER_GOOGLE,
                    &unique_str(&format!("mpsub{}", i)),
                    &unique_email(&format!("mp{}", i)).to_lowercase(),
                )
                .await?;
            }

            let query =
                admin_users::validate_search_query(Some(search_term.clone()), Some(2), None)
                    .map_err(backend::AppError::from)?;
            let page1 = admin_users::search_users_for_admin(txn, query)
                .await
                .map_err(backend::AppError::from)?;

            assert_eq!(page1.items.len(), 2);
            let cursor1 = page1.next_cursor.as_ref().expect("expected next_cursor");

            let query2 = admin_users::validate_search_query(
                Some(search_term.clone()),
                Some(2),
                Some(cursor1.clone()),
            )
            .map_err(backend::AppError::from)?;
            let page2 = admin_users::search_users_for_admin(txn, query2)
                .await
                .map_err(backend::AppError::from)?;

            assert_eq!(page2.items.len(), 2);
            let cursor2 = page2.next_cursor.as_ref().expect("expected next_cursor");

            let query3 = admin_users::validate_search_query(
                Some(search_term.clone()),
                Some(2),
                Some(cursor2.clone()),
            )
            .map_err(backend::AppError::from)?;
            let page3 = admin_users::search_users_for_admin(txn, query3)
                .await
                .map_err(backend::AppError::from)?;

            assert_eq!(page3.items.len(), 1);
            assert!(page3.next_cursor.is_none());

            let all_ids: Vec<i64> = page1
                .items
                .iter()
                .chain(page2.items.iter())
                .chain(page3.items.iter())
                .map(|i| i.id)
                .collect();
            assert_eq!(all_ids.len(), 5);
            let unique_ids: std::collections::HashSet<_> = all_ids.iter().collect();
            assert_eq!(
                unique_ids.len(),
                5,
                "pagination must not duplicate or skip items"
            );

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn search_filters_by_email_from_identity() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    let unique_part = unique_str("email-filter");
    let email = format!("alice.{}@example.com", unique_part);
    let username = format!("alice_{}", unique_str("un"));

    backend::db::txn::with_txn(None, &state, |txn| {
        Box::pin(async move {
            let user = seed_user_with_sub(txn, &username, Some(&email)).await?;
            backend::repos::auth_identities::create_identity(
                txn,
                user.id,
                PROVIDER_GOOGLE,
                &unique_str("google-sub"),
                &email.to_lowercase(),
            )
            .await?;

            let query =
                admin_users::validate_search_query(Some(unique_part.clone()), Some(20), None)
                    .map_err(backend::AppError::from)?;
            let result = admin_users::search_users_for_admin(txn, query)
                .await
                .map_err(backend::AppError::from)?;

            assert!(!result.items.is_empty());
            assert!(result.items.iter().any(|i| i.id == user.id));
            let expected_email = email.to_lowercase();
            assert!(
                result
                    .items
                    .iter()
                    .any(|i| i.email.as_deref() == Some(expected_email.as_str())),
                "email from identity must appear in search results"
            );

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn search_excludes_ai_users() -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    let unique_part = unique_str("ai-exclude");
    let search_term = format!("ai_user_{}", unique_part);
    let email = unique_email("ai-exclude");

    backend::db::txn::with_txn(None, &state, |txn| {
        Box::pin(async move {
            let user = seed_user_with_sub(txn, &search_term, Some(&email)).await?;
            backend::repos::auth_identities::create_identity(
                txn,
                user.id,
                PROVIDER_GOOGLE,
                &unique_str("ai-sub"),
                &email.to_lowercase(),
            )
            .await?;

            let mut ai_user: users_entity::ActiveModel = user.clone().into();
            ai_user.is_ai = Set(true);
            ai_user.update(txn).await?;

            let query =
                admin_users::validate_search_query(Some(unique_part.clone()), Some(20), None)
                    .map_err(backend::AppError::from)?;
            let result = admin_users::search_users_for_admin(txn, query)
                .await
                .map_err(backend::AppError::from)?;

            assert!(
                !result.items.iter().any(|i| i.id == user.id),
                "AI users must be excluded from admin search"
            );

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}

#[tokio::test]
async fn search_display_name_prefers_username_over_email() -> Result<(), Box<dyn std::error::Error>>
{
    let state = build_test_state().await?;

    let unique_part = unique_str("display");
    let username = format!("alice_{}", unique_part);
    let email = unique_email("display");

    backend::db::txn::with_txn(None, &state, |txn| {
        Box::pin(async move {
            let user = seed_user_with_sub(txn, &username, Some(&email)).await?;
            backend::repos::auth_identities::create_identity(
                txn,
                user.id,
                PROVIDER_GOOGLE,
                &unique_str("google-sub"),
                &email.to_lowercase(),
            )
            .await?;

            let query =
                admin_users::validate_search_query(Some(unique_part.clone()), Some(20), None)
                    .map_err(backend::AppError::from)?;
            let result = admin_users::search_users_for_admin(txn, query)
                .await
                .map_err(backend::AppError::from)?;

            let item = result
                .items
                .iter()
                .find(|i| i.id == user.id)
                .expect("user in results");
            assert_eq!(
                item.display_name.as_deref(),
                Some(username.as_str()),
                "display_name must prefer username over email"
            );

            Ok::<_, backend::AppError>(())
        })
    })
    .await?;

    Ok(())
}
