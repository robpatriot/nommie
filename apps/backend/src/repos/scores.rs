//! Scores repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::scores_sea as scores_adapter;
use crate::entities::round_scores;
use crate::errors::domain::DomainError;

/// Score domain model
#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    pub id: i64,
    pub round_id: i64,
    pub player_seat: i16,
    pub bid_value: i16,
    pub tricks_won: i16,
    pub bid_met: bool,
    pub base_score: i16,
    pub bonus: i16,
    pub round_score: i16,
    pub total_score_after: i16,
    pub created_at: time::OffsetDateTime,
}

/// Data for creating a score (reduces parameter count)
#[derive(Debug, Clone)]
pub struct ScoreData {
    pub round_id: i64,
    pub player_seat: i16,
    pub bid_value: i16,
    pub tricks_won: i16,
    pub bid_met: bool,
    pub base_score: i16,
    pub bonus: i16,
    pub round_score: i16,
    pub total_score_after: i16,
}

// Free functions (generic) for score operations

/// Create a score record for a player
pub async fn create_score(
    txn: &DatabaseTransaction,
    data: ScoreData,
) -> Result<Score, DomainError> {
    let dto = scores_adapter::ScoreCreate {
        round_id: data.round_id,
        player_seat: data.player_seat,
        bid_value: data.bid_value,
        tricks_won: data.tricks_won,
        bid_met: data.bid_met,
        base_score: data.base_score,
        bonus: data.bonus,
        round_score: data.round_score,
        total_score_after: data.total_score_after,
    };
    let score = scores_adapter::create_score(txn, dto).await?;
    Ok(Score::from(score))
}

/// Find all scores for a round (ordered by player_seat)
pub async fn find_all_by_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Vec<Score>, DomainError> {
    let scores = scores_adapter::find_all_by_round(conn, round_id).await?;
    Ok(scores.into_iter().map(Score::from).collect())
}

/// Find score for a specific player in a round
pub async fn find_by_round_and_seat<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
    player_seat: i16,
) -> Result<Option<Score>, DomainError> {
    let score = scores_adapter::find_by_round_and_seat(conn, round_id, player_seat).await?;
    Ok(score.map(Score::from))
}

/// Get current total scores for all players in a game (latest round)
/// Returns array of [seat0_total, seat1_total, seat2_total, seat3_total]
pub async fn get_current_totals(
    txn: &DatabaseTransaction,
    game_id: i64,
) -> Result<[i16; 4], DomainError> {
    let totals = scores_adapter::get_current_totals(txn, game_id).await?;
    Ok(totals)
}

// Conversions between SeaORM models and domain models

impl From<round_scores::Model> for Score {
    fn from(model: round_scores::Model) -> Self {
        Self {
            id: model.id,
            round_id: model.round_id,
            player_seat: model.player_seat,
            bid_value: model.bid_value,
            tricks_won: model.tricks_won,
            bid_met: model.bid_met,
            base_score: model.base_score,
            bonus: model.bonus,
            round_score: model.round_score,
            total_score_after: model.total_score_after,
            created_at: model.created_at,
        }
    }
}
