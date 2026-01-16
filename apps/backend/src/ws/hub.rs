use std::error::Error as StdError;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use actix::prelude::*;
use dashmap::DashMap;
use rand::random;
use redis::aio::{ConnectionManager, PubSub};
use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tokio_stream::StreamExt;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::ws::game::GameWsSession;

/// Recipient + address for a single websocket session
type SessionHandle = (Recipient<SnapshotBroadcast>, Addr<GameWsSession>);
/// Map of session token to session handle for a specific game_id
type SessionMap = DashMap<Uuid, SessionHandle>;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct SnapshotBroadcast {
    pub version: i32,
}

pub struct GameSessionRegistry {
    sessions: DashMap<i64, SessionMap>,
    active_connections: AtomicUsize,
}

impl GameSessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            active_connections: AtomicUsize::new(0),
        }
    }

    pub fn register(
        &self,
        game_id: i64,
        recipient: Recipient<SnapshotBroadcast>,
        addr: Addr<GameWsSession>,
    ) -> Uuid {
        let token = Uuid::new_v4();
        let entry = self.sessions.entry(game_id).or_default();
        entry.insert(token, (recipient, addr));

        let active = self.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
        info!(
            game_id,
            active_connections = active,
            "Websocket session registered"
        );

        token
    }

    pub fn unregister(&self, game_id: i64, token: Uuid) {
        let active_before = self.active_connections.load(Ordering::Relaxed);

        let (was_present, now_empty) = match self.sessions.get_mut(&game_id) {
            Some(entry) => {
                // Acquire mutable guard - allows mutation of inner map
                let was_present = entry.remove(&token).is_some();
                let now_empty = entry.is_empty();
                // Guard is dropped here when entry goes out of scope
                (was_present, now_empty)
            }
            _ => (false, false),
        };

        // Now that the guard is dropped, we can safely remove the outer map entry if needed
        if now_empty {
            self.sessions.remove(&game_id);
        }

        if was_present {
            let previous = self.active_connections.load(Ordering::Relaxed);
            if previous > 0 {
                self.active_connections.fetch_sub(1, Ordering::Relaxed);
            }
            let active_after = previous.saturating_sub(1);
            info!(
                game_id,
                token = %token,
                active_connections_before = active_before,
                active_connections_after = active_after,
                "[WS REGISTRY] Websocket session unregistered"
            );
        } else {
            warn!(
                game_id,
                token = %token,
                active_connections = active_before,
                "[WS REGISTRY] Attempted to unregister session that was not found"
            );
        }
    }

    pub fn broadcast(&self, game_id: i64, message: SnapshotBroadcast) {
        if let Some(entry) = self.sessions.get(&game_id) {
            for recipient_tuple in entry.iter() {
                recipient_tuple.value().0.do_send(message.clone());
            }
        }
    }

    /// Initiate shutdown for all active websocket connections
    /// Returns a vector of futures that will resolve when each shutdown message is processed
    pub fn close_all_connections(
        &self,
    ) -> Vec<actix::dev::Request<GameWsSession, crate::ws::game::Shutdown>> {
        // Clone addrs first to drop DashMap guards before sending shutdowns, avoiding contention
        let mut addrs = Vec::new();
        for entry in self.sessions.iter() {
            for session_entry in entry.value().iter() {
                let (_, addr) = session_entry.value();
                addrs.push(addr.clone());
            }
        }

        addrs
            .into_iter()
            .map(|addr| addr.send(crate::ws::game::Shutdown))
            .collect()
    }

    /// Get current number of active connections
    pub fn active_connections_count(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }
}

pub struct RealtimeBroker {
    registry: Arc<GameSessionRegistry>,
    publisher: Mutex<ConnectionManager>,
}

impl RealtimeBroker {
    pub async fn connect(redis_url: &str) -> Result<Arc<Self>, AppError> {
        let client = Client::open(redis_url).map_err(|err| AppError::Config {
            detail: format!("Invalid REDIS_URL: {err}"),
            source: Box::new(err),
        })?;

        let manager = ConnectionManager::new(client.clone())
            .await
            .map_err(|err| AppError::Internal {
                code: ErrorCode::ConfigError,
                detail: "Unable to initialize Redis connection manager".to_string(),
                source: Box::new(err),
            })?;

        let registry = Arc::new(GameSessionRegistry::new());
        let broker = Arc::new(Self {
            registry: registry.clone(),
            publisher: Mutex::new(manager),
        });

        spawn_subscriber(redis_url, registry);

        Ok(broker)
    }

    pub fn registry(&self) -> Arc<GameSessionRegistry> {
        self.registry.clone()
    }

