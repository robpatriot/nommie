use std::sync::Arc;
use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::Serialize;
use serde_json::to_string;
use tracing::{info, warn};
use uuid::Uuid;

use crate::domain::state::Seat;
use crate::extractors::current_user::CurrentUser;
use crate::extractors::game_id::GameId;
use crate::extractors::game_membership::GameMembership;
use crate::routes::games::{build_snapshot_response, GameSnapshotResponse};
use crate::state::app_state::AppState;
use crate::ws::hub::{GameSessionRegistry, SnapshotBroadcast};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(40);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Shutdown;

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
enum OutgoingMessage {
    Snapshot {
        data: GameSnapshotResponse,
        viewer_seat: Option<Seat>,
    },
    Ack {
        message: &'static str,
    },
}

pub async fn upgrade(
    req: HttpRequest,
    stream: web::Payload,
    game_id: GameId,
    current_user: CurrentUser,
    _membership: GameMembership,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let (snapshot_response, viewer_seat) =
        build_snapshot_response(Some(&req), &app_state, game_id.0, &current_user)
            .await
            .map_err(Error::from)?;

    let registry = app_state.realtime.as_ref().map(|broker| broker.registry());
    let initial_lock_version = snapshot_response.lock_version;

    let session = GameWsSession::new(
        app_state.clone(),
        registry,
        game_id.0,
        current_user,
        vec![
            OutgoingMessage::Ack {
                message: "connected",
            },
            OutgoingMessage::Snapshot {
                data: snapshot_response,
                viewer_seat,
            },
        ],
        initial_lock_version,
    );

    ws::start(session, &req, stream)
}

pub struct GameWsSession {
    session_id: Uuid,
    game_id: i64,
    user_id: i64,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
    last_heartbeat: Instant,
    pending_messages: Vec<OutgoingMessage>,
    registry: Option<Arc<GameSessionRegistry>>,
    registry_token: Option<Uuid>,
    last_lock_version: i32,
    heartbeat_handle: Option<actix::SpawnHandle>,
}

impl GameWsSession {
    fn new(
        app_state: web::Data<AppState>,
        registry: Option<Arc<GameSessionRegistry>>,
        game_id: i64,
        current_user: CurrentUser,
        pending_messages: Vec<OutgoingMessage>,
        initial_lock_version: i32,
    ) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            game_id,
            user_id: current_user.id,
            current_user,
            app_state,
            last_heartbeat: Instant::now(),
            pending_messages,
            registry,
            registry_token: None,
            last_lock_version: initial_lock_version,
            heartbeat_handle: None,
        }
    }

    fn start_heartbeat(&mut self, ctx: &mut ws::WebsocketContext<Self>) {
        let handle = ctx.run_interval(HEARTBEAT_INTERVAL, |actor, ctx| {
            if Instant::now().duration_since(actor.last_heartbeat) > CLIENT_TIMEOUT {
                warn!(
                    session_id = %actor.session_id,
                    game_id = actor.game_id,
                    user_id = actor.user_id,
                    "Websocket client heartbeat timed out"
                );
                ctx.close(Some(ws::CloseReason::from(ws::CloseCode::Normal)));
                ctx.stop();
                return;
            }

            ctx.ping(b"keepalive");
        });
        self.heartbeat_handle = Some(handle);
    }

    fn flush_pending(&mut self, ctx: &mut ws::WebsocketContext<Self>) {
        for message in self.pending_messages.drain(..) {
            match to_string(&message) {
                Ok(payload) => ctx.text(payload),
                Err(err) => warn!(
                    session_id = %self.session_id,
                    error = %err,
                    "Failed to serialize websocket message"
                ),
            }
        }
    }
}

