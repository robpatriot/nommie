use std::sync::Arc;
use std::time::Duration;

use crate::infra::state::resolve_dependencies;
use crate::state::app_state::AppState;

/// Minimum polling interval during startup.
const STARTUP_POLL_INTERVAL: Duration = Duration::from_secs(1);
/// Base interval for recovery backoff.
const RECOVERY_BASE_INTERVAL: Duration = Duration::from_secs(1);
/// Maximum recovery polling interval.
const RECOVERY_MAX_INTERVAL: Duration = Duration::from_secs(30);

/// Spawn the background dependency monitor.
///
/// Polls dependencies during startup until both are OK.
pub fn spawn_startup_monitor(state: Arc<AppState>) {
    tokio::spawn(async move {
        tracing::info!("readiness: starting dependency monitor (startup)");
        run_poll_loop(&state, true).await;
        tracing::info!("readiness: startup monitoring complete – polling stopped");
    });
}

/// Spawn a recovery monitor loop. Called when a request-level error causes
/// the service to transition to `Recovering`.
pub fn spawn_recovery_monitor(state: Arc<AppState>) {
    tokio::spawn(async move {
        tracing::info!("readiness: starting dependency monitor (recovery)");
        run_poll_loop(&state, false).await;
        tracing::info!("readiness: recovery monitoring complete – polling stopped");
    });
}

/// Internal poll loop used by both startup and recovery monitors.
async fn run_poll_loop(state: &AppState, is_startup: bool) {
    let mut attempt: u32 = 0;
    let manager = state.readiness().clone();

    loop {
        // Authoritative resolution attempt - handles both PINGing and Reconnecting
        if resolve_dependencies(state).await.is_err() {
            break; // terminal error; readiness state already updated to Failed
        }

        // If we're now healthy, stop polling
        let mode = manager.mode();
        if mode == crate::readiness::types::ServiceMode::Healthy {
            break;
        }

        // If permanently failed, stop polling
        if mode == crate::readiness::types::ServiceMode::Failed {
            tracing::info!("readiness: monitor stopping – service in permanent failed state");
            break;
        }

        // Compute next interval
        let interval = if is_startup {
            STARTUP_POLL_INTERVAL
        } else {
            // Exponential backoff capped at RECOVERY_MAX_INTERVAL
            let backoff = RECOVERY_BASE_INTERVAL * 2u32.saturating_pow(attempt.min(10));
            backoff.min(RECOVERY_MAX_INTERVAL)
        };

        attempt += 1;
        tokio::time::sleep(interval).await;
    }
}
