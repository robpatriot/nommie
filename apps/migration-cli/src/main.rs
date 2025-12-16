use clap::{Parser, ValueEnum};
use db_infra::config::db::{DbKind, RuntimeEnv};
use db_infra::infra::db::orchestrate_migration;
use migration::MigrationCommand;

#[derive(Clone, ValueEnum)]
enum Env {
    Prod,
    Test,
}

#[derive(Clone, ValueEnum)]
enum Db {
    Postgres,
    SqliteFile,
}

#[derive(Parser)]
#[command(name = "migration-cli")]
#[command(about = "Nommie database migration tool")]
struct Args {
    /// Migration command to run
    #[arg(value_enum)]
    command: String,

    /// Runtime environment
    #[arg(short, long, value_enum, default_value = "test")]
    env: Env,

    /// Database type
    #[arg(
        short,
        long,
        value_enum,
        default_value = "postgres",
        help = "Database type: postgres, sqlite-file"
    )]
    db: Db,
}

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

    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            // Check if this is a database type error and provide helpful message
            if e.to_string().contains("invalid value") && e.to_string().contains("--db") {
                eprintln!("❌ Unsupported database type provided.");
                eprintln!("");
                eprintln!("Note: SQLite in-memory databases are not supported for CLI operations.");
                eprintln!(
                    "Reason: In-memory databases are ephemeral - each CLI command creates a fresh"
                );
                eprintln!(
                    "database that is destroyed when the command completes, making migration"
                );
                eprintln!("operations pointless.");
                eprintln!("");
                eprintln!("Supported database types:");
                eprintln!("  • postgres    - PostgreSQL database");
                eprintln!("  • sqlite-file - SQLite file database");
                eprintln!("");
                eprintln!("Example: cargo run --manifest-path apps/migration-cli/Cargo.toml -- --db sqlite-file status");
                std::process::exit(1);
            }
            // For other errors, use clap's default error handling
            eprintln!("{e}");
            std::process::exit(2);
        }
    };

    let command = match args.command.as_str() {
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

    let env = match args.env {
        Env::Prod => RuntimeEnv::Prod,
        Env::Test => RuntimeEnv::Test,
    };

    let db_kind = match args.db {
        Db::Postgres => DbKind::Postgres,
        Db::SqliteFile => DbKind::SqliteFile,
    };

    // Run migration orchestration
    if let Err(e) = orchestrate_migration(env, db_kind, command).await {
        eprintln!("Migration failed: {e}");
        std::process::exit(1);
    }
}
