use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use matcher::types::{Order, OrderSide, TimeInForce, Trade};
use serde::{Deserialize, Serialize};

use crate::{AppState, middleware::AuthUser};

// Add order request
#[derive(Deserialize)]
pub struct AddOrderRequest {
    pub symbol: String, // e.g. "BTC-USD", "SOL-USD"
    pub price_tick: u64,
    pub quantity: u64,
    pub side: OrderSide,
    pub time_in_force: TimeInForce,
}

// Add order response
#[derive(Serialize)]
pub struct AddOrderResponse {
    pub order: Option<OrderResponse>,
    pub trades: Vec<TradeResponse>,
    pub success: bool,
    pub message: String,
}

// Cancel order request
#[derive(Deserialize)]
pub struct CancelOrderRequest {
    pub symbol: String, // e.g. "BTC-USD", "SOL-USD"
    pub price_tick: u64,
    pub side: OrderSide,
}

// Cancel order response
#[derive(Serialize)]
pub struct CancelOrderResponse {
    pub success: bool,
    pub message: String,
}

// Depth request query parameters
#[derive(Deserialize)]
pub struct DepthRequest {
    pub symbol: String,
    #[serde(default = "default_levels")]
    pub levels: Option<usize>,
}

fn default_levels() -> Option<usize> {
    Some(100)
}

// Depth response
#[derive(Serialize)]
pub struct DepthResponse {
    pub symbol: String,
    pub bids: Vec<DepthLevelResponse>,
    pub asks: Vec<DepthLevelResponse>,
}

// Depth level response
#[derive(Serialize)]
pub struct DepthLevelResponse {
    pub price_tick: u64,
    pub quantity: u64,
}

// Order response model
#[derive(Serialize)]
pub struct OrderResponse {
    pub id: u64,
    pub symbol: String,
    pub price_tick: u64,
    pub quantity: u64,
    pub quantity_filled: u64,
    pub side: OrderSide,
    pub time_in_force: TimeInForce,
    pub timestamp: u64,
    pub is_cancelled: bool,
}

// Trade response model
#[derive(Serialize)]
pub struct TradeResponse {
    pub id: u64,
    pub symbol: String,
    pub taker_order_id: u64,
    pub maker_order_id: u64,
    pub taker_user_id: u64,
    pub maker_user_id: u64,
    pub quantity: u64,
    pub price_tick: u64,
    pub timestamp: u64,
}

// Convert Order to OrderResponse
impl OrderResponse {
    pub fn from_order_with_symbol(order: &Order, symbol: &str) -> Self {
        OrderResponse {
            id: order.id,
            symbol: symbol.to_string(),
            price_tick: order.price_tick,
            quantity: order.quantity,
            quantity_filled: order.quantity_filled,
            side: order.side,
            time_in_force: order.time_in_force,
            timestamp: order.timestamp,
            is_cancelled: order.is_cancelled,
        }
    }
}

// Convert Trade to TradeResponse
impl TradeResponse {
    pub fn from_trade_with_symbol(trade: &Trade, symbol: &str) -> Self {
        TradeResponse {
            id: trade.id,
            symbol: symbol.to_string(),
            taker_order_id: trade.taker_order_id,
            maker_order_id: trade.maker_order_id,
            taker_user_id: trade.taker_user_id,
            maker_user_id: trade.maker_user_id,
            quantity: trade.quantity,
            price_tick: trade.price_tick,
            timestamp: trade.timestamp,
        }
    }
}

