use actix_web::{web, App, HttpServer};
use backend::{
    bootstrap::db,
    middleware::{cors_middleware, RequestTrace, StructuredLogger},
    routes,
};

mod telemetry;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    telemetry::init_tracing();

    // Load environment variables early
    dotenvy::dotenv().ok();

    // Ensure required environment variables are set
    if std::env::var("APP_JWT_SECRET").is_err() {
        eprintln!("❌ APP_JWT_SECRET environment variable must be set");
        std::process::exit(1);
    }

    println!("🚀 Starting Nommie Backend on http://127.0.0.1:3001");

    // Load environment and connect to database (app role only, no migrations)
    let db = db::connect_from_env()
        .await
        .expect("Failed to connect to database");

    println!("✅ Database connected (migrations handled by pnpm db:migrate)");

    HttpServer::new(move || {
        App::new()
            .wrap(cors_middleware())
            .wrap(RequestTrace)
            .wrap(StructuredLogger)
            .app_data(web::Data::new(db.clone()))
            .configure(routes::configure)
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await
}
