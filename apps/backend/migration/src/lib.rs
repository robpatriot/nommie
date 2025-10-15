pub use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{DatabaseConnection, Statement};

mod m20250823_000001_init; // keep filename + module name in sync

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20250823_000001_init::Migration)]
    }
}

#[derive(Debug)]
pub enum MigrationCommand {
    Up,
    Down,
    Fresh,
    Reset,
    Refresh,
    Status,
}

/// Internal migration function that bypasses environment parsing
/// Used by both CLI and tests
pub async fn migrate_internal(
    db: &DatabaseConnection,
    command: MigrationCommand,
) -> Result<(), ::backend::error::AppError> {
    let db_info = get_db_diagnostics(db).await?;

    // Log diagnostics to tracing (controlled by logging config)
    tracing::info!("▶ cmd={command:?}  profile={}", db_info.profile);
    tracing::info!("▶ connected to DB: {}", db_info.name);
    tracing::info!("▶ runner sees {} migration(s)", db_info.mig_count);

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
            tracing::info!("✅ {command:?} OK for {}", db_info.profile);
            Ok(())
        }
        Err(e) => {
            tracing::error!("❌ {command:?} failed for {}: {e}", db_info.profile);
            Err(::backend::error::AppError::from(e))
        }
    }
}

#[derive(Debug)]
struct DbDiagnostics {
    profile: String,
    name: String,
    mig_count: usize,
}

async fn get_db_diagnostics(
    db: &DatabaseConnection,
) -> Result<DbDiagnostics, ::backend::error::AppError> {
    let profile = format!("{:?}", db.get_database_backend());

    let name = match db.get_database_backend() {
        sea_orm::DatabaseBackend::Postgres => {
            let stmt = Statement::from_string(
                db.get_database_backend(),
                String::from("select current_database() as name"),
            );
            if let Some(row) = db
                .query_one(stmt)
                .await
                .map_err(::backend::error::AppError::from)?
            {
                row.try_get("", "name")
                    .map_err(::backend::error::AppError::from)?
            } else {
                "<unknown>".to_string()
            }
        }
        sea_orm::DatabaseBackend::Sqlite => {
            let stmt = Statement::from_string(
                db.get_database_backend(),
                String::from("select sqlite_version() as name"),
            );
            if let Some(row) = db
                .query_one(stmt)
                .await
                .map_err(::backend::error::AppError::from)?
            {
                row.try_get("", "name")
                    .map_err(::backend::error::AppError::from)?
            } else {
                "<unknown>".to_string()
            }
        }
        _ => "<unsupported>".to_string(),
    };

    let mig_count = <Migrator as MigratorTrait>::migrations().len();

    Ok(DbDiagnostics {
        profile,
        name,
        mig_count,
    })
}
