use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;

use serde_json::{json, Value};
use time::OffsetDateTime;

use super::types::{
    CheckStatus, DependencyCheck, DependencyName, DependencyStatus, MigrationState, ServiceMode,
};

/// Thresholds controlling state transitions.
const FAILURE_THRESHOLD: u32 = 2;
const RECOVERY_THRESHOLD: u32 = 2;

/// Thread-safe readiness state manager.
///
/// Maintains cached dependency state and migration results.
/// All public methods acquire locks internally – callers never see the lock.
pub struct ReadinessManager {
    inner: RwLock<Inner>,
}

struct Inner {
    mode: ServiceMode,
    dependencies: HashMap<DependencyName, DependencyStatus>,
    migration: MigrationState,
    boot_time: Instant,
}

impl ReadinessManager {
    /// Create a new manager in `Startup` mode with unknown dependency states.
    pub fn new() -> Self {
        let mut deps = HashMap::new();
        deps.insert(
            DependencyName::Postgres,
            DependencyStatus::new(DependencyName::Postgres),
        );
        deps.insert(
            DependencyName::Redis,
            DependencyStatus::new(DependencyName::Redis),
        );

        Self {
            inner: RwLock::new(Inner {
                mode: ServiceMode::Startup,
                dependencies: deps,
                migration: MigrationState::default(),
                boot_time: Instant::now(),
            }),
        }
    }

    /// Returns `true` iff the service is in `Healthy` mode.
    ///
    /// This ensures that migrations are done AND dependencies have met the
    /// required success thresholds.
    pub fn is_ready(&self) -> bool {
        self.mode() == ServiceMode::Healthy
    }

    /// Current service mode.
    #[allow(clippy::expect_used)]
    pub fn mode(&self) -> ServiceMode {
        self.inner.read().expect("readiness lock poisoned").mode
    }

    // ── Mutation ────────────────────────────────────────────────────

    /// Record migration outcome. A failure is a **hard** failure → immediate `Failed`.
    #[allow(clippy::expect_used)]
    pub fn set_migration_result(&self, ok: bool, error: Option<String>) {
        let mut inner = self.inner.write().expect("readiness lock poisoned");
        inner.migration.completed = ok;
        inner.migration.error = error.clone();

        if !ok {
            let previous = inner.mode;
            inner.mode = ServiceMode::Failed;
            if previous != ServiceMode::Failed {
                tracing::error!(
                    previous_mode = %previous,
                    new_mode = %ServiceMode::Failed,
                    migration_error = error.as_deref().unwrap_or("unknown"),
                    "readiness: hard failure – migrations failed, service permanently not ready"
                );
            }
        }
    }

