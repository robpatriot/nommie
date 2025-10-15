use std::env;

use backend::infra::db::connect_db_for_migration;
use migration::{migrate, MigrationCommand};

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

    // Select database target via env - no default for safety
    let target = match env::var("MIGRATION_TARGET").as_deref() {
        Ok("prod") | Ok("pg_test") | Ok("sqlite_test") => env::var("MIGRATION_TARGET").unwrap(),
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
    let db = connect_db_for_migration(&target)
        .await
        .expect("Failed to connect to database");

    // Use migration function
    if let Err(e) = migrate(&db, command).await {
        eprintln!("Migration failed: {e}");
        std::process::exit(1);
    }
}
