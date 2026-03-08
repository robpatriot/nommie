//! Nommie Backend Binary Entry Point
//!
//! This is the main entry point for the backend server. It uses the backend library
//! to configure and run the HTTP server.

use std::sync::Arc;
use std::time::Duration;

use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_extensible_rate_limit::RateLimiter;
use actix_web::{App, HttpMessage, HttpServer};
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

    // ── Readiness manager ──────────────────────────────────────────
    let readiness = Arc::new(backend::readiness::ReadinessManager::new());

    // ── Build application state (Resilient Mode) ───────────────────
    //
    // By providing the ReadinessManager to the builder, we enable
    // Resilient Mode. Failures in DB or Redis will be caught, reported
    // to the manager, and the service will continue to start.
    let google_verifier = std::sync::Arc::new(
        backend::auth::google::GoogleVerifierImpl::new(config.google_client_id.clone())
            .await
            .map_err(|e| std::io::Error::other(format!("Google OIDC init failed: {e}")))?,
    );
    let app_state = backend::infra::state::build_state()
        .with_env(config.runtime_env)
        .with_db(config.db_kind)
        .with_security(security_config)
        .with_email_allowlist(config.email_allowlist.clone())
        .with_google_verifier(google_verifier)
        .with_redis_url(Some(config.redis_url.clone()))
        .with_readiness(readiness.clone())
        .build()
        .await
        .map_err(|e| std::io::Error::other(format!("Critical assembly failure: {e}")))?;

    // Check TLS certificate expiry (logs warning if expiring soon)
    backend::bin_support::tls_checks::check_postgres_cert_expiry();

    // Log email allowlist status
    match &app_state.config.email_allowlist {
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

    // Console diagnostic summary
    if app_state.db().is_some() {
        println!("✅ Database connected and migrations applied");
    } else {
        eprintln!("⚠️  Database connection/migration failed — NOT READY (searching in background)");
    }

    if app_state.realtime().is_some() {
        println!("✅ Redis connected (realtime enabled)");
    } else {
        eprintln!("⚠️  Redis connection failed — NOT READY (searching in background)");
    }

    // ── Spawn dependency monitor ───────────────────────────────────
    let app_state_arc = Arc::new(app_state);
    backend::readiness::monitor::spawn_monitor(app_state_arc.clone());

    // Wrap AppState with web::Data before passing to HttpServer
    let data = actix_web::web::Data::from(app_state_arc);
    let data_for_shutdown = data.clone();

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
                    .wrap(backend::middleware::readiness_gate::ReadinessGate)
                    .wrap(backend::middleware::db_readiness_reporter::DbReadinessReporter)
                    .wrap(auth_limiter)
                    .configure(backend::routes::auth::configure_routes),
            )
            .service(
                // Games routes with general rate limiting (100 req/min) and security headers
                actix_web::web::scope("/api/games")
                    .wrap(backend::middleware::security_headers::SecurityHeaders)
                    .wrap(backend::middleware::readiness_gate::ReadinessGate)
                    .wrap(backend::middleware::db_readiness_reporter::DbReadinessReporter)
                    .wrap(api_limiter.clone())
                    .wrap(backend::middleware::jwt_extract::JwtExtract)
                    .configure(backend::routes::games::configure_routes),
            )
            .service(
                // User routes with general rate limiting (100 req/min) and security headers
                actix_web::web::scope("/api/user")
                    .wrap(backend::middleware::security_headers::SecurityHeaders)
                    .wrap(backend::middleware::readiness_gate::ReadinessGate)
                    .wrap(backend::middleware::db_readiness_reporter::DbReadinessReporter)
                    .wrap(api_limiter.clone())
                    .wrap(backend::middleware::jwt_extract::JwtExtract)
                    .configure(backend::routes::user_options::configure_routes),
            )
            // More specific /api/* scopes must be registered before the generic /api scope
            // so that e.g. /api/auth/login matches /api/auth, not /api (which only has livez/readyz).
            .service(
                // WebSocket token endpoint - issues short-lived tokens for WS connections
                actix_web::web::scope("/api/ws")
                    .wrap(backend::middleware::security_headers::SecurityHeaders)
                    .wrap(backend::middleware::readiness_gate::ReadinessGate)
                    .wrap(backend::middleware::db_readiness_reporter::DbReadinessReporter)
                    .wrap(api_limiter.clone())
                    .wrap(backend::middleware::jwt_extract::JwtExtract)
                    .configure(backend::routes::realtime::configure_routes),
            )
            // WebSocket upgrade endpoint (generic, authenticated transport)
            .service(
                actix_web::web::scope("/ws")
                    .wrap(backend::middleware::jwt_extract::JwtExtract)
                    .service(
                        // GET /ws
                        actix_web::web::resource("")
                            .route(actix_web::web::get().to(backend::ws::session::upgrade))
                            .name("websocket_upgrade"),
                    ),
            )
            // API health probes - no rate limiting, no readiness gate
            .service(
                actix_web::web::scope("/api/internal")
                    .wrap(backend::middleware::security_headers::SecurityHeaders)
                    .configure(backend::routes::health::configure_api_internal_routes),
            )
            .service(
                actix_web::web::scope("/api")
                    .wrap(backend::middleware::security_headers::SecurityHeaders)
                    .configure(backend::routes::health::configure_api_health_routes),
            )
    })
    .bind((host.as_str(), port))?
    .run();

    let server_handle = server.handle();
    spawn_shutdown_handler(
        server_handle,
        data_for_shutdown,
        config.websocket_timeout_secs,
    );

    server.await
}

/// Create JSON payload configuration with size limits and error handling.
/// Uses the canonical AppError path so responses align with RFC 7807 and the
/// standard API error contract (code, trace_id, application/problem+json).
fn create_json_config(max_size: usize) -> actix_web::web::JsonConfig {
    let max_size_mb = max_size / (1024 * 1024);
    actix_web::web::JsonConfig::default()
        .limit(max_size)
        .error_handler(move |err, req| {
            use actix_web::error::JsonPayloadError;
            use actix_web::http::StatusCode;

            let app_err = match err {
                JsonPayloadError::Overflow { limit: _ } => backend::AppError::Validation {
                    code: backend::ErrorCode::PayloadTooLarge,
                    detail: format!("Payload too large. Maximum size is {}MB.", max_size_mb),
                    status: StatusCode::PAYLOAD_TOO_LARGE,
                },
                JsonPayloadError::ContentType => backend::AppError::bad_request(
                    backend::ErrorCode::InvalidHeader,
                    "Content type error",
                ),
                _ => backend::AppError::bad_request(
                    backend::ErrorCode::BadRequest,
                    "Invalid JSON payload",
                ),
            };

            let trace_id = req
                .extensions()
                .get::<String>()
                .cloned()
                .unwrap_or_else(|| "unknown".to_string());
            let response = app_err.to_http_response_with_trace_id(trace_id);
            actix_web::error::InternalError::from_response(err, response).into()
        })
}

/// Spawn a task to handle graceful shutdown signals
fn spawn_shutdown_handler(
    server_handle: actix_web::dev::ServerHandle,
    app_state: actix_web::web::Data<backend::state::app_state::AppState>,
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

        // Close all websocket connections and wait for shutdown messages to be processed.
        // Resolve the registry at shutdown time because Redis recovery can swap it.
        if let Some(registry) = app_state.websocket_registry() {
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
