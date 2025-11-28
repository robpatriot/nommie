pub mod error_mappings;
pub mod error_shape;
pub mod extractor_current_user_db;
pub mod extractor_game_id;
pub mod extractor_game_membership;
pub mod extractor_game_membership_roles;
pub mod handler_games_ai;
pub mod handler_games_bid;
pub mod handler_games_if_match;
pub mod handler_games_ready;
pub mod handler_players;
pub mod healthcheck;
pub mod rate_limiting;
pub mod security_headers;
pub mod state_builder;
// trace_span is in its own test binary (trace_span_tests.rs)
pub mod validated_json;
