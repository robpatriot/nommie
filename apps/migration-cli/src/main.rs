use std::env;

use backend::config::db::{DbOwner, DbProfile};
use backend::infra::db::bootstrap_db;
use migration::{migrate, MigrationCommand};

#[tokio::main]
async fn main() {
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

    let target = match env::var("MIGRATION_TARGET").as_deref() {
        Ok("prod") | Ok("pg_test") | Ok("sqlite_test") => env::var("MIGRATION_TARGET").unwrap(),
        _ => {
            eprintln!("❌ MIGRATION_TARGET must be one of: prod | pg_test | sqlite_test");
            std::process::exit(1);
        }
    };

    let cmd = env::args().nth(1).unwrap_or_else(|| "up".to_string());
    let command = match cmd.as_str() {
        "up" => MigrationCommand::Up,
        "down" => MigrationCommand::Down,
        "fresh" => MigrationCommand::Fresh,
        "reset" => MigrationCommand::Reset,
        "refresh" => MigrationCommand::Refresh,
        "status" => MigrationCommand::Status,
        other => { eprintln!("Unknown command: {other}. Use: up | down | fresh | reset | refresh | status"); std::process::exit(2); }
    };

    let profile = match target.as_str() {
        "prod" => DbProfile::Prod,
        "pg_test" => DbProfile::Test,
        "sqlite_test" => DbProfile::SqliteFile { file: None },
        _ => unreachable!(),
    };

    // Build connection and ensure schema with Owner privileges
    let db = bootstrap_db(profile, DbOwner::Owner).await
        .expect("DB bootstrap failed");

    // Allow running the chosen command as well (e.g., status, fresh)
    if let Err(e) = migrate(&db, command).await {
        eprintln!("Migration failed: {e}");
        std::process::exit(1);
    }
}
