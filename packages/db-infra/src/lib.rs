//! Shared database configuration and migration infrastructure.
//! Used by the backend and the migration CLI.

pub mod config;
pub mod error;
pub mod infra;

pub use config::db;
pub use config::db::sqlite_file_spec;
pub use error::DbInfraError;
pub use infra::db::core::{build_admin_pool, orchestrate_migration, orchestrate_migration_internal, sanitize_db_url};
