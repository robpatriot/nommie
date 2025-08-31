use actix_web::{test, App};
use backend::{
    routes,
    test_support::{get_test_db_url, schema_guard::ensure_schema_ready},
};
use sea_orm::Database;

#[actix_web::test]
async fn test_health_endpoint() {
    let db_url = get_test_db_url();
    let db = Database::connect(&db_url)
        .await
        .expect("connect to test database");

    // Ensure schema is ready (this will panic if not)
    ensure_schema_ready(&db).await;

    let app = test::init_service(App::new().configure(routes::configure)).await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());
    assert_eq!(resp.status().as_u16(), 200);

    let body = test::read_body(resp).await;
    assert_eq!(body, "ok");
}
