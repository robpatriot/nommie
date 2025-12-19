use std::time::Duration;

use backend::db::txn::SharedTxn;
use tokio::time::Instant;

pub async fn rollback_eventually(shared: SharedTxn) -> Result<(), sea_orm::DbErr> {
    let deadline = Instant::now() + Duration::from_secs(2);

    while shared.strong_count() > 1 {
        if Instant::now() >= deadline {
            return Err(sea_orm::DbErr::Custom(
                "Cannot rollback: transaction is still shared".to_string(),
            ));
        }
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    shared.rollback().await
}
