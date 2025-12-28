//! AI profile repository functions for domain layer.

use sea_orm::{ConnectionTrait, DatabaseTransaction};

use crate::adapters::ai_profiles_sea as ai_profiles_adapter;
use crate::entities::ai_profiles;
use crate::errors::domain::DomainError;

/// Values persisted for AI profiles
#[derive(Debug, Clone, PartialEq)]
pub struct AiProfile {
    pub id: i64,
    pub registry_name: String,
    pub registry_version: String,
    pub variant: String,
    pub display_name: String,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<serde_json::Value>,
    pub memory_level: Option<i32>,
    pub deprecated: bool,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
}

/// Draft data for creating a new AI profile
#[derive(Debug, Clone, PartialEq)]
pub struct AiProfileDraft {
    pub registry_name: String,
    pub registry_version: String,
    pub variant: String,
    pub display_name: String,
    pub playstyle: Option<String>,
    pub difficulty: Option<i32>,
    pub config: Option<serde_json::Value>,
    pub memory_level: Option<i32>,
    pub deprecated: bool,
}

impl AiProfileDraft {
    /// Create a new AiProfileDraft with the required fields.
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
            deprecated: false,
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

impl From<ai_profiles::Model> for AiProfile {
    fn from(model: ai_profiles::Model) -> Self {
        Self {
            id: model.id,
            registry_name: model.registry_name,
            registry_version: model.registry_version,
            variant: model.variant,
            display_name: model.display_name,
            playstyle: model.playstyle,
            difficulty: model.difficulty,
            config: model.config,
            memory_level: model.memory_level,
            deprecated: model.deprecated,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}

/// Create a new AI profile in the catalog.
pub async fn create_profile(
    txn: &DatabaseTransaction,
    draft: AiProfileDraft,
) -> Result<AiProfile, DomainError> {
    let mut dto = ai_profiles_adapter::AiProfileCreate::new(
        draft.registry_name,
        draft.registry_version,
        draft.variant,
        draft.display_name,
    );
    if let Some(ps) = draft.playstyle {
        dto = dto.with_playstyle(ps);
    }
    if let Some(diff) = draft.difficulty {
        dto = dto.with_difficulty(diff);
    }
    if let Some(cfg) = draft.config {
        dto = dto.with_config(cfg);
    }
    if let Some(ml) = draft.memory_level {
        dto = dto.with_memory_level(ml);
    }
    // Set deprecated field directly (not using builder since it's not Option)
    dto.deprecated = draft.deprecated;
    let profile = ai_profiles_adapter::create_profile(txn, dto).await?;
    Ok(AiProfile::from(profile))
}

/// Find an AI profile by ID.
pub async fn find_by_id<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    id: i64,
) -> Result<Option<AiProfile>, DomainError> {
    let profile = ai_profiles_adapter::find_by_id(conn, id).await?;
    Ok(profile.map(AiProfile::from))
}

/// Find a profile by registry signature.
pub async fn find_by_registry_variant<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    registry_name: &str,
    registry_version: &str,
    variant: &str,
) -> Result<Option<AiProfile>, DomainError> {
    let profile = ai_profiles_adapter::find_by_registry_variant(
        conn,
        registry_name,
        registry_version,
        variant,
    )
    .await?;
    Ok(profile.map(AiProfile::from))
}

/// List all AI profiles.
pub async fn list_all<C: ConnectionTrait + Send + Sync>(
    conn: &C,
) -> Result<Vec<AiProfile>, DomainError> {
    let profiles = ai_profiles_adapter::list_all(conn).await?;
    Ok(profiles.into_iter().map(AiProfile::from).collect())
}

/// Find AI profiles for multiple IDs (batch query for optimization).
pub async fn find_batch_by_ids<C: ConnectionTrait + Send + Sync>(
    conn: &C,
    profile_ids: &[i64],
) -> Result<Vec<ai_profiles::Model>, DomainError> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

    let profiles = ai_profiles::Entity::find()
        .filter(ai_profiles::Column::Id.is_in(profile_ids.iter().copied()))
        .all(conn)
        .await
        .map_err(crate::infra::db_errors::map_db_err)?;

    Ok(profiles)
}

/// Update an AI profile's configuration.
pub async fn update_profile(
    txn: &DatabaseTransaction,
    profile: AiProfile,
) -> Result<AiProfile, DomainError> {
    let mut dto = ai_profiles_adapter::AiProfileUpdate::new(profile.id);
    dto = dto
        .with_registry_name(profile.registry_name.clone())
        .with_registry_version(profile.registry_version.clone())
        .with_variant(profile.variant.clone())
        .with_display_name(profile.display_name);
    if let Some(ps) = profile.playstyle {
        dto = dto.with_playstyle(ps);
    }
    if let Some(diff) = profile.difficulty {
        dto = dto.with_difficulty(diff);
    }
    if let Some(cfg) = profile.config {
        dto = dto.with_config(cfg);
    }
    if let Some(ml) = profile.memory_level {
        dto = dto.with_memory_level(ml);
    }
    let dto = dto.with_deprecated(profile.deprecated);
    let updated = ai_profiles_adapter::update_profile(txn, dto).await?;
    Ok(AiProfile::from(updated))
}

