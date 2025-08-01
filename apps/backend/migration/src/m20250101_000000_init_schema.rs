use sea_orm_migration::prelude::*;
use sea_orm::Statement;
use uuid::Uuid;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create users table
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Users::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Users::ExternalId).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(Users::Name).string().null())
                    .col(ColumnDef::new(Users::IsAi).boolean().not_null().default(false))
                    .col(ColumnDef::new(Users::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Users::UpdatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;

        // Insert 3 AI users for development
        let now = chrono::Utc::now();
        let ai_users = vec![
            (
                "ai_user_1",
                "__ai+1@nommie.dev",
                "ChessMaster Bot",
                now,
                now,
            ),
            (
                "ai_user_2", 
                "__ai+2@nommie.dev",
                "Strategy Sage",
                now,
                now,
            ),
            (
                "ai_user_3",
                "__ai+3@nommie.dev", 
                "Tactical Turtle",
                now,
                now,
            ),
        ];

        for (external_id, email, name, created_at, updated_at) in ai_users {
            manager
                .get_connection()
                .execute(
                    Statement::from_sql_and_values(
                        manager.get_database_backend(),
                        r#"INSERT INTO users (id, external_id, email, name, is_ai, created_at, updated_at) 
                           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
                        vec![
                            Uuid::new_v4().into(),
                            external_id.into(),
                            email.into(),
                            name.into(),
                            true.into(),
                            created_at.into(),
                            updated_at.into(),
                        ],
                    ),
                )
                .await?;
        }

        // Create games table
        manager
            .create_table(
                Table::create()
                    .table(Games::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Games::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Games::State).string_len(20).not_null())
                    .col(ColumnDef::new(Games::Phase).string_len(20).not_null().default("bidding"))
                    .col(ColumnDef::new(Games::CurrentTurn).integer().null())
                    .col(ColumnDef::new(Games::CreatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Games::UpdatedAt).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(Games::StartedAt).timestamp_with_time_zone().null())
                    .to_owned(),
            )
            .await?;

        // Create game_players table
        manager
            .create_table(
                Table::create()
                    .table(GamePlayers::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(GamePlayers::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(GamePlayers::GameId).uuid().not_null())
                    .col(ColumnDef::new(GamePlayers::UserId).uuid().not_null())
                    .col(ColumnDef::new(GamePlayers::TurnOrder).integer().null())
                    .col(ColumnDef::new(GamePlayers::IsReady).boolean().not_null().default(false))
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_players_game_id")
                            .from(GamePlayers::Table, GamePlayers::GameId)
                            .to(Games::Table, Games::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_players_user_id")
                            .from(GamePlayers::Table, GamePlayers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        // Create game_rounds table
        manager
            .create_table(
                Table::create()
                    .table(GameRounds::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(GameRounds::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(GameRounds::GameId).uuid().not_null())
                    .col(ColumnDef::new(GameRounds::RoundNumber).integer().not_null())
                    .col(ColumnDef::new(GameRounds::DealerPlayerId).uuid().null())
                    .col(ColumnDef::new(GameRounds::TrumpSuit).string_len(10).null())
                    .col(ColumnDef::new(GameRounds::CreatedAt).timestamp_with_time_zone().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_rounds_game_id")
                            .from(GameRounds::Table, GameRounds::GameId)
                            .to(Games::Table, Games::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_rounds_dealer_player_id")
                            .from(GameRounds::Table, GameRounds::DealerPlayerId)
                            .to(GamePlayers::Table, GamePlayers::Id)
                            .on_delete(ForeignKeyAction::SetNull)
                    )
                    .to_owned(),
            )
            .await?;

        // Create round_bids table
        manager
            .create_table(
                Table::create()
                    .table(RoundBids::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(RoundBids::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(RoundBids::RoundId).uuid().not_null())
                    .col(ColumnDef::new(RoundBids::PlayerId).uuid().not_null())
                    .col(ColumnDef::new(RoundBids::Bid).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_round_bids_round_id")
                            .from(RoundBids::Table, RoundBids::RoundId)
                            .to(GameRounds::Table, GameRounds::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_round_bids_player_id")
                            .from(RoundBids::Table, RoundBids::PlayerId)
                            .to(GamePlayers::Table, GamePlayers::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order due to foreign key constraints
        manager
            .drop_table(Table::drop().table(RoundBids::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(GameRounds::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(GamePlayers::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Games::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    ExternalId,
    Email,
    Name,
    IsAi,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Games {
    Table,
    Id,
    State,
    Phase,
    CurrentTurn,
    CreatedAt,
    UpdatedAt,
    StartedAt,
}

#[derive(DeriveIden)]
enum GamePlayers {
    Table,
    Id,
    GameId,
    UserId,
    TurnOrder,
    IsReady,
}

#[derive(DeriveIden)]
enum GameRounds {
    Table,
    Id,
    GameId,
    RoundNumber,
    DealerPlayerId,
    TrumpSuit,
    CreatedAt,
}

#[derive(DeriveIden)]
enum RoundBids {
    Table,
    Id,
    RoundId,
    PlayerId,
    Bid,
} 