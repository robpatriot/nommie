use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_extensible_rate_limit::RateLimiter;
use actix_web::{web, App, HttpServer};
use backend::config::db::{DbKind, RuntimeEnv};
use backend::config::email_allowlist::EmailAllowlist;
use backend::infra::state::build_state;
use backend::middleware::cors::cors_middleware;
use backend::middleware::jwt_extract::JwtExtract;
use backend::middleware::rate_limit::{api_rate_limit_config, auth_rate_limit_config};
use backend::middleware::request_trace::RequestTrace;
use backend::middleware::security_headers::SecurityHeaders;
use backend::middleware::structured_logger::StructuredLogger;
use backend::middleware::trace_span::TraceSpan;
use backend::routes;
use backend::state::security_config::SecurityConfig;
use backend::ws;

mod telemetry;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    telemetry::init_tracing();

    // Environment variables must be set by the runtime environment:
    // - Docker: Set via docker-compose env_file or docker run --env-file
    // - Local dev: Source env files manually (e.g., set -a; . ./.env; set +a)
    //
    // Validate critical environment variables up front and fail fast with
    // clear, human-friendly messages if anything is misconfigured.
    validate_env_or_exit();

    let host = std::env::var("BACKEND_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("BACKEND_PORT")
        .unwrap_or_else(|_| "3001".to_string())
        .parse::<u16>()
        .unwrap_or_else(|_| {
            eprintln!("❌ BACKEND_PORT must be a valid port number");
            std::process::exit(1);
        });

    println!("🚀 Starting Nommie Backend on http://{}:{}", host, port);

    let jwt = std::env::var("BACKEND_JWT_SECRET").expect("BACKEND_JWT_SECRET must be set");
    let security_config = SecurityConfig::new(jwt.as_bytes());

    // Create application state using unified builder
    let app_state = match build_state()
        .with_env(RuntimeEnv::Prod)
        .with_db(DbKind::Postgres)
        .with_security(security_config)
        .with_email_allowlist(EmailAllowlist::from_env())
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

    // Log email allowlist status
    match &app_state.email_allowlist {
        Some(allowlist) => {
            println!(
                "🔒 Email allowlist enabled with {} pattern(s)",
                allowlist.pattern_count()
            );
        }
        None => {
            println!("🔓 Email allowlist disabled (open signup/login)");
        }
    }

    // Check TLS certificate expiry (logs warning if expiring soon)
    backend::config::tls::check_postgres_cert_expiry();

    // Wrap AppState with web::Data before passing to HttpServer
    let data = web::Data::new(app_state);

    HttpServer::new(move || {
        // Create rate limiters for different route groups (one per worker thread)
        let auth_backend = InMemoryBackend::builder().build();
        let auth_input = auth_rate_limit_config().build();
        let auth_limiter = RateLimiter::builder(auth_backend, auth_input)
            .add_headers()
            .build();

        let api_backend = InMemoryBackend::builder().build();
        let api_input = api_rate_limit_config().build();
        let api_limiter = RateLimiter::builder(api_backend, api_input)
            .add_headers()
            .build();

        // Configure request body size limits to prevent DoS attacks
        // 1MB limit for JSON payloads (sufficient for game actions)
        let json_config = web::JsonConfig::default()
            .limit(1024 * 1024) // 1MB
            .error_handler(|err, _req| {
                use actix_web::error::JsonPayloadError;
                use actix_web::HttpResponse;

                let (status, detail) = match err {
                    JsonPayloadError::Overflow { limit: _ } => (
                        actix_web::http::StatusCode::PAYLOAD_TOO_LARGE,
                        "Payload too large. Maximum size is 1MB.",
                    ),
                    JsonPayloadError::ContentType => (
                        actix_web::http::StatusCode::BAD_REQUEST,
                        "Content type error",
                    ),
                    _ => (
                        actix_web::http::StatusCode::BAD_REQUEST,
                        "Invalid JSON payload",
                    ),
                };

                actix_web::error::InternalError::from_response(
                    err,
                    HttpResponse::build(status).json(serde_json::json!({
                        "type": "https://tools.ietf.org/html/rfc7231#section-6.5.11",
                        "title": "Payload Too Large",
                        "status": status.as_u16(),
                        "detail": detail,
                    })),
                )
                .into()
            });

        // 1MB limit for form data and other payloads
        let payload_config = web::PayloadConfig::default().limit(1024 * 1024); // 1MB

        App::new()
            .wrap(cors_middleware())
            .wrap(StructuredLogger)
            .wrap(TraceSpan)
            .wrap(RequestTrace)
            .app_data(data.clone())
            .app_data(json_config)
            .app_data(payload_config)
            .service(
                // Auth routes with strict rate limiting (5 req/min) and security headers
                web::scope("/api/auth")
                    .wrap(SecurityHeaders)
                    .wrap(auth_limiter)
                    .configure(routes::auth::configure_routes),
            )
            .service(
                // Games routes with general rate limiting (100 req/min) and security headers
                web::scope("/api/games")
                    .wrap(SecurityHeaders)
                    .wrap(api_limiter.clone())
                    .wrap(JwtExtract)
                    .configure(routes::games::configure_routes),
            )
                // User routes with general rate limiting (100 req/min) and security headers
                web::scope("/api/user")
                    .wrap(SecurityHeaders)
                    .wrap(api_limiter)
                    .wrap(JwtExtract)
                    .configure(routes::user_options::configure_routes),
            )
            // Health check route - security headers only, no rate limiting
            .service(
                web::scope("/health")
                    .wrap(SecurityHeaders)
                    .configure(routes::health::configure_routes),
            )
            // Root route - security headers (but no Cache-Control: no-store)
            .service(
                web::scope("")
                    .wrap(SecurityHeaders)
                    .route("/", web::get().to(routes::health::root)),
            )
            // WebSocket routes for real-time game updates
            .service(
                web::scope("/ws").wrap(JwtExtract).service(
                    web::resource("/games/{game_id}").route(web::get().to(ws::game::upgrade)),
                ),
            )
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

/// Validate critical environment variables at startup and exit with a clear
/// error message if any required values are missing or obviously invalid.
fn validate_env_or_exit() {
    use std::env;

    // BACKEND_JWT_SECRET: required, non-empty, minimum length
    match env::var("BACKEND_JWT_SECRET") {
        Ok(secret) if secret.len() >= 32 => {}
        Ok(_) => {
            eprintln!(
                "❌ BACKEND_JWT_SECRET is too short. It should be at least 32 characters for security."
            );
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("❌ BACKEND_JWT_SECRET must be set.");
            std::process::exit(1);
        }
    }

    // Database environment for production: basic presence checks
    // (detailed validation still happens in db config errors).
    if cfg!(not(test)) {
        if let Ok(runtime_env) = env::var("RUNTIME_ENV") {
            if runtime_env.eq_ignore_ascii_case("prod") {
                for name in &["PROD_DB", "APP_DB_USER", "APP_DB_PASSWORD"] {
                    if env::var(name).is_err() {
                        eprintln!("❌ {name} must be set when RUNTIME_ENV=prod.");
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}
