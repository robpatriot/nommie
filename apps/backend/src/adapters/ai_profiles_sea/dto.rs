//! DTOs for AI profile adapter.

/// DTO for creating an AI profile.
#[derive(Debug, Clone)]
pub struct AiProfileCreate {
    pub registry_name: String,
    pub registry_version: String,
    pub variant: String,
    pub display_name: String,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<serde_json::Value>,
    pub memory_level: Option<i32>,
}

impl AiProfileCreate {
    pub fn new(
        registry_name: impl Into<String>,
        registry_version: impl Into<String>,
        variant: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            registry_name: registry_name.into(),
            registry_version: registry_version.into(),
            variant: variant.into(),
            display_name: display_name.into(),
            playstyle: None,
            difficulty: None,
            config: None,
            memory_level: None,
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

    pub fn with_memory_level(mut self, memory_level: i32) -> Self {
        self.memory_level = Some(memory_level);
        self
    }
}

/// DTO for updating an AI profile config.
#[derive(Debug, Clone)]
pub struct AiProfileUpdate {
    pub id: i64,
    pub registry_name: Option<String>,
    pub registry_version: Option<String>,
    pub variant: Option<String>,
    pub display_name: Option<String>,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<serde_json::Value>,
    pub memory_level: Option<i32>,
}

impl AiProfileUpdate {
    pub fn new(id: i64) -> Self {
        Self {
            id,
            registry_name: None,
            registry_version: None,
            variant: None,
            display_name: None,
            playstyle: None,
            difficulty: None,
            config: None,
            memory_level: None,
        }
    }

    pub fn with_registry_name(mut self, registry_name: impl Into<String>) -> Self {
        self.registry_name = Some(registry_name.into());
        self
    }

    pub fn with_registry_version(mut self, registry_version: impl Into<String>) -> Self {
        self.registry_version = Some(registry_version.into());
        self
    }

    pub fn with_variant(mut self, variant: impl Into<String>) -> Self {
        self.variant = Some(variant.into());
        self
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
