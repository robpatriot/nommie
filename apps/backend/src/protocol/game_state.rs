use serde::{Deserialize, Serialize};

use crate::domain::snapshot::GameSnapshot;
use crate::domain::state::Seat;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameSnapshotResponse {
    pub(crate) snapshot: GameSnapshot,
    pub(crate) viewer: ViewerState,
    pub(crate) version: i32,
}

/// Viewer-relative context for a specific game snapshot.
///
/// This is intentionally game-scoped: it only makes sense in the context of a game room.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seat: Option<Seat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hand: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid_constraints: Option<BidConstraintsResponse>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BidConstraintsResponse {
    pub zero_bid_locked: bool,
}
