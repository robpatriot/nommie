use actix_web::{App, HttpServer, web};
use backend::{configure_routes, bootstrap::db, middleware::request_trace::RequestTrace};

mod telemetry;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    telemetry::init_tracing();
    
    println!("🚀 Starting Nommie Backend on http://127.0.0.1:3001");

    // Load environment and connect to database (app role only, no migrations)
    let db = db::connect_from_env()
        .await
        .expect("Failed to connect to database");

    println!("✅ Database connected (migrations handled by pnpm db:migrate)");

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
