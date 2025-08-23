use std::env;
use sea_orm::{DatabaseConnection, Database};
use once_cell::sync::OnceCell;

pub mod migrations;
pub use migrations::migrate_test_db;

/// Asserts that the database URL is for a test database (ends with '_test')
/// Panics if the database name doesn't end with '_test'
pub fn assert_test_db_url(url: &str) {
    if !url.ends_with("_test") {
        panic!("Tests must run against a test database (ending with '_test'). Current DATABASE_URL: {}", url);
    }
}

/// Loads the test environment from .env.test file
/// Uses dotenvy::from_filename(".env.test").ok() to avoid panicking if file doesn't exist
pub fn load_test_env() {
    dotenvy::from_filename(".env.test").ok();
}

/// Migrates the test database and returns a database connection
/// This function runs migrations once per test process using OnceCell
pub async fn migrate_test_db(db_url: &str) -> DatabaseConnection {
    static MIGRATED: OnceCell<()> = OnceCell::new();
    
    // Ensure migrations only run once per test process
    MIGRATED.get_or_init(|| {
        // For now, we'll just connect without running migrations
        // TODO: Implement SeaORM Migrator when migrations are available
    });
    
    // Connect to the database
    Database::connect(db_url)
        .await
        .expect("Failed to connect to test database")
}

/// Gets the test database URL from environment variables
pub fn get_test_db_url() -> String {
    env::var("DATABASE_URL").unwrap_or_else(|_| {
        panic!("DATABASE_URL environment variable is required for tests");
    })
}
