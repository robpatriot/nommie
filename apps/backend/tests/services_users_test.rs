use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serial_test::serial;

use backend::{
    bootstrap::db::connect_db,
    config::db::{DbOwner, DbProfile},
    entities::user_credentials,
    services::users::ensure_user,
};

#[tokio::test]
#[serial]
async fn test_ensure_user_inserts_then_reuses() {
    let db = connect_db(DbProfile::Test, DbOwner::App)
        .await
        .expect("connect to _test database");

    // First call - should create a new user
    let (user1, email1) = ensure_user("alice@example.com", Some("Alice"), "google-sub-123", &db)
        .await
        .expect("should create user successfully");

    // Verify user was created with expected values
    assert_eq!(user1.username, Some("Alice".to_string()));
    assert!(!user1.is_ai);
    assert!(user1.id > 0); // ID should be a positive number
    assert_eq!(email1, "alice@example.com");

    // Second call with same email but different name - should return same user
    let (user2, _email2) = ensure_user(
        "alice@example.com",
        Some("Alice Smith"), // Different name
        "google-sub-456",    // Different google_sub
        &db,
    )
    .await
    .expect("should return existing user");

    // Verify idempotency - same user ID
    assert_eq!(user1.id, user2.id);
    assert_eq!(user2.username, Some("Alice".to_string())); // Username should not change

    // Verify that only one user_credentials row exists for this email
    let credential_count = user_credentials::Entity::find()
        .filter(user_credentials::Column::Email.eq("alice@example.com"))
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
        .filter(user_credentials::Column::Email.eq("alice@example.com"))
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
        Some("google-sub-123".to_string()),
        "google_sub should be the first one set"
    );
}
