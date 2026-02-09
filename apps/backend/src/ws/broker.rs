use std::error::Error as StdError;
use std::sync::Arc;
use std::time::Duration;

use rand::random;
use redis::aio::{ConnectionManager, PubSub};
use redis::{AsyncCommands, Client};
use tokio::sync::Mutex;
use tokio::time::{sleep, Instant};
use tokio_stream::StreamExt;
use tracing::{error, info, warn};

use crate::error::AppError;
use crate::errors::ErrorCode;
use crate::ws::hub::WsRegistry;
use crate::ws::session::HubEvent;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventEnvelope {
    GameStateAvailable {
        game_id: i64,
        version: i32,
    },
    YourTurn {
        user_id: i64,
        game_id: i64,
        version: i32,
    },
    LongWaitInvalidated {
        game_id: i64,
    },
}

pub struct RealtimeBroker {
    registry: Arc<WsRegistry>,
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

        let registry = Arc::new(WsRegistry::new());
        let broker = Arc::new(Self {
            registry: registry.clone(),
            publisher: Mutex::new(manager),
        });

        spawn_subscriber(redis_url, registry);

        Ok(broker)
    }

    pub fn registry(&self) -> Arc<WsRegistry> {
        self.registry.clone()
    }

    /// Publish a "game_state_available" event for a given game.
    pub async fn publish_game_state(&self, game_id: i64, version: i32) -> Result<(), AppError> {
        let envelope = EventEnvelope::GameStateAvailable { game_id, version };
        self.publish_to_channel(format!("game:{game_id}"), envelope)
            .await
    }

    /// Publish a user-scoped "your_turn" event (optional convenience API).
    pub async fn publish_your_turn(
        &self,
        user_id: i64,
        game_id: i64,
        version: i32,
    ) -> Result<(), AppError> {
        let envelope = EventEnvelope::YourTurn {
            user_id,
            game_id,
            version,
        };
        self.publish_to_channel(format!("user:{user_id}"), envelope)
            .await
    }

    /// Publish a "long_wait_invalidated" event for a given game.
    /// This is published on the user channel so the user can refresh LW navigation
    /// even when not currently subscribed to the game's realtime topic.
    pub async fn publish_long_wait_invalidated(
        &self,
        user_id: i64,
        game_id: i64,
    ) -> Result<(), AppError> {
        let envelope = EventEnvelope::LongWaitInvalidated { game_id };
        self.publish_to_channel(format!("user:{user_id}"), envelope)
            .await
    }

    async fn publish_to_channel(
        &self,
        channel: String,
        envelope: EventEnvelope,
    ) -> Result<(), AppError> {
        let encoded = serde_json::to_string(&envelope).map_err(|err| AppError::Internal {
            code: ErrorCode::InternalError,
            detail: "Failed to serialize realtime envelope".to_string(),
            source: Box::new(err),
        })?;

        let mut attempt = 0u32;
        loop {
            attempt += 1;

            let publish_res = {
                let mut publisher = self.publisher.lock().await;
                publisher
                    .publish::<_, _, ()>(channel.clone(), encoded.clone())
                    .await
            };

            match publish_res {
                Ok(_) => return Ok(()),
                Err(err) => {
                    let app_err = AppError::Internal {
                        code: ErrorCode::InternalError,
                        detail: "Failed to publish realtime event to Redis".to_string(),
                        source: Box::new(err),
                    };

                    if attempt >= PUBLISHER_MAX_ATTEMPTS || !is_transient_error(&app_err) {
                        return Err(app_err);
                    }

                    let delay_ms = PUBLISHER_INITIAL_RETRY_DELAY_MS
                        .saturating_mul(2_u64.pow(attempt - 1))
                        .min(PUBLISHER_MAX_RETRY_DELAY_MS);
                    warn!(
                        error = %app_err,
                        attempt,
                        retry_delay_ms = delay_ms,
                        "Redis publish failed, retrying"
                    );
                    sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }
}

// Subscriber retry configuration (background task)
const INITIAL_RETRY_DELAY_SECS: u64 = 1;
const MAX_RETRY_DELAY_SECS: u64 = 60;
const RETRY_DELAY_MULTIPLIER: f64 = 2.0;
const JITTER_PERCENT: f64 = 0.2;

// Publisher retry configuration (HTTP request path)
const PUBLISHER_MAX_ATTEMPTS: u32 = 3;
const PUBLISHER_INITIAL_RETRY_DELAY_MS: u64 = 50;
const PUBLISHER_MAX_RETRY_DELAY_MS: u64 = 200;

fn spawn_subscriber(redis_url: &str, registry: Arc<WsRegistry>) {
    let redis_url = redis_url.to_string();
    tokio::spawn(async move {
        run_subscription_loop_with_retry(&redis_url, registry).await;
    });
}

fn is_transient_error(err: &AppError) -> bool {
    if let AppError::Config { .. } = err {
        return false;
    }

    let error_msg = err.to_string().to_lowercase();

    if error_msg.contains("authentication failed")
        || error_msg.contains("invalid redis_url")
        || error_msg.contains("unsupported")
        || error_msg.contains("non-tcp protocol")
    {
        return false;
    }

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

    if let Some(source) = StdError::source(err) {
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

    true
}

fn calculate_retry_delay(attempt: u32) -> Duration {
    let base_delay =
        INITIAL_RETRY_DELAY_SECS as f64 * RETRY_DELAY_MULTIPLIER.powi(attempt as i32 - 1);
    let capped_delay = base_delay.min(MAX_RETRY_DELAY_SECS as f64);

    let jitter_range = capped_delay * JITTER_PERCENT;
    let jitter = (random::<f64>() * 2.0 - 1.0) * jitter_range;
    let final_delay = (capped_delay + jitter).max(0.1);

    Duration::from_secs_f64(final_delay)
}

async fn run_subscription_loop_with_retry(redis_url: &str, registry: Arc<WsRegistry>) {
    let mut attempt = 0u32;

    loop {
        attempt += 1;

        let loop_res = run_subscription_loop(redis_url, registry.clone()).await;
        match loop_res {
            Ok(()) => {
                info!("Redis subscription loop completed normally");
                break;
            }
            Err(err) => {
                if !is_transient_error(&err) {
                    error!(
                        error = %err,
                        attempt,
                        "Redis subscription failed with permanent error, exiting"
                    );
                    break;
                }

                let delay = calculate_retry_delay(attempt);
                warn!(
                    error = %err,
                    attempt,
                    retry_delay_secs = delay.as_secs_f64(),
                    "Redis subscription failed, retrying"
                );
                sleep(delay).await;

                if attempt >= 20 {
                    attempt = 10;
                }
            }
        }
    }
}

async fn run_subscription_loop(redis_url: &str, registry: Arc<WsRegistry>) -> Result<(), AppError> {
    let client = Client::open(redis_url).map_err(|err| AppError::Internal {
        code: ErrorCode::ConfigError,
        detail: format!("Failed to create Redis client: {err}"),
        source: Box::new(err),
    })?;

    let conn_info = client.get_connection_info();

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

    let mut pubsub = PubSub::new(conn_info.redis_settings(), stream)
        .await
        .map_err(|err| AppError::Internal {
            code: ErrorCode::ConfigError,
            detail: format!("Failed to create Redis pubsub: {err}"),
            source: Box::new(err),
        })?;

    info!("Subscribing to Redis patterns 'game:*' and 'user:*'");
    pubsub
        .psubscribe("game:*")
        .await
        .map_err(|err| AppError::Internal {
            code: ErrorCode::ConfigError,
            detail: "Failed to subscribe to Redis channel pattern game:*".to_string(),
            source: Box::new(err),
        })?;
    pubsub
        .psubscribe("user:*")
        .await
        .map_err(|err| AppError::Internal {
            code: ErrorCode::ConfigError,
            detail: "Failed to subscribe to Redis channel pattern user:*".to_string(),
            source: Box::new(err),
        })?;

    info!("Redis subscription established, processing messages");

    let mut stream = pubsub.into_on_message();

    loop {
        let started = Instant::now();
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

        match serde_json::from_str::<EventEnvelope>(&payload) {
            Ok(EventEnvelope::GameStateAvailable { game_id, version }) => {
                // Optional safety: ensure channel matches the kind we expect.
                if parse_game_channel(&channel).is_none() {
                    warn!(
                        channel = %channel,
                        game_id,
                        "[WS BROKER] GameStateAvailable received on non-game channel"
                    );
                }
                registry.broadcast_game_state_available(game_id, version);
            }

            Ok(EventEnvelope::YourTurn {
                user_id,
                game_id,
                version,
            }) => {
                // Optional safety: ensure channel matches the kind we expect.
                if parse_user_channel(&channel).is_none() {
                    warn!(
                        channel = %channel,
                        user_id,
                        "[WS BROKER] YourTurn received on non-user channel"
                    );
                }
                registry
                    .broadcast_to_user_excl_topic(user_id, HubEvent::YourTurn { game_id, version });
            }

            Ok(EventEnvelope::LongWaitInvalidated { game_id }) => {
                // User-scoped delivery: LW invalidations must reach the user even when not
                // subscribed to the game topic (e.g. lobby/header navigation).
                let Some(user_id) = parse_user_channel(&channel) else {
                    warn!(
                        channel = %channel,
                        game_id,
                        "[WS BROKER] LongWaitInvalidated received on non-user channel"
                    );
                    continue;
                };

                registry.broadcast_to_user(user_id, HubEvent::LongWaitInvalidated { game_id });
            }

            Err(err) => {
                error!(
                    error = %err,
                    elapsed_ms = started.elapsed().as_millis(),
                    channel = %channel,
                    "Failed to decode Redis realtime payload"
                );
            }
        }
    }

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

fn parse_game_channel(channel: &str) -> Option<i64> {
    let mut parts = channel.split(':');
    let prefix = parts.next()?;
    if prefix != "game" {
        return None;
    }
    let id = parts.next()?;
    id.parse().ok()
}

fn parse_user_channel(channel: &str) -> Option<i64> {
    let mut parts = channel.split(':');
    let prefix = parts.next()?;
    if prefix != "user" {
        return None;
    }
    let id = parts.next()?;
    id.parse().ok()
}
