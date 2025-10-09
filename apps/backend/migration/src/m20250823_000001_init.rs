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
    HandSize,
    DealerPos,
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
    CreatedAt,
    UpdatedAt,
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
                    .col(ColumnDef::new(Games::HandSize).small_integer().null())
                    .col(ColumnDef::new(Games::DealerPos).small_integer().null())
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

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // drop in reverse order + drop index before table
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
