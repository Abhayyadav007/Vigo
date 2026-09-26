//! WebSocket endpoints. Each session authenticates with a Firebase ID token
//! in its first message, then streams Redis Pub/Sub events (via the
//! per-instance [`hub::PubSubHub`]) so it works across backend instances.

pub mod hub;

use std::time::Duration;

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{CloseFrame, Message, Utf8Bytes, WebSocket},
    },
    response::Response,
};
use chrono::Utc;
use tokio::{sync::broadcast::error::RecvError, time::Instant};
use uuid::Uuid;

use crate::{
    auth::Role,
    cache::events,
    dto::{
        order::OrderStatusChanged,
        ws::{WsClientMessage, WsServerMessage},
    },
    services::auth_service,
    state::AppState,
};

const AUTH_TIMEOUT: Duration = Duration::from_secs(10);
const PING_EVERY: Duration = Duration::from_secs(25);

/// Close codes (4000-4999 are application-defined).
const CLOSE_UNAUTHORIZED: u16 = 4001;
const CLOSE_FORBIDDEN: u16 = 4003;
const CLOSE_TOKEN_EXPIRED: u16 = 4008;

/// `GET /v1/ws/picker`: live order events for the picker's store.
pub async fn picker(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| session(socket, state, Audience::StoreStaff(Role::Picker)))
}

#[derive(Clone, Copy)]
enum Audience {
    /// Staff of one store (picker now, rider in phase 6): their store's feed.
    StoreStaff(Role),
}

struct Authed {
    channel: String,
    expires_at: Instant,
}

async fn session(mut socket: WebSocket, state: AppState, audience: Audience) {
    let authed = match authenticate(&mut socket, &state, audience).await {
        Ok(a) => a,
        Err((code, reason)) => {
            let _ = send(
                &mut socket,
                &WsServerMessage::Error {
                    code: code_name(code).into(),
                    message: reason.into(),
                },
            )
            .await;
            close(&mut socket, code, reason).await;
            return;
        }
    };

    let mut sub = state.hub.subscribe(&authed.channel).await;
    if send(&mut socket, &WsServerMessage::Ready).await.is_err() {
        return;
    }
    let mut ping = tokio::time::interval(PING_EVERY);
    ping.tick().await;

    loop {
        tokio::select! {
            msg = sub.recv() => {
                let out = match msg {
                    Ok(payload) => match serde_json::from_str::<OrderStatusChanged>(&payload) {
                        Ok(event) => WsServerMessage::Order { event },
                        Err(_) => continue,
                    },
                    Err(RecvError::Lagged(_)) => WsServerMessage::Resync,
                    Err(RecvError::Closed) => break,
                };
                if send(&mut socket, &out).await.is_err() {
                    break;
                }
            }
            incoming = socket.recv() => match incoming {
                // Clients have nothing to say after auth; pongs and pings are handled by axum.
                Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                Some(Ok(_)) => {}
            },
            _ = ping.tick() => {
                if socket.send(Message::Ping(Default::default())).await.is_err() {
                    break;
                }
            }
            () = tokio::time::sleep_until(authed.expires_at) => {
                // The client reconnects with a fresh token.
                close(&mut socket, CLOSE_TOKEN_EXPIRED, "token expired").await;
                break;
            }
        }
    }
}

async fn authenticate(
    socket: &mut WebSocket,
    state: &AppState,
    audience: Audience,
) -> Result<Authed, (u16, &'static str)> {
    let first = tokio::time::timeout(AUTH_TIMEOUT, socket.recv())
        .await
        .map_err(|_| (CLOSE_UNAUTHORIZED, "auth timeout"))?;
    let Some(Ok(Message::Text(text))) = first else {
        return Err((CLOSE_UNAUTHORIZED, "expected an auth message"));
    };
    let WsClientMessage::Auth { token } = serde_json::from_str(&text)
        .map_err(|_| (CLOSE_UNAUTHORIZED, "expected an auth message"))?;

    let claims = state
        .verifier
        .verify(&token)
        .await
        .map_err(|_| (CLOSE_UNAUTHORIZED, "invalid token"))?;
    let session = auth_service::resolve_session(state, &claims.sub)
        .await
        .map_err(|_| (CLOSE_FORBIDDEN, "not allowed"))?;

    let channel = match audience {
        Audience::StoreStaff(role) => {
            let store = session.store_id.filter(|_| session.role == role);
            let store: Uuid = store.ok_or((CLOSE_FORBIDDEN, "not allowed"))?;
            events::store_channel(store)
        }
    };
    let secs_left = (claims.exp - Utc::now().timestamp()).max(0);
    Ok(Authed {
        channel,
        expires_at: Instant::now() + Duration::from_secs(secs_left.unsigned_abs()),
    })
}

fn code_name(code: u16) -> &'static str {
    match code {
        CLOSE_FORBIDDEN => "FORBIDDEN",
        CLOSE_TOKEN_EXPIRED => "TOKEN_EXPIRED",
        _ => "UNAUTHORIZED",
    }
}

async fn send(socket: &mut WebSocket, msg: &WsServerMessage) -> Result<(), axum::Error> {
    let text = serde_json::to_string(msg).unwrap_or_default();
    socket.send(Message::Text(Utf8Bytes::from(text))).await
}

async fn close(socket: &mut WebSocket, code: u16, reason: &'static str) {
    let _ = socket
        .send(Message::Close(Some(CloseFrame {
            code,
            reason: Utf8Bytes::from_static(reason),
        })))
        .await;
}
