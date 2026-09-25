use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::order::OrderStatusChanged;

/// Client -> server. The first message must be `auth`.
#[derive(Debug, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum WsClientMessage {
    /// Firebase ID token, sent as the first message (tokens in URLs leak into logs).
    Auth { token: String },
}

/// Server -> client.
#[derive(Debug, Serialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export)]
pub enum WsServerMessage {
    /// Authenticated and subscribed.
    Ready,
    /// An order changed status.
    Order { event: OrderStatusChanged },
    /// Messages were dropped (slow client); refetch current state.
    Resync,
    /// Fatal; the server closes the socket after sending it.
    Error { code: String, message: String },
}
