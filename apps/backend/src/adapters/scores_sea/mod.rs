//! SeaORM adapter for scores repository.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, Order,
    QueryFilter, QueryOrder, Set,
};

use crate::entities::{game_rounds, round_scores};

pub mod dto;

pub use dto::ScoreCreate;

/// Find all scores for a round (ordered by player_seat)
pub async fn find_all_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Vec<round_scores::Model>, sea_orm::DbErr> {
    round_scores::Entity::find()
        .filter(round_scores::Column::RoundId.eq(round_id))
        .order_by(round_scores::Column::PlayerSeat, Order::Asc)
        .all(conn)
        .await
}

/// Find score for a specific player in a round
pub async fn find_by_round_and_seat<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
    player_seat: i16,
) -> Result<Option<round_scores::Model>, sea_orm::DbErr> {
    round_scores::Entity::find()
        .filter(round_scores::Column::RoundId.eq(round_id))
        .filter(round_scores::Column::PlayerSeat.eq(player_seat))
        .one(conn)
        .await
}

/// Create a score record
pub async fn create_score(
    txn: &DatabaseTransaction,
    dto: ScoreCreate,
) -> Result<round_scores::Model, sea_orm::DbErr> {
    let now = time::OffsetDateTime::now_utc();

    let score = round_scores::ActiveModel {
        id: sea_orm::NotSet,
        round_id: Set(dto.round_id),
        player_seat: Set(dto.player_seat),
        bid_value: Set(dto.bid_value),
        tricks_won: Set(dto.tricks_won),
        bid_met: Set(dto.bid_met),
        base_score: Set(dto.base_score),
        bonus: Set(dto.bonus),
        round_score: Set(dto.round_score),
        total_score_after: Set(dto.total_score_after),
        created_at: Set(now),
    };

    score.insert(txn).await
}

/// Get current total scores for all players in a game
/// Returns [seat0, seat1, seat2, seat3] by finding latest round scores
pub async fn get_current_totals(
    txn: &DatabaseTransaction,
    game_id: i64,
) -> Result<[i16; 4], sea_orm::DbErr> {
    // Find the latest round for this game
    let latest_round = game_rounds::Entity::find()
        .filter(game_rounds::Column::GameId.eq(game_id))
        .order_by(game_rounds::Column::RoundNo, Order::Desc)
        .one(txn)
        .await?;

    if let Some(round) = latest_round {
        // Get all scores for this round
        let scores = find_all_by_round(txn, round.id).await?;

        // Build array (initialize to 0)
        let mut totals = [0i16; 4];
        for score in scores {
            if score.player_seat >= 0 && score.player_seat < 4 {
                totals[score.player_seat as usize] = score.total_score_after;
            }
        }
        Ok(totals)
    } else {
        // No rounds yet, all scores are 0
        Ok([0, 0, 0, 0])
    }
}
