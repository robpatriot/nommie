//! Database infrastructure - connection management, migrations, and diagnostics.

pub mod core;
pub mod diagnostics;
pub mod locking;

// Re-export commonly used config types for convenience
// Re-export the main bootstrap and admin pool functions
#[allow(unused_imports)]
pub use core::{bootstrap_db, build_admin_pool};

pub use crate::config::db::{DbKind, DbOwner, RuntimeEnv};
