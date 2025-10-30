use actix_web::{web, App, HttpServer};
use backend::config::db::{DbKind, RuntimeEnv};
use backend::infra::state::build_state;
use backend::middleware::cors::cors_middleware;
use backend::middleware::request_trace::RequestTrace;
use backend::middleware::structured_logger::StructuredLogger;
use backend::middleware::trace_span::TraceSpan;
use backend::routes;
use backend::state::security_config::SecurityConfig;

mod telemetry;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    telemetry::init_tracing();

    println!("🚀 Starting Nommie Backend on http://127.0.0.1:3001");

    let jwt = match std::env::var("APP_JWT_SECRET") {
        Ok(jwt) => jwt,
        Err(_) => {
            eprintln!("❌ APP_JWT_SECRET must be set");
            std::process::exit(1);
        }
    };
    let security_config = SecurityConfig::new(jwt.as_bytes());

    // Create application state using unified builder
    let app_state = match build_state()
        .with_env(RuntimeEnv::Prod)
        .with_db(DbKind::Postgres)
        .with_security(security_config)
        .build()
        .await
    {
        Ok(state) => state,
        Err(e) => {
            eprintln!("❌ Failed to build application state: {e}");
            std::process::exit(1);
        }
    };

    println!("✅ Database connected");

    // Wrap AppState with web::Data before passing to HttpServer
    let data = web::Data::new(app_state);

    HttpServer::new(move || {
        App::new()
            .wrap(cors_middleware())
            .wrap(StructuredLogger)
            .wrap(RequestTrace)
            .wrap(TraceSpan)
            .app_data(data.clone())
            .configure(routes::configure)
    })
    .bind(("127.0.0.1", 3001))?
    .run()
    .await
}
