#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OrderSide {
    Bid,
    Ask,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TimeInForce {
    /// Good Till Canceled - remains active until filled or explicitly canceled
    GTC,
    /// Fill Or Kill - must be filled immediately and completely, or is canceled
    FOK,
    /// Immediate Or Cancel - fills immediately what it can, cancels the rest
    IOC,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Order {
    pub id: u64,
    /// Price in integer ticks (see price_multiplier in OrderBook)
    pub price_tick: u64,
    pub quantity: u64,
    pub quantity_filled: u64,
    pub side: OrderSide,
    pub time_in_force: TimeInForce,
    pub timestamp: u64,
    pub is_cancelled: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Trade {
    pub id: u64,
    pub taker_order_id: u64,
    pub maker_order_id: u64,
    pub quantity: u64,
    pub price_tick: u64,
    pub timestamp: u64,
}
