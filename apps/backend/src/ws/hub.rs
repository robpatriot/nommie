use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use actix::prelude::*;
use dashmap::DashMap;
use redis::aio::{ConnectionManager, PubSub};
use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
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

        let (was_present, now_empty) = if let Some(entry) = self.sessions.get_mut(&game_id) {
            // Acquire mutable guard - allows mutation of inner map
            let was_present = entry.remove(&token).is_some();
            let now_empty = entry.is_empty();
            // Guard is dropped here when entry goes out of scope
            (was_present, now_empty)
        } else {
            (false, false)
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
        let mut publisher = self.publisher.lock().await;
        let envelope = RedisEnvelope { game_id, version };
        let encoded = serde_json::to_string(&envelope).map_err(|err| AppError::Internal {
            code: ErrorCode::InternalError,
            detail: "Failed to serialize snapshot broadcast".to_string(),
            source: Box::new(err),
        })?;
        let channel = format!("game:{game_id}");
        publisher
            .publish::<_, _, ()>(channel, encoded)
            .await
            .map_err(|err| AppError::Internal {
                code: ErrorCode::InternalError,
                detail: "Failed to publish snapshot to Redis".to_string(),
                source: Box::new(err),
            })?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct RedisEnvelope {
    game_id: i64,
    version: i32,
}

fn spawn_subscriber(redis_url: &str, registry: Arc<GameSessionRegistry>) {
    let redis_url = redis_url.to_string();
    tokio::spawn(async move {
        if let Err(err) = run_subscription_loop(&redis_url, registry).await {
            error!(error = %err, "Redis subscription loop exited");
        }
    });
}

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

    pubsub
        .psubscribe("game:*")
        .await
        .map_err(|err| AppError::Internal {
            code: ErrorCode::ConfigError,
            detail: "Failed to subscribe to Redis channel pattern".to_string(),
            source: Box::new(err),
        })?;

    let mut stream = pubsub.into_on_message();
    while let Some(msg) = stream.next().await {
        let channel_result = msg.get_channel::<String>();
        let payload_result = msg.get_payload::<String>();
        if let (Ok(channel), Ok(payload)) = (channel_result, payload_result) {
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
    }

    info!("Redis subscription loop completed");
    Ok(())
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
