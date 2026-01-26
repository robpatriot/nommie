use std::future::Future;
use std::pin::Pin;

use sea_orm::DatabaseTransaction;

use crate::domain::game_transition::{derive_game_transitions, GameTransition};
use crate::repos::games;
use crate::services::game_flow::orchestration::load_turn_view;
use crate::services::game_flow::GameFlowService;
use crate::AppError;

#[derive(Debug)]
pub struct GameFlowMutationResult {
    pub final_game: crate::repos::games::Game,
    pub old_version: i32,
    pub transitions: Vec<GameTransition>,
}

impl GameFlowMutationResult {
    pub fn final_version(&self) -> i32 {
        self.final_game.version
    }
}

impl GameFlowService {
    pub async fn run_mutation<'a, F>(
        &'a self,
        txn: &'a DatabaseTransaction,
        game_id: i64,
        expected_version: i32,
        mutation: F,
    ) -> Result<GameFlowMutationResult, AppError>
    where
        F: FnOnce(
                &'a GameFlowService,
                &'a DatabaseTransaction,
            ) -> Pin<Box<dyn Future<Output = Result<(), AppError>> + Send + 'a>>
            + 'a,
    {
        let before = load_turn_view(txn, game_id).await?;
        let old_version = before.version;

        if old_version != expected_version {
            return Err(AppError::conflict(
                crate::errors::ErrorCode::OptimisticLock,
                format!(
                    "Game lock version mismatch: expected {}, but game has version {}",
                    expected_version, old_version
                ),
            ));
        }

        // Execute mutation
        mutation(self, txn).await?;

        let after = load_turn_view(txn, game_id).await?;
        let transitions = derive_game_transitions(before.turn, after.turn);

        let final_game = games::require_game(txn, game_id).await?;

        Ok(GameFlowMutationResult {
            final_game,
            old_version,
            transitions,
        })
    }
}
