use std::sync::Arc;

use sea_orm::DatabaseConnection;

use super::security_config::SecurityConfig;
use crate::adapters::games_sea::GameRepoSea;
use crate::adapters::memberships_sea::MembershipRepoSea;
use crate::adapters::players_sea::PlayerRepoSea;
use crate::repos::games::GameRepo;
use crate::repos::memberships::MembershipRepo;
use crate::repos::players::PlayerRepo;

/// Application state containing shared resources
pub struct AppState {
    /// Database connection (optional)
    db: Option<DatabaseConnection>,
    /// Security configuration including JWT settings
    pub security: SecurityConfig,
    /// Repository instances (trait objects)
    pub games_repo: Arc<dyn GameRepo>,
    pub memberships_repo: Arc<dyn MembershipRepo>,
    pub players_repo: Arc<dyn PlayerRepo>,
}

impl AppState {
    fn new_inner(db: Option<DatabaseConnection>, security: SecurityConfig) -> Self {
        Self {
            db,
            security,

            games_repo: Arc::new(GameRepoSea::new()) as Arc<dyn GameRepo>,
            memberships_repo: Arc::new(MembershipRepoSea::new()) as Arc<dyn MembershipRepo>,
            players_repo: Arc::new(PlayerRepoSea::new()) as Arc<dyn PlayerRepo>,
        }
    }

    /// Create a new AppState with the given database connection and security config
    pub fn new(db: DatabaseConnection, security: SecurityConfig) -> Self {
        Self::new_inner(Some(db), security)
    }

    /// Create a new AppState with no database connection
    pub fn new_without_db(security: SecurityConfig) -> Self {
        Self::new_inner(None, security)
    }

    /// Get a reference to the database connection if available
    pub fn db(&self) -> Option<&DatabaseConnection> {
        self.db.as_ref()
    }
}
