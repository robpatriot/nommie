// apps/backend/tests/support/websocket.rs
// WebSocket test utilities (updated for GET /ws + protocol handshake)

use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use actix_web::{web, App, HttpServer};
use backend::db::txn::SharedTxn;
use backend::middleware::jwt_extract::JwtExtract;
use backend::middleware::request_trace::RequestTrace;
use backend::middleware::structured_logger::StructuredLogger;
use backend::middleware::trace_span::TraceSpan;
use backend::state::app_state::AppState;
use backend::ws::hub::WsRegistry;

use crate::support::test_middleware::TestTxnInjector;

/// Create a test registry and attach it to AppState.
///
/// Returns both the updated state and the registry so tests can:
/// 1) Use the state for WebSocket connections
/// 2) Call registry.broadcast_* directly to simulate realtime events
pub fn attach_test_registry(state: AppState) -> (AppState, Arc<WsRegistry>) {
    let registry = Arc::new(WsRegistry::new());
    let state = state.with_websocket_registry(registry.clone());
    (state, registry)
}

/// Helper to broadcast a snapshot update (simulates game mutation triggering broadcast).
pub fn broadcast_snapshot(registry: &WsRegistry, game_id: i64, version: i32) {
    registry.broadcast_game_state_available(game_id, version);
}

pub async fn wait_for_connections(
    registry: &WsRegistry,
    expected: usize,
    timeout: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = tokio::time::Instant::now();
    loop {
        if registry.active_connections_count() == expected {
            return Ok(());
        }
        if start.elapsed() >= timeout {
            return Err(format!(
                "timeout waiting for active_connections_count == {expected} (got {})",
                registry.active_connections_count()
            )
            .into());
        }
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// Start a test HTTP server with WebSocket routes.
///
/// Exposes GET /ws with JwtExtract (supports ?token=...).
pub async fn start_test_server(
    state: AppState,
    shared_txn: SharedTxn,
) -> Result<
    (
        actix_web::dev::ServerHandle,
        std::net::SocketAddr,
        tokio::task::JoinHandle<Result<(), std::io::Error>>,
    ),
    Box<dyn std::error::Error>,
> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    let state_data = web::Data::new(state);
    let txn_injector = TestTxnInjector::new(shared_txn);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(state_data.clone())
            .wrap(StructuredLogger)
            .wrap(TraceSpan)
            .wrap(RequestTrace)
            .wrap(txn_injector.clone())
            .service(
                web::scope("/ws")
                    .wrap(JwtExtract)
                    .service(web::resource("").route(web::get().to(backend::ws::session::upgrade))),
            )
    })
    .listen(listener)?
    .run();

    let server_handle = server.handle();
    let join = tokio::spawn(server);

    Ok((server_handle, addr, join))
}
