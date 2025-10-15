// apps/backend/migration/src/main.rs
use std::env;

use ::backend::config::db::{DbOwner, DbProfile};
use ::backend::infra::db::connect_db;
use migration::{migrate_internal, MigrationCommand};

#[tokio::main]
async fn main() {
    // Set up logging to stdout for CLI - only show our migration info, no timestamps
    tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .without_time()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_line_number(false)
        .with_file(false)
        .with_env_filter("migration=info,sqlx=warn")
        .init();

    // Select database profile via env - no default for safety
    let profile = match env::var("MIGRATION_TARGET").as_deref() {
        Ok("prod") => DbProfile::Prod,
        Ok("pg_test") => DbProfile::Test,
        Ok("sqlite_test") => DbProfile::SqliteFile { file: None },
        _ => {
            eprintln!("❌ MIGRATION_TARGET must be set to one of: prod, pg_test, sqlite_test");
            eprintln!("   This prevents accidental production database operations");
            std::process::exit(1);
        }
    };

    // Subcommand: up | down | fresh | reset | refresh | status
    let cmd = env::args().nth(1).unwrap_or_else(|| "up".to_string());
    let command = match cmd.as_str() {
        "up" => MigrationCommand::Up,
        "down" => MigrationCommand::Down,
        "fresh" => MigrationCommand::Fresh,
        "reset" => MigrationCommand::Reset,
        "refresh" => MigrationCommand::Refresh,
        "status" => MigrationCommand::Status,
        other => {
            eprintln!(
                "Unknown command: {other}. Use: up | down | fresh | reset | refresh | status"
            );
            std::process::exit(2);
        }
    };

    // Connect with owner privileges (can create/drop types/tables)
    let db = connect_db(profile.clone(), DbOwner::Owner)
        .await
        .expect("Failed to connect to database");

    // Use internal migration function
    if let Err(e) = migrate_internal(&db, command).await {
        eprintln!("Migration failed: {e}");
        std::process::exit(1);
    }
}