/// Check if an existing profile matches the expected defaults from the registry.
///
/// Compares all profile values (display_name, playstyle, difficulty, config, memory_level)
/// to determine if an update is needed. Keys (registry_name, registry_version, variant)
/// are not compared here as they're used for lookup.
pub fn profile_matches_defaults(
    existing: &AiProfile,
    defaults: &crate::ai::registry::AiProfileDefaults,
) -> bool {
    existing.display_name == defaults.display_name
        && existing.playstyle == defaults.playstyle.map(|s| s.to_string())
        && existing.difficulty == defaults.difficulty
        && existing.config == defaults.config
        && existing.memory_level == defaults.memory_level
}

/// Ensure default AI profiles exist and are up-to-date with canonical values from the AI registry.
///
/// This function reads the canonical list of AI profiles from the AI registry and ensures
/// they exist in the database with the correct default values. If a profile exists, it is
/// updated to match the defaults. If it doesn't exist, it is created.
///
/// This should be called once at application startup after the database is connected.
///
/// Optimized to:
/// - Fetch all existing profiles in a single query (instead of N queries)
/// - Skip UPDATE queries when profile values haven't changed
///
/// This reduces overhead during test runs where profiles rarely change.
pub async fn ensure_default_ai_profiles(
    conn: &sea_orm::DatabaseConnection,
) -> Result<(), DomainError> {
    use std::collections::HashMap;

    use sea_orm::TransactionTrait;

    use crate::ai::registry;

    // Use a transaction to ensure all profiles are created/updated atomically
    let txn = conn
        .begin()
        .await
        .map_err(crate::infra::db_errors::map_db_err)?;

    // Fetch all existing profiles in a single query
    let existing_profiles = list_all(&txn).await?;

    // Build a lookup map: (registry_name, registry_version, variant) -> AiProfile
    let mut profile_map: HashMap<(String, String, String), AiProfile> = HashMap::new();
    for profile in existing_profiles {
        let key = (
            profile.registry_name.clone(),
            profile.registry_version.clone(),
            profile.variant.clone(),
        );
        profile_map.insert(key, profile);
    }

    // Build set of registry keys for checking deprecated profiles
    use std::collections::HashSet;
    let registry_keys: HashSet<(String, String, String)> = registry::registered_ais()
        .iter()
        .map(|factory| {
            (
                factory.name.to_string(),
                factory.version.to_string(),
                factory.profile.variant.to_string(),
            )
        })
        .collect();

    // Process all registry profiles (create/update/un-deprecate)
    for factory in registry::registered_ais() {
        let profile_defaults = &factory.profile;

        // Convert profile defaults to AiProfileDraft
        let draft = AiProfileDraft {
            registry_name: factory.name.to_string(),
            registry_version: factory.version.to_string(),
            variant: profile_defaults.variant.to_string(),
            display_name: profile_defaults.display_name.to_string(),
            playstyle: profile_defaults.playstyle.map(|s| s.to_string()),
            difficulty: profile_defaults.difficulty,
            config: profile_defaults.config.clone(),
            memory_level: profile_defaults.memory_level,
            deprecated: false,
        };

        // Look up existing profile in memory (instead of database query)
        let lookup_key = (
            draft.registry_name.clone(),
            draft.registry_version.clone(),
            draft.variant.clone(),
        );

        match profile_map.get(&lookup_key) {
            Some(existing) => {
                // Profile exists - check if update is needed by comparing values
                // Note: registry_name, registry_version, and variant are already matched by the lookup key
                // If profile was deprecated but is now back in registry, un-deprecate it
                let needs_update =
                    !profile_matches_defaults(existing, profile_defaults) || existing.deprecated;
                if needs_update {
                    // Profile exists but values changed, or was deprecated - update to match defaults
                    let updated = AiProfile {
                        id: existing.id,
                        registry_name: draft.registry_name.clone(),
                        registry_version: draft.registry_version.clone(),
                        variant: draft.variant.clone(),
                        display_name: draft.display_name.clone(),
                        playstyle: draft.playstyle.clone(),
                        difficulty: draft.difficulty,
                        config: draft.config.clone(),
                        memory_level: draft.memory_level,
                        deprecated: false, // Profile is in registry, so not deprecated
                        created_at: existing.created_at, // Preserve original creation time
                        updated_at: time::OffsetDateTime::now_utc(),
                    };
                    update_profile(&txn, updated).await?;
                }
                // If no update needed, skip the database write
            }
            None => {
                // Profile doesn't exist - create it
                create_profile(&txn, draft).await?;
            }
        }
    }

    // Mark profiles as deprecated if they're not in the registry
    // Do this after processing registry profiles so we don't deprecate and then immediately un-deprecate
    for (key, existing_profile) in profile_map.iter() {
        if !registry_keys.contains(key) && !existing_profile.deprecated {
            let mut updated = existing_profile.clone();
            updated.deprecated = true;
            updated.updated_at = time::OffsetDateTime::now_utc();
            update_profile(&txn, updated).await?;
        }
    }

    // Commit the transaction
    txn.commit()
        .await
        .map_err(crate::infra::db_errors::map_db_err)?;

    Ok(())
}
