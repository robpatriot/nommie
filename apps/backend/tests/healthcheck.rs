use actix_test;
use backend::{build_app, assert_test_db_url, load_test_env, migrate_test_db, get_test_db_url};

#[actix_test::test]
async fn test_health_endpoint() {
    // Load test environment
    load_test_env();
    
    // Get and validate test database URL
    let db_url = get_test_db_url();
    assert_test_db_url(&db_url);
    
    // Migrate test database
    let _db = migrate_test_db(&db_url).await;
    
    // Build service with the configurator closure
    let app = actix_test::init_service(build_app()).await;
    
    // Send GET /health request
    let req = actix_test::TestRequest::get().uri("/health").to_request();
    let resp = actix_test::call_service(&app, req).await;
    
    // Assert response
    assert!(resp.status().is_success());
    assert_eq!(resp.status().as_u16(), 200);
    
    // Assert response body
    let body = actix_test::read_body(resp).await;
    assert_eq!(body, "ok");
}
