use std::sync::Arc;

use actix::prelude::*;
use dashmap::DashMap;
use redis::aio::{Connection, ConnectionManager};
use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio_stream::StreamExt;
use tracing::{error, info};
use uuid::Uuid;

use crate::error::AppError;
use crate::errors::ErrorCode;
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct SnapshotBroadcast {
    pub lock_version: i32,
}

#[derive(Default)]
pub struct GameSessionRegistry {
    sessions: DashMap<i64, DashMap<Uuid, Recipient<SnapshotBroadcast>>>,
}

impl GameSessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    pub fn register(&self, game_id: i64, recipient: Recipient<SnapshotBroadcast>) -> Uuid {
        let token = Uuid::new_v4();
        let entry = self.sessions.entry(game_id).or_insert_with(DashMap::new);
        entry.insert(token, recipient);
        token
    }

    pub fn unregister(&self, game_id: i64, token: Uuid) {
        if let Some(entry) = self.sessions.get(&game_id) {
            entry.remove(&token);
            if entry.is_empty() {
                self.sessions.remove(&game_id);
            }
        }
    }

    pub fn broadcast(&self, game_id: i64, message: SnapshotBroadcast) {
        if let Some(entry) = self.sessions.get(&game_id) {
            for recipient in entry.iter() {
                let _ = recipient.value().do_send(message.clone());
            }
        }
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

        let conn = client
            .get_async_connection()
            .await
            .map_err(|err| AppError::Internal {
                code: ErrorCode::ConfigError,
                detail: "Unable to connect to Redis for realtime sync".to_string(),
                source: Box::new(err),
            })?;

        let manager = ConnectionManager::new(conn)
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

        spawn_subscriber(client, registry);

        Ok(broker)
    }

    pub fn registry(&self) -> Arc<GameSessionRegistry> {
        self.registry.clone()
    }

    pub async fn publish_snapshot(&self, game_id: i64, lock_version: i32) -> Result<(), AppError> {
        let mut publisher = self.publisher.lock().await;
        let envelope = RedisEnvelope {
            game_id,
            lock_version,
        };
        let encoded = serde_json::to_string(&envelope).map_err(|err| AppError::Internal {
            code: ErrorCode::InternalError,
            detail: "Failed to serialize snapshot broadcast".to_string(),
            source: Box::new(err),
        })?;
        let channel = format!("game:{game_id}");
        publisher
            .publish(channel, encoded)
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
    lock_version: i32,
}

fn spawn_subscriber(client: Client, registry: Arc<GameSessionRegistry>) {
    tokio::spawn(async move {
        if let Err(err) = run_subscription_loop(client, registry).await {
            error!(error = %err, "Redis subscription loop exited");
        }
    });
}

async fn run_subscription_loop(
    client: Client,
    registry: Arc<GameSessionRegistry>,
) -> Result<(), AppError> {
    let mut connection: Connection =
        client
            .get_async_connection()
            .await
            .map_err(|err| AppError::Internal {
                code: ErrorCode::ConfigError,
                detail: "Unable to connect to Redis for subscription".to_string(),
                source: Box::new(err),
            })?;
    let mut pubsub = connection.as_pubsub();
    pubsub
        .psubscribe("game:*")
        .await
        .map_err(|err| AppError::Internal {
            code: ErrorCode::ConfigError,
            detail: "Failed to subscribe to Redis channel pattern".to_string(),
            source: Box::new(err),
        })?;

    let mut stream = pubsub.on_message();
    while let Some(msg) = stream.next().await {
        if let (Ok(channel), Ok(payload)) =
            (msg.get_channel::<String>(), msg.get_payload::<String>())
        {
            if let Some(game_id) = parse_channel(&channel) {
                match serde_json::from_str::<RedisEnvelope>(&payload) {
                    Ok(envelope) => {
                        registry.broadcast(
                            game_id,
                            SnapshotBroadcast {
                                lock_version: envelope.lock_version,
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
