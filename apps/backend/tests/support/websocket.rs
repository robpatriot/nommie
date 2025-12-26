// WebSocket test utilities

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
use backend::ws::game::upgrade;
use backend::ws::hub::{GameSessionRegistry, SnapshotBroadcast};

use crate::support::test_middleware::TestTxnInjector;

/// Create a test registry and attach it to AppState
///
/// Returns both the updated state and the registry so tests can:
/// 1. Use the state for WebSocket connections
/// 2. Call registry.broadcast() directly to simulate game mutations
///
/// This is safe for concurrent test execution since each test gets
/// its own isolated registry instance.
pub fn attach_test_registry(state: AppState) -> (AppState, Arc<GameSessionRegistry>) {
    let registry = Arc::new(GameSessionRegistry::new());
    let state = state.with_websocket_registry(registry.clone());
    (state, registry)
}

/// Helper to broadcast a snapshot update (simulates game mutation triggering broadcast)
///
/// In production, game mutations call publish_snapshot which uses Redis pub/sub.
/// In tests, we call this directly to trigger broadcasts to connected WebSocket clients.
pub fn broadcast_snapshot(registry: &GameSessionRegistry, game_id: i64, version: i32) {
    registry.broadcast(game_id, SnapshotBroadcast { version });
}

pub async fn wait_for_connections(
    registry: &GameSessionRegistry,
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

/// Start a test HTTP server with WebSocket routes
///
/// This function creates a real HTTP server bound to a random port, allowing tests
/// to connect via real WebSocket clients (e.g., tokio-tungstenite). The server
/// includes test middleware to inject SharedTxn into request extensions for
/// transaction-per-test isolation.
///
/// # Returns
/// Returns a tuple of (server_handle, socket_addr, join_handle) where:
/// - `server_handle` can be used to gracefully stop the server
/// - `socket_addr` is the address the server is listening on
/// - `join_handle` can be awaited to wait for server shutdown and check for errors
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
            .wrap(txn_injector.clone()) // Inject SharedTxn into request extensions
            .service(
                web::scope("/ws")
                    .wrap(JwtExtract)
                    .service(web::resource("/games/{game_id}").route(web::get().to(upgrade))),
            )
    })
    .listen(listener)?
    .run();

    // Start server in background and return handle + join
    let server_handle = server.handle();
    let join = tokio::spawn(server);

    Ok((server_handle, addr, join))
}
