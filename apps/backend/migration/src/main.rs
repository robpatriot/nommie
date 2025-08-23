use sea_orm_migration::prelude::*;

// If your package name is the default "migration", this works as-is.
// If not, change `migration::Migrator` to `<your_package_name>::Migrator`.
#[tokio::main]
async fn main() {
    cli::run_cli(migration::Migrator).await;
}
