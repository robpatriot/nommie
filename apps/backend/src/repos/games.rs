//! Game repository trait for domain layer.

use async_trait::async_trait;

use crate::db::DbConn;
use crate::errors::domain::DomainError;

/// Game domain model
#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub id: i64,
    pub join_code: String,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

/// Repository trait for game operations.
/// 
/// This trait is domain-facing and contains no SeaORM imports.
/// Adapters implement this trait using SeaORM entities.
#[async_trait]
pub trait GameRepo: Send + Sync {
    /// Find game by ID
    async fn find_by_id(&self, conn: &dyn DbConn, game_id: i64) -> Result<Option<Game>, DomainError>;

    /// Find game by join code
    async fn find_by_join_code(&self, conn: &dyn DbConn, join_code: &str) -> Result<Option<Game>, DomainError>;

    /// Create a new game
    async fn create_game(&self, conn: &dyn DbConn, join_code: &str) -> Result<Game, DomainError>;
}
