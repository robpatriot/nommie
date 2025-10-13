//! DTOs for AI overrides adapter.

/// DTO for creating AI overrides.
#[derive(Debug, Clone)]
pub struct AiOverrideCreate {
    pub game_player_id: i64,
    pub name: Option<String>,
    pub memory_level: Option<i32>,
    pub config: Option<serde_json::Value>,
}

impl AiOverrideCreate {
    pub fn new(game_player_id: i64) -> Self {
        Self {
            game_player_id,
            name: None,
            memory_level: None,
            config: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_memory_level(mut self, memory_level: i32) -> Self {
        self.memory_level = Some(memory_level);
        self
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }
}

/// DTO for updating AI overrides.
#[derive(Debug, Clone)]
pub struct AiOverrideUpdate {
    pub id: i64,
    pub game_player_id: i64,
    pub name: Option<String>,
    pub memory_level: Option<i32>,
    pub config: Option<serde_json::Value>,
}

impl AiOverrideUpdate {
    pub fn new(id: i64, game_player_id: i64) -> Self {
        Self {
            id,
            game_player_id,
            name: None,
            memory_level: None,
            config: None,
        }
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_memory_level(mut self, memory_level: i32) -> Self {
        self.memory_level = Some(memory_level);
        self
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }
}
