//! Database infrastructure - connection management, migrations, and diagnostics.

pub mod core;
pub mod diagnostics;
pub mod locking;

// Re-export commonly used config types for convenience
// Re-export the main bootstrap function
pub use core::bootstrap_db;
// Re-export admin pool builder for tests
pub use core::build_admin_pool;
// Re-export the orchestrate migration function
pub use core::orchestrate_migration;

// Re-export diagnostics for external use
pub use diagnostics::sqlite_diagnostics;

pub use crate::config::db::{DbKind, DbOwner, RuntimeEnv};
