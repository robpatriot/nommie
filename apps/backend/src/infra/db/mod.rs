//! Database infrastructure - connection management, migrations, and diagnostics.

pub mod core;

// Diagnostics and locking are provided by db-infra and re-exported here
// Re-export commonly used config types and core DB functions
#[allow(unused_imports)]
pub use core::bootstrap_db;

pub use db_infra::infra::db::core::orchestrate_migration;
pub use db_infra::infra::db::{diagnostics, locking};

pub use crate::config::db::{DbKind, DbOwner, RuntimeEnv};
