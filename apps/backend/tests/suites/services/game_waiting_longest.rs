// Tests for GameService::game_waiting_longest()
//
// This module tests the complex prioritization logic for determining which games
// are waiting for a user's action, including:
// - Prioritizing games with human players over AI-only games
// - Ordering by oldest waiting_since timestamp within each category
// - Returning up to 5 game IDs for client-side navigation

use backend::db::txn::with_txn;
use backend::entities::games::{self, GameState};
use backend::services::games::GameService;
use backend::AppError;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, Set};
use time::OffsetDateTime;

use crate::support::build_test_state;
use crate::support::factory::create_test_user;
use crate::support::game_phases::{
    setup_game_in_bidding_phase, setup_game_in_bidding_phase_with_seats, SeatSpec,
};
use crate::support::test_utils::test_user_sub;

/// Helper: Create a game in a specific state with controlled updated_at timestamp
async fn create_game_with_state_and_time(
    txn: &sea_orm::DatabaseTransaction,
    test_name: &str,
    state: GameState,
    updated_at: OffsetDateTime,
) -> Result<i64, AppError> {
    let user_sub = test_user_sub(&format!("{}_creator", test_name));
    let user_id = create_test_user(txn, &user_sub, Some("Creator")).await?;

    let now = OffsetDateTime::now_utc();
    let game = games::ActiveModel {
        id: sea_orm::NotSet,
        created_by: Set(Some(user_id)),
        visibility: Set(games::GameVisibility::Public),
        state: Set(state),
        created_at: Set(now),
        updated_at: Set(updated_at),
        waiting_since: Set(None),
        started_at: Set(None),
        ended_at: Set(None),
        name: Set(Some(format!("Test Game {}", test_name))),
        rules_version: Set("1.0".to_string()),
        rng_seed: Set(crate::support::test_utils::test_seed(test_name).to_vec()),
        current_round: Set(None),
        starting_dealer_pos: Set(None),
        current_trick_no: Set(0i16),
        current_round_id: Set(None),
        version: Set(1),
    };

    let inserted = game.insert(txn).await?;
    Ok(inserted.id)
}

/// Helper: Small sleep to ensure distinct timestamps
async fn sleep_ms(ms: u64) {
    tokio::time::sleep(tokio::time::Duration::from_millis(ms)).await;
}

/// Helper: Update game's waiting_since timestamp
async fn update_game_waiting_since(
    txn: &sea_orm::DatabaseTransaction,
    game_id: i64,
    waiting_since: OffsetDateTime,
) -> Result<(), AppError> {
    let game = games::Entity::find_by_id(game_id)
        .one(txn)
        .await?
        .ok_or_else(|| {
            AppError::internal(
                backend::ErrorCode::InternalError,
                "Game not found".to_string(),
                std::io::Error::new(std::io::ErrorKind::NotFound, "Game not found"),
            )
        })?;

    let mut active_game = game.into_active_model();
    active_game.waiting_since = Set(Some(waiting_since));
    active_game.update(txn).await?;
    Ok(())
}

#[tokio::test]
async fn returns_empty_when_user_has_no_games() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create a user with no game memberships
            let user_id =
                create_test_user(txn, &test_user_sub("no_games_user"), Some("No Games User"))
                    .await?;

            let service = GameService;
            let result = service.game_waiting_longest(txn, user_id).await?;

            assert_eq!(result, Vec::<i64>::new());
            Ok(())
        })
    })
    .await
}

#[tokio::test]
async fn returns_empty_when_all_games_are_non_actionable() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let user_id =
                create_test_user(txn, &test_user_sub("non_actionable_user"), Some("User")).await?;

            let now = OffsetDateTime::now_utc();

            // Create games in non-actionable states
            let completed_game =
                create_game_with_state_and_time(txn, "completed_game", GameState::Completed, now)
                    .await?;
            let abandoned_game =
                create_game_with_state_and_time(txn, "abandoned_game", GameState::Abandoned, now)
                    .await?;

            // Add user as player to both games
            crate::support::db_memberships::create_test_game_player(
                txn,
                completed_game,
                user_id,
                0,
            )
            .await?;
            crate::support::db_memberships::create_test_game_player(
                txn,
                abandoned_game,
                user_id,
                1,
            )
            .await?;

            let service = GameService;
            let result = service.game_waiting_longest(txn, user_id).await?;

            // Should return empty - no actionable games
            assert_eq!(result, Vec::<i64>::new());
            Ok(())
        })
    })
    .await
}