    /// Update a dependency's health after a check.
    ///
    /// Returns `true` if a mode transition occurred (caller can use this to
    /// decide whether to wake the monitor task).
    #[allow(clippy::expect_used)]
    pub fn update_dependency(&self, name: DependencyName, check: DependencyCheck) -> bool {
        let mut inner = self.inner.write().expect("readiness lock poisoned");

        // Hard-failed services never recover.
        if inner.mode == ServiceMode::Failed {
            return false;
        }

        let dep = inner
            .dependencies
            .get_mut(&name)
            .expect("dependency must be registered");

        let now = OffsetDateTime::now_utc();
        dep.checked_at = Some(now);

        match check {
            DependencyCheck::Ok { latency } => {
                dep.status = CheckStatus::Ok;
                dep.last_ok = Some(now);
                dep.latency_ms = Some(latency.as_millis() as u64);
                dep.consecutive_successes += 1;
                dep.consecutive_failures = 0;

                if dep.consecutive_successes == 1 && inner.mode != ServiceMode::Startup {
                    tracing::info!(
                        dependency = %name,
                        "readiness: dependency check succeeded"
                    );
                }
            }
            DependencyCheck::Down { error, latency } => {
                dep.status = CheckStatus::Down;
                dep.latency_ms = Some(latency.as_millis() as u64);
                dep.consecutive_failures += 1;
                dep.consecutive_successes = 0;

                if dep.consecutive_failures == 1 {
                    dep.last_error = Some(error.clone());
                    tracing::warn!(
                        dependency = %name,
                        error = %error,
                        "readiness: first dependency failure detected"
                    );
                } else {
                    dep.last_error = Some(error);
                }
            }
        }

        // ── Evaluate mode transitions ──────────────────────────────

        let previous = inner.mode;

        let all_healthy = inner
            .dependencies
            .values()
            .all(|d| d.status == CheckStatus::Ok && d.consecutive_successes >= RECOVERY_THRESHOLD);

        let any_over_threshold = inner
            .dependencies
            .values()
            .any(|d| d.consecutive_failures >= FAILURE_THRESHOLD);

        let new_mode = match previous {
            ServiceMode::Startup => {
                if all_healthy && inner.migration.completed && inner.migration.error.is_none() {
                    ServiceMode::Healthy
                } else {
                    ServiceMode::Startup
                }
            }
            ServiceMode::Healthy => {
                if any_over_threshold {
                    ServiceMode::Recovering
                } else {
                    ServiceMode::Healthy
                }
            }
            ServiceMode::Recovering => {
                if all_healthy && inner.migration.completed && inner.migration.error.is_none() {
                    ServiceMode::Healthy
                } else {
                    ServiceMode::Recovering
                }
            }
            ServiceMode::Failed => ServiceMode::Failed,
        };

        let transitioned = new_mode != previous;
        if transitioned {
            inner.mode = new_mode;
            match new_mode {
                ServiceMode::Healthy => {
                    tracing::info!(
                        previous_mode = %previous,
                        new_mode = %new_mode,
                        "readiness: transitioned to READY"
                    );
                }
                ServiceMode::Recovering => {
                    let failing: Vec<String> = inner
                        .dependencies
                        .values()
                        .filter(|d| d.consecutive_failures >= FAILURE_THRESHOLD)
                        .map(|d| format!("{}", d.name))
                        .collect();
                    tracing::error!(
                        previous_mode = %previous,
                        new_mode = %new_mode,
                        failing_dependencies = ?failing,
                        "readiness: transitioned to NOT READY – dependency failures exceeded threshold"
                    );
                }
                _ => {
                    tracing::info!(
                        previous_mode = %previous,
                        new_mode = %new_mode,
                        "readiness: mode transition"
                    );
                }
            }
        }

        transitioned
    }

    // ── JSON serialisation ─────────────────────────────────────────

    /// Minimal public response body.
    pub fn to_public_json(&self) -> Value {
        if self.is_ready() {
            json!({ "status": "ready", "ready": true })
        } else {
            json!({ "status": "not_ready", "ready": false })
        }
    }

    /// Rich internal response body.
    #[allow(clippy::expect_used)]
    pub fn to_internal_json(&self) -> Value {
        let inner = self.inner.read().expect("readiness lock poisoned");
        let uptime_secs = inner.boot_time.elapsed().as_secs();

        let deps: Vec<Value> = inner
            .dependencies
            .values()
            .map(|d| serde_json::to_value(d).unwrap_or_default())
            .collect();

        json!({
            "service": "backend",
            "uptime_seconds": uptime_secs,
            "state": {
                "mode": inner.mode,
                "ready": self.is_ready(),
            },
            "dependencies": deps,
            "migration": inner.migration,
        })
    }

    /// Rich internal healthz body (always returned, even when not ready).
    #[allow(clippy::expect_used)]
    pub fn to_internal_healthz_json(&self) -> Value {
        let inner = self.inner.read().expect("readiness lock poisoned");
        let uptime_secs = inner.boot_time.elapsed().as_secs();
        json!({
            "service": "backend",
            "status": "alive",
            "uptime_seconds": uptime_secs,
        })
    }
}

