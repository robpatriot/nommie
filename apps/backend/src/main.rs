use actix_web::{web, App, HttpServer};
use backend::{
    bootstrap::db::DbProfile,
    middleware::{cors_middleware, RequestTrace, StructuredLogger},
    routes,
    state::SecurityConfig,
    test_support::build_state,
};

mod telemetry;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    telemetry::init_tracing();

    // Load environment variables early
    dotenvy::dotenv().ok();

    println!("🚀 Starting Nommie Backend on http://127.0.0.1:3001");

    // Ensure required environment variables are set
    if std::env::var("APP_JWT_SECRET").is_err() {
        eprintln!("❌ APP_JWT_SECRET environment variable must be set");
        std::process::exit(1);
    }

    // Create security configuration from environment
    let jwt_secret =
        std::env::var("APP_JWT_SECRET").expect("APP_JWT_SECRET environment variable must be set");
    let security_config = SecurityConfig::new(jwt_secret.as_bytes());

    // Create application state using unified builder
    let app_state = build_state()
        .with_db(DbProfile::Prod)
        .with_security(security_config)
        .build()
        .await
        .expect("Failed to build application state");

    println!("✅ Database connected (migrations handled by pnpm db:migrate)");

    HttpServer::new(move || {
        App::new()
            .wrap(cors_middleware())
            .wrap(RequestTrace)
            .wrap(StructuredLogger)
            .app_data(web::Data::new(app_state.clone()))
            .configure(routes::configure)
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await
}
