use std::sync::Arc;
use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::txn::SharedTxn;
use crate::extractors::current_user::CurrentUser;
use crate::protocol::game_state::ViewerState;
use crate::state::app_state::AppState;
use crate::ws::game;
use crate::ws::hub::WsRegistry;
use crate::ws::protocol::{ClientMsg, ErrorCode, ServerMsg, Topic, PROTOCOL_VERSION};
use crate::AppError;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(20);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(40);

#[derive(Message)]
#[rtype(result = "()")]
pub struct Shutdown;

pub async fn upgrade(
    req: HttpRequest,
    stream: web::Payload,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let conn_id = Uuid::new_v4();
    let registry = app_state.websocket_registry();

    // IMPORTANT: In tests, this is injected by TestTxnInjector so websocket handlers
    // can see uncommitted rows. In production this will be None.
    let shared_txn = SharedTxn::from_req(&req);

    let session = WsSession::new(conn_id, current_user, app_state, registry, shared_txn);
    ws::start(session, &req, stream)
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum HubEvent {
    GameStateAvailable { topic: Topic, version: i32 },
    YourTurn { game_id: i64, version: i32 },
    LongWaitInvalidated { game_id: i64 },
}

impl HubEvent {
    /// If present, this is the topic to exclude when using
    /// `broadcast_to_user_excl_topic` (i.e. don't notify sessions already
    /// subscribed to the topic that will receive the primary stream).
    pub fn excl_topic(&self) -> Option<Topic> {
        match self {
            // YourTurn is an out-of-band hint; exclude sessions already in the game.
            HubEvent::YourTurn { game_id, .. } => Some(Topic::Game { id: *game_id }),
            HubEvent::LongWaitInvalidated { .. } => None,

            // By default, other events do not exclude any topic subscribers.
            _ => None,
        }
    }
}

pub struct WsSession {
    conn_id: Uuid,
    user_id: i64,
    current_user: CurrentUser,
    app_state: web::Data<AppState>,
    registry: Option<Arc<WsRegistry>>,

    // Transaction-per-test hook (None in production)
    shared_txn: Option<SharedTxn>,

    last_heartbeat: Instant,
    heartbeat_handle: Option<actix::SpawnHandle>,

    hello_done: bool,
}

impl WsSession {
    fn new(
        conn_id: Uuid,
        current_user: CurrentUser,
        app_state: web::Data<AppState>,
        registry: Option<Arc<WsRegistry>>,
        shared_txn: Option<SharedTxn>,
    ) -> Self {
        Self {
            conn_id,
            user_id: current_user.id,
            current_user,
            app_state,
            registry,
            shared_txn,
            last_heartbeat: Instant::now(),
            heartbeat_handle: None,
            hello_done: false,
        }
    }

    fn send_json(ctx: &mut ws::WebsocketContext<Self>, msg: &ServerMsg) {
        match serde_json::to_string(msg) {
            Ok(payload) => ctx.text(payload),
            Err(err) => warn!(error = %err, "[WS SESSION] failed to serialize outbound message"),
        }
    }

    fn send_error_and_close(
        &self,
        ctx: &mut ws::WebsocketContext<Self>,
        code: ErrorCode,
        message: impl Into<String>,
    ) {
        let msg = ServerMsg::Error {
            code,
            message: message.into(),
        };
        Self::send_json(ctx, &msg);
        ctx.close(Some(ws::CloseReason::from(ws::CloseCode::Error)));
        ctx.stop();
    }

    fn start_heartbeat(&mut self, ctx: &mut ws::WebsocketContext<Self>) {
        let handle = ctx.run_interval(HEARTBEAT_INTERVAL, |actor, ctx| {
            if Instant::now().duration_since(actor.last_heartbeat) > CLIENT_TIMEOUT {
                warn!(
                    conn_id = %actor.conn_id,
                    user_id = actor.user_id,
                    "[WS SESSION] heartbeat timed out"
                );
                ctx.close(Some(ws::CloseReason::from(ws::CloseCode::Normal)));
                ctx.stop();
                return;
            }
            ctx.ping(b"keepalive");
        });
        self.heartbeat_handle = Some(handle);
    }
}

impl Actor for WsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        info!(
            conn_id = %self.conn_id,
            user_id = self.user_id,
            "[WS SESSION] started"
        );

        if let Some(registry) = &self.registry {
            let recipient = ctx.address().recipient::<HubEvent>();
            let addr = ctx.address();
            registry.register_connection(self.user_id, self.conn_id, recipient, addr);
        }

        self.start_heartbeat(ctx);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        if let Some(registry) = &self.registry {
            registry.unregister_connection(self.conn_id);
        }
        info!(
            conn_id = %self.conn_id,
            user_id = self.user_id,
            "[WS SESSION] stopped"
        );
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSession {
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
                self.last_heartbeat = Instant::now();

                let parsed: Result<ClientMsg, _> = serde_json::from_str(&text);
                let Ok(cmd) = parsed else {
                    self.send_error_and_close(ctx, ErrorCode::BadRequest, "Malformed JSON");
                    return;
                };

                match cmd {
                    ClientMsg::Hello { protocol } => {
                        if protocol != PROTOCOL_VERSION {
                            self.send_error_and_close(
                                ctx,
                                ErrorCode::BadProtocol,
                                "Unsupported protocol version",
                            );
                            return;
                        }
                        self.hello_done = true;
                        Self::send_json(
                            ctx,
                            &ServerMsg::HelloAck {
                                protocol: PROTOCOL_VERSION,
                                user_id: self.user_id,
                            },
                        );
                    }

                    ClientMsg::Subscribe { topic } => {
                        if !self.hello_done {
                            self.send_error_and_close(
                                ctx,
                                ErrorCode::BadRequest,
                                "Must send hello first",
                            );
                            return;
                        }

                        let Topic::Game { id: game_id } = topic.clone();

                        let app_state = self.app_state.clone();
                        let user = self.current_user.clone();
                        let registry = self.registry.clone();
                        let conn_id = self.conn_id;
                        let shared_txn = self.shared_txn.clone();

                        ctx.spawn(
                            async move {
                                let txn_opt = shared_txn.as_ref();

                                game::authorize_game_subscription(
                                    txn_opt, &app_state, game_id, &user,
                                )
                                .await?;

                                let (version, game_snapshot, viewer) =
                                    game::build_game_state(txn_opt, &app_state, game_id, &user)
                                        .await?;

                                Ok::<
                                    (i32, crate::domain::snapshot::GameSnapshot, ViewerState),
                                    crate::error::AppError,
                                >((version, game_snapshot, viewer))
                            }
                            .into_actor(self)
                            .map(move |res, _actor, ctx| match res {
                                Ok((version, game_snapshot, viewer)) => {
                                    if let Some(registry) = &registry {
                                        registry.subscribe(conn_id, Topic::Game { id: game_id });
                                    }

                                    // Ordering guarantee: ack then game_state
                                    Self::send_json(
                                        ctx,
                                        &ServerMsg::Ack {
                                            message: "subscribed",
                                        },
                                    );
                                    Self::send_json(
                                        ctx,
                                        &ServerMsg::GameState {
                                            topic: Topic::Game { id: game_id },
                                            version,
                                            game: game_snapshot,
                                            viewer,
                                        },
                                    );
                                }
                                Err(err) => {
                                    tracing::error!(
                                        ?err,
                                        game_id,
                                        conn_id = %conn_id,
                                        "[WS SESSION] subscribe failed"
                                    );

                                    match &err {
                                        AppError::Forbidden { detail, .. } => {
                                            Self::send_json(
                                                ctx,
                                                &ServerMsg::Error {
                                                    code: crate::ws::protocol::ErrorCode::Forbidden,
                                                    message: detail.clone(),
                                                },
                                            );
                                        }

                                        _ => {
                                            ctx.close(Some(ws::CloseReason::from(
                                                ws::CloseCode::Error,
                                            )));
                                            ctx.stop();
                                        }
                                    }
                                }
                            }),
                        );
                    }

                    ClientMsg::Unsubscribe { topic } => {
                        if !self.hello_done {
                            self.send_error_and_close(
                                ctx,
                                ErrorCode::BadRequest,
                                "Must send hello first",
                            );
                            return;
                        }
                        if let Some(registry) = &self.registry {
                            registry.unsubscribe(self.conn_id, &topic);
                        }
                        Self::send_json(
                            ctx,
                            &ServerMsg::Ack {
                                message: "unsubscribed",
                            },
                        );
                    }
                }
            }
            Ok(ws::Message::Binary(_)) => {
                self.last_heartbeat = Instant::now();
                self.send_error_and_close(ctx, ErrorCode::BadRequest, "Binary not supported");
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Nop) => {
                self.last_heartbeat = Instant::now();
            }
            Err(err) => {
                warn!(
                    conn_id = %self.conn_id,
                    user_id = self.user_id,
                    error = %err,
                    "[WS SESSION] protocol error"
                );
                ctx.close(Some(ws::CloseReason::from(ws::CloseCode::Error)));
                ctx.stop();
            }
        }
    }
}

