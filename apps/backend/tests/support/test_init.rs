/// Test initialization that sets the transaction policy to rollback on success.
///
/// This constructor runs once per integration test binary, ensuring that all
/// tests use the rollback-on-success policy by default. Tests that omit this
/// file will still run with the default commit behavior.
#[ctor::ctor]
fn init_test_txn_policy() {
    backend::db::txn_policy::set_txn_policy(backend::db::txn_policy::TxnPolicy::RollbackOnOk);
}
