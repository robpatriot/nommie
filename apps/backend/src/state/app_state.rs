use std::sync::Arc;
use sea_orm::DatabaseConnection;

use super::security_config::SecurityConfig;
use crate::adapters::{users_sea::UserRepoSea, games_sea::GameRepoSea, memberships_sea::MembershipRepoSea, players_sea::PlayerRepoSea};
use crate::repos::{users::UserRepo, games::GameRepo, memberships::MembershipRepo, players::PlayerRepo};

/// Application state containing shared resources
pub struct AppState {
    /// Database connection (optional)
    db: Option<DatabaseConnection>,
    /// Security configuration including JWT settings
    pub security: SecurityConfig,
    /// Repository instances (trait objects)
    pub users_repo: Arc<dyn UserRepo>,
    pub games_repo: Arc<dyn GameRepo>,
    pub memberships_repo: Arc<dyn MembershipRepo>,
    pub players_repo: Arc<dyn PlayerRepo>,
}

impl AppState {
    /// Create a new AppState with the given database connection and security config
    pub fn new(db: DatabaseConnection, security: SecurityConfig) -> Self {
        Self {
            db: Some(db),
            security: security.clone(),
            users_repo: Arc::new(UserRepoSea::new()) as Arc<dyn UserRepo>,
            games_repo: Arc::new(GameRepoSea::new()) as Arc<dyn GameRepo>,
            memberships_repo: Arc::new(MembershipRepoSea::new()) as Arc<dyn MembershipRepo>,
            players_repo: Arc::new(PlayerRepoSea::new()) as Arc<dyn PlayerRepo>,
        }
    }

    /// Create a new AppState with no database connection
    pub fn new_without_db(security: SecurityConfig) -> Self {
        Self { 
            db: None, 
            security: security.clone(),
            users_repo: Arc::new(UserRepoSea::new()) as Arc<dyn UserRepo>,
            games_repo: Arc::new(GameRepoSea::new()) as Arc<dyn GameRepo>,
            memberships_repo: Arc::new(MembershipRepoSea::new()) as Arc<dyn MembershipRepo>,
            players_repo: Arc::new(PlayerRepoSea::new()) as Arc<dyn PlayerRepo>,
        }
    }

    /// Get a reference to the database connection if available
    pub fn db(&self) -> Option<&DatabaseConnection> {
        self.db.as_ref()
    }
}
