use axum::{
    Router,
    routing::{delete, get, post},
};
use matcher::orderbook::OrderBook;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;

mod middleware;
mod models;
mod routes;

use models::InMemoryStorage;
use routes::markets::get_markets;
use routes::orders::{add_order, cancel_order, get_depth};
use routes::users::{get_profile, login};

// Application state containing multiple order books and in-memory storage
#[derive(Clone)]
pub struct AppState {
    pub order_books: Arc<Mutex<HashMap<String, OrderBook>>>,
    pub storage: InMemoryStorage,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize tracing
    tracing_subscriber::fmt::init();

    // Initialize in-memory storage
    let storage = InMemoryStorage::new();
    tracing::info!("In-memory storage initialized successfully");

    // Create order books for different symbols
    let mut order_books = HashMap::new();
    order_books.insert(
        "BTC-USD".to_string(),
        OrderBook::new("BTC-USD".to_string(), 1_000_000), // 100 = 2 decimal places
    );
    order_books.insert(
        "SOL-USD".to_string(),
        OrderBook::new("SOL-USD".to_string(), 1_000_000), // 100 = 2 decimal places
    );

    let state = AppState {
        order_books: Arc::new(Mutex::new(order_books)),
        storage,
    };

    // build our application with routes
    let app = Router::new()
        .route("/", get(root))
        .route("/orders", post(add_order))
        .route("/orders/{id}", delete(cancel_order))
        .route("/depth", get(get_depth))
        .route("/markets", get(get_markets))
        .route("/login", post(login))
        .route("/users/profile", get(get_profile))
        .route("/profile", get(get_profile))
        .route("/health", get(health_check))
        .layer(ServiceBuilder::new().layer(CorsLayer::permissive()))
        .with_state(state);

    // run our app with hyper, listening globally on port 6957
    let listener = tokio::net::TcpListener::bind("0.0.0.0:6957").await?;
    tracing::info!("Server running on http://0.0.0.0:6957");
    axum::serve(listener, app).await?;

    Ok(())
}

// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

// Root endpoint
async fn root() -> &'static str {
    "Trade Engine API - Use POST /login to authenticate, POST /orders to add orders, DELETE /orders/{id} to cancel"
}
