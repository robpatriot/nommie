use std::sync::atomic::{AtomicUsize, Ordering};

use actix::prelude::*;
use dashmap::{DashMap, DashSet};
use tracing::{info, warn};
use uuid::Uuid;

pub use crate::ws::broker::RealtimeBroker;
use crate::ws::protocol::Topic;
use crate::ws::session::{HubEvent, Shutdown, WsSession};

/// Recipient + address for a single websocket session
type SessionHandle = (Recipient<HubEvent>, Addr<WsSession>);

/// WsRegistry is an in-memory routing table for websocket connections.
///
/// Responsibilities:
/// - Track active websocket connections
/// - Index connections by user_id and subscribed topics
/// - Provide fan-out helpers for user- and topic-scoped events
///
/// Non-responsibilities (by design):
/// - WebSocket protocol handling
/// - Authorization
/// - Snapshot construction
/// - Redis or other I/O
pub struct WsRegistry {
    // user_id -> (conn_id -> session)
    by_user: DashMap<i64, DashMap<Uuid, SessionHandle>>,
    // topic -> (conn_id -> session)
    by_topic: DashMap<Topic, DashMap<Uuid, SessionHandle>>,
    // conn_id -> metadata
    meta: DashMap<Uuid, ConnMeta>,
    active_connections: AtomicUsize,
}

#[derive(Clone)]
struct ConnMeta {
    user_id: i64,
    topics: DashSet<Topic>,
}

impl WsRegistry {
    pub fn new() -> Self {
        Self {
            by_user: DashMap::new(),
            by_topic: DashMap::new(),
            meta: DashMap::new(),
            active_connections: AtomicUsize::new(0),
        }
    }

    pub fn register_connection(
        &self,
        user_id: i64,
        conn_id: Uuid,
        recipient: Recipient<HubEvent>,
        addr: Addr<WsSession>,
    ) {
        self.meta.insert(
            conn_id,
            ConnMeta {
                user_id,
                topics: DashSet::new(),
            },
        );

        let user_entry = self.by_user.entry(user_id).or_default();
        user_entry.insert(conn_id, (recipient, addr));

        let active = self.active_connections.fetch_add(1, Ordering::Relaxed) + 1;
        info!(
            user_id,
            conn_id = %conn_id,
            active_connections = active,
            "[WS HUB] registered connection"
        );
    }

    pub fn unregister_connection(&self, conn_id: Uuid) {
        let Some((_, meta)) = self.meta.remove(&conn_id) else {
            warn!(conn_id = %conn_id, "[WS HUB] unregister: conn_id not found");
            return;
        };

        if let Some(user_map_ref) = self.by_user.get(&meta.user_id) {
            user_map_ref.remove(&conn_id);
            let should_remove_user = user_map_ref.is_empty();
            drop(user_map_ref);
            if should_remove_user {
                self.by_user.remove(&meta.user_id);
            }
        }

        for topic_ref in meta.topics.iter() {
            let topic = topic_ref.key().clone();
            if let Some(topic_map_ref) = self.by_topic.get(&topic) {
                topic_map_ref.remove(&conn_id);
                let should_remove_topic = topic_map_ref.is_empty();
                drop(topic_map_ref);
                if should_remove_topic {
                    self.by_topic.remove(&topic);
                }
            }
        }

        let prev = self.active_connections.load(Ordering::Relaxed);
        if prev > 0 {
            self.active_connections.fetch_sub(1, Ordering::Relaxed);
        }

        info!(
            user_id = meta.user_id,
            conn_id = %conn_id,
            active_connections_before = prev,
            active_connections_after = prev.saturating_sub(1),
            "[WS HUB] unregistered connection"
        );
    }

    pub fn subscribe(&self, conn_id: Uuid, topic: Topic) {
        let (user_id, already_subscribed) = {
            let Some(meta_ref) = self.meta.get_mut(&conn_id) else {
                return;
            };
            let user_id = meta_ref.user_id;
            let already = meta_ref.topics.contains(&topic);
            if !already {
                meta_ref.topics.insert(topic.clone());
            }
            (user_id, already)
        };

        if already_subscribed {
            return;
        }

        let handle: SessionHandle = {
            let Some(user_map_ref) = self.by_user.get(&user_id) else {
                return;
            };
            let Some(handle_ref) = user_map_ref.get(&conn_id) else {
                drop(user_map_ref);
                return;
            };
            let cloned = handle_ref.value().clone();
            drop(handle_ref);
            drop(user_map_ref);
            cloned
        };

        let topic_entry = self.by_topic.entry(topic).or_default();
        topic_entry.insert(conn_id, handle);
    }

    pub fn unsubscribe(&self, conn_id: Uuid, topic: &Topic) {
        let removed = {
            let Some(meta_ref) = self.meta.get_mut(&conn_id) else {
                return;
            };
            meta_ref.topics.remove(topic).is_some()
        };

        if !removed {
            return;
        }

        if let Some(topic_map_ref) = self.by_topic.get(topic) {
            topic_map_ref.remove(&conn_id);
            let should_remove_topic = topic_map_ref.is_empty();
            drop(topic_map_ref);
            if should_remove_topic {
                self.by_topic.remove(topic);
            }
        }
    }

    pub fn broadcast_to_user(&self, user_id: i64, event: HubEvent) {
        if let Some(user_map_ref) = self.by_user.get(&user_id) {
            for entry in user_map_ref.iter() {
                entry.value().0.do_send(event.clone());
            }
        }
    }

    pub fn broadcast_to_user_excl_topic(&self, user_id: i64, event: HubEvent) {
        let Some(excl) = event.excl_topic() else {
            self.broadcast_to_user(user_id, event);
            return;
        };

        // Collect recipients first so we don't hold map refs while sending.
        let mut recipients: Vec<Recipient<HubEvent>> = Vec::new();

        if let Some(user_map_ref) = self.by_user.get(&user_id) {
            for entry in user_map_ref.iter() {
                let conn_id = *entry.key();

                // If meta is missing (race with disconnect), skip.
                let Some(meta_ref) = self.meta.get(&conn_id) else {
                    continue;
                };

                // Exclude sessions already subscribed to the topic.
                if meta_ref.topics.contains(&excl) {
                    continue;
                }

                recipients.push(entry.value().0.clone());
            }
        }

        for r in recipients {
            r.do_send(event.clone());
        }
    }

    pub fn broadcast_to_topic(&self, topic: &Topic, event: HubEvent) {
        if let Some(topic_map_ref) = self.by_topic.get(topic) {
            for entry in topic_map_ref.iter() {
                entry.value().0.do_send(event.clone());
            }
        }
    }

    pub fn broadcast_game_state_available(&self, game_id: i64, version: i32) {
        let topic = Topic::Game { id: game_id };
        self.broadcast_to_topic(
            &topic,
            HubEvent::GameStateAvailable {
                topic: topic.clone(),
                version,
            },
        );
    }

    pub fn active_connections_count(&self) -> usize {
        self.active_connections.load(Ordering::Relaxed)
    }

    pub fn close_all_connections(&self) -> Vec<actix::dev::Request<WsSession, Shutdown>> {
        let mut addrs = Vec::new();
        for user_entry in self.by_user.iter() {
            for conn_entry in user_entry.value().iter() {
                let (_, addr) = conn_entry.value();
                addrs.push(addr.clone());
            }
        }
        addrs.into_iter().map(|addr| addr.send(Shutdown)).collect()
    }
}

impl Default for WsRegistry {
    fn default() -> Self {
        Self::new()
    }
}