impl Default for ReadinessManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn initial_state_is_startup_and_not_ready() {
        let mgr = ReadinessManager::new();
        assert_eq!(mgr.mode(), ServiceMode::Startup);
        assert!(!mgr.is_ready());
    }

    #[test]
    fn becomes_healthy_when_migrations_and_deps_ok() {
        let mgr = ReadinessManager::new();
        mgr.set_migration_result(true, None);

        // Both deps need 2 successes now
        for _ in 0..2 {
            mgr.update_dependency(
                DependencyName::Postgres,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
            mgr.update_dependency(
                DependencyName::Redis,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
        }

        assert_eq!(mgr.mode(), ServiceMode::Healthy);
        assert!(mgr.is_ready());
    }

    #[test]
    fn migration_failure_causes_permanent_failed() {
        let mgr = ReadinessManager::new();
        mgr.set_migration_result(false, Some("schema mismatch".into()));

        assert_eq!(mgr.mode(), ServiceMode::Failed);
        assert!(!mgr.is_ready());

        // Even if deps become OK, stays failed
        for _ in 0..2 {
            mgr.update_dependency(
                DependencyName::Postgres,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
            mgr.update_dependency(
                DependencyName::Redis,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
        }

        assert_eq!(mgr.mode(), ServiceMode::Failed);
        assert!(!mgr.is_ready());
    }

    #[test]
    fn single_transient_failure_does_not_transition_to_recovering() {
        let mgr = ReadinessManager::new();
        mgr.set_migration_result(true, None);

        // Get to healthy (needs 2)
        for _ in 0..2 {
            mgr.update_dependency(
                DependencyName::Postgres,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
            mgr.update_dependency(
                DependencyName::Redis,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
        }
        assert_eq!(mgr.mode(), ServiceMode::Healthy);

        // Single failure – still healthy
        mgr.update_dependency(
            DependencyName::Redis,
            DependencyCheck::Down {
                error: "timeout".into(),
                latency: Duration::from_millis(5000),
            },
        );
        assert_eq!(mgr.mode(), ServiceMode::Healthy);
    }

    #[test]
    fn two_consecutive_failures_transitions_to_recovering() {
        let mgr = ReadinessManager::new();
        mgr.set_migration_result(true, None);

        for _ in 0..2 {
            mgr.update_dependency(
                DependencyName::Postgres,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
            mgr.update_dependency(
                DependencyName::Redis,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
        }
        assert_eq!(mgr.mode(), ServiceMode::Healthy);

        // Two consecutive failures on Redis
        mgr.update_dependency(
            DependencyName::Redis,
            DependencyCheck::Down {
                error: "timeout".into(),
                latency: Duration::from_millis(5000),
            },
        );
        mgr.update_dependency(
            DependencyName::Redis,
            DependencyCheck::Down {
                error: "timeout".into(),
                latency: Duration::from_millis(5000),
            },
        );
        assert_eq!(mgr.mode(), ServiceMode::Recovering);
        assert!(!mgr.is_ready());
    }

    #[test]
    fn recovery_requires_two_consecutive_successes() {
        let mgr = ReadinessManager::new();
        mgr.set_migration_result(true, None);

        // Get to healthy
        for _ in 0..2 {
            mgr.update_dependency(
                DependencyName::Postgres,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
            mgr.update_dependency(
                DependencyName::Redis,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
        }
        assert_eq!(mgr.mode(), ServiceMode::Healthy);

        // Break Redis
        mgr.update_dependency(
            DependencyName::Redis,
            DependencyCheck::Down {
                error: "down".into(),
                latency: Duration::from_millis(100),
            },
        );
        mgr.update_dependency(
            DependencyName::Redis,
            DependencyCheck::Down {
                error: "down".into(),
                latency: Duration::from_millis(100),
            },
        );
        assert_eq!(mgr.mode(), ServiceMode::Recovering);

        // One success – not enough
        mgr.update_dependency(
            DependencyName::Redis,
            DependencyCheck::Ok {
                latency: Duration::from_millis(1),
            },
        );
        assert_eq!(mgr.mode(), ServiceMode::Recovering);

        // Second success – recovers
        mgr.update_dependency(
            DependencyName::Redis,
            DependencyCheck::Ok {
                latency: Duration::from_millis(1),
            },
        );
        assert_eq!(mgr.mode(), ServiceMode::Healthy);
        assert!(mgr.is_ready());
    }

    #[test]
    fn public_json_shape() {
        let mgr = ReadinessManager::new();
        let j = mgr.to_public_json();
        assert_eq!(j["status"], "not_ready");
        assert_eq!(j["ready"], false);

        mgr.set_migration_result(true, None);
        for _ in 0..2 {
            mgr.update_dependency(
                DependencyName::Postgres,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
            mgr.update_dependency(
                DependencyName::Redis,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
        }

        let j = mgr.to_public_json();
        assert_eq!(j["status"], "ready");
        assert_eq!(j["ready"], true);
    }

    #[test]
    fn internal_json_contains_all_fields() {
        let mgr = ReadinessManager::new();
        mgr.set_migration_result(true, None);
        for _ in 0..2 {
            mgr.update_dependency(
                DependencyName::Postgres,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(2),
                },
            );
            mgr.update_dependency(
                DependencyName::Redis,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(3),
                },
            );
        }

        let j = mgr.to_internal_json();
        assert_eq!(j["service"], "backend");
        assert!(j["uptime_seconds"].is_number());
        assert_eq!(j["state"]["mode"], "healthy");
        assert_eq!(j["state"]["ready"], true);
        assert!(j["dependencies"].is_array());
        assert!(j["migration"]["completed"].as_bool().unwrap_or(false));
    }
}
