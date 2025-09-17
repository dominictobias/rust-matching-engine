use axum::{
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use matcher::types::Trade;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use crate::AppState;

// Notification types that can be sent to users
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum NotificationType {
    #[serde(rename = "trade_fill")]
    TradeFill {
        trade: TradeNotification,
        symbol: String,
    },
    #[serde(rename = "order_cancelled")]
    OrderCancelled {
        order_id: u64,
        symbol: String,
        reason: String,
    },
    #[serde(rename = "connection_established")]
    ConnectionEstablished { user_id: u64, message: String },
}

// Trade notification structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeNotification {
    pub id: u64,
    pub taker_order_id: u64,
    pub maker_order_id: u64,
    pub taker_user_id: u64,
    pub maker_user_id: u64,
    pub quantity: u64,
    pub price_tick: u64,
    pub timestamp: u64,
    pub is_taker: bool, // Whether this user was the taker or maker
}

impl TradeNotification {
    pub fn from_trade(trade: &Trade, user_id: u64) -> Self {
        Self {
            id: trade.id,
            taker_order_id: trade.taker_order_id,
            maker_order_id: trade.maker_order_id,
            taker_user_id: trade.taker_user_id,
            maker_user_id: trade.maker_user_id,
            quantity: trade.quantity,
            price_tick: trade.price_tick,
            timestamp: trade.timestamp,
            is_taker: user_id == trade.taker_user_id,
        }
    }
}

// Global notification manager
pub type NotificationManager = Arc<Mutex<HashMap<u64, broadcast::Sender<NotificationType>>>>;

// Create a new notification manager
pub fn create_notification_manager() -> NotificationManager {
    Arc::new(Mutex::new(HashMap::new()))
}

// WebSocket handler
pub async fn websocket_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket_with_auth(socket, state))
}

// Handle socket with authentication via first message
async fn handle_socket_with_auth(socket: WebSocket, state: AppState) {
    tracing::info!("WebSocket connection established, awaiting authentication");

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Wait for authentication message
    let user_id = match receiver.next().await {
        Some(Ok(Message::Text(text))) => {
            match serde_json::from_str::<AuthMessage>(&text) {
                Ok(auth_msg) => {
                    // Validate session ID and get user
                    match state.storage.get_user_by_session_id(&auth_msg.session_id) {
                        Some(user) => {
                            tracing::info!("User {} authenticated via WebSocket", user.user_id);
                            user.user_id
                        }
                        None => {
                            tracing::warn!(
                                "Invalid session ID in WebSocket auth: {}",
                                auth_msg.session_id
                            );
                            let _ = sender
                                .send(Message::Text(
                                    serde_json::to_string(
                                        &NotificationType::ConnectionEstablished {
                                            user_id: 0,
                                            message: "Authentication failed: invalid session ID"
                                                .to_string(),
                                        },
                                    )
                                    .unwrap_or_default()
                                    .into(),
                                ))
                                .await;
                            return;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse WebSocket auth message: {}", e);
                    let _ = sender
                        .send(Message::Text(
                            "Authentication failed: invalid message format"
                                .to_string()
                                .into(),
                        ))
                        .await;
                    return;
                }
            }
        }
        Some(Ok(Message::Close(_))) => {
            tracing::info!("WebSocket connection closed before authentication");
            return;
        }
        Some(Err(e)) => {
            tracing::error!("WebSocket error during authentication: {}", e);
            return;
        }
        None => {
            tracing::warn!("WebSocket connection closed before authentication");
            return;
        }
        _ => {
            tracing::warn!("Unexpected message type during WebSocket authentication");
            return;
        }
    };

    // Continue with authenticated socket handling
    handle_authenticated_socket(sender, receiver, user_id, state).await;
}

// Authentication message structure
#[derive(Debug, Deserialize)]
struct AuthMessage {
    #[serde(rename = "sessionId")]
    session_id: String,
}

async fn handle_authenticated_socket(
    mut sender: futures_util::stream::SplitSink<WebSocket, Message>,
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    user_id: u64,
    state: AppState,
) {
    tracing::info!("WebSocket connection established for user {}", user_id);

    // Create a broadcast channel for this user
    let (tx, mut rx) = broadcast::channel(100);

    // Store the sender in the notification manager
    {
        let mut notification_manager = state.notification_manager.lock().unwrap();
        notification_manager.insert(user_id, tx.clone());
    }

    // Send connection established message
    let connection_msg = NotificationType::ConnectionEstablished {
        user_id,
        message: "Successfully connected to notifications".to_string(),
    };

    if let Ok(msg_text) = serde_json::to_string(&connection_msg) {
        if sender.send(Message::Text(msg_text.into())).await.is_err() {
            tracing::warn!("Failed to send connection message to user {}", user_id);
        }
    }

    // Spawn a task to handle incoming messages from the client
    let incoming_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    tracing::debug!("Received message from user {}: {}", user_id, text);
                    // Handle incoming messages if needed (e.g., subscription management)
                    // For now, we just log them
                }
                Ok(Message::Close(_)) => {
                    tracing::info!("WebSocket connection closed by user {}", user_id);
                    break;
                }
                Err(e) => {
                    tracing::error!("WebSocket error for user {}: {}", user_id, e);
                    break;
                }
                _ => {
                    // Handle other message types if needed
                }
            }
        }
    });

    // Handle outgoing notifications
    let outgoing_task = tokio::spawn(async move {
        while let Ok(notification) = rx.recv().await {
            match serde_json::to_string(&notification) {
                Ok(msg_text) => {
                    if sender.send(Message::Text(msg_text.into())).await.is_err() {
                        tracing::warn!("Failed to send notification to user {}", user_id);
                        break;
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to serialize notification for user {}: {}",
                        user_id,
                        e
                    );
                }
            }
        }
    });

    // Wait for either task to complete (websocket connection closed/error, or send to user error)
    tokio::select! {
        _ = incoming_task => {
            tracing::info!("Incoming task completed for user {}", user_id);
        }
        _ = outgoing_task => {
            tracing::info!("Outgoing task completed for user {}", user_id);
        }
    }

    // Clean up: remove the user from the notification manager
    {
        let mut notification_manager = state.notification_manager.lock().unwrap();
        notification_manager.remove(&user_id);
    }

    tracing::info!("WebSocket connection closed for user {}", user_id);
}

pub fn send_notification_to_user(
    notification_manager: &NotificationManager,
    user_id: u64,
    notification: NotificationType,
) {
    let manager = notification_manager.lock().unwrap();
    if let Some(tx) = manager.get(&user_id) {
        if let Err(e) = tx.send(notification) {
            tracing::warn!("Failed to send notification to user {}: {}", user_id, e);
        }
    }
}

// Send trade notifications to both taker and maker
pub fn send_trade_notifications(
    notification_manager: &NotificationManager,
    trade: &Trade,
    symbol: &str,
) {
    // Send notification to taker
    let taker_notification = NotificationType::TradeFill {
        trade: TradeNotification::from_trade(trade, trade.taker_user_id),
        symbol: symbol.to_string(),
    };
    send_notification_to_user(
        notification_manager,
        trade.taker_user_id,
        taker_notification,
    );

    // Send notification to maker (if different from taker)
    if trade.maker_user_id != trade.taker_user_id {
        let maker_notification = NotificationType::TradeFill {
            trade: TradeNotification::from_trade(trade, trade.maker_user_id),
            symbol: symbol.to_string(),
        };
        send_notification_to_user(
            notification_manager,
            trade.maker_user_id,
            maker_notification,
        );
    }
}
