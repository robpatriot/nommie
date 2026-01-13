//! Nommie Backend Binary Entry Point
//!
//! This is the main entry point for the backend server. It uses the backend library
//! to configure and run the HTTP server.

use std::sync::Arc;
use std::time::Duration;

use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_extensible_rate_limit::RateLimiter;
use actix_web::{App, HttpServer};
use tokio::signal;
#[cfg(unix)]
use tokio::signal::unix::{signal as unix_signal, SignalKind};
use tokio::time::timeout;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    backend::telemetry::init_tracing();

    // Set transaction policy to CommitOnOk for production
    backend::db::txn_policy::set_txn_policy(backend::db::txn_policy::TxnPolicy::CommitOnOk);

    // Environment variables must be set by the runtime environment:
    // Load and validate configuration, converting AppError to io::Error for main()
    let config = backend::bin_support::config_app::Config::from_env()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("❌ {}", e)))?;

    println!(
        "🚀 Starting Nommie Backend on http://{}:{}",
        config.host, config.port
    );
    let security_config =
        backend::state::security_config::SecurityConfig::new(config.jwt_secret.as_bytes());

    // Create application state using unified builder
    let app_state = backend::infra::state::build_state()
        .with_env(config.runtime_env)
        .with_db(config.db_kind)
        .with_security(security_config)
        .with_email_allowlist(config.email_allowlist)
        .with_redis_url(Some(config.redis_url.clone()))
        .build()
        .await
        .map_err(|e| std::io::Error::other(format!("❌ Failed to build application state: {e}")))?;

    println!("✅ Database connected");

    // Check TLS certificate expiry (logs warning if expiring soon)
    backend::bin_support::tls_checks::check_postgres_cert_expiry();

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

    // Wrap AppState with web::Data before passing to HttpServer
    let data = actix_web::web::Data::new(app_state);

    // Clone registry for shutdown handler
    let registry_for_shutdown = data.websocket_registry();

    // Extract host and port for server binding
    let host = config.host.clone();
    let port = config.port;

    // Capture payload limits for use in closure
    let max_json_payload_size = config.max_json_payload_size;
    let max_payload_size = config.max_payload_size;

    let server = HttpServer::new(move || {
        let data_clone = data.clone();
        // Create rate limiters for different route groups (one per worker thread)
        let auth_backend = InMemoryBackend::builder().build();
        let auth_input = backend::middleware::rate_limit::auth_rate_limit_config().build();
        let auth_limiter = RateLimiter::builder(auth_backend, auth_input)
            .add_headers()
            .build();

        let api_backend = InMemoryBackend::builder().build();
        let api_input = backend::middleware::rate_limit::api_rate_limit_config().build();
        let api_limiter = RateLimiter::builder(api_backend, api_input)
            .add_headers()
            .build();

        let json_config = create_json_config(max_json_payload_size);
        let payload_config = actix_web::web::PayloadConfig::default().limit(max_payload_size);

        App::new()
            .wrap(backend::middleware::cors::cors_middleware())
            .wrap(backend::middleware::structured_logger::StructuredLogger)
            .wrap(backend::middleware::trace_span::TraceSpan)
            .wrap(backend::middleware::request_trace::RequestTrace)
            .app_data(data_clone)
            .app_data(json_config)
            .app_data(payload_config)
            .service(
                // Auth routes with strict rate limiting (5 req/min) and security headers
                actix_web::web::scope("/api/auth")
                    .wrap(backend::middleware::security_headers::SecurityHeaders)
                    .wrap(auth_limiter)
                    .configure(backend::routes::auth::configure_routes),
            )
            .service(
                // Games routes with general rate limiting (100 req/min) and security headers
                actix_web::web::scope("/api/games")
                    .wrap(backend::middleware::security_headers::SecurityHeaders)
                    .wrap(api_limiter.clone())
                    .wrap(backend::middleware::jwt_extract::JwtExtract)
                    .configure(backend::routes::games::configure_routes),
            )
            .service(
                // User routes with general rate limiting (100 req/min) and security headers
                actix_web::web::scope("/api/user")
                    .wrap(backend::middleware::security_headers::SecurityHeaders)
                    .wrap(api_limiter.clone())
                    .wrap(backend::middleware::jwt_extract::JwtExtract)
                    .configure(backend::routes::user_options::configure_routes),
            )
            .service(
                // WebSocket token endpoint - issues short-lived tokens for WS connections
                actix_web::web::scope("/api/ws")
                    .wrap(backend::middleware::security_headers::SecurityHeaders)
                    .wrap(api_limiter.clone())
                    .wrap(backend::middleware::jwt_extract::JwtExtract)
                    .configure(backend::routes::realtime::configure_routes),
            )
            // Health check route - security headers only, no rate limiting
            .service(
                actix_web::web::scope("/health")
                    .wrap(backend::middleware::security_headers::SecurityHeaders)
                    .configure(backend::routes::health::configure_routes),
            )
            // WebSocket upgrade endpoint for real-time game updates
            .service(
                actix_web::web::scope("/ws")
                    .wrap(backend::middleware::jwt_extract::JwtExtract)
                    .service(
                        actix_web::web::resource("/games/{game_id}")
                            .route(actix_web::web::get().to(backend::ws::game::upgrade))
                            .name("websocket_game_upgrade"),
                    ),
            )
            // Root route - security headers (but no Cache-Control: no-store)
            .service(
                actix_web::web::scope("")
                    .wrap(backend::middleware::security_headers::SecurityHeaders)
                    .route("/", actix_web::web::get().to(backend::routes::health::root)),
            )
    })
    .bind((host.as_str(), port))?
    .run();

    let server_handle = server.handle();
    spawn_shutdown_handler(
        server_handle,
        registry_for_shutdown,
        config.websocket_timeout_secs,
    );

    server.await
}

