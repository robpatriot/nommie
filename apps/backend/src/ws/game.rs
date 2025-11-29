use std::sync::Arc;
use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::Serialize;
use serde_json::to_string;
use tracing::{info, warn};
use uuid::Uuid;

use crate::extractors::current_user::CurrentUser;
use crate::extractors::game_id::GameId;
use crate::extractors::game_membership::GameMembership;
use crate::routes::games::{build_snapshot_response, GameSnapshotResponse};
use crate::state::app_state::AppState;
use crate::ws::hub::{GameSessionRegistry, SnapshotBroadcast};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(40);

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OutgoingMessage {
    Snapshot { data: GameSnapshotResponse },
    Ack { message: &'static str },
    Error { code: &'static str, message: String },
}

pub async fn upgrade(
    req: HttpRequest,
    stream: web::Payload,
    game_id: GameId,
    current_user: CurrentUser,
    _membership: GameMembership,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let (snapshot_response, _) =
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
    user_sub: String,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
    last_heartbeat: Instant,
    pending_messages: Vec<OutgoingMessage>,
    registry: Option<Arc<GameSessionRegistry>>,
    registry_token: Option<Uuid>,
    last_lock_version: i32,
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
            user_sub: current_user.sub,
            current_user,
            app_state,
            last_heartbeat: Instant::now(),
            pending_messages,
            registry,
            registry_token: None,
            last_lock_version: initial_lock_version,
        }
    }

    fn start_heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |actor, ctx| {
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
            let token = registry.register(self.game_id, ctx.address().recipient());
            self.registry_token = Some(token);
        }

        self.start_heartbeat(ctx);
        self.flush_pending(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        if let (Some(registry), Some(token)) = (&self.registry, self.registry_token) {
            registry.unregister(self.game_id, token);
        }

        info!(
            session_id = %self.session_id,
            game_id = self.game_id,
            user_id = self.user_id,
            "Websocket session stopped"
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
            Ok(ws::Message::Text(text)) => {
                warn!(
                    session_id = %self.session_id,
                    game_id = self.game_id,
                    user_id = self.user_id,
                    text = %text,
                    "Unexpected websocket text message"
                );
            }
            Ok(ws::Message::Binary(_)) => {}
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {}
            Ok(ws::Message::Nop) => {}
            Err(err) => {
                warn!(
                    session_id = %self.session_id,
                    game_id = self.game_id,
                    user_id = self.user_id,
                    error = %err,
                    "Websocket protocol error"
                );
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
                    Ok((snapshot, _viewer_seat)) => {
                        actor.last_lock_version = snapshot.lock_version;
                        let outgoing = OutgoingMessage::Snapshot { data: snapshot };
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