impl Actor for GameWsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(
            session_id = %self.session_id,
            game_id = self.game_id,
            user_id = self.user_id,
            "Websocket session started"
        );

        if let Some(registry) = &self.registry {
            let recipient = ctx.address().recipient();
            let addr = ctx.address();
            let token = registry.register(self.game_id, recipient, addr);
            self.registry_token = Some(token);
        }

        self.start_heartbeat(ctx);
        self.flush_pending(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        let stopped_start = std::time::Instant::now();
        if let (Some(registry), Some(token)) = (&self.registry, self.registry_token) {
            let before = registry.active_connections_count();
            registry.unregister(self.game_id, token);
            let after = registry.active_connections_count();
            info!(
                session_id = %self.session_id,
                game_id = self.game_id,
                user_id = self.user_id,
                before_count = before,
                after_count = after,
                "[WS SESSION] Actor stopped() - unregistered (fallback, token was still set)"
            );
        } else {
            info!(
                session_id = %self.session_id,
                game_id = self.game_id,
                user_id = self.user_id,
                duration_ms = stopped_start.elapsed().as_millis(),
                "[WS SESSION] Actor stopped() - already unregistered"
            );
        }

        info!(
            session_id = %self.session_id,
            game_id = self.game_id,
            user_id = self.user_id,
            duration_ms = stopped_start.elapsed().as_millis(),
            "[WS SESSION] Websocket session fully stopped"
        );
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GameWsSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(payload)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&payload);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(_)) => {
                // Any received message confirms the connection is alive
                self.last_heartbeat = Instant::now();
                // Frontend doesn't send text messages, so this is unexpected but harmless
                // Silently ignore
            }
            Ok(ws::Message::Binary(_)) => {
                // Any received message confirms the connection is alive
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                // Don't update heartbeat on close - connection is terminating
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                // Any received message confirms the connection is alive
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Nop) => {
                // Any received message confirms the connection is alive
                self.last_heartbeat = Instant::now();
            }
            Err(err) => {
                // Don't update heartbeat on protocol errors
                warn!(
                    session_id = %self.session_id,
                    game_id = self.game_id,
                    user_id = self.user_id,
                    error = %err,
                    "Websocket protocol error"
                );
                ctx.close(Some(ws::CloseReason::from(ws::CloseCode::Error)));
                ctx.stop();
            }
        }
    }
}

impl Handler<SnapshotBroadcast> for GameWsSession {
    type Result = ();

    fn handle(&mut self, msg: SnapshotBroadcast, ctx: &mut Self::Context) -> Self::Result {
        if msg.lock_version <= self.last_lock_version {
            return;
        }

        let app_state = self.app_state.clone();
        let current_user = self.current_user.clone();
        let game_id = self.game_id;

        ctx.spawn(
            async move { build_snapshot_response(None, &app_state, game_id, &current_user).await }
                .into_actor(self)
                .map(|result, actor, ctx| match result {
                    Ok((snapshot, viewer_seat)) => {
                        actor.last_lock_version = snapshot.lock_version;
                        let outgoing = OutgoingMessage::Snapshot {
                            data: snapshot,
                            viewer_seat,
                        };
                        match to_string(&outgoing) {
                            Ok(serialized) => ctx.text(serialized),
                            Err(err) => warn!(
                                session_id = %actor.session_id,
                                game_id = actor.game_id,
                                error = %err,
                                "Failed to serialize broadcast snapshot"
                            ),
                        }
                    }
                    Err(err) => {
                        warn!(
                            session_id = %actor.session_id,
                            game_id = actor.game_id,
                            error = %err,
                            "Failed to build snapshot for websocket client"
                        );
                    }
                }),
        );
    }
}

impl Handler<Shutdown> for GameWsSession {
    type Result = ();

    fn handle(&mut self, _msg: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        // Immediately unregister from registry (the stopped() method will also try, but this ensures it happens)
        if let (Some(registry), Some(token)) = (&self.registry, self.registry_token.take()) {
            registry.unregister(self.game_id, token);
        }

        // Cancel heartbeat interval to prevent any further pings
        if let Some(handle) = self.heartbeat_handle.take() {
            ctx.cancel_future(handle);
        }

        // Stop the actor immediately - this will close the connection forcefully
        // without waiting for client acknowledgment. We don't call ctx.close() first
        // because that would send a graceful close frame and wait for acknowledgment,
        // which can cause a 1-second delay during shutdown.
        ctx.stop();
    }
}