#[tokio::test]
async fn returns_single_game_when_users_turn() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create a game in bidding phase where it's the user's turn
            let setup = setup_game_in_bidding_phase(txn, "users_turn_test").await?;
            let _user_id = setup.user_ids[0]; // First player (seat 0)

            // In bidding, first to bid is (dealer + 1) % 4
            // If dealer is seat 0, first bidder is seat 1
            // We need to make it user's turn, so let's check who should bid first
            let game = backend::repos::games::require_game(txn, setup.game_id).await?;
            let dealer = game.dealer_pos().expect("Dealer should be set");
            let first_bidder = ((dealer + 1) % 4) as usize;

            // Use the user at the first bidder position
            let user_id = setup.user_ids[first_bidder];

            let service = GameService;
            let result = service.game_waiting_longest(txn, user_id).await?;

            assert_eq!(result, vec![setup.game_id]);
            Ok(())
        })
    })
    .await
}

#[tokio::test]
async fn returns_empty_when_not_users_turn() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            // Create a game in bidding phase
            let setup = setup_game_in_bidding_phase(txn, "not_users_turn_test").await?;

            // Determine who should bid first
            let game = backend::repos::games::require_game(txn, setup.game_id).await?;
            let dealer = game.dealer_pos().expect("Dealer should be set");
            let first_bidder = ((dealer + 1) % 4) as usize;

            // Pick a user who is NOT the first bidder
            let user_id = setup.user_ids[(first_bidder + 1) % 4];

            let service = GameService;
            let result = service.game_waiting_longest(txn, user_id).await?;

            assert_eq!(result, Vec::<i64>::new());
            Ok(())
        })
    })
    .await
}

#[tokio::test]
async fn prioritizes_oldest_waiting_game() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let now = OffsetDateTime::now_utc();

            // Create two games in bidding phase with different timestamps
            let older_setup = setup_game_in_bidding_phase(txn, "older_game").await?;
            sleep_ms(20).await;
            let newer_setup = setup_game_in_bidding_phase(txn, "newer_game").await?;

            // Set explicit timestamps to ensure ordering
            let older_time = now - time::Duration::seconds(60);
            let newer_time = now - time::Duration::seconds(30);

            update_game_waiting_since(txn, older_setup.game_id, older_time).await?;
            update_game_waiting_since(txn, newer_setup.game_id, newer_time).await?;

            // Determine first bidder for both games and use same user
            let older_game = backend::repos::games::require_game(txn, older_setup.game_id).await?;
            let older_dealer = older_game.dealer_pos().expect("Dealer should be set");
            let older_first_bidder = ((older_dealer + 1) % 4) as usize;

            let newer_game = backend::repos::games::require_game(txn, newer_setup.game_id).await?;
            let newer_dealer = newer_game.dealer_pos().expect("Dealer should be set");
            let newer_first_bidder = ((newer_dealer + 1) % 4) as usize;

            // Create a shared user and add them to both games at the first bidder position
            let user_id =
                create_test_user(txn, &test_user_sub("shared_user"), Some("Shared User")).await?;

            // Replace the first bidder in both games with our shared user
            crate::support::db_memberships::attach_human_to_seat(
                txn,
                older_setup.game_id,
                older_first_bidder as u8,
                user_id,
            )
            .await
            .map_err(|e| {
                AppError::internal(
                    backend::ErrorCode::InternalError,
                    format!("Failed to attach user: {e}"),
                    std::io::Error::other(e.to_string()),
                )
            })?;

            crate::support::db_memberships::attach_human_to_seat(
                txn,
                newer_setup.game_id,
                newer_first_bidder as u8,
                user_id,
            )
            .await
            .map_err(|e| {
                AppError::internal(
                    backend::ErrorCode::InternalError,
                    format!("Failed to attach user: {e}"),
                    std::io::Error::other(e.to_string()),
                )
            })?;

            let service = GameService;
            let result = service.game_waiting_longest(txn, user_id).await?;

            // Should return older game first
            assert_eq!(result, vec![older_setup.game_id, newer_setup.game_id]);
            Ok(())
        })
    })
    .await
}

