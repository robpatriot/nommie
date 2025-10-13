use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::extension::postgres::Type as PgType;
use sea_orm_migration::sea_query::{ColumnDef, ForeignKeyAction, Index, Table};

#[derive(DeriveMigrationName)]
pub struct Migration;

// ----- Iden enums for tables & columns -----
#[derive(Iden)]
enum Users {
    Table,
    Id,
    Sub,
    Username,
    IsAi,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum UserCredentials {
    Table,
    Id,
    UserId,
    PasswordHash,
    Email,
    GoogleSub,
    LastLogin,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Games {
    Table,
    Id,
    CreatedBy,
    Visibility,
    State,
    CreatedAt,
    UpdatedAt,
    StartedAt,
    EndedAt,
    Name,
    JoinCode,
    RulesVersion,
    RngSeed,
    CurrentRound,
    StartingDealerPos,
    CurrentTrickNo,
    CurrentRoundId,
    LockVersion,
}

#[derive(Iden)]
enum GameStateEnum {
    #[iden = "game_state"]
    Type,
}

#[derive(Iden)]
enum GameVisibilityEnum {
    #[iden = "game_visibility"]
    Type,
}

#[derive(Iden)]
enum CardSuitEnum {
    #[iden = "card_suit"]
    Type,
}

#[derive(Iden)]
enum CardTrumpEnum {
    #[iden = "card_trump"]
    Type,
}

#[derive(Iden)]
enum GamePlayers {
    Table,
    Id,
    GameId,
    UserId,
    TurnOrder,
    IsReady,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum AiProfiles {
    Table,
    Id,
    UserId,
    Playstyle,
    Difficulty,
    Config,
    MemoryLevel,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum AiOverrides {
    Table,
    Id,
    GamePlayerId,
    Name,
    MemoryLevel,
    Config,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum GameRounds {
    Table,
    Id,
    GameId,
    RoundNo,
    HandSize,
    DealerPos,
    Trump,
    CreatedAt,
    CompletedAt,
}

#[derive(Iden)]
enum RoundHands {
    Table,
    Id,
    RoundId,
    PlayerSeat,
    Cards,
    CreatedAt,
}

#[derive(Iden)]
enum RoundBids {
    Table,
    Id,
    RoundId,
    PlayerSeat,
    BidValue,
    BidOrder,
    CreatedAt,
}

#[derive(Iden)]
enum RoundTricks {
    Table,
    Id,
    RoundId,
    TrickNo,
    LeadSuit,
    WinnerSeat,
    CreatedAt,
}

#[derive(Iden)]
enum TrickPlays {
    Table,
    Id,
    TrickId,
    PlayerSeat,
    Card,
    PlayOrder,
    PlayedAt,
}

#[derive(Iden)]
enum RoundScores {
    Table,
    Id,
    RoundId,
    PlayerSeat,
    BidValue,
    TricksWon,
    BidMet,
    BaseScore,
    Bonus,
    RoundScore,
    TotalScoreAfter,
    CreatedAt,
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // users
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(Users::Sub).string().not_null())
                    .col(ColumnDef::new(Users::Username).string().null())
                    .col(
                        ColumnDef::new(Users::IsAi)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Users::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index on users.sub
        manager
            .create_index(
                Index::create()
                    .name("idx_users_sub_unique")
                    .table(Users::Table)
                    .col(Users::Sub)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // user_credentials
        manager
            .create_table(
                Table::create()
                    .table(UserCredentials::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserCredentials::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(
                        ColumnDef::new(UserCredentials::UserId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserCredentials::PasswordHash)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UserCredentials::Email)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(UserCredentials::GoogleSub)
                            .string()
                            .null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(UserCredentials::LastLogin)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UserCredentials::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserCredentials::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_credentials_user_id")
                            .from(UserCredentials::Table, UserCredentials::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // unique index on user_credentials.user_id
        manager
            .create_index(
                Index::create()
                    .name("ux_user_credentials_user_id")
                    .table(UserCredentials::Table)
                    .col(UserCredentials::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create Postgres enums
        manager
            .create_type(
                PgType::create()
                    .as_enum(GameStateEnum::Type)
                    .values([
                        "LOBBY",
                        "DEALING",
                        "BIDDING",
                        "TRUMP_SELECTION",
                        "TRICK_PLAY",
                        "SCORING",
                        "BETWEEN_ROUNDS",
                        "COMPLETED",
                        "ABANDONED",
                    ])
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                PgType::create()
                    .as_enum(GameVisibilityEnum::Type)
                    .values(["PUBLIC", "PRIVATE"])
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                PgType::create()
                    .as_enum(CardSuitEnum::Type)
                    .values(["CLUBS", "DIAMONDS", "HEARTS", "SPADES"])
                    .to_owned(),
            )
            .await?;

        manager
            .create_type(
                PgType::create()
                    .as_enum(CardTrumpEnum::Type)
                    .values(["CLUBS", "DIAMONDS", "HEARTS", "SPADES", "NO_TRUMP"])
                    .to_owned(),
            )
            .await?;

        // games table
        manager
            .create_table(
                Table::create()
                    .table(Games::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Games::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(Games::CreatedBy).big_integer().null())
                    .col(
                        ColumnDef::new(Games::Visibility)
                            .custom(GameVisibilityEnum::Type)
                            .not_null()
                            .default("PRIVATE"),
                    )
                    .col(
                        ColumnDef::new(Games::State)
                            .custom(GameStateEnum::Type)
                            .not_null()
                            .default("LOBBY"),
                    )
                    .col(
                        ColumnDef::new(Games::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Games::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Games::StartedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Games::EndedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(Games::Name).text().null())
                    .col(
                        ColumnDef::new(Games::JoinCode)
                            .string_len(10)
                            .unique_key()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Games::RulesVersion)
                            .text()
                            .not_null()
                            .default("nommie-1.0.0"),
                    )
                    .col(ColumnDef::new(Games::RngSeed).big_integer().null())
                    .col(ColumnDef::new(Games::CurrentRound).small_integer().null())
                    .col(
                        ColumnDef::new(Games::StartingDealerPos)
                            .small_integer()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Games::CurrentTrickNo)
                            .small_integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(Games::CurrentRoundId).big_integer().null())
                    .col(
                        ColumnDef::new(Games::LockVersion)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_games_created_by")
                            .from(Games::Table, Games::CreatedBy)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for games table
        manager
            .create_index(
                Index::create()
                    .name("ix_games_created_by")
                    .table(Games::Table)
                    .col(Games::CreatedBy)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ix_games_state")
                    .table(Games::Table)
                    .col(Games::State)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ix_games_visibility")
                    .table(Games::Table)
                    .col(Games::Visibility)
                    .to_owned(),
            )
            .await?;

        // game_players
        manager
            .create_table(
                Table::create()
                    .table(GamePlayers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GamePlayers::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(GamePlayers::GameId).big_integer().not_null())
                    .col(ColumnDef::new(GamePlayers::UserId).big_integer().not_null())
                    .col(ColumnDef::new(GamePlayers::TurnOrder).integer().not_null())
                    .col(
                        ColumnDef::new(GamePlayers::IsReady)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(GamePlayers::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GamePlayers::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_players_user_id")
                            .from(GamePlayers::Table, GamePlayers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_players_game_id")
                            .from(GamePlayers::Table, GamePlayers::GameId)
                            .to(Games::Table, Games::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique constraints for game_players
        manager
            .create_index(
                Index::create()
                    .name("ux_game_players_game_user")
                    .table(GamePlayers::Table)
                    .col(GamePlayers::GameId)
                    .col(GamePlayers::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ux_game_players_game_turn")
                    .table(GamePlayers::Table)
                    .col(GamePlayers::GameId)
                    .col(GamePlayers::TurnOrder)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // ai_profiles
        manager
            .create_table(
                Table::create()
                    .table(AiProfiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AiProfiles::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(AiProfiles::UserId).big_integer().not_null())
                    .col(ColumnDef::new(AiProfiles::Playstyle).string().null())
                    .col(ColumnDef::new(AiProfiles::Difficulty).integer().null())
                    .col(ColumnDef::new(AiProfiles::Config).json_binary().null())
                    .col(ColumnDef::new(AiProfiles::MemoryLevel).integer().null())
                    .col(
                        ColumnDef::new(AiProfiles::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AiProfiles::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_ai_profiles_user_id")
                            .from(AiProfiles::Table, AiProfiles::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // ai_overrides - per-instance AI configuration overrides
        manager
            .create_table(
                Table::create()
                    .table(AiOverrides::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AiOverrides::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(
                        ColumnDef::new(AiOverrides::GamePlayerId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(AiOverrides::Name).string().null())
                    .col(ColumnDef::new(AiOverrides::MemoryLevel).integer().null())
                    .col(ColumnDef::new(AiOverrides::Config).json_binary().null())
                    .col(
                        ColumnDef::new(AiOverrides::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(AiOverrides::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_ai_overrides_game_player_id")
                            .from(AiOverrides::Table, AiOverrides::GamePlayerId)
                            .to(GamePlayers::Table, GamePlayers::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create unique index on ai_overrides.game_player_id
        manager
            .create_index(
                Index::create()
                    .name("ux_ai_overrides_game_player_id")
                    .table(AiOverrides::Table)
                    .col(AiOverrides::GamePlayerId)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // game_rounds table
        manager
            .create_table(
                Table::create()
                    .table(GameRounds::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GameRounds::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(GameRounds::GameId).big_integer().not_null())
                    .col(
                        ColumnDef::new(GameRounds::RoundNo)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameRounds::HandSize)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameRounds::DealerPos)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameRounds::Trump)
                            .custom(CardTrumpEnum::Type)
                            .null(),
                    )
                    .col(
                        ColumnDef::new(GameRounds::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(GameRounds::CompletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_rounds_game_id")
                            .from(GameRounds::Table, GameRounds::GameId)
                            .to(Games::Table, Games::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ix_game_rounds_game_id")
                    .table(GameRounds::Table)
                    .col(GameRounds::GameId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ux_game_rounds_game_round")
                    .table(GameRounds::Table)
                    .col(GameRounds::GameId)
                    .col(GameRounds::RoundNo)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // round_hands table
        manager
            .create_table(
                Table::create()
                    .table(RoundHands::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RoundHands::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(RoundHands::RoundId).big_integer().not_null())
                    .col(
                        ColumnDef::new(RoundHands::PlayerSeat)
                            .small_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RoundHands::Cards).json_binary().not_null())
                    .col(
                        ColumnDef::new(RoundHands::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_round_hands_round_id")
                            .from(RoundHands::Table, RoundHands::RoundId)
                            .to(GameRounds::Table, GameRounds::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ix_round_hands_round_id")
                    .table(RoundHands::Table)
                    .col(RoundHands::RoundId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ux_round_hands_round_seat")
                    .table(RoundHands::Table)
                    .col(RoundHands::RoundId)
                    .col(RoundHands::PlayerSeat)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // round_bids table
        manager
            .create_table(
                Table::create()
                    .table(RoundBids::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RoundBids::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(RoundBids::RoundId).big_integer().not_null())
                    .col(
                        ColumnDef::new(RoundBids::PlayerSeat)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundBids::BidValue)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundBids::BidOrder)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundBids::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_round_bids_round_id")
                            .from(RoundBids::Table, RoundBids::RoundId)
                            .to(GameRounds::Table, GameRounds::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ix_round_bids_round_id")
                    .table(RoundBids::Table)
                    .col(RoundBids::RoundId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ux_round_bids_round_seat")
                    .table(RoundBids::Table)
                    .col(RoundBids::RoundId)
                    .col(RoundBids::PlayerSeat)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ux_round_bids_round_order")
                    .table(RoundBids::Table)
                    .col(RoundBids::RoundId)
                    .col(RoundBids::BidOrder)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // round_tricks table
        manager
            .create_table(
                Table::create()
                    .table(RoundTricks::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RoundTricks::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(
                        ColumnDef::new(RoundTricks::RoundId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundTricks::TrickNo)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundTricks::LeadSuit)
                            .custom(CardSuitEnum::Type)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundTricks::WinnerSeat)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundTricks::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_round_tricks_round_id")
                            .from(RoundTricks::Table, RoundTricks::RoundId)
                            .to(GameRounds::Table, GameRounds::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ix_round_tricks_round_id")
                    .table(RoundTricks::Table)
                    .col(RoundTricks::RoundId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ux_round_tricks_round_trick")
                    .table(RoundTricks::Table)
                    .col(RoundTricks::RoundId)
                    .col(RoundTricks::TrickNo)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // trick_plays table
        manager
            .create_table(
                Table::create()
                    .table(TrickPlays::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(TrickPlays::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(ColumnDef::new(TrickPlays::TrickId).big_integer().not_null())
                    .col(
                        ColumnDef::new(TrickPlays::PlayerSeat)
                            .small_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(TrickPlays::Card).json_binary().not_null())
                    .col(
                        ColumnDef::new(TrickPlays::PlayOrder)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(TrickPlays::PlayedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_trick_plays_trick_id")
                            .from(TrickPlays::Table, TrickPlays::TrickId)
                            .to(RoundTricks::Table, RoundTricks::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ix_trick_plays_trick_id")
                    .table(TrickPlays::Table)
                    .col(TrickPlays::TrickId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ix_trick_plays_played_at")
                    .table(TrickPlays::Table)
                    .col(TrickPlays::PlayedAt)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ux_trick_plays_trick_seat")
                    .table(TrickPlays::Table)
                    .col(TrickPlays::TrickId)
                    .col(TrickPlays::PlayerSeat)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ux_trick_plays_trick_order")
                    .table(TrickPlays::Table)
                    .col(TrickPlays::TrickId)
                    .col(TrickPlays::PlayOrder)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // round_scores table
        manager
            .create_table(
                Table::create()
                    .table(RoundScores::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(RoundScores::Id)
                            .big_integer()
                            .not_null()
                            .primary_key()
                            .auto_increment(),
                    )
                    .col(
                        ColumnDef::new(RoundScores::RoundId)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundScores::PlayerSeat)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundScores::BidValue)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundScores::TricksWon)
                            .small_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(RoundScores::BidMet).boolean().not_null())
                    .col(
                        ColumnDef::new(RoundScores::BaseScore)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundScores::Bonus)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundScores::RoundScore)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundScores::TotalScoreAfter)
                            .small_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(RoundScores::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_round_scores_round_id")
                            .from(RoundScores::Table, RoundScores::RoundId)
                            .to(GameRounds::Table, GameRounds::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ix_round_scores_round_id")
                    .table(RoundScores::Table)
                    .col(RoundScores::RoundId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ix_round_scores_total")
                    .table(RoundScores::Table)
                    .col(RoundScores::TotalScoreAfter)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("ux_round_scores_round_seat")
                    .table(RoundScores::Table)
                    .col(RoundScores::RoundId)
                    .col(RoundScores::PlayerSeat)
                    .unique()
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // drop in reverse order + drop index before table

        // Drop round_scores
        manager
            .drop_index(
                Index::drop()
                    .name("ux_round_scores_round_seat")
                    .table(RoundScores::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ix_round_scores_total")
                    .table(RoundScores::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ix_round_scores_round_id")
                    .table(RoundScores::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(RoundScores::Table).to_owned())
            .await?;

        // Drop trick_plays
        manager
            .drop_index(
                Index::drop()
                    .name("ux_trick_plays_trick_order")
                    .table(TrickPlays::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ux_trick_plays_trick_seat")
                    .table(TrickPlays::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ix_trick_plays_played_at")
                    .table(TrickPlays::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ix_trick_plays_trick_id")
                    .table(TrickPlays::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(TrickPlays::Table).to_owned())
            .await?;

        // Drop round_tricks
        manager
            .drop_index(
                Index::drop()
                    .name("ux_round_tricks_round_trick")
                    .table(RoundTricks::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ix_round_tricks_round_id")
                    .table(RoundTricks::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(RoundTricks::Table).to_owned())
            .await?;

        // Drop round_bids
        manager
            .drop_index(
                Index::drop()
                    .name("ux_round_bids_round_order")
                    .table(RoundBids::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ux_round_bids_round_seat")
                    .table(RoundBids::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ix_round_bids_round_id")
                    .table(RoundBids::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(RoundBids::Table).to_owned())
            .await?;

        // Drop round_hands
        manager
            .drop_index(
                Index::drop()
                    .name("ux_round_hands_round_seat")
                    .table(RoundHands::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ix_round_hands_round_id")
                    .table(RoundHands::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(RoundHands::Table).to_owned())
            .await?;

        // Drop game_rounds
        manager
            .drop_index(
                Index::drop()
                    .name("ux_game_rounds_game_round")
                    .table(GameRounds::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ix_game_rounds_game_id")
                    .table(GameRounds::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(GameRounds::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AiProfiles::Table).to_owned())
            .await?;

        // Drop game_players unique constraints and table
        manager
            .drop_index(
                Index::drop()
                    .name("ux_game_players_game_turn")
                    .table(GamePlayers::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ux_game_players_game_user")
                    .table(GamePlayers::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(GamePlayers::Table).to_owned())
            .await?;

        // Drop games indexes and table
        manager
            .drop_index(
                Index::drop()
                    .name("ix_games_visibility")
                    .table(Games::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ix_games_state")
                    .table(Games::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ix_games_created_by")
                    .table(Games::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Games::Table).to_owned())
            .await?;

        // Drop enum types
        manager
            .drop_type(PgType::drop().name(CardTrumpEnum::Type).to_owned())
            .await?;

        manager
            .drop_type(PgType::drop().name(CardSuitEnum::Type).to_owned())
            .await?;

        manager
            .drop_type(PgType::drop().name(GameVisibilityEnum::Type).to_owned())
            .await?;

        manager
            .drop_type(PgType::drop().name(GameStateEnum::Type).to_owned())
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("ux_user_credentials_user_id")
                    .table(UserCredentials::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(UserCredentials::Table).to_owned())
            .await?;

        // Drop users.sub unique index before dropping users table
        manager
            .drop_index(
                Index::drop()
                    .name("idx_users_sub_unique")
                    .table(Users::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;

        Ok(())
    }
}
