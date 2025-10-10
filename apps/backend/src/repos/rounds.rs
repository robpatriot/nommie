//! Round repository functions for domain layer (generic over ConnectionTrait).

use sea_orm::ConnectionTrait;

use crate::adapters::rounds_sea as rounds_adapter;
use crate::entities::game_rounds;
use crate::errors::domain::DomainError;

/// Round domain model
#[derive(Debug, Clone, PartialEq)]
pub struct Round {
    pub id: i64,
    pub game_id: i64,
    pub round_no: i16,
    pub hand_size: i16,
    pub dealer_pos: i16,
    pub trump: Option<Trump>,
    pub created_at: time::OffsetDateTime,
    pub completed_at: Option<time::OffsetDateTime>,
}

/// Trump selection for a round (domain type)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trump {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
    NoTrump,
}

// Free functions (generic) for round operations

/// Find a round by game_id and round_no
pub async fn find_by_game_and_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    round_no: i16,
) -> Result<Option<Round>, DomainError> {
    let round = rounds_adapter::find_by_game_and_round(conn, game_id, round_no).await?;
    Ok(round.map(Round::from))
}

/// Find a round by its ID
pub async fn find_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Option<Round>, DomainError> {
    let round = rounds_adapter::find_by_id(conn, round_id).await?;
    Ok(round.map(Round::from))
}

/// Create a new round
pub async fn create_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    round_no: i16,
    hand_size: i16,
    dealer_pos: i16,
) -> Result<Round, DomainError> {
    let dto = rounds_adapter::RoundCreate {
        game_id,
        round_no,
        hand_size,
        dealer_pos,
    };
    let round = rounds_adapter::create_round(conn, dto).await?;
    Ok(Round::from(round))
}

/// Update trump selection for a round
pub async fn update_trump<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
    trump: Trump,
) -> Result<Round, DomainError> {
    let dto = rounds_adapter::RoundUpdateTrump {
        round_id,
        trump: trump.into(),
    };
    let round = rounds_adapter::update_trump(conn, dto).await?;
    Ok(Round::from(round))
}

/// Mark a round as completed
pub async fn complete_round<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    round_id: i64,
) -> Result<Round, DomainError> {
    let round = rounds_adapter::complete_round(conn, round_id).await?;
    Ok(Round::from(round))
}

// Conversions between SeaORM models and domain models

impl From<game_rounds::Model> for Round {
    fn from(model: game_rounds::Model) -> Self {
        Self {
            id: model.id,
            game_id: model.game_id,
            round_no: model.round_no,
            hand_size: model.hand_size,
            dealer_pos: model.dealer_pos,
            trump: model.trump.map(Trump::from),
            created_at: model.created_at,
            completed_at: model.completed_at,
        }
    }
}

impl From<game_rounds::CardTrump> for Trump {
    fn from(ct: game_rounds::CardTrump) -> Self {
        match ct {
            game_rounds::CardTrump::Clubs => Trump::Clubs,
            game_rounds::CardTrump::Diamonds => Trump::Diamonds,
            game_rounds::CardTrump::Hearts => Trump::Hearts,
            game_rounds::CardTrump::Spades => Trump::Spades,
            game_rounds::CardTrump::NoTrump => Trump::NoTrump,
        }
    }
}

impl From<Trump> for game_rounds::CardTrump {
    fn from(t: Trump) -> Self {
        match t {
            Trump::Clubs => game_rounds::CardTrump::Clubs,
            Trump::Diamonds => game_rounds::CardTrump::Diamonds,
            Trump::Hearts => game_rounds::CardTrump::Hearts,
            Trump::Spades => game_rounds::CardTrump::Spades,
            Trump::NoTrump => game_rounds::CardTrump::NoTrump,
        }
    }
}
