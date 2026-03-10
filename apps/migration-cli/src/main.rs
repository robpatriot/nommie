use clap::{Parser, ValueEnum};
use db_infra::config::db::RuntimeEnv;
use db_infra::infra::db::orchestrate_migration;
use migration::MigrationCommand;

#[derive(Clone, ValueEnum)]
enum Env {
    Prod,
    Test,
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

    // Run migration orchestration (Postgres only)
    if let Err(e) = orchestrate_migration(env, command).await {
        eprintln!("Migration failed: {e}");
        std::process::exit(1);
    }
}
