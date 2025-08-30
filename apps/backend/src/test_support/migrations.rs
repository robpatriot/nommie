use sea_orm::DatabaseConnection;

/// Tests must never run migrations. Use `pnpm db:fresh:test` to prepare the test database schema.
///
/// This function will panic with instructions to run the proper command.
pub async fn migrate_test_db(_db_url: &str) -> DatabaseConnection {
    panic!(
        "Tests must never run migrations. Run `pnpm db:fresh:test` to prepare the test database schema, then run tests."
    );
}
