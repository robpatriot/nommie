use actix_web::{App, HttpServer, web};
use backend::{configure_routes, bootstrap::db, middleware::request_trace::RequestTrace};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 Starting Nommie Backend on http://127.0.0.1:3001");

    // Load environment and connect to database
    let db = db::connect_from_env()
        .await
        .expect("Failed to connect to database");
    
    // Run migrations (idempotent)
    db::run_migrations(&db)
        .await
        .expect("Failed to run database migrations");

    println!("✅ Database connected and migrations applied");

    HttpServer::new(move || {
        App::new()
            .wrap(RequestTrace)
            .app_data(web::Data::new(db.clone()))
            .configure(configure_routes)
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await
}
