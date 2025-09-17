use backend::config::db::DbProfile;
use backend::db::txn::with_txn;
use backend::infra::state::build_state;

#[actix_web::test]
async fn test_allows_auto_commit_on_real_db_without_shared_txn(
) -> Result<(), Box<dyn std::error::Error>> {
    // Build state with a real Test DB
    let state = build_state().with_db(DbProfile::Test).build().await?;

    // Call with_txn with None for request and assert it returns Ok("ok")
    let result = with_txn(None, &state, |_txn| {
        Box::pin(async { Ok::<_, backend::error::AppError>("ok") })
    })
    .await?;

    assert_eq!(result, "ok");

    Ok(())
}
