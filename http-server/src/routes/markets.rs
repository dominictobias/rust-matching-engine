use crate::AppState;
use axum::{Json, extract::State, response::Json as ResponseJson};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketAsset {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub icon: String,
    pub price: f64,
    pub change24h: f64,
    pub tick_multiplier: u64,
}

pub async fn get_markets(State(state): State<AppState>) -> ResponseJson<Vec<MarketAsset>> {
    let order_books = state.order_books.lock().unwrap();

    let markets = vec![
        MarketAsset {
            id: "BTCUSD".to_string(),
            symbol: "BTC-USD".to_string(),
            name: "Bitcoin".to_string(),
            icon: "https://cdn.jsdelivr.net/npm/cryptocurrency-icons@0.16.1/svg/color/btc.svg"
                .to_string(),
            price: 115_771.03,
            change24h: 2.5,
            tick_multiplier: order_books
                .get("BTC-USD")
                .map(|ob| ob.tick_multiplier())
                .unwrap_or(100),
        },
        MarketAsset {
            id: "SOLUSD".to_string(),
            symbol: "SOL-USD".to_string(),
            name: "Solana".to_string(),
            icon: "https://solana.com/src/img/branding/solanaLogoMark.svg".to_string(),
            price: 246.64,
            change24h: -1.2,
            tick_multiplier: order_books
                .get("SOL-USD")
                .map(|ob| ob.tick_multiplier())
                .unwrap_or(100),
        },
    ];

    Json(markets)
}
