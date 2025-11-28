//! Player repository functions for domain layer (generic over ConnectionTrait).

use sea_orm::{ConnectionTrait, EntityTrait};

use crate::entities::game_players::PlayerKind;
use crate::entities::{ai_profiles, users};
use crate::errors::domain::{DomainError, NotFoundKind};
use crate::repos::ai_overrides;
use crate::repos::memberships::GameMembership;

/// Generate a friendly AI name from a seed and seat index.
///
/// Uses a deterministic algorithm to select from a predefined list of AI names.
pub fn friendly_ai_name(seed: i64, seat_index: usize) -> String {
    const AI_NAMES: [&str; 16] = [
        "Atlas", "Blaze", "Comet", "Dynamo", "Echo", "Flare", "Glyph", "Helix", "Ion", "Jet",
        "Kilo", "Lumen", "Nova", "Orion", "Pulse", "Quark",
    ];

    let idx = ((seed as usize) ^ seat_index) % AI_NAMES.len();
    AI_NAMES[idx].to_string()
}

/// Resolve display name for a membership with all fallback logic.
///
/// This is the consolidated display name resolution function that handles all cases:
/// 1. AI override name (if provided and non-empty)
/// 2. User username or sub (for humans)
/// 3. AI profile display_name (for AIs)
/// 4. Friendly AI name fallback (for AIs)
/// 5. Final fallback to "Player {seat+1}" (if `with_final_fallback` is true)
///
/// # Arguments
/// * `conn` - Database connection
/// * `membership` - The game membership
/// * `seat` - The seat number (0-3), used for fallback names
/// * `with_final_fallback` - If true, falls back to "Player {seat+1}" if no other name found
///
/// # Returns
/// * `Ok(String)` - The resolved display name
/// * `Err(DomainError)` - If database error occurs
pub async fn resolve_display_name_for_membership<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    membership: &GameMembership,
    seat: u8,
    with_final_fallback: bool,
) -> Result<String, DomainError> {
    // Priority 1: Check AI override name (if available)
    if let Some(override_record) = ai_overrides::find_by_game_player_id(conn, membership.id).await?
    {
        if let Some(name) = override_record.name {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_owned());
            }
        }
    }

    // Priority 2: Resolve based on player kind
    match membership.player_kind {
        PlayerKind::Human => {
            if let Some(user_id) = membership.user_id {
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

                // Check if user is actually an AI user
                if user.is_ai {
                    return Ok(friendly_ai_name(user.id, seat as usize));
                }

                // Use username if available, otherwise sub
                if let Some(username) = &user.username {
                    let trimmed = username.trim();
                    if !trimmed.is_empty() {
                        return Ok(trimmed.to_owned());
                    }
                }
                return Ok(user.sub.clone());
            }
        }
        PlayerKind::Ai => {
            if let Some(profile_id) = membership.ai_profile_id {
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
                // Fallback to friendly name based on profile_id
                return Ok(friendly_ai_name(profile_id, seat as usize));
            }
            // No profile_id - use friendly name based on membership.id
            return Ok(friendly_ai_name(membership.id, seat as usize));
        }
    }

    // Priority 3: Final fallback (if enabled) or error
    if with_final_fallback {
        return Ok(format!("Player {}", seat as usize + 1));
    }

    // No name could be resolved and fallback is disabled - this indicates invalid data
    Err(DomainError::not_found(
        NotFoundKind::Player,
        format!(
            "Could not resolve display name for player at seat {} (membership id: {})",
            seat, membership.id
        ),
    ))
}

// Free functions (generic) mirroring the previous trait methods

/// Get the display name of a player in a game by seat.
///
/// Uses the consolidated `resolve_display_name_for_membership` function internally.
/// Does not include the final "Player {seat+1}" fallback for backwards compatibility.
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
    use crate::repos::memberships;

    // Get all memberships and find the one at this seat
    let memberships = memberships::find_all_by_game(conn, game_id).await?;
    let membership = memberships
        .iter()
        .find(|m| m.turn_order == seat)
        .ok_or_else(|| {
            DomainError::not_found(
                NotFoundKind::Player,
                format!("Player not found at seat {}", seat),
            )
        })?;

    // Use consolidated function without final fallback (for backwards compatibility)
    resolve_display_name_for_membership(conn, membership, seat, false).await
}
