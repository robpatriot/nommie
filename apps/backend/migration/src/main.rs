use ::backend::{connect_db, DbOwner, DbProfile};
use sea_orm_migration::prelude::*;
use std::env;

// If your package name is the default "migration", this works as-is.
// If not, change `migration::Migrator` to `<your_package_name>::Migrator`.
#[tokio::main]
async fn main() {
    // Determine target database from MIGRATION_TARGET env var
    let target = env::var("MIGRATION_TARGET").unwrap_or_else(|_| "prod".to_string());
    let profile = match target.as_str() {
        "test" => DbProfile::Test,
        "prod" | _ => DbProfile::Prod,
    };

    // Connect to database using Owner credentials
    let db = connect_db(profile.clone(), DbOwner::Owner)
        .await
        .expect("Failed to connect to database");

    // Run migrations
    migration::Migrator::up(&db, None)
        .await
        .expect("Failed to run migrations");

    println!(
        "✅ Migrations completed successfully for {:?} database",
        profile
    );
}
