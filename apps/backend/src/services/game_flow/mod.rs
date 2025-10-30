//! Game flow orchestration service - bridges pure domain logic with DB persistence.
//!
//! This service provides fine-grained transition methods for game state progression
//! and a test/bot helper that composes them into a happy path.

mod ai_coordinator;
mod orchestration;
mod player_actions;
mod round_lifecycle;

/// Game flow service - generic over ConnectionTrait for transaction support.
#[derive(Default)]
pub struct GameFlowService;
