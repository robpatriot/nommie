//! Player repository functions for domain layer (generic over ConnectionTrait).

use sea_orm::{ConnectionTrait, EntityTrait};

use crate::adapters::players_sea as players_adapter;
use crate::entities::game_players::PlayerKind;
use crate::entities::{ai_profiles, users};
use crate::errors::domain::{DomainError, NotFoundKind};
use crate::routes::games::friendly_ai_name;

// Free functions (generic) mirroring the previous trait methods

/// Get the display name of a player in a game by seat.
///
/// # Arguments
/// * `conn` - Database connection
/// * `game_id` - The game identifier
/// * `seat` - The seat number (0-3)
///
/// # Returns
/// * `Ok(String)` - The player's display name
/// * `Err(DomainError)` - If player not found or DB error
pub async fn get_display_name_by_seat<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    game_id: i64,
    seat: u8,
) -> Result<String, DomainError> {
    let game_player = players_adapter::get_player_by_seat(conn, game_id, seat).await?;

    let Some(game_player) = game_player else {
        return Err(DomainError::not_found(
            NotFoundKind::Player,
            "Player not found at seat",
        ));
    };

    match game_player.player_kind {
        PlayerKind::Human => {
            let user_id = game_player.human_user_id.ok_or_else(|| {
                DomainError::not_found(NotFoundKind::Player, "Human player missing user reference")
            })?;

            let user = users::Entity::find_by_id(user_id)
                .one(conn)
                .await
                .map_err(DomainError::from)?
                .ok_or_else(|| {
                    DomainError::not_found(
                        NotFoundKind::Player,
                        format!("User {} not found", user_id),
                    )
                })?;

            if user.is_ai {
                return Ok(friendly_ai_name(user.id, seat as usize));
            }

            let display_name = user.username.unwrap_or_else(|| user.sub.clone());
            Ok(display_name)
        }
        PlayerKind::Ai => {
            if let Some(profile_id) = game_player.ai_profile_id {
                if let Some(profile) = ai_profiles::Entity::find_by_id(profile_id)
                    .one(conn)
                    .await
                    .map_err(DomainError::from)?
                {
                    let trimmed = profile.display_name.trim();
                    if !trimmed.is_empty() {
                        return Ok(trimmed.to_owned());
                    }
                }
                return Ok(friendly_ai_name(profile_id, seat as usize));
            }

            Ok(friendly_ai_name(game_player.id, seat as usize))
        }
    }
}
