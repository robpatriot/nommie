pub use sea_orm_migration::prelude::*;

mod m20250823_000001_init; // keep filename + module name in sync

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250823_000001_init::Migration),
        ]
    }
}
