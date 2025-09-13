mod support;

use backend::infra::state::build_state;
use support::state_builder_ext::StateBuilderTestExt;

#[tokio::test]
async fn test_build_works_with_mock_db() {
    // This should work - tests that with_mock_db() works correctly
    let _state = build_state().with_mock_db().build().await.unwrap();
}