impl Handler<HubEvent> for WsSession {
    type Result = ();

    fn handle(&mut self, msg: HubEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            HubEvent::YourTurn { game_id, version } => {
                Self::send_json(ctx, &ServerMsg::YourTurn { game_id, version });
            }
            HubEvent::LongWaitInvalidated { game_id } => {
                Self::send_json(ctx, &ServerMsg::LongWaitInvalidated { game_id });
            }
            HubEvent::GameStateAvailable { topic, version: _ } => {
                let Topic::Game { id: game_id } = topic.clone();

                let app_state = self.app_state.clone();
                let user = self.current_user.clone();
                let shared_txn = self.shared_txn.clone();

                ctx.spawn(
                    async move {
                        let txn_opt = shared_txn.as_ref();
                        let (ver, game_snapshot, viewer) =
                            game::build_game_state(txn_opt, &app_state, game_id, &user).await?;
                        Ok::<
                            (i32, crate::domain::snapshot::GameSnapshot, ViewerState),
                            crate::error::AppError,
                        >((ver, game_snapshot, viewer))
                    }
                    .into_actor(self)
                    .map(move |res, actor, ctx| match res {
                        Ok((ver, game_snapshot, viewer)) => {
                            Self::send_json(
                                ctx,
                                &ServerMsg::GameState {
                                    topic: Topic::Game { id: game_id },
                                    version: ver,
                                    game: game_snapshot,
                                    viewer,
                                },
                            );
                        }
                        Err(err) => {
                            tracing::error!(
                                ?err,
                                conn_id = %actor.conn_id,
                                user_id = actor.user_id,
                                game_id,
                                "[WS SESSION] build_game_state failed"
                            );

                            match &err {
                                AppError::Forbidden { .. } => {
                                    // Protocol/auth error: keep socket open.
                                    Self::send_json(
                                        ctx,
                                        &ServerMsg::Error {
                                            code: ErrorCode::Forbidden,
                                            message: "Not a member of this game".to_string(),
                                        },
                                    );
                                }
                                _ => {
                                    // Internal failure: close to avoid a "live but broken" session.
                                    ctx.close(Some(ws::CloseReason::from(ws::CloseCode::Error)));
                                    ctx.stop();
                                }
                            }
                        }
                    }),
                );
            }
        }
    }
}

impl Handler<Shutdown> for WsSession {
    type Result = ();

    fn handle(&mut self, _msg: Shutdown, ctx: &mut Self::Context) -> Self::Result {
        if let Some(registry) = &self.registry {
            registry.unregister_connection(self.conn_id);
        }

        if let Some(handle) = self.heartbeat_handle.take() {
            ctx.cancel_future(handle);
        }

        ctx.stop();
    }
}