    pub async fn publish_snapshot(&self, game_id: i64, version: i32) -> Result<(), AppError> {
        let envelope = RedisEnvelope { game_id, version };
        let encoded = serde_json::to_string(&envelope).map_err(|err| AppError::Internal {
            code: ErrorCode::InternalError,
            detail: "Failed to serialize snapshot broadcast".to_string(),
            source: Box::new(err),
        })?;
        let channel = format!("game:{game_id}");

        // Retry with exponential backoff for transient errors
        // Use shorter delays since we're in the HTTP request path
        let mut attempt = 0u32;
        loop {
            attempt += 1;

            let publish_res = {
                let mut publisher = self.publisher.lock().await;
                let res = publisher
                    .publish::<_, _, ()>(channel.clone(), encoded.clone())
                    .await;
                res
            };
            match publish_res {
                Ok(_) => return Ok(()),
                Err(err) => {
                    let app_err = AppError::Internal {
                        code: ErrorCode::InternalError,
                        detail: "Failed to publish snapshot to Redis".to_string(),
                        source: Box::new(err),
                    };

                    // Check if we should retry
                    if attempt >= PUBLISHER_MAX_ATTEMPTS || !is_transient_error(&app_err) {
                        return Err(app_err);
                    }

                    // Calculate retry delay (shorter than subscriber since we're in HTTP path)
                    let delay_ms = PUBLISHER_INITIAL_RETRY_DELAY_MS
                        .saturating_mul(2_u64.pow(attempt - 1))
                        .min(PUBLISHER_MAX_RETRY_DELAY_MS);
                    let delay = Duration::from_millis(delay_ms);

                    warn!(
                        error = %app_err,
                        attempt,
                        max_attempts = PUBLISHER_MAX_ATTEMPTS,
                        retry_delay_ms = delay_ms,
                        "Redis publish failed, retrying"
                    );

                    sleep(delay).await;
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct RedisEnvelope {
    game_id: i64,
    version: i32,
}

// Retry configuration constants for subscriber (long-running background task)
const INITIAL_RETRY_DELAY_SECS: u64 = 1;
const MAX_RETRY_DELAY_SECS: u64 = 60;
const RETRY_DELAY_MULTIPLIER: f64 = 2.0;
const JITTER_PERCENT: f64 = 0.2; // ±20% jitter

// Retry configuration constants for publisher (HTTP request path - must be fast)
const PUBLISHER_MAX_ATTEMPTS: u32 = 3;
const PUBLISHER_INITIAL_RETRY_DELAY_MS: u64 = 50;
const PUBLISHER_MAX_RETRY_DELAY_MS: u64 = 200;

fn spawn_subscriber(redis_url: &str, registry: Arc<GameSessionRegistry>) {
    let redis_url = redis_url.to_string();
    tokio::spawn(async move {
        run_subscription_loop_with_retry(&redis_url, registry).await;
    });
}

/// Determine if an error is transient and should trigger a retry
fn is_transient_error(err: &AppError) -> bool {
    // Check error variant first - config errors are permanent
    if let AppError::Config { .. } = err {
        return false;
    }

    // Check the underlying error source using the Error trait
    let error_msg = err.to_string().to_lowercase();

    // Check for permanent error indicators in the message
    if error_msg.contains("authentication failed")
        || error_msg.contains("invalid redis_url")
        || error_msg.contains("unsupported")
        || error_msg.contains("non-tcp protocol")
    {
        return false;
    }

    // Check for transient error indicators
    if error_msg.contains("connection refused")
        || error_msg.contains("connection reset")
        || error_msg.contains("connection aborted")
        || error_msg.contains("timed out")
        || error_msg.contains("timeout")
        || error_msg.contains("broken pipe")
        || error_msg.contains("network")
        || error_msg.contains("io error")
        || error_msg.contains("stream ended")
    {
        return true;
    }

    // Check the underlying error source
    if let Some(source) = StdError::source(err) {
        // Check for IO errors (typically transient)
        if let Some(io_err) = source.downcast_ref::<std::io::Error>() {
            match io_err.kind() {
                std::io::ErrorKind::ConnectionRefused => return true,
                std::io::ErrorKind::ConnectionAborted => return true,
                std::io::ErrorKind::ConnectionReset => return true,
                std::io::ErrorKind::TimedOut => return true,
                std::io::ErrorKind::WouldBlock => return true,
                std::io::ErrorKind::Interrupted => return true,
                std::io::ErrorKind::PermissionDenied => return false,
                std::io::ErrorKind::Unsupported => return false,
                _ => {}
            }
        }
    }

    // Default to transient for safety - better to retry than give up
    true
}

/// Calculate exponential backoff delay with jitter
fn calculate_retry_delay(attempt: u32) -> Duration {
    let base_delay =
        INITIAL_RETRY_DELAY_SECS as f64 * RETRY_DELAY_MULTIPLIER.powi(attempt as i32 - 1);
    let capped_delay = base_delay.min(MAX_RETRY_DELAY_SECS as f64);

    // Add jitter: ±20% random variation
    let jitter_range = capped_delay * JITTER_PERCENT;
    let jitter = (random::<f64>() * 2.0 - 1.0) * jitter_range;
    let final_delay = (capped_delay + jitter).max(0.1); // Minimum 100ms

    Duration::from_secs_f64(final_delay)
}

/// Main retry loop that handles reconnection logic
async fn run_subscription_loop_with_retry(redis_url: &str, registry: Arc<GameSessionRegistry>) {
    let mut attempt = 0u32;
    let mut consecutive_failures = 0u32;

    loop {
        attempt += 1;

        let loop_res = run_subscription_loop(redis_url, registry.clone()).await;

        match loop_res {
            Ok(()) => {
                // Normal completion (shouldn't happen in production, but handle gracefully)
                info!("Redis subscription loop completed normally");
                break;
            }
            Err(err) => {
                consecutive_failures += 1;

                // Check if error is permanent (should not retry)
                if !is_transient_error(&err) {
                    error!(
                        error = %err,
                        attempt,
                        "Redis subscription failed with permanent error, exiting"
                    );
                    break;
                }

                // Calculate retry delay
                let delay = calculate_retry_delay(attempt);

                warn!(
                    error = %err,
                    attempt,
                    consecutive_failures,
                    retry_delay_secs = delay.as_secs_f64(),
                    "Redis subscription failed, retrying"
                );

                // Wait before retrying
                sleep(delay).await;

                // Reset attempt counter periodically to prevent overflow
                // After 20 attempts, we're at max delay anyway, so reset
                if attempt >= 20 {
                    attempt = 10; // Reset to a point where we're near max delay
                }
            }
        }
    }
}
/// Inner function that performs a single connection attempt and message processing
async fn run_subscription_loop(
    redis_url: &str,
    registry: Arc<GameSessionRegistry>,
) -> Result<(), AppError> {
    // Create a client to get connection info
    let client = Client::open(redis_url).map_err(|err| AppError::Internal {
        code: ErrorCode::ConfigError,
        detail: format!("Failed to create Redis client: {err}"),
        source: Box::new(err),
    })?;

    let conn_info = client.get_connection_info();

    // Create a TCP stream for pubsub
    let addr = match conn_info.addr().clone() {
        redis::ConnectionAddr::Tcp(host, port) => (host, port),
        _ => {
            return Err(AppError::Internal {
                code: ErrorCode::ConfigError,
                detail: "Only TCP protocol is supported for pubsub".to_string(),
                source: Box::new(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Non-TCP protocol",
                )),
            });
        }
    };

    info!(
        "Connecting to Redis for subscription at {}:{}",
        addr.0, addr.1
    );

    let stream = tokio::net::TcpStream::connect(addr)
        .await
        .map_err(|err| AppError::Internal {
            code: ErrorCode::ConfigError,
            detail: format!("Failed to connect to Redis for subscription: {err}"),
            source: Box::new(err),
        })?;

    // Use the RedisConnectionInfo from ConnectionInfo
    let mut pubsub = PubSub::new(conn_info.redis_settings(), stream)
        .await
        .map_err(|err| AppError::Internal {
            code: ErrorCode::ConfigError,
            detail: format!("Failed to create Redis pubsub: {err}"),
            source: Box::new(err),
        })?;

    info!("Subscribing to Redis pattern 'game:*'");

    pubsub
        .psubscribe("game:*")
        .await
        .map_err(|err| AppError::Internal {
            code: ErrorCode::ConfigError,
            detail: "Failed to subscribe to Redis channel pattern".to_string(),
            source: Box::new(err),
        })?;

    info!("Redis subscription established, processing messages");

    let mut stream = pubsub.into_on_message();

    loop {
        let next_msg = stream.next().await;
        let Some(msg) = next_msg else {
            break;
        };

        let Ok(channel) = msg.get_channel::<String>() else {
            continue;
        };
        let Ok(payload) = msg.get_payload::<String>() else {
            continue;
        };

        if let Some(game_id) = parse_channel(channel.as_str()) {
            match serde_json::from_str::<RedisEnvelope>(payload.as_str()) {
                Ok(envelope) => {
                    registry.broadcast(
                        game_id,
                        SnapshotBroadcast {
                            version: envelope.version,
                        },
                    );
                }
                Err(err) => {
                    error!(error = %err, "Failed to decode Redis snapshot payload");
                }
            }
        }
    }

    // Stream ended (connection lost)
    warn!("Redis subscription stream ended, connection lost");
    Err(AppError::Internal {
        code: ErrorCode::InternalError,
        detail: "Redis subscription stream ended unexpectedly".to_string(),
        source: Box::new(std::io::Error::new(
            std::io::ErrorKind::ConnectionAborted,
            "Stream ended",
        )),
    })
}

fn parse_channel(channel: &str) -> Option<i64> {
    channel
        .split(':')
        .nth(1)
        .and_then(|value| value.parse().ok())
}

impl Default for GameSessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
