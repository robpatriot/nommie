use serde::Serialize;
use time::OffsetDateTime;

/// The overall operating mode of the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceMode {
    /// Still starting up – waiting for dependencies and migrations.
    Startup,
    /// All dependencies healthy and migrations complete.
    Healthy,
    /// One or more dependencies failed – polling to recover.
    Recovering,
    /// A hard (deterministic) failure occurred – will not recover without intervention.
    Failed,
}

impl std::fmt::Display for ServiceMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Startup => write!(f, "startup"),
            Self::Healthy => write!(f, "healthy"),
            Self::Recovering => write!(f, "recovering"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Identifies a backend dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyName {
    Postgres,
    Redis,
}

impl std::fmt::Display for DependencyName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Postgres => write!(f, "postgres"),
            Self::Redis => write!(f, "redis"),
        }
    }
}

/// Current check status of a dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Ok,
    Down,
    Unknown,
}

/// Per-dependency health tracking state.
#[derive(Debug, Clone, Serialize)]
pub struct DependencyStatus {
    pub name: DependencyName,
    pub status: CheckStatus,
    #[serde(
        serialize_with = "serialize_opt_datetime",
        skip_serializing_if = "Option::is_none"
    )]
    pub checked_at: Option<OffsetDateTime>,
    #[serde(
        serialize_with = "serialize_opt_datetime",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_ok: Option<OffsetDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    pub consecutive_successes: u32,
    pub consecutive_failures: u32,
}

impl DependencyStatus {
    pub fn new(name: DependencyName) -> Self {
        Self {
            name,
            status: CheckStatus::Unknown,
            checked_at: None,
            last_ok: None,
            last_error: None,
            latency_ms: None,
            consecutive_successes: 0,
            consecutive_failures: 0,
        }
    }
}

/// Migration completion state.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MigrationState {
    pub completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Result of a single dependency health check.
#[derive(Debug, Clone)]
pub enum DependencyCheck {
    Ok {
        latency: std::time::Duration,
    },
    Down {
        error: String,
        latency: std::time::Duration,
    },
}

impl DependencyCheck {
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }
}

// ── Serde helpers ──────────────────────────────────────────────────

fn serialize_opt_datetime<S>(dt: &Option<OffsetDateTime>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match dt {
        Some(dt) => {
            let s = dt
                .format(&time::format_description::well_known::Rfc3339)
                .map_err(serde::ser::Error::custom)?;
            serializer.serialize_str(&s)
        }
        None => serializer.serialize_none(),
    }
}
