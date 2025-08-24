use sea_orm::{Database, DatabaseConnection};
use std::env;

/// Connect to database using DATABASE_URL from environment
/// This function does NOT run any migrations
pub async fn connect_from_env() -> Result<DatabaseConnection, sea_orm::DbErr> {
    dotenvy::dotenv().ok();
    
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    
    Database::connect(&database_url).await
}

/// Run database migrations (idempotent)
/// This should only be called from migration scripts (pnpm db:migrate), never from main.rs or tests
pub async fn run_migrations(conn: &DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    use migration::{Migrator, MigratorTrait};
    
    Migrator::up(conn, None).await?;
    Ok(())
}
