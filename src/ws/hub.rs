//! WebSocket Hub implementation
//!
//! Manages WebSocket connections and broadcasts messages

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    Extension,
};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::middleware::auth::CurrentUser;
use crate::state::AppState;
use crate::task::{TaskNotification, TASK_MANAGER};

/// Global WebSocket hub instance
pub static HUB: std::sync::LazyLock<Hub> = std::sync::LazyLock::new(Hub::new);

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    #[serde(rename = "taskInfo")]
    TaskInfo(serde_json::Value),
    #[serde(rename = "taskDeleted")]
    TaskDeleted(String),
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}

/// WebSocket Hub
pub struct Hub {
    /// Connected clients by user ID
    clients: DashMap<i64, Vec<mpsc::UnboundedSender<WsMessage>>>,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            clients: DashMap::new(),
        }
    }

    /// Register a new client
    pub fn register(&self, user_id: i64, tx: mpsc::UnboundedSender<WsMessage>) {
        self.clients.entry(user_id).or_insert_with(Vec::new).push(tx);
        tracing::debug!("WebSocket client registered for user {}", user_id);
    }

    /// Unregister a client
    pub fn unregister(&self, user_id: i64, tx: &mpsc::UnboundedSender<WsMessage>) {
        if let Some(mut clients) = self.clients.get_mut(&user_id) {
            clients.retain(|c| !c.same_channel(tx));
            if clients.is_empty() {
                drop(clients);
                self.clients.remove(&user_id);
            }
        }
        tracing::debug!("WebSocket client unregistered for user {}", user_id);
    }

}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket upgrade handler
pub async fn serve_ws(
    ws: WebSocketUpgrade,
    State(_state): State<AppState>,
    Extension(current_user): Extension<CurrentUser>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, current_user))
}

/// Handle a WebSocket connection
async fn handle_socket(socket: WebSocket, user: CurrentUser) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

    // Register client
    HUB.register(user.id, tx.clone());

    // Subscribe to task notifications
    let mut task_rx = TASK_MANAGER.subscribe();

    // Spawn task to handle outgoing messages
    let user_id = user.id;
    let tx_clone = tx.clone();
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Handle messages from channel
                Some(msg) = rx.recv() => {
                    let text = serde_json::to_string(&msg).unwrap_or_default();
                    if sender.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
                // Handle task notifications
                Ok(notification) = task_rx.recv() => {
                    // Only send notifications for this user
                    let should_send = match &notification {
                        TaskNotification::TaskInfo(info) => info.user_id == user_id,
                        TaskNotification::TaskDeleted(_) => true, // Send all delete notifications
                    };

                    if should_send {
                        let message = match notification {
                            TaskNotification::TaskInfo(info) => {
                                WsMessage::TaskInfo(serde_json::to_value(info).unwrap_or_default())
                            }
                            TaskNotification::TaskDeleted(id) => WsMessage::TaskDeleted(id),
                        };
                        let text = serde_json::to_string(&message).unwrap_or_default();
                        if sender.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                }
                else => break,
            }
        }
    });

    // Handle incoming messages
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Parse message
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        match ws_msg {
                            WsMessage::Ping => {
                                let _ = tx_clone.send(WsMessage::Pong);
                            }
                            _ => {}
                        }
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }

    // Unregister client
    HUB.unregister(user.id, &tx);
}