#[tokio::test]
async fn prioritizes_games_with_humans_over_ai_only() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let now = OffsetDateTime::now_utc();

            // Create shared user who will be the "viewer" in both games.
            // In round 1, dealer is deterministic (seat 0), so the first bidder is seat 1.
            let user_id =
                create_test_user(txn, &test_user_sub("priority_user"), Some("User")).await?;

            // Create a second human to ensure this game qualifies as "has other humans".
            let other_user_id =
                create_test_user(txn, &test_user_sub("other_human"), Some("Other Human")).await?;

            // Create game with humans (newer): viewer at seat 1, plus another human (seat 2).
            let human_setup = setup_game_in_bidding_phase_with_seats(
                txn,
                "human_game",
                [
                    SeatSpec::Ai,
                    SeatSpec::ExistingHuman { user_id },
                    SeatSpec::ExistingHuman {
                        user_id: other_user_id,
                    },
                    SeatSpec::Ai,
                ],
            )
            .await?;
            let human_time = now - time::Duration::seconds(60);
            update_game_waiting_since(txn, human_setup.game_id, human_time).await?;

            sleep_ms(20).await;

            // Create AI-only game (older): viewer at seat 1, remaining seats are AI.
            let ai_only_setup = setup_game_in_bidding_phase_with_seats(
                txn,
                "ai_only_game",
                [
                    SeatSpec::Ai,
                    SeatSpec::ExistingHuman { user_id },
                    SeatSpec::Ai,
                    SeatSpec::Ai,
                ],
            )
            .await?;
            let ai_only_time = now - time::Duration::seconds(120);
            update_game_waiting_since(txn, ai_only_setup.game_id, ai_only_time).await?;

            let service = GameService;
            let result = service.game_waiting_longest(txn, user_id).await?;

            // Should return human game first despite being newer
            assert_eq!(result, vec![human_setup.game_id, ai_only_setup.game_id]);
            Ok(())
        })
    })
    .await
}

#[tokio::test]
async fn returns_maximum_five_games() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let now = OffsetDateTime::now_utc();
            let mut game_ids = Vec::new();

            // Create 5 games in bidding phase
            for i in 0..5 {
                let setup =
                    setup_game_in_bidding_phase(txn, &format!("max_five_game_{}", i)).await?;
                let timestamp = now - time::Duration::seconds(100 - (i * 10));
                update_game_waiting_since(txn, setup.game_id, timestamp).await?;
                game_ids.push((setup.game_id, setup));
                sleep_ms(10).await;
            }

            // Create shared user and attach to all games at first bidder position
            let user_id =
                create_test_user(txn, &test_user_sub("max_five_user"), Some("Max Five User"))
                    .await?;

            for (game_id, _setup) in &game_ids {
                let game = backend::repos::games::require_game(txn, *game_id).await?;
                let dealer = game.dealer_pos().expect("Dealer should be set");
                let first_bidder = ((dealer + 1) % 4) as usize;

                crate::support::db_memberships::attach_human_to_seat(
                    txn,
                    *game_id,
                    first_bidder as u8,
                    user_id,
                )
                .await
                .map_err(|e| {
                    AppError::internal(
                        backend::ErrorCode::InternalError,
                        format!("Failed to attach user: {e}"),
                        std::io::Error::other(e.to_string()),
                    )
                })?;
            }

            let service = GameService;
            let result = service.game_waiting_longest(txn, user_id).await?;

            // Should return exactly 5 games (oldest waiting_since first)
            assert_eq!(result.len(), 5);
            assert_eq!(result[0], game_ids[0].0); // Oldest
            assert_eq!(result[1], game_ids[1].0);
            assert_eq!(result[2], game_ids[2].0);
            assert_eq!(result[3], game_ids[3].0);
            assert_eq!(result[4], game_ids[4].0); // Newest among the five
            Ok(())
        })
    })
    .await
}

#[tokio::test]
async fn orders_by_game_id_when_waiting_since_equal() -> Result<(), AppError> {
    let state = build_test_state().await?;

    with_txn(None, &state, |txn| {
        Box::pin(async move {
            let now = OffsetDateTime::now_utc();

            // Create two games and force identical waiting_since.
            let a_setup = setup_game_in_bidding_phase(txn, "tie_break_a").await?;
            let b_setup = setup_game_in_bidding_phase(txn, "tie_break_b").await?;

            update_game_waiting_since(txn, a_setup.game_id, now).await?;
            update_game_waiting_since(txn, b_setup.game_id, now).await?;

            let user_id = create_test_user(
                txn,
                &test_user_sub("tie_break_user"),
                Some("Tie Break User"),
            )
            .await?;

            for game_id in [a_setup.game_id, b_setup.game_id] {
                let game = backend::repos::games::require_game(txn, game_id).await?;
                let dealer = game.dealer_pos().expect("Dealer should be set");
                let first_bidder = ((dealer + 1) % 4) as usize;

                crate::support::db_memberships::attach_human_to_seat(
                    txn,
                    game_id,
                    first_bidder as u8,
                    user_id,
                )
                .await
                .map_err(|e| {
                    AppError::internal(
                        backend::ErrorCode::InternalError,
                        format!("Failed to attach user: {e}"),
                        std::io::Error::other(e.to_string()),
                    )
                })?;
            }

            let service = GameService;
            let result = service.game_waiting_longest(txn, user_id).await?;

            let (min_id, max_id) = if a_setup.game_id < b_setup.game_id {
                (a_setup.game_id, b_setup.game_id)
            } else {
                (b_setup.game_id, a_setup.game_id)
            };

            assert_eq!(result, vec![min_id, max_id]);
            Ok(())
        })
    })
    .await
}
