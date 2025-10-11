//! DTOs for AI profile adapter.

/// DTO for creating an AI profile.
#[derive(Debug, Clone)]
pub struct AiProfileCreate {
    pub user_id: i64,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<serde_json::Value>,
}

impl AiProfileCreate {
    pub fn new(user_id: i64) -> Self {
        Self {
            user_id,
            playstyle: None,
            difficulty: None,
            config: None,
        }
    }

    pub fn with_playstyle(mut self, playstyle: impl Into<String>) -> Self {
        self.playstyle = Some(playstyle.into());
        self
    }

    pub fn with_difficulty(mut self, difficulty: i32) -> Self {
        self.difficulty = Some(difficulty);
        self
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }
}

/// DTO for updating an AI profile config.
#[derive(Debug, Clone)]
pub struct AiProfileUpdate {
    pub id: i64,
    pub user_id: i64,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<serde_json::Value>,
}

impl AiProfileUpdate {
    pub fn new(id: i64, user_id: i64) -> Self {
        Self {
            id,
            user_id,
            playstyle: None,
            difficulty: None,
            config: None,
        }
    }

    pub fn with_playstyle(mut self, playstyle: impl Into<String>) -> Self {
        self.playstyle = Some(playstyle.into());
        self
    }

    pub fn with_difficulty(mut self, difficulty: i32) -> Self {
        self.difficulty = Some(difficulty);
        self
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }
}
