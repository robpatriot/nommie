use actix_web::{web, App, HttpServer};
use backend::config::db::{DbKind, RuntimeEnv};
use backend::infra::state::build_state;
use backend::middleware::cors::cors_middleware;
use backend::middleware::jwt_extract::JwtExtract;
use backend::middleware::request_trace::RequestTrace;
use backend::middleware::structured_logger::StructuredLogger;
use backend::middleware::trace_span::TraceSpan;
use backend::routes;
use backend::state::security_config::SecurityConfig;

mod telemetry;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    telemetry::init_tracing();

    // Environment variables must be set by the runtime environment:
    // - Docker: Set via docker-compose env_file or docker run --env-file
    // - Local dev: Source env files manually (e.g., set -a; . ./.env; set +a)
    let host = std::env::var("BACKEND_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("BACKEND_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or_else(|_| {
            eprintln!("❌ BACKEND_PORT must be a valid port number");
            std::process::exit(1);
        });

    println!("🚀 Starting Nommie Backend on http://{}:{}", host, port);

    let jwt = match std::env::var("BACKEND_JWT_SECRET") {
        Ok(jwt) => jwt,
        Err(_) => {
            eprintln!("❌ BACKEND_JWT_SECRET must be set");
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
            .wrap(TraceSpan)
            .wrap(RequestTrace)
            .app_data(data.clone())
            .service(
                web::scope("/api/games")
                    .wrap(JwtExtract)
                    .configure(routes::games::configure_routes),
            )
            .configure(routes::configure)
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
