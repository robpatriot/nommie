// Verifies ACTIVE_TXNS is decremented on drop when with_txn() is cancelled (e.g. task aborted).
// Uses bounded polling (200–500ms total) and clear timeout messages for determinism.

use std::time::Duration;

use actix_web::rt;
use backend::db::txn::{active_txns_count, with_txn};
use tokio::sync::oneshot;
use tokio::time::sleep;

use crate::support::build_test_state;

const POLL_INTERVAL_MS: u64 = 10;
const MAX_POLLS: u32 = 25; // ~250ms per phase

#[actix_web::test]
async fn active_txns_counter_does_not_drift_on_cancellation(
) -> Result<(), Box<dyn std::error::Error>> {
    let state = build_test_state().await?;

    let before = active_txns_count();

    let (_tx, rx) = oneshot::channel::<()>();
    let handle = rt::spawn(async move {
        let _ = with_txn(None, &state, |_txn| {
            Box::pin(async move {
                let _ = rx.await;
                Ok::<_, backend::AppError>(())
            })
        })
        .await;
    });

    let mut saw_increment = false;
    for _ in 0..MAX_POLLS {
        let current = active_txns_count();
        if current == before + 1 {
            saw_increment = true;
            break;
        }
        sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
    let current_after_wait = active_txns_count();
    assert!(
        saw_increment,
        "ACTIVE_TXNS should increment while txn is in-flight (before={before}, current={current_after_wait}, expected={})",
        before + 1
    );

    handle.abort();
    let _ = handle.await;

    let mut back_to_before = false;
    for _ in 0..MAX_POLLS {
        if active_txns_count() == before {
            back_to_before = true;
            break;
        }
        sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
    let final_count = active_txns_count();
    assert!(
        back_to_before,
        "ACTIVE_TXNS should return to original value after abort (before={before}, final={final_count})"
    );

    Ok(())
}
