use actix_web::http::StatusCode;
use serde_json::Value;

#[derive(Debug)]
pub struct GameStateEnvelope {
    pub json: Value,
    pub version: i64,
    pub game_id: i64,
}

impl GameStateEnvelope {
    pub fn topic(&self) -> &Value {
        self.json
            .get("topic")
            .expect("response should include topic")
    }

    pub fn game(&self) -> &Value {
        self.json
            .get("game")
            .expect("response should include game payload")
    }

    pub fn viewer(&self) -> &Value {
        self.json
            .get("viewer")
            .expect("response should include viewer payload")
    }
}

pub fn parse_game_state_envelope_ok(status: StatusCode, body: &[u8]) -> GameStateEnvelope {
    assert!(
        status.is_success(),
        "parse_game_state_envelope_ok called for non-2xx status: {status}"
    );
    parse_game_state_envelope(body)
}

/// Parse + validate the HTTP snapshot response body, which should now be the
/// same shape as WS `ServerMsg::GameState`.
pub fn parse_game_state_envelope(body: &[u8]) -> GameStateEnvelope {
    let json: Value = serde_json::from_slice(body).expect("Body should be valid JSON");

    let msg_type = json
        .get("type")
        .and_then(|v| v.as_str())
        .expect("response should include type");
    assert_eq!(msg_type, "game_state", "expected game_state message");

    let topic = json.get("topic").expect("response should include topic");
    let kind = topic
        .get("kind")
        .and_then(|v| v.as_str())
        .expect("topic.kind should be a string");
    assert_eq!(kind, "game", "expected topic.kind == game");

    let game_id = topic
        .get("id")
        .and_then(|v| v.as_i64())
        .expect("topic.id should be a number");

    let version = json
        .get("version")
        .and_then(|v| v.as_i64())
        .expect("response should include numeric version");

    // Ensure expected fields exist
    json.get("game")
        .expect("response should include game payload");
    json.get("viewer")
        .expect("response should include viewer payload");

    GameStateEnvelope {
        json,
        version,
        game_id,
    }
}

/// High-signal sanity checks that are stable across tests.
pub fn assert_game_snapshot_shape(game: &Value) {
    assert!(game.get("game").is_some(), "Should have game field");
    assert!(game.get("phase").is_some(), "Should have phase field");
}

pub fn parse_error_json(body: &[u8]) -> Value {
    serde_json::from_slice(body).expect("Body should be valid JSON error response")
}

/// Convenience: assert the error response is one of our canonical domain errors.
///
/// Your error format appears to be:
/// { "type": "https://nommie.app/errors/ERROR_CODE", ... }
pub fn assert_error_type_url(json: &Value, expected_code: &str) {
    let ty = json
        .get("type")
        .and_then(|v| v.as_str())
        .expect("error response should include type");
    assert!(
        ty.ends_with(expected_code),
        "expected error type to end with {expected_code}, got {ty}"
    );
}
