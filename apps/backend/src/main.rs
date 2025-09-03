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

    let jwt = std::env::var("APP_JWT_SECRET").unwrap_or_else(|_| {
        eprintln!("❌ APP_JWT_SECRET must be set");
        std::process::exit(1);
    });
    let security_config = SecurityConfig::new(jwt.as_bytes());

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
