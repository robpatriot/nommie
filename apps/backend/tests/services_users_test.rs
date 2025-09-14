use backend::config::db::{DbOwner, DbProfile};
use backend::entities::user_credentials;
use backend::infra::db::connect_db;
use backend::services::users::ensure_user;
use backend::utils::unique::{unique_email, unique_str};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn test_ensure_user_inserts_then_reuses() {
    let db = connect_db(DbProfile::Test, DbOwner::App)
        .await
        .expect("connect to _test database");

    // First call - should create a new user
    let test_email = unique_email("alice");
    let test_google_sub = unique_str("google-sub");
    let user1 = ensure_user(
        test_email.clone(),
        Some("Alice".to_string()),
        test_google_sub.clone(),
        &db,
    )
    .await
    .expect("should create user successfully");

    // Verify user was created with expected values
    assert_eq!(user1.username, Some("Alice".to_string()));
    assert!(!user1.is_ai);
    assert!(user1.id > 0); // ID should be a positive number

    // Second call with same email but different name - should return same user
    let different_google_sub = unique_str("google-sub");
    let user2 = ensure_user(
        test_email.clone(),
        Some("Alice Smith".to_string()), // Different name
        different_google_sub.clone(),    // Different google_sub
        &db,
    )
    .await
    .expect("should return existing user");

    // Verify idempotency - same user ID
    assert_eq!(user1.id, user2.id);
    assert_eq!(user2.username, Some("Alice".to_string())); // Username should not change

    // Verify that only one user_credentials row exists for this email
    let credential_count = user_credentials::Entity::find()
        .filter(user_credentials::Column::Email.eq(&test_email))
        .count(&db)
        .await
        .expect("should count credentials successfully");

    // Should have exactly one credential row
    assert_eq!(
        credential_count, 1,
        "Should have exactly one credential row"
    );

    // Verify that the credential row has the correct user_id
    let credential = user_credentials::Entity::find()
        .filter(user_credentials::Column::Email.eq(&test_email))
        .one(&db)
        .await
        .expect("should query successfully")
        .expect("should have credential row");

    assert_eq!(
        credential.user_id, user1.id,
        "Credential should link to the correct user"
    );
    assert!(credential.last_login.is_some(), "last_login should be set");
    assert_eq!(
        credential.google_sub,
        Some(test_google_sub.clone()),
        "google_sub should be the first one set"
    );
}