// Add order endpoint
pub async fn add_order(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Json(payload): Json<AddOrderRequest>,
) -> (StatusCode, Json<AddOrderResponse>) {
    // Validate quantity
    if payload.quantity == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(AddOrderResponse {
                order: None,
                trades: Vec::new(),
                success: false,
                message: "Quantity must be greater than 0".to_string(),
            }),
        );
    }

    // Get the appropriate order book for the symbol first to get tick_multiplier
    let tick_multiplier = {
        let order_books = state.order_books.lock().unwrap();
        match order_books.get(&payload.symbol) {
            Some(book) => book.tick_multiplier(),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(AddOrderResponse {
                        order: None,
                        trades: Vec::new(),
                        success: false,
                        message: format!("Symbol '{}' not supported", payload.symbol),
                    }),
                );
            }
        }
    };

    // Debit funds before placing order
    if let Err(error_msg) = state.storage.debit_funds_for_order(
        _user.user_id,
        &payload.symbol,
        payload.side,
        payload.quantity,
        payload.price_tick,
        tick_multiplier,
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(AddOrderResponse {
                order: None,
                trades: Vec::new(),
                success: false,
                message: error_msg,
            }),
        );
    }

    // Get the appropriate order book for the symbol
    let mut order_books = state.order_books.lock().unwrap();
    let order_book = match order_books.get_mut(&payload.symbol) {
        Some(book) => book,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(AddOrderResponse {
                    order: None,
                    trades: Vec::new(),
                    success: false,
                    message: format!("Symbol '{}' not supported", payload.symbol),
                }),
            );
        }
    };

    // Add order to the order book - Serde already parsed the enums!
    let (order, trades) = order_book.add_order(
        _user.user_id,
        payload.price_tick,
        payload.quantity,
        payload.side,
        payload.time_in_force,
    );

    // Process trades and settle accounts
    for trade in &trades {
        if let Err(error_msg) = state.storage.settle_trade(
            trade,
            &payload.symbol,
            trade.taker_user_id,
            trade.maker_user_id,
            tick_multiplier,
        ) {
            tracing::error!("Failed to settle trade {}: {}", trade.id, error_msg);
            // Continue processing other trades even if one fails
        }
    }

    // If order was rejected, credit funds back
    if order.is_none() {
        let _ = state.storage.credit_funds_back(
            _user.user_id,
            &payload.symbol,
            payload.side,
            payload.quantity,
            payload.price_tick,
            tick_multiplier,
        );
    } else if let Some(ref placed_order) = order {
        // Handle partial fills - only refund unfilled portion if order is completely filled
        // For resting orders, keep funds debited until order is filled or cancelled
        let unfilled_quantity = placed_order.quantity - placed_order.quantity_filled;
        if unfilled_quantity > 0 && placed_order.quantity_filled > 0 {
            // Only refund if there was a partial fill (some filled, some unfilled)
            // For completely unfilled resting orders, keep funds debited
            let _ = state.storage.handle_partial_fill_refund(
                _user.user_id,
                &payload.symbol,
                payload.side,
                unfilled_quantity,
                payload.price_tick,
                tick_multiplier,
            );
        }
    }

    let response = AddOrderResponse {
        order: order
            .as_ref()
            .map(|o| OrderResponse::from_order_with_symbol(o, &payload.symbol)),
        trades: trades
            .iter()
            .map(|t| TradeResponse::from_trade_with_symbol(t, &payload.symbol))
            .collect(),
        success: order.is_some(),
        message: if order.is_some() {
            "Order accepted".to_string()
        } else {
            "Order rejected".to_string()
        },
    };

    let status = if order.is_some() {
        StatusCode::CREATED
    } else {
        StatusCode::BAD_REQUEST
    };

    (status, Json(response))
}

// Cancel order endpoint
pub async fn cancel_order(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Path(order_id): Path<u64>,
    Json(payload): Json<CancelOrderRequest>,
) -> (StatusCode, Json<CancelOrderResponse>) {
    // Get the appropriate order book for the symbol
    let tick_multiplier = {
        let order_books = state.order_books.lock().unwrap();
        match order_books.get(&payload.symbol) {
            Some(book) => book.tick_multiplier(),
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(CancelOrderResponse {
                        success: false,
                        message: format!("Symbol '{}' not supported", payload.symbol),
                    }),
                );
            }
        }
    };

    let mut order_books = state.order_books.lock().unwrap();
    let order_book = match order_books.get_mut(&payload.symbol) {
        Some(book) => book,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(CancelOrderResponse {
                    success: false,
                    message: format!("Symbol '{}' not supported", payload.symbol),
                }),
            );
        }
    };

    // Cancel order in the order book - Serde already parsed the enum!
    let success = order_book.cancel_order(order_id, payload.price_tick, payload.side);

    // If order was successfully cancelled, refund the funds back to the user
    if success {
        // Get the cancelled order details to refund the correct amount
        if let Some(cancelled_order) = order_book.get_order_by_id(order_id) {
            let unfilled_quantity = cancelled_order.quantity - cancelled_order.quantity_filled;
            if unfilled_quantity > 0 {
                let _ = state.storage.credit_funds_back(
                    _user.user_id,
                    &payload.symbol,
                    payload.side,
                    unfilled_quantity,
                    payload.price_tick,
                    tick_multiplier,
                );
            }
        }
    }

    let response = CancelOrderResponse {
        success,
        message: if success {
            "Order cancelled successfully".to_string()
        } else {
            "Failed to cancel order - order not found or invalid parameters".to_string()
        },
    };

    let status = if success {
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    };

    (status, Json(response))
}

// Get orderbook depth endpoint
pub async fn get_depth(
    State(state): State<AppState>,
    AuthUser(_user): AuthUser,
    Query(params): Query<DepthRequest>,
) -> (StatusCode, Json<DepthResponse>) {
    // Get levels with default value
    let levels = params.levels.unwrap_or(100);

    // Validate levels parameter
    if levels == 0 || levels > 1000 {
        return (
            StatusCode::BAD_REQUEST,
            Json(DepthResponse {
                symbol: params.symbol.clone(),
                bids: Vec::new(),
                asks: Vec::new(),
            }),
        );
    }

    // Get the appropriate order book for the symbol
    let order_books = state.order_books.lock().unwrap();
    let order_book = match order_books.get(&params.symbol) {
        Some(book) => book,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(DepthResponse {
                    symbol: params.symbol.clone(),
                    bids: Vec::new(),
                    asks: Vec::new(),
                }),
            );
        }
    };

    // Get depth from the order book
    let depth = order_book.get_depth(levels);

    let response = DepthResponse {
        symbol: params.symbol.clone(),
        bids: depth
            .bids
            .iter()
            .map(|level| DepthLevelResponse {
                price_tick: level.price_tick,
                quantity: level.quantity,
            })
            .collect(),
        asks: depth
            .asks
            .iter()
            .map(|level| DepthLevelResponse {
                price_tick: level.price_tick,
                quantity: level.quantity,
            })
            .collect(),
    };

    (StatusCode::OK, Json(response))
}