/// Create JSON payload configuration with size limits and error handling
fn create_json_config(max_size: usize) -> actix_web::web::JsonConfig {
    let max_size_mb = max_size / (1024 * 1024);
    actix_web::web::JsonConfig::default()
        .limit(max_size)
        .error_handler(move |err, _req| {
            use actix_web::error::JsonPayloadError;
            use actix_web::HttpResponse;

            let (status, detail) = match err {
                JsonPayloadError::Overflow { limit: _ } => {
                    let msg = format!("Payload too large. Maximum size is {}MB.", max_size_mb);
                    (actix_web::http::StatusCode::PAYLOAD_TOO_LARGE, msg)
                }
                JsonPayloadError::ContentType => (
                    actix_web::http::StatusCode::BAD_REQUEST,
                    "Content type error".to_string(),
                ),
                _ => (
                    actix_web::http::StatusCode::BAD_REQUEST,
                    "Invalid JSON payload".to_string(),
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
        })
}

/// Spawn a task to handle graceful shutdown signals
fn spawn_shutdown_handler(
    server_handle: actix_web::dev::ServerHandle,
    registry: Option<Arc<backend::ws::hub::GameSessionRegistry>>,
    timeout_secs: u64,
) {
    tokio::spawn(async move {
        // Listen for both SIGINT (Ctrl+C) and SIGTERM
        // Actix-web also listens for these, but we intercept them first to close websockets
        #[cfg(unix)]
        {
            match unix_signal(SignalKind::terminate()) {
                Ok(mut sigterm) => {
                    tokio::select! {
                        _ = signal::ctrl_c() => {}
                        _ = sigterm.recv() => {}
                    }
                }
                Err(_) => {
                    let _ = signal::ctrl_c().await;
                }
            }
        }

        #[cfg(not(unix))]
        {
            let _ = signal::ctrl_c().await;
        }

        // Close all websocket connections and wait for shutdown messages to be processed
        if let Some(registry) = registry {
            let shutdown_futures = registry.close_all_connections();
            if !shutdown_futures.is_empty() {
                let total = shutdown_futures.len();

                match timeout(
                    Duration::from_secs(timeout_secs),
                    futures::future::join_all(shutdown_futures),
                )
                .await
                {
                    Ok(results) => {
                        let completed = results.iter().filter(|r| r.is_ok()).count();
                        let failed = results.iter().filter(|r| r.is_err()).count();
                        tracing::info!(
                            total_connections = total,
                            completed = completed,
                            failed = failed,
                            "websocket shutdown completed: all connections closed gracefully"
                        );
                        println!("[shutdown] websocket close_all completed: total={}, completed={}, failed={}",
                            total, completed, failed
                        );
                    }
                    Err(_) => {
                        tracing::warn!(
                            total_connections = total,
                            timeout_secs = timeout_secs,
                            "websocket shutdown timeout: some connections did not close gracefully within timeout period"
                        );
                        println!(
                            "[shutdown] websocket close_all timed out after {}s (total={})",
                            timeout_secs, total
                        );
                    }
                };
            }
        }

        // Stop the HTTP server deterministically after websocket teardown
        let _ = server_handle.stop(true).await;
    });
}
