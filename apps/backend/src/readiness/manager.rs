use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Instant;

use serde_json::{json, Value};
use time::OffsetDateTime;
use tokio::sync::Notify;

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
    notify: Notify,
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
            notify: Notify::new(),
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
    pub fn mode(&self) -> ServiceMode {
        match self.inner.read() {
            Ok(inner) => inner.mode,
            Err(poisoned) => {
                let inner = poisoned.into_inner();
                tracing::error!(
                    previous_mode = %inner.mode,
                    "readiness: RwLock poisoned when reading service mode; treating as failed"
                );
                ServiceMode::Failed
            }
        }
    }

    /// Notify the readiness monitor that state may have changed.
    pub fn wake_monitor(&self) {
        self.notify.notify_one();
    }

    /// Get a future that completes when the readiness monitor is notified.
    pub fn notified(&self) -> tokio::sync::futures::Notified<'_> {
        self.notify.notified()
    }

    // ── Mutation ────────────────────────────────────────────────────

    /// Record migration outcome. A failure is a **hard** failure → immediate `Failed`.
    pub fn set_migration_result(&self, ok: bool, error: Option<String>) {
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::error!(
                    "readiness: RwLock poisoned when setting migration result; continuing with last known state"
                );
                poisoned.into_inner()
            }
        };
        inner.migration.completed = ok;
        inner.migration.error = error.clone();

        let mut should_wake = false;

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
        } else if inner.mode != ServiceMode::Healthy {
            // Migrations have now completed successfully while not healthy;
            // wake the monitor so it can re-evaluate dependencies promptly.
            should_wake = true;
        }

        drop(inner);

        if should_wake {
            self.wake_monitor();
        }
    }

    /// Mark a dependency as permanently disabled/not applicable.
    ///
    /// This is intended only for optional dependencies (currently Redis) in
    /// controlled scenarios such as tests or bootstrap. Disabling core
    /// dependencies like Postgres in production is unsupported.
    ///
    /// Returns `true` if this call caused a mode transition.
    pub fn disable_dependency(&self, name: DependencyName, reason: String) -> bool {
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::error!(
                    dependency = %name,
                    "readiness: RwLock poisoned when disabling dependency; continuing with last known state"
                );
                poisoned.into_inner()
            }
        };

        // Hard-failed services never recover.
        if inner.mode == ServiceMode::Failed {
            return false;
        }

        // Only Redis is intended to be optional/disable-able.
        if name != DependencyName::Redis {
            tracing::warn!(
                dependency = %name,
                "readiness: disable_dependency is only supported for Redis; ignoring request"
            );
            return false;
        }

        if let Some(dep) = inner.dependencies.get_mut(&name) {
            dep.status = CheckStatus::Disabled {
                reason: reason.clone(),
            };
            dep.last_error = None;
            dep.last_ok = None;
            dep.latency_ms = None;
            dep.consecutive_successes = 0;
            dep.consecutive_failures = 0;
            dep.checked_at = None;
        }

        let previous = inner.mode;
        let new_mode = compute_new_mode(&inner);
        let transitioned = new_mode != previous;

        if transitioned {
            inner.mode = new_mode;
            tracing::info!(
                previous_mode = %previous,
                new_mode = %new_mode,
                dependency = %name,
                disable_reason = %reason,
                "readiness: dependency disabled – recomputed mode"
            );
        }

        transitioned
    }

    /// Update a dependency's health after a check.
    ///
    /// Returns `true` if a mode transition occurred (caller can use this to
    /// decide whether to wake the monitor task).
    pub fn update_dependency(&self, name: DependencyName, check: DependencyCheck) -> bool {
        let mut inner = match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => {
                tracing::error!(
                    dependency = %name,
                    "readiness: RwLock poisoned when updating dependency; continuing with last known state"
                );
                poisoned.into_inner()
            }
        };

        // Hard-failed services never recover.
        if inner.mode == ServiceMode::Failed {
            return false;
        }

        let Some(dep) = inner.dependencies.get_mut(&name) else {
            tracing::error!(
                dependency = %name,
                "readiness: update_dependency called for unregistered dependency; ignoring update"
            );
            return false;
        };

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
        let new_mode = compute_new_mode(&inner);

        let transitioned = new_mode != previous;
        let mut should_wake = false;

        if transitioned {
            let transitioned_to_recovering =
                previous != ServiceMode::Recovering && new_mode == ServiceMode::Recovering;

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

                    if transitioned_to_recovering {
                        should_wake = true;
                    }
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

        drop(inner);

        if should_wake {
            self.wake_monitor();
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
    pub fn to_internal_json(&self) -> Value {
        match self.inner.read() {
            Ok(inner) => {
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
                        "ready": inner.mode == ServiceMode::Healthy,
                    },
                    "dependencies": deps,
                    "migration": inner.migration,
                })
            }
            Err(poisoned) => {
                let inner = poisoned.into_inner();
                let uptime_secs = inner.boot_time.elapsed().as_secs();
                tracing::error!(
                    "readiness: RwLock poisoned when building internal readiness JSON; reporting error state"
                );
                json!({
                    "service": "backend",
                    "uptime_seconds": uptime_secs,
                    "state": {
                        "mode": ServiceMode::Failed,
                        "ready": false,
                    },
                    "error": "readiness_lock_poisoned",
                })
            }
        }
    }

    /// Rich internal healthz body (always returned, even when not ready).
    pub fn to_internal_healthz_json(&self) -> Value {
        match self.inner.read() {
            Ok(inner) => {
                let uptime_secs = inner.boot_time.elapsed().as_secs();
                json!({
                    "service": "backend",
                    "status": "alive",
                    "uptime_seconds": uptime_secs,
                })
            }
            Err(poisoned) => {
                let inner = poisoned.into_inner();
                let uptime_secs = inner.boot_time.elapsed().as_secs();
                tracing::error!(
                    "readiness: RwLock poisoned when building internal healthz JSON; reporting error state"
                );
                json!({
                    "service": "backend",
                    "status": "error",
                    "uptime_seconds": uptime_secs,
                    "error": "readiness_lock_poisoned",
                })
            }
        }
    }
}

