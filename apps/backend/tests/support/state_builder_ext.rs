//! Test-only extension trait for StateBuilder
//!
//! This module provides test-specific extensions to the StateBuilder that allow
//! tests to easily create mock database connections without polluting production code.

use backend::infra::state::StateBuilder;
use sea_orm::{DatabaseBackend, MockDatabase};

/// Test-only extension trait for StateBuilder
pub trait StateBuilderTestExt {
    /// Create a strict SeaORM mock connection for Postgres and delegate to `with_existing_db`
    fn with_mock_db(self) -> Self;

    /// Create a strict SeaORM mock connection with pre-configured query/exec results
    fn with_mock_db_with_results<F>(self, setup_fn: F) -> Self
    where
        F: FnOnce(&mut MockDatabase);
}

impl StateBuilderTestExt for StateBuilder {
    fn with_mock_db(self) -> Self {
        let mock_db = MockDatabase::new(DatabaseBackend::Postgres);
        let conn = mock_db.into_connection();
        self.with_existing_db(conn).assume_schema_ready()
    }

    fn with_mock_db_with_results<F>(self, setup_fn: F) -> Self
    where
        F: FnOnce(&mut MockDatabase),
    {
        let mut mock_db = MockDatabase::new(DatabaseBackend::Postgres);
        setup_fn(&mut mock_db);
        let conn = mock_db.into_connection();
        self.with_existing_db(conn).assume_schema_ready()
    }
}
