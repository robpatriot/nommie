//! DTOs for AI profile adapter.

/// DTO for creating an AI profile.
#[derive(Debug, Clone)]
pub struct AiProfileCreate {
    pub user_id: i64,
    pub display_name: String,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<serde_json::Value>,
    pub memory_level: Option<i32>,
}

impl AiProfileCreate {
    pub fn new(user_id: i64, display_name: impl Into<String>) -> Self {
        Self {
            user_id,
            display_name: display_name.into(),
            playstyle: None,
            difficulty: None,
            config: None,
            memory_level: None,
        }
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
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

    pub fn with_memory_level(mut self, memory_level: i32) -> Self {
        self.memory_level = Some(memory_level);
        self
    }
}

/// DTO for updating an AI profile config.
#[derive(Debug, Clone)]
pub struct AiProfileUpdate {
    pub id: i64,
    pub user_id: i64,
    pub display_name: Option<String>,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<serde_json::Value>,
    pub memory_level: Option<i32>,
}

impl AiProfileUpdate {
    pub fn new(id: i64, user_id: i64) -> Self {
        Self {
            id,
            user_id,
            display_name: None,
            playstyle: None,
            difficulty: None,
            config: None,
            memory_level: None,
        }
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
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

    pub fn with_memory_level(mut self, memory_level: i32) -> Self {
        self.memory_level = Some(memory_level);
        self
    }
}