impl Default for ReadinessManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Single source of truth for mode transitions based on dependency and migration state.
fn compute_new_mode(inner: &Inner) -> ServiceMode {
    let previous = inner.mode;

    let all_healthy = inner.dependencies.values().all(|d| match &d.status {
        CheckStatus::Ok => d.consecutive_successes >= RECOVERY_THRESHOLD,
        CheckStatus::Disabled { .. } => true,
        _ => false,
    });

    let any_over_threshold = inner.dependencies.values().any(|d| {
        matches!(d.status, CheckStatus::Down) && d.consecutive_failures >= FAILURE_THRESHOLD
    });

    match previous {
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
    fn update_dependency_returns_true_on_transition_to_recovering() {
        let mgr = ReadinessManager::new();
        mgr.set_migration_result(true, None);

        // Reach healthy first.
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

        // First failure does not cross the threshold; no transition yet.
        let transitioned = mgr.update_dependency(
            DependencyName::Redis,
            DependencyCheck::Down {
                error: "timeout".into(),
                latency: Duration::from_millis(5000),
            },
        );
        assert!(!transitioned);
        assert_eq!(mgr.mode(), ServiceMode::Healthy);

        // Second consecutive failure crosses the threshold and triggers the transition.
        let transitioned = mgr.update_dependency(
            DependencyName::Redis,
            DependencyCheck::Down {
                error: "timeout".into(),
                latency: Duration::from_millis(5000),
            },
        );
        assert!(transitioned);
        assert_eq!(mgr.mode(), ServiceMode::Recovering);
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

    #[test]
    fn dependency_status_unknown_serializes_with_state_field() {
        let mgr = ReadinessManager::new();
        let j = mgr.to_internal_json();
        let deps = j["dependencies"].as_array().unwrap();
        let postgres = deps
            .iter()
            .find(|d| d["name"] == "postgres")
            .expect("postgres dependency present");
        assert_eq!(postgres["status"]["state"], "unknown");
    }

    #[test]
    fn disabled_dependencies_are_treated_as_healthy() {
        let mgr = ReadinessManager::new();
        mgr.set_migration_result(true, None);

        mgr.disable_dependency(DependencyName::Redis, "disabled for test".to_string());

        for _ in 0..2 {
            mgr.update_dependency(
                DependencyName::Postgres,
                DependencyCheck::Ok {
                    latency: Duration::from_millis(1),
                },
            );
        }

        assert_eq!(mgr.mode(), ServiceMode::Healthy);
        assert!(mgr.is_ready());

        let j = mgr.to_internal_json();
        let deps = j["dependencies"].as_array().unwrap();
        let redis = deps
            .iter()
            .find(|d| d["name"] == "redis")
            .expect("redis dependency present");
        assert_eq!(redis["status"]["state"], "disabled");
        assert_eq!(redis["status"]["reason"], "disabled for test");
    }

    #[test]
    fn disabling_failing_dependency_recovers_immediately() {
        let mgr = ReadinessManager::new();
        mgr.set_migration_result(true, None);

        // Drive both dependencies to healthy first.
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

        // Now drive Redis into failure to enter Recovering.
        for _ in 0..2 {
            mgr.update_dependency(
                DependencyName::Redis,
                DependencyCheck::Down {
                    error: "timeout".to_string(),
                    latency: Duration::from_millis(5000),
                },
            );
        }
        assert_eq!(mgr.mode(), ServiceMode::Recovering);
        assert!(!mgr.is_ready());

        // Disable Redis and ensure we immediately recover to Healthy.
        let transitioned =
            mgr.disable_dependency(DependencyName::Redis, "disabled for test".to_string());
        assert!(transitioned);
        assert_eq!(mgr.mode(), ServiceMode::Healthy);
        assert!(mgr.is_ready());

        let j = mgr.to_internal_json();
        let deps = j["dependencies"].as_array().unwrap();
        let redis = deps
            .iter()
            .find(|d| d["name"] == "redis")
            .expect("redis dependency present");
        assert_eq!(redis["status"]["state"], "disabled");
        assert_eq!(redis["status"]["reason"], "disabled for test");
    }

    #[test]
    fn disabling_non_redis_dependency_is_ignored() {
        let mgr = ReadinessManager::new();
        mgr.set_migration_result(true, None);

        // Bring both dependencies to healthy.
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

        // Attempt to disable Postgres – should be ignored and not transition.
        let transitioned =
            mgr.disable_dependency(DependencyName::Postgres, "should be ignored".to_string());
        assert!(!transitioned);
        assert_eq!(mgr.mode(), ServiceMode::Healthy);
        assert!(mgr.is_ready());
    }

    #[tokio::test]
    async fn wake_monitor_is_buffered_for_notified() {
        let mgr = ReadinessManager::new();

        mgr.wake_monitor();

        let result = tokio::time::timeout(Duration::from_millis(50), mgr.notified()).await;
        assert!(
            result.is_ok(),
            "notified did not resolve after wake_monitor"
        );
    }
}
