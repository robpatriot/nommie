use std::sync::Arc;
use std::time::Duration;

use crate::infra::state::resolve_dependencies;
use crate::readiness::types::ServiceMode;
use crate::state::app_state::AppState;

/// Minimum polling interval during startup.
const STARTUP_POLL_INTERVAL: Duration = Duration::from_secs(1);
/// Base interval for recovery backoff.
const RECOVERY_BASE_INTERVAL: Duration = Duration::from_secs(1);
/// Maximum recovery polling interval.
const RECOVERY_MAX_INTERVAL: Duration = Duration::from_secs(30);

/// Spawn the background dependency monitor.
///
/// Single long-lived task responsible for both startup and recovery monitoring.
pub fn spawn_monitor(state: Arc<AppState>) {
    tokio::spawn(async move {
        tracing::info!("readiness: starting dependency monitor");
        run_monitor(&state).await;
        tracing::info!("readiness: dependency monitoring complete – polling stopped");
    });
}

/// Internal monitor loop driving readiness based on `ServiceMode`.
async fn run_monitor(state: &AppState) {
    let mut recovering_attempt: u32 = 0;
    let manager = state.readiness().clone();

    loop {
        let mode = manager.mode();

        match mode {
            ServiceMode::Healthy => {
                // Park until woken. Failure detection is passive: with_txn
                // reports begin/commit failures to the ReadinessManager, which
                // transitions to Recovering and calls wake_monitor() after
                // FAILURE_THRESHOLD consecutive failures.
                recovering_attempt = 0;
                manager.notified().await;
            }
            // `Failed` can come from a true hard dependency/migration failure
            // (via `set_migration_result(false, ...)` inside `resolve_dependencies`) OR from
            // a readiness-infrastructure failure where `ReadinessManager::mode()` treats a
            // poisoned RwLock as failed. In the poisoning case, we never call
            // `resolve_dependencies` and instead stop the monitor based on `mode()` alone.
            ServiceMode::Failed => {
                tracing::info!(
                    "readiness: monitor stopping – service in failed mode (dependency or readiness manager failure)"
                );
                break;
            }
            ServiceMode::Startup | ServiceMode::Recovering => {
                if mode == ServiceMode::Startup {
                    // Do not apply exponential backoff while starting up.
                    recovering_attempt = 0;
                }

                // Authoritative resolution attempt - handles both pinging and reconnecting.
                if resolve_dependencies(state).await.is_err() {
                    // Terminal error; readiness state already updated to Failed.
                    tracing::error!(
                        "readiness: monitor stopping due to terminal dependency resolution error"
                    );
                    break;
                }

                let after_mode = manager.mode();

                if after_mode == ServiceMode::Healthy {
                    // Immediately return to idle state; next loop iteration will park on notified().
                    recovering_attempt = 0;
                    continue;
                }

                if after_mode == ServiceMode::Failed {
                    tracing::info!(
                        "readiness: monitor stopping – service entered permanent failed state"
                    );
                    break;
                }

                let interval = if after_mode == ServiceMode::Startup {
                    STARTUP_POLL_INTERVAL
                } else {
                    // Exponential backoff capped at RECOVERY_MAX_INTERVAL while recovering.
                    let backoff =
                        RECOVERY_BASE_INTERVAL * 2u32.saturating_pow(recovering_attempt.min(10));
                    recovering_attempt = recovering_attempt.saturating_add(1);
                    backoff.min(RECOVERY_MAX_INTERVAL)
                };

                tokio::select! {
                    _ = tokio::time::sleep(interval) => {},
                    _ = manager.notified() => {},
                }
            }
        }
    }
}
