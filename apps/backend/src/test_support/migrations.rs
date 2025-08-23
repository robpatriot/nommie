use sea_orm::{DatabaseConnection, Database};
use once_cell::sync::OnceCell;
use migration::Migrator;

/// Migrates the test database and returns a database connection
/// This function runs migrations once per test process using OnceCell
pub async fn migrate_test_db(db_url: &str) -> DatabaseConnection {
    static MIGRATED: OnceCell<()> = OnceCell::new();
    
    // Connect to the database first
    let db = Database::connect(db_url)
        .await
        .expect("Failed to connect to test database");
    
    // Ensure migrations only run once per test process
    MIGRATED.get_or_init(|| {
        // Run migrations synchronously in the OnceCell init
        // We'll use tokio::runtime::Handle to run the async migration
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            Migrator::up(&db, None)
                .await
                .expect("Failed to run database migrations");
        });
    });
    
    db
}
