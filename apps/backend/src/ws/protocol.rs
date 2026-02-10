use serde::{Deserialize, Serialize};

use crate::domain::snapshot::GameSnapshot;
use crate::protocol::game_state::ViewerState;

pub const PROTOCOL_VERSION: i32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Topic {
    #[serde(rename_all = "snake_case")]
    Game { id: i64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMsg {
    Hello { protocol: i32 },
    Subscribe { topic: Topic },
    Unsubscribe { topic: Topic },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameStateMsg {
    pub topic: Topic,
    pub version: i32,
    pub game: GameSnapshot,
    pub viewer: ViewerState,
}

/// Ack command type for machine-correlatable acknowledgements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AckCommand {
    Subscribe,
    Unsubscribe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ack {
    pub command: AckCommand,
    pub topic: Topic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
    HelloAck { protocol: i32, user_id: i64 },

    Ack(Ack),

    GameState(Box<GameStateMsg>),

    YourTurn { game_id: i64, version: i32 },

    LongWaitInvalidated { game_id: i64 },

    Error { code: ErrorCode, message: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadProtocol,
    BadTopic,
    BadRequest,
    Forbidden,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::BadProtocol => "bad_protocol",
            ErrorCode::BadTopic => "bad_topic",
            ErrorCode::BadRequest => "bad_request",
            ErrorCode::Forbidden => "forbidden",
        }
    }
}
