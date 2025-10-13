//! SeaORM adapter for AI overrides.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, NotSet,
    QueryFilter, Set,
};

use crate::entities::ai_overrides;

pub mod dto;

pub use dto::{AiOverrideCreate, AiOverrideUpdate};

// Adapter functions return DbErr; repos layer maps to DomainError via From<DbErr>.

pub async fn create_override(
    txn: &DatabaseTransaction,
    dto: AiOverrideCreate,
) -> Result<ai_overrides::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();
    let override_active = ai_overrides::ActiveModel {
        id: NotSet,
        game_player_id: Set(dto.game_player_id),
        name: Set(dto.name),
        memory_level: Set(dto.memory_level),
        config: Set(dto.config),
        created_at: Set(now),
        updated_at: Set(now),
    };

    override_active.insert(txn).await
}

pub async fn find_by_game_player_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_player_id: i64,
) -> Result<Option<ai_overrides::Model>, sea_orm::DbErr> {
    ai_overrides::Entity::find()
        .filter(ai_overrides::Column::GamePlayerId.eq(game_player_id))
        .one(conn)
        .await
}

pub async fn update_override(
    txn: &DatabaseTransaction,
    dto: AiOverrideUpdate,
) -> Result<ai_overrides::Model, sea_orm::DbErr> {
    let override_model = ai_overrides::ActiveModel {
        id: Set(dto.id),
        game_player_id: Set(dto.game_player_id),
        name: Set(dto.name),
        memory_level: Set(dto.memory_level),
        config: Set(dto.config),
        created_at: NotSet,
        updated_at: Set(time::OffsetDateTime::now_utc()),
    };
    override_model.update(txn).await
}

pub async fn delete_by_game_player_id(
    txn: &DatabaseTransaction,
    game_player_id: i64,
) -> Result<(), sea_orm::DbErr> {
    ai_overrides::Entity::delete_many()
        .filter(ai_overrides::Column::GamePlayerId.eq(game_player_id))
        .exec(txn)
        .await?;
    Ok(())
}
