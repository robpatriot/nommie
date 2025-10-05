//! SeaORM adapter for game repository.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, QueryFilter, Set};

use crate::db::{as_database_connection, as_database_transaction, DbConn};
use crate::entities::games;
use crate::errors::domain::{DomainError, InfraErrorKind};
use crate::infra::db_errors;
use crate::repos::games::{Game, GameRepo};

/// SeaORM implementation of GameRepo.
#[derive(Debug, Default)]
pub struct GameRepoSea;

impl GameRepoSea {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl GameRepo for GameRepoSea {
    async fn find_by_id(
        &self,
        conn: &dyn DbConn,
        game_id: i64,
    ) -> Result<Option<Game>, DomainError> {
        let game = {
            if let Some(txn) = as_database_transaction(conn) {
                games::Entity::find()
                    .filter(games::Column::Id.eq(game_id))
                    .one(txn)
                    .await
            } else if let Some(db) = as_database_connection(conn) {
                games::Entity::find()
                    .filter(games::Column::Id.eq(game_id))
                    .one(db)
                    .await
            } else {
                return Err(DomainError::infra(
                    InfraErrorKind::Other("Connection type".to_string()),
                    "Unsupported DbConn type for SeaORM".to_string(),
                ));
            }
        }
        .map_err(db_errors::map_db_err)?;

        Ok(game.map(|g| Game {
            id: g.id,
            join_code: g.join_code.unwrap_or_default(),
            created_at: g.created_at,
            updated_at: g.updated_at,
        }))
    }

    async fn find_by_join_code(
        &self,
        conn: &dyn DbConn,
        join_code: &str,
    ) -> Result<Option<Game>, DomainError> {
        let game = {
            if let Some(txn) = as_database_transaction(conn) {
                games::Entity::find()
                    .filter(games::Column::JoinCode.eq(join_code))
                    .one(txn)
                    .await
            } else if let Some(db) = as_database_connection(conn) {
                games::Entity::find()
                    .filter(games::Column::JoinCode.eq(join_code))
                    .one(db)
                    .await
            } else {
                return Err(DomainError::infra(
                    InfraErrorKind::Other("Connection type".to_string()),
                    "Unsupported DbConn type for SeaORM".to_string(),
                ));
            }
        }
        .map_err(db_errors::map_db_err)?;

        Ok(game.map(|g| Game {
            id: g.id,
            join_code: g.join_code.unwrap_or_default(),
            created_at: g.created_at,
            updated_at: g.updated_at,
        }))
    }

    async fn create_game(&self, conn: &dyn DbConn, join_code: &str) -> Result<Game, DomainError> {
        let now = time::OffsetDateTime::now_utc();
        let game_active = games::ActiveModel {
            id: NotSet,
            created_by: NotSet,
            visibility: Set(games::GameVisibility::Private),
            state: Set(games::GameState::Lobby),
            created_at: Set(now),
            updated_at: Set(now),
            started_at: NotSet,
            ended_at: NotSet,
            name: NotSet,
            join_code: Set(Some(join_code.to_string())),
            rules_version: Set("1.0".to_string()),
            rng_seed: NotSet,
            current_round: NotSet,
            hand_size: NotSet,
            dealer_pos: NotSet,
            lock_version: Set(1),
        };

        let game = if let Some(txn) = as_database_transaction(conn) {
            game_active.insert(txn).await
        } else if let Some(db) = as_database_connection(conn) {
            game_active.insert(db).await
        } else {
            return Err(DomainError::infra(
                InfraErrorKind::Other("Connection type".to_string()),
                "Unsupported DbConn type for SeaORM".to_string(),
            ));
        }
        .map_err(db_errors::map_db_err)?;

        Ok(Game {
            id: game.id,
            join_code: game.join_code.unwrap_or_default(),
            created_at: game.created_at,
            updated_at: game.updated_at,
        })
    }
}
