use sea_orm_migration::prelude::*; // Only migration prelude
use sea_orm_migration::sea_query::{ColumnDef, ForeignKeyAction}; // Explicit imports

use uuid::Uuid;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create Users Table
        manager
            .create_table(
                Table::create()
                    .table("users") // Table name directly as string
                    .if_not_exists()
                    .col(ColumnDef::new("id").uuid().not_null().primary_key())
                    .col(ColumnDef::new("username").string().not_null())
                    .col(ColumnDef::new("is_ai").boolean().not_null().default(false))
                    .col(ColumnDef::new("created_at").timestamp().not_null())
                    .col(ColumnDef::new("updated_at").timestamp().not_null())
                    .to_owned(),
            )
            .await?;

        // Create UserCredentials Table
        manager
            .create_table(
                Table::create()
                    .table("user_credentials") // Table name directly as string
                    .if_not_exists()
                    .col(ColumnDef::new("id").uuid().not_null().primary_key())
                    .col(ColumnDef::new("user_id").uuid().not_null())
                    .col(ColumnDef::new("password_hash").string().not_null())
                    .col(ColumnDef::new("email").string().not_null().unique_key()) // Corrected method
                    .col(ColumnDef::new("last_login").timestamp().null())
                    .col(ColumnDef::new("created_at").timestamp().not_null())
                    .col(ColumnDef::new("updated_at").timestamp().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_user_credentials_user_id")
                            .from("user_credentials", "user_id")
                            .to("users", "id")
                            .on_delete(ForeignKeyAction::Cascade), // Disambiguated ForeignKeyAction
                    )
                    .to_owned(),
            )
            .await?;

        // Create GamePlayers Table
        manager
            .create_table(
                Table::create()
                    .table("game_players") // Table name directly as string
                    .if_not_exists()
                    .col(ColumnDef::new("id").uuid().not_null().primary_key())
                    .col(ColumnDef::new("game_id").uuid().not_null())
                    .col(ColumnDef::new("user_id").uuid().not_null())
                    .col(ColumnDef::new("turn_order").integer().not_null())
                    .col(ColumnDef::new("is_ready").boolean().default(false))
                    .col(ColumnDef::new("created_at").timestamp().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_game_players_user_id")
                            .from("game_players", "user_id")
                            .to("users", "id")
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create AIProfiles Table
        manager
            .create_table(
                Table::create()
                    .table("ai_profiles") // Table name directly as string
                    .if_not_exists()
                    .col(ColumnDef::new("id").uuid().not_null().primary_key())
                    .col(ColumnDef::new("user_id").uuid().not_null())
                    .col(ColumnDef::new("playstyle").string().null())
                    .col(ColumnDef::new("difficulty").integer().null())
                    .col(ColumnDef::new("created_at").timestamp().not_null())
                    .col(ColumnDef::new("updated_at").timestamp().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_ai_profiles_user_id")
                            .from("ai_profiles", "user_id")
                            .to("users", "id")
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Additional tables would follow a similar pattern...

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order (not using enums, just direct strings)
        manager
            .drop_table(Table::drop().table("ai_profiles").to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table("game_players").to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table("user_credentials").to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table("users").to_owned())
            .await?;

        Ok(())
    }
}
