pub use sea_orm::{ConnectionTrait, DatabaseConnection};
pub use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::Statement;

mod m20250823_000001_init; // keep filename + module name in sync

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20250823_000001_init::Migration)]
    }
}

#[derive(Debug, Clone)]
pub enum MigrationCommand {
    Up,
    Down,
    Fresh,
    Reset,
    Refresh,
    Status,
}

/// Migration function that bypasses environment parsing
/// Used by both CLI and tests
pub async fn migrate(db: &DatabaseConnection, command: MigrationCommand) -> Result<(), DbErr> {
    let db_info_before = get_db_diagnostics(db).await?;

    // Log diagnostics before command execution
    tracing::info!("▶ cmd={command:?}  profile={}", db_info_before.profile);
    tracing::info!("▶ connected to DB: {}", db_info_before.name);
    tracing::info!(
        "▶ BEFORE: runner has {} migration(s) defined, {} applied",
        db_info_before.defined_migrations_count,
        db_info_before.mig_count
    );

    // Execute the migration command
    let result = match command {
        MigrationCommand::Up => Migrator::up(db, None).await,
        MigrationCommand::Down => Migrator::down(db, None).await,
        MigrationCommand::Fresh => Migrator::fresh(db).await,
        MigrationCommand::Reset => Migrator::reset(db).await,
        MigrationCommand::Refresh => Migrator::refresh(db).await,
        MigrationCommand::Status => Migrator::status(db).await,
    };

    match result {
        Ok(()) => {
            // Get diagnostics after command execution (except for Status command which doesn't change state)
            if !matches!(command, MigrationCommand::Status) {
                let db_info_after = get_db_diagnostics(db).await?;
                tracing::info!(
                    "▶ AFTER: runner has {} migration(s) defined, {} applied",
                    db_info_after.defined_migrations_count,
                    db_info_after.mig_count
                );
            }
            tracing::info!("✅ {command:?} OK for {}", db_info_before.profile);
            Ok(())
        }
        Err(e) => {
            tracing::error!("❌ {command:?} failed for {}: {e}", db_info_before.profile);
            Err(e)
        }
    }
}

#[derive(Debug)]
struct DbDiagnostics {
    profile: String,
    name: String,
    mig_count: usize,
    defined_migrations_count: usize,
}

async fn get_db_diagnostics(db: &DatabaseConnection) -> Result<DbDiagnostics, sea_orm::DbErr> {
    let profile = format!("{:?}", db.get_database_backend());

    let name = match db.get_database_backend() {
        sea_orm::DatabaseBackend::Postgres => {
            let stmt = Statement::from_string(
                db.get_database_backend(),
                String::from("select current_database() as name"),
            );
            if let Some(row) = db.query_one(stmt).await? {
                row.try_get("", "name")?
            } else {
                "<unknown>".to_string()
            }
        }
        sea_orm::DatabaseBackend::Sqlite => {
            let stmt = Statement::from_string(
                db.get_database_backend(),
                String::from("SELECT file FROM pragma_database_list WHERE name = 'main'"),
            );
            if let Some(row) = db.query_one(stmt).await? {
                if let Ok(file) = row.try_get::<String>("", "file") {
                    if file.is_empty() {
                        ":memory:".to_string()
                    } else {
                        file
                    }
                } else {
                    "<unknown>".to_string()
                }
            } else {
                "<unknown>".to_string()
            }
        }
        _ => "<unsupported>".to_string(),
    };

    // Use SeaORM directly since we're in migration context with admin pool
    let applied_migrations = Migrator::get_applied_migrations(db).await?;
    let applied_migrations_count = applied_migrations.len();
    let defined_migrations_count = Migrator::migrations().len();

    Ok(DbDiagnostics {
        profile,
        name,
        mig_count: applied_migrations_count,
        defined_migrations_count,
    })
}

/// Count the number of migrations that have been applied to the database.
/// Queries the seaql_migrations table directly to work with app-level database users
/// that only have SELECT permissions (no CREATE required).
/// Returns 0 if the migration table doesn't exist yet.
pub async fn count_applied_migrations(db: &DatabaseConnection) -> Result<usize, DbErr> {
    let stmt = Statement::from_string(
        db.get_database_backend(),
        String::from("SELECT COUNT(*) AS cnt FROM seaql_migrations"),
    );

    match db.query_one(stmt).await {
        Ok(Some(row)) => row.try_get::<i64>("", "cnt").map(|c| c as usize),
        Ok(None) => Ok(0),              // Table exists but empty
        Err(DbErr::Exec(_)) => Ok(0),   // Table doesn't exist yet
        Err(e) => Err(e),
    }
}

/// Get the version string of the latest applied migration.
/// Queries the seaql_migrations table directly to work with app-level database users
/// that only have SELECT permissions (no CREATE required).
/// Returns None if no migrations have been applied or the migration table doesn't exist.
pub async fn get_latest_migration_version(
    db: &DatabaseConnection,
) -> Result<Option<String>, DbErr> {
    let stmt = Statement::from_string(
        db.get_database_backend(),
        String::from(
            "SELECT version FROM seaql_migrations ORDER BY applied_at DESC LIMIT 1",
        ),
    );

    match db.query_one(stmt).await {
        Ok(Some(row)) => row.try_get::<String>("", "version").map(Some),
        Ok(None) => Ok(None),           // Table exists but no rows
        Err(DbErr::Exec(_)) => Ok(None),// Table doesn't exist yet
        Err(e) => Err(e),
    }
}
