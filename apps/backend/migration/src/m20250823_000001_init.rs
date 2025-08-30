use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_query::{ColumnDef, ForeignKeyAction};

#[derive(DeriveMigrationName)]
pub struct Migration;

// ----- Iden enums for tables & columns -----
#[derive(Iden)]
enum Users {
    Table,
    Id,
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
enum GamePlayers {
    Table,
    Id,
    GameId,
    UserId,
    TurnOrder,
    IsReady,
    CreatedAt,
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
                    .col(ColumnDef::new(Users::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Users::Username).string().null())
                    .col(
                        ColumnDef::new(Users::IsAi)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Users::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Users::UpdatedAt).timestamp().not_null())
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
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserCredentials::UserId).uuid().not_null())
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
                            .timestamp()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(UserCredentials::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserCredentials::UpdatedAt)
                            .timestamp()
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

        // game_players
        manager
            .create_table(
                Table::create()
                    .table(GamePlayers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GamePlayers::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(GamePlayers::GameId).uuid().not_null())
                    .col(ColumnDef::new(GamePlayers::UserId).uuid().not_null())
                    .col(ColumnDef::new(GamePlayers::TurnOrder).integer().not_null())
                    .col(
                        ColumnDef::new(GamePlayers::IsReady)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(GamePlayers::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_players_user_id")
                            .from(GamePlayers::Table, GamePlayers::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
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
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AiProfiles::UserId).uuid().not_null())
                    .col(ColumnDef::new(AiProfiles::Playstyle).string().null())
                    .col(ColumnDef::new(AiProfiles::Difficulty).integer().null())
                    .col(ColumnDef::new(AiProfiles::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(AiProfiles::UpdatedAt).timestamp().not_null())
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

        manager
            .drop_table(Table::drop().table(GamePlayers::Table).to_owned())
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

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;

        Ok(())
    }
}
