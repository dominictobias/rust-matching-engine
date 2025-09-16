use super::types::{Order, OrderSide, TimeInForce, Trade};
use std::collections::{BTreeMap, VecDeque};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct PriceLevel {
    orders: VecDeque<Order>,
    total_quantity: u64,
}

/// Represents a price level for depth retrieval
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepthLevel {
    pub price_tick: u64,
    pub quantity: u64,
}

/// Depth data for both sides of the orderbook
#[derive(Debug, Clone)]
pub struct OrderBookDepth {
    pub bids: Vec<DepthLevel>,
    pub asks: Vec<DepthLevel>,
}

/// Represents one side of the orderbook (bid or ask)
/// Contains the best and worst price ticks and traversal direction
pub struct OrderbookSide {
    pub best_tick: Option<u64>,
    pub worst_tick: Option<u64>,
    /// true if higher prices are better (for bids), false if lower prices are better (for asks)
    pub higher_is_better: bool,
    /// Price levels stored in a BTreeMap for efficient ordered access
    pub levels: BTreeMap<u64, PriceLevel>,
}

pub struct OrderBook {
    symbol: String,

    /// Ask side of the orderbook (lower prices are better)
    ask_side: OrderbookSide,
    /// Bid side of the orderbook (higher prices are better)
    bid_side: OrderbookSide,

    /// Multiplier to convert decimal prices to integer ticks
    tick_multiplier: u64,

    order_id_counter: u64,
    trade_id_counter: u64,
    total_orders: u64,
}

#[inline(always)]
fn get_current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

impl OrderBook {
    /// Gets a mutable reference to the appropriate side based on OrderSide
    fn get_side_mut(&mut self, side: OrderSide) -> &mut OrderbookSide {
        match side {
            OrderSide::Bid => &mut self.bid_side,
            OrderSide::Ask => &mut self.ask_side,
        }
    }

    /// Gets the best tick from the opposite side (for matching)
    fn get_opposite_best_tick(&self, side: OrderSide) -> Option<u64> {
        match side {
            OrderSide::Bid => self.ask_side.best_tick,
            OrderSide::Ask => self.bid_side.best_tick,
        }
    }

    /// Creates a new, empty OrderBook instance with specified symbol and tick multiplier
    pub fn new(symbol: String, tick_multiplier: u64) -> Self {
        OrderBook {
            symbol,
            ask_side: OrderbookSide {
                best_tick: None,
                worst_tick: None,
                higher_is_better: false, // Lower prices are better for asks
                levels: BTreeMap::new(),
            },
            bid_side: OrderbookSide {
                best_tick: None,
                worst_tick: None,
                higher_is_better: true, // Higher prices are better for bids
                levels: BTreeMap::new(),
            },
            tick_multiplier,
            order_id_counter: 0,
            trade_id_counter: 0,
            total_orders: 0,
        }
    }

    pub fn add_order(
        &mut self,
        user_id: u64,
        price_tick: u64,
        quantity: u64,
        side: OrderSide,
        time_in_force: TimeInForce,
    ) -> (Option<Order>, Vec<Trade>) {
        let order_id = self.order_id_counter;
        self.order_id_counter += 1;
        let timestamp = get_current_timestamp();

        let best_tick = self.get_opposite_best_tick(side);

        // If there's nothing on the matching side, IOC and FOK can exit
        if best_tick.is_none()
            && (time_in_force == TimeInForce::FOK || time_in_force == TimeInForce::IOC)
        {
            return (None, Vec::new());
        }

        // Create the order
        let mut order = Order {
            id: order_id,
            user_id,
            price_tick,
            quantity,
            quantity_filled: 0,
            side,
            time_in_force,
            timestamp,
            is_cancelled: false,
        };

        // FOK is rejected if we cannot fill the entire order
        if time_in_force == TimeInForce::FOK && !self.can_fill_fok(&order) {
            return (None, Vec::new());
        }

        // Match against the book
        let trades = match best_tick {
            Some(_) => self.match_order(&mut order),
            None => Vec::new(),
        };

        // For GTC limit orders add to the book if not fully filled
        if time_in_force == TimeInForce::GTC
            && order.quantity > order.quantity_filled
            && price_tick > 0
        {
            self.add_limit_order(order.clone());
        }

        // Handle different time in force types for remaining quantity
        if order.quantity > order.quantity_filled {
            match time_in_force {
                TimeInForce::FOK => {
                    // This path should not be reachable due to the pre-check.
                    unreachable!(
                        "FOK orders should be fully filled or rejected before this point."
                    );
                }
                TimeInForce::IOC => {
                    // IOC orders are cancelled if not fully filled immediately.
                    // Do not add to book.
                    return (None, trades);
                }
                TimeInForce::GTC => {
                    if price_tick > 0 {
                        // Only GTC limit orders are added to the book
                        return (Some(order), trades);
                    }
                    // GTC Market orders that are not fully filled should be cancelled if no liquidity
                    if trades.is_empty() {
                        return (None, trades);
                    }
                    return (Some(order), trades);
                }
            }
        }

        (Some(order), trades)
    }

    // Returns the price range to iterate over for matching
    fn get_tick_iter_bounds(&self, order: &Order) -> (u64, u64) {
        let opposite_side = match order.side {
            OrderSide::Bid => &self.ask_side,
            OrderSide::Ask => &self.bid_side,
        };

        let start_tick = opposite_side.best_tick.unwrap();

        let end_tick: u64 = match order.price_tick {
            0 => match order.side {
                OrderSide::Ask => {
                    // For market sell orders, match against all available bid liquidity
                    // We want to iterate from best_bid down to worst_bid
                    self.bid_side.worst_tick.unwrap_or(0)
                }
                OrderSide::Bid => {
                    // For market buy orders, match against all available ask liquidity
                    // We want to iterate from best_ask up to worst_ask
                    self.ask_side.worst_tick.unwrap_or(u64::MAX)
                }
            },
            _ => order.price_tick,
        };

        // For limit orders, we should only match if the prices cross
        // For market orders (price_tick = 0), we match all available liquidity
        if order.price_tick > 0 {
            match order.side {
                OrderSide::Bid => {
                    // Buy order: only match if ask price <= buy price
                    if start_tick > end_tick {
                        return (0, 0); // No match possible
                    }
                }
                OrderSide::Ask => {
                    // Sell order: only match if bid price >= sell price
                    if start_tick < end_tick {
                        return (0, 0); // No match possible
                    }
                }
            }
        }

        (start_tick, end_tick)
    }

    fn can_fill_fok(&self, order: &Order) -> bool {
        let best_tick = self.get_opposite_best_tick(order.side);

        // If there are no orders on the matching side, we can't match
        if best_tick.is_none() {
            return false;
        }

        let (start_tick, end_tick) = self.get_tick_iter_bounds(order);

        let mut qty_till_price: u64 = 0;

        // Get the opposite side's levels
        let opposite_side = match order.side {
            OrderSide::Bid => &self.ask_side,
            OrderSide::Ask => &self.bid_side,
        };

        // Iterate over the price range in the appropriate direction
        if start_tick <= end_tick {
            // Ascending order (for asks matching against bids)
            for tick in start_tick..=end_tick {
                if let Some(level) = opposite_side.levels.get(&tick) {
                    qty_till_price += level.total_quantity;
                }
                if qty_till_price >= order.quantity {
                    return true;
                }
            }
        } else {
            // Descending order (for bids matching against asks)
            for tick in (end_tick..=start_tick).rev() {
                if let Some(level) = opposite_side.levels.get(&tick) {
                    qty_till_price += level.total_quantity;
                }
                if qty_till_price >= order.quantity {
                    return true;
                }
            }
        }

        false
    }

    // If this is called we have a best_tick and worst_tick
    fn match_order(&mut self, order: &mut Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        let (start_tick, end_tick) = self.get_tick_iter_bounds(order);

        // If no match is possible, return empty trades
        if start_tick == 0 && end_tick == 0 {
            return trades;
        }

        // Get the opposite side's levels
        let opposite_side = match order.side {
            OrderSide::Bid => &mut self.ask_side,
            OrderSide::Ask => &mut self.bid_side,
        };

        // For market orders, we need to handle the iteration direction correctly
        let tick_range = if order.price_tick == 0 && order.side == OrderSide::Ask {
            // Market sell order: iterate from best_bid down to worst_bid
            (end_tick..=start_tick).rev().collect::<Vec<_>>()
        } else if order.price_tick == 0 && order.side == OrderSide::Bid {
            // Market buy order: iterate from best_ask up to worst_ask
            (start_tick..=end_tick).collect::<Vec<_>>()
        } else {
            // Limit orders: iterate from end_tick to start_tick (inclusive)
            if start_tick >= end_tick {
                (end_tick..=start_tick).collect::<Vec<_>>()
            } else {
                (start_tick..=end_tick).collect::<Vec<_>>()
            }
        };

        'outer: for tick in tick_range {
            if let Some(level) = opposite_side.levels.get_mut(&tick) {
                while let Some(mut resting_order) = level.orders.pop_front() {
                    if resting_order.is_cancelled {
                        // Do nothing, effectively dropping the order
                        continue;
                    }

                    let quantity_to_fill = (order.quantity - order.quantity_filled)
                        .min(resting_order.quantity - resting_order.quantity_filled);

                    if quantity_to_fill == 0 {
                        unreachable!("There should never be an empty resting order in the book.");
                    }

                    let trade = Trade {
                        id: self.trade_id_counter,
                        taker_order_id: order.id,
                        maker_order_id: resting_order.id,
                        taker_user_id: order.user_id,
                        maker_user_id: resting_order.user_id,
                        quantity: quantity_to_fill,
                        price_tick: resting_order.price_tick,
                        timestamp: get_current_timestamp(),
                    };
                    self.trade_id_counter += 1;
                    trades.push(trade);

                    order.quantity_filled += quantity_to_fill;
                    resting_order.quantity_filled += quantity_to_fill;
                    level.total_quantity -= quantity_to_fill;

                    if resting_order.quantity > resting_order.quantity_filled {
                        // If the resting order is only partially filled, push it back
                        level.orders.push_front(resting_order);
                    } else {
                        self.total_orders -= 1;
                    }

                    // The order is fully filled, we can exit
                    if order.quantity == order.quantity_filled {
                        // Remove the level if it's empty before breaking
                        if level.total_quantity == 0 {
                            opposite_side.levels.remove(&tick);
                        }
                        break 'outer;
                    }
                }
            }

            // Remove the level if it's empty (after processing all orders in the level)
            if let Some(level) = opposite_side.levels.get(&tick) {
                if level.total_quantity == 0 {
                    opposite_side.levels.remove(&tick);
                }
            }
        }

        // Update best and worst ticks if needed after matching
        self.update_price_ticks_after_match(order.side);

        trades
    }

    /// Updates best and worst price ticks after matching orders
    /// This handles the case where matching may have consumed price levels
    fn update_price_ticks_after_match(&mut self, order_side: OrderSide) {
        match order_side {
            OrderSide::Ask => {
                // We matched against bid side, so update bid ticks
                self.update_side_ticks(OrderSide::Bid);
            }
            OrderSide::Bid => {
                // We matched against ask side, so update ask ticks
                self.update_side_ticks(OrderSide::Ask);
            }
        }
    }

    /// Updates ticks for a given side after potential level consumption
    fn update_side_ticks(&mut self, side: OrderSide) {
        let side_mut = self.get_side_mut(side);

        // If we don't have any levels, clear the ticks
        if side_mut.levels.is_empty() {
            side_mut.best_tick = None;
            side_mut.worst_tick = None;
            return;
        }

        // Update best and worst ticks from the BTreeMap
        if side_mut.higher_is_better {
            // For bids: best is highest price, worst is lowest price
            side_mut.best_tick = side_mut.levels.last_key_value().map(|(&k, _)| k);
            side_mut.worst_tick = side_mut.levels.first_key_value().map(|(&k, _)| k);
        } else {
            // For asks: best is lowest price, worst is highest price
            side_mut.best_tick = side_mut.levels.first_key_value().map(|(&k, _)| k);
            side_mut.worst_tick = side_mut.levels.last_key_value().map(|(&k, _)| k);
        }
    }

    fn add_limit_order(&mut self, order: Order) {
        let price_tick = order.price_tick;
        let order_side = order.side;

        let side_mut = self.get_side_mut(order_side);
        let level = side_mut
            .levels
            .entry(price_tick)
            .or_insert_with(|| PriceLevel {
                orders: VecDeque::new(),
                total_quantity: 0,
            });

        level.orders.push_back(order.clone());
        level.total_quantity += order.quantity - order.quantity_filled;

        // Update best/worst ticks based on BTreeMap keys
        if side_mut.higher_is_better {
            // For bids: best is highest price, worst is lowest price
            side_mut.best_tick = side_mut.levels.keys().max().copied();
            side_mut.worst_tick = side_mut.levels.keys().min().copied();
        } else {
            // For asks: best is lowest price, worst is highest price
            side_mut.best_tick = side_mut.levels.keys().min().copied();
            side_mut.worst_tick = side_mut.levels.keys().max().copied();
        }

        self.total_orders += 1;
    }

    /// Get the total number of orders in the book
    pub fn total_orders(&self) -> u64 {
        self.total_orders
    }

    /// Get the symbol for this orderbook
    pub fn symbol(&self) -> &str {
        &self.symbol
    }

    /// Get the tick multiplier for this orderbook
    pub fn tick_multiplier(&self) -> u64 {
        self.tick_multiplier
    }

    /// Get the best bid price tick
    pub fn best_bid_tick(&self) -> Option<u64> {
        self.bid_side.best_tick
    }

    /// Get the best ask price tick
    pub fn best_ask_tick(&self) -> Option<u64> {
        self.ask_side.best_tick
    }

    /// Get an order by its ID
    pub fn get_order_by_id(&self, order_id: u64) -> Option<&Order> {
        // Search in bid side levels
        for level in self.bid_side.levels.values() {
            for order in &level.orders {
                if order.id == order_id {
                    return Some(order);
                }
            }
        }

        // Search in ask side levels
        for level in self.ask_side.levels.values() {
            for order in &level.orders {
                if order.id == order_id {
                    return Some(order);
                }
            }
        }

        None
    }

    pub fn cancel_order(&mut self, order_id: u64, price_tick: u64, side: OrderSide) -> bool {
        let side_mut = self.get_side_mut(side);

        if let Some(level) = side_mut.levels.get_mut(&price_tick) {
            if let Ok(index) = level.orders.binary_search_by_key(&order_id, |o| o.id) {
                // Check if the side matches
                let order = &level.orders[index];
                if order.side != side {
                    return false;
                }

                // Precompute if we need to update ticks and get order details
                let need_update_ticks;
                let order_side_value;
                let remaining_quantity_after_cancel;
                {
                    let order = &level.orders[index];
                    remaining_quantity_after_cancel =
                        level.total_quantity - (order.quantity - order.quantity_filled);
                    need_update_ticks = remaining_quantity_after_cancel == 0;
                    order_side_value = order.side;
                }

                let mut cancelled = false;
                if let Some(order) = level.orders.get_mut(index) {
                    if !order.is_cancelled {
                        order.is_cancelled = true;
                        level.total_quantity -= order.quantity - order.quantity_filled;
                        cancelled = true;

                        // If the level is now empty, remove it from the BTreeMap
                        if level.total_quantity == 0 {
                            side_mut.levels.remove(&price_tick);
                        }
                    }
                }

                // Decrement total orders after releasing the borrow
                if cancelled {
                    self.total_orders -= 1;
                }

                // Update ticks if needed after the level was consumed
                if cancelled && need_update_ticks {
                    self.update_side_ticks(order_side_value);
                }

                return cancelled;
            }
        }
        false
    }

    /// Get orderbook depth up to the specified number of levels
    /// Returns the top N levels for both bids and asks
    pub fn get_depth(&self, levels: usize) -> OrderBookDepth {
        let mut bids = Vec::new();
        let mut asks = Vec::new();

        // Get top N bid levels (highest prices first)
        // BTreeMap iterates in ascending order, so we need to reverse for bids
        let bid_iter = self.bid_side.levels.iter().rev().take(levels);
        for (price_tick, level) in bid_iter {
            bids.push(DepthLevel {
                price_tick: *price_tick,
                quantity: level.total_quantity,
            });
        }

        // Get top N ask levels (lowest prices first)
        // BTreeMap iterates in ascending order, which is perfect for asks
        let ask_iter = self.ask_side.levels.iter().take(levels);
        for (price_tick, level) in ask_iter {
            asks.push(DepthLevel {
                price_tick: *price_tick,
                quantity: level.total_quantity,
            });
        }

        OrderBookDepth { bids, asks }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OrderSide, TimeInForce};

    fn setup_book() -> OrderBook {
        OrderBook::new("TEST-USD".to_string(), 100) // 100 = 2 decimal places
    }

    #[test]
    fn test_new_order_book() {
        let book = setup_book();
        assert!(book.bid_side.best_tick.is_none());
        assert!(book.ask_side.best_tick.is_none());
        assert!(book.bid_side.worst_tick.is_none());
        assert!(book.ask_side.worst_tick.is_none());
        assert_eq!(book.total_orders, 0);
        assert_eq!(book.order_id_counter, 0);
        assert_eq!(book.trade_id_counter, 0);
        assert_eq!(book.tick_multiplier, 100);
    }

    #[test]
    fn test_add_gtc_limit_order() {
        let mut book = setup_book();
        let price_tick = 101;
        let quantity = 10;

        // Add a buy order
        let (order, trades) =
            book.add_order(1, price_tick, quantity, OrderSide::Bid, TimeInForce::GTC);

        assert!(order.is_some());
        let order = order.unwrap();
        assert_eq!(order.price_tick, price_tick);
        assert_eq!(order.quantity, quantity);
        assert_eq!(order.side, OrderSide::Bid);
        assert!(trades.is_empty());

        assert_eq!(book.bid_side.best_tick, Some(price_tick));
        assert_eq!(book.bid_side.worst_tick, Some(price_tick));
        assert!(book.ask_side.best_tick.is_none());
        let level = book.bid_side.levels.get(&price_tick).unwrap();
        assert_eq!(level.total_quantity, quantity);
        assert_eq!(level.orders.len(), 1);
        assert_eq!(book.total_orders, 1);

        // Add a sell order
        let sell_price_tick = 102;
        let (sell_order, trades) =
            book.add_order(1, sell_price_tick, 5, OrderSide::Ask, TimeInForce::GTC);
        assert!(sell_order.is_some());
        assert!(trades.is_empty());
        assert_eq!(book.ask_side.best_tick, Some(sell_price_tick));
        assert_eq!(book.ask_side.worst_tick, Some(sell_price_tick));
        let ask_level = book.ask_side.levels.get(&sell_price_tick).unwrap();
        assert_eq!(ask_level.total_quantity, 5);
        assert_eq!(book.total_orders, 2);
    }

    #[test]
    fn test_simple_order_match() {
        let mut book = setup_book();

        // Add a resting sell order
        book.add_order(1, 101, 10, OrderSide::Ask, TimeInForce::GTC);

        // Add a matching buy order
        let (buy_order, trades) = book.add_order(1, 101, 5, OrderSide::Bid, TimeInForce::GTC);

        assert!(buy_order.is_some());
        let buy_order = buy_order.unwrap();
        assert_eq!(buy_order.quantity_filled, 5);
        assert_eq!(trades.len(), 1);

        let trade = &trades[0];
        assert_eq!(trade.quantity, 5);
        assert_eq!(trade.price_tick, 101);
        assert_eq!(trade.taker_order_id, buy_order.id);

        // Check the state of the resting order
        let ask_level = book.ask_side.levels.get(&101).unwrap();
        assert_eq!(ask_level.total_quantity, 5);
        assert_eq!(ask_level.orders[0].quantity_filled, 5);
    }

    #[test]
    fn test_market_order_full_fill() {
        let mut book = setup_book();
        book.add_order(1, 101, 10, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(1, 102, 10, OrderSide::Ask, TimeInForce::GTC);

        // Market buy order, price_tick = 0
        let (market_order, trades) = book.add_order(1, 0, 15, OrderSide::Bid, TimeInForce::GTC);

        assert!(market_order.is_some());
        let market_order = market_order.unwrap();
        assert_eq!(market_order.quantity_filled, 15);
        assert_eq!(trades.len(), 2);

        assert_eq!(trades[0].quantity, 10);
        assert_eq!(trades[0].price_tick, 101);
        assert_eq!(trades[1].quantity, 5);
        assert_eq!(trades[1].price_tick, 102);

        // Best ask should be gone, next best ask is now the best
        assert_eq!(book.ask_side.best_tick, Some(102));
        let level = book.ask_side.levels.get(&102).unwrap();
        assert_eq!(level.total_quantity, 5);
    }

    #[test]
    fn test_cancel_order() {
        let mut book = setup_book();
        let (order, _) = book.add_order(1, 101, 10, OrderSide::Bid, TimeInForce::GTC);
        let order_id = order.unwrap().id;

        let cancelled = book.cancel_order(order_id, 101, OrderSide::Bid);
        assert!(cancelled);

        // After cancelling the only order in the level, the level should be None
        assert!(book.bid_side.levels.get(&101).is_none());

        // Try to cancel again
        let cancelled_again = book.cancel_order(order_id, 101, OrderSide::Bid);
        assert!(!cancelled_again);
    }

    #[test]
    fn test_ioc_order_partial_fill() {
        let mut book = setup_book();
        book.add_order(1, 101, 5, OrderSide::Ask, TimeInForce::GTC);

        // IOC order for 10, only 5 available
        let (order, trades) = book.add_order(1, 102, 10, OrderSide::Bid, TimeInForce::IOC);

        // IOC orders are not added to the book, so we get None
        assert!(order.is_none());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 5);

        // The resting order should be gone
        assert!(book.ask_side.levels.get(&101).is_none());
        assert!(book.ask_side.best_tick.is_none());
        assert_eq!(book.total_orders, 0);
    }

    #[test]
    fn test_fok_order_success() {
        let mut book = setup_book();
        book.add_order(1, 101, 10, OrderSide::Ask, TimeInForce::GTC);

        // FOK order that can be filled
        let (order, trades) = book.add_order(1, 101, 10, OrderSide::Bid, TimeInForce::FOK);

        assert!(order.is_some());
        assert_eq!(order.unwrap().quantity_filled, 10);
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 10);

        // Book should be empty
        assert!(book.ask_side.best_tick.is_none());
    }

    #[test]
    fn test_fok_order_fail() {
        let mut book = setup_book();
        book.add_order(1, 101, 5, OrderSide::Ask, TimeInForce::GTC);

        // FOK order that cannot be fully filled
        let (order, trades) = book.add_order(1, 101, 10, OrderSide::Bid, TimeInForce::FOK);

        // Order should be rejected
        assert!(order.is_none());
        assert!(trades.is_empty());

        // Book should be unchanged
        let level = book.ask_side.levels.get(&101).unwrap();
        assert_eq!(level.total_quantity, 5);
        assert_eq!(book.total_orders, 1);
    }

    #[test]
    fn test_cancel_order_updates_best_tick() {
        let mut book = setup_book();
        // Add two orders on the buy side
        let (order1, _) = book.add_order(1, 101, 10, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::GTC);
        let order1_id = order1.unwrap().id;

        assert_eq!(book.bid_side.best_tick, Some(101));

        // Cancel the order at the best tick
        let cancelled = book.cancel_order(order1_id, 101, OrderSide::Bid);
        assert!(cancelled);

        // The best tick should be updated to the next best price
        assert_eq!(book.bid_side.best_tick, Some(100));

        // After cancelling the only order in the level, the level should be None
        assert!(book.bid_side.levels.get(&101).is_none());
    }

    #[test]
    fn test_add_order_updates_best_tick() {
        let mut book = setup_book();

        // Test buy side - higher prices should become new best tick
        book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::GTC);
        assert_eq!(book.bid_side.best_tick, Some(100));

        book.add_order(1, 101, 5, OrderSide::Bid, TimeInForce::GTC);
        assert_eq!(book.bid_side.best_tick, Some(101)); // Higher price becomes best

        book.add_order(1, 99, 5, OrderSide::Bid, TimeInForce::GTC);
        assert_eq!(book.bid_side.best_tick, Some(101)); // Lower price doesn't change best

        // Test sell side - lower prices should become new best tick
        book.add_order(1, 110, 10, OrderSide::Ask, TimeInForce::GTC);
        assert_eq!(book.ask_side.best_tick, Some(110));

        book.add_order(1, 109, 5, OrderSide::Ask, TimeInForce::GTC);
        assert_eq!(book.ask_side.best_tick, Some(109)); // Lower price becomes best

        book.add_order(1, 111, 5, OrderSide::Ask, TimeInForce::GTC);
        assert_eq!(book.ask_side.best_tick, Some(109)); // Higher price doesn't change best
    }

    #[test]
    fn test_match_order_updates_best_tick() {
        let mut book = setup_book();

        // Set up sell side with multiple price levels
        book.add_order(1, 101, 10, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(1, 102, 10, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(1, 103, 10, OrderSide::Ask, TimeInForce::GTC);
        assert_eq!(book.ask_side.best_tick, Some(101));

        // Market buy order that fully consumes the best ask level
        let (order, trades) = book.add_order(1, 0, 10, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price_tick, 101);
        assert_eq!(trades[0].quantity, 10);

        // Best tick should be updated to the next best price
        assert_eq!(book.ask_side.best_tick, Some(102));

        // Another market buy that consumes the next level partially
        let (order, trades) = book.add_order(1, 0, 5, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price_tick, 102);
        assert_eq!(trades[0].quantity, 5);

        // Best tick should remain the same since level still has quantity
        assert_eq!(book.ask_side.best_tick, Some(102));
        let level = book.ask_side.levels.get(&102).unwrap();
        assert_eq!(level.total_quantity, 5);

        // Final market buy that fully consumes the 102 level
        let (order, trades) = book.add_order(1, 0, 5, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price_tick, 102);
        assert_eq!(trades[0].quantity, 5);

        // Best tick should now move to 103
        assert_eq!(book.ask_side.best_tick, Some(103));
    }

    #[test]
    fn test_match_order_updates_best_tick_bid_side() {
        let mut book = setup_book();

        // Set up bid side with multiple price levels
        book.add_order(1, 103, 10, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 102, 10, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 101, 10, OrderSide::Bid, TimeInForce::GTC);
        assert_eq!(book.bid_side.best_tick, Some(103));

        // Market sell order that fully consumes the best bid level
        let (order, trades) = book.add_order(1, 0, 10, OrderSide::Ask, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price_tick, 103);
        assert_eq!(trades[0].quantity, 10);

        // Best tick should be updated to the next best price
        assert_eq!(book.bid_side.best_tick, Some(102));

        // Another market sell that fully consumes two levels
        let (order, trades) = book.add_order(1, 0, 20, OrderSide::Ask, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].price_tick, 102);
        assert_eq!(trades[0].quantity, 10);
        assert_eq!(trades[1].price_tick, 101);
        assert_eq!(trades[1].quantity, 10);

        // Best tick should be None as all levels are consumed
        assert_eq!(book.bid_side.best_tick, None);
    }

    #[test]
    fn test_match_order_multiple_levels_updates_best_tick() {
        let mut book = setup_book();

        // Set up ask side with multiple small orders at the same price
        book.add_order(1, 101, 3, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(1, 101, 3, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(1, 101, 4, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(1, 102, 20, OrderSide::Ask, TimeInForce::GTC);

        assert_eq!(book.ask_side.best_tick, Some(101));
        let level = book.ask_side.levels.get(&101).unwrap();
        assert_eq!(level.total_quantity, 10);
        assert_eq!(level.orders.len(), 3);

        // Large buy order that consumes all orders at 101 and moves to 102
        let (order, trades) = book.add_order(1, 0, 15, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 4); // 3 orders at 101 + 1 partial at 102

        // Verify trades
        assert_eq!(trades[0].price_tick, 101);
        assert_eq!(trades[1].price_tick, 101);
        assert_eq!(trades[2].price_tick, 101);
        assert_eq!(trades[3].price_tick, 102);
        assert_eq!(
            trades[0].quantity + trades[1].quantity + trades[2].quantity,
            10
        );
        assert_eq!(trades[3].quantity, 5);

        // Best tick should be updated to 102
        assert_eq!(book.ask_side.best_tick, Some(102));

        // Verify the 101 level is cleared
        assert!(book.ask_side.levels.get(&101).is_none());

        // Verify remaining quantity at 102
        let level = book.ask_side.levels.get(&102).unwrap();
        assert_eq!(level.total_quantity, 15);
    }

    // Additional comprehensive tests
    #[test]
    fn test_market_order_no_liquidity() {
        let mut book = setup_book();

        // Market order with no liquidity should be cancelled
        let (order, trades) = book.add_order(1, 0, 10, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_none());
        assert!(trades.is_empty());
    }

    #[test]
    fn test_ioc_order_no_liquidity() {
        let mut book = setup_book();

        // IOC order with no liquidity should be rejected
        let (order, trades) = book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::IOC);
        assert!(order.is_none());
        assert!(trades.is_empty());
    }

    #[test]
    fn test_fok_order_no_liquidity() {
        let mut book = setup_book();

        // FOK order with no liquidity should be rejected
        let (order, trades) = book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::FOK);
        assert!(order.is_none());
        assert!(trades.is_empty());
    }

    #[test]
    fn test_limit_order_improvement() {
        let mut book = setup_book();

        // Add a sell order at 100
        book.add_order(1, 100, 10, OrderSide::Ask, TimeInForce::GTC);

        // Add a buy order at 99 (should not match)
        let (order, trades) = book.add_order(1, 99, 5, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert!(trades.is_empty());

        // Add a buy order at 100 (should match)
        let (order, trades) = book.add_order(1, 100, 5, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 5);
    }

    #[test]
    fn test_multiple_orders_same_price() {
        let mut book = setup_book();

        // Add multiple orders at the same price
        book.add_order(1, 100, 5, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 100, 3, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 100, 2, OrderSide::Bid, TimeInForce::GTC);

        let level = book.bid_side.levels.get(&100).unwrap();
        assert_eq!(level.total_quantity, 10);
        assert_eq!(level.orders.len(), 3);
        assert_eq!(book.total_orders, 3);
    }

    #[test]
    fn test_cancel_nonexistent_order() {
        let mut book = setup_book();

        // Try to cancel an order that doesn't exist
        let cancelled = book.cancel_order(999, 100, OrderSide::Bid);
        assert!(!cancelled);
    }

    #[test]
    fn test_cancel_order_wrong_price() {
        let mut book = setup_book();

        // Add an order
        let (order, _) = book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::GTC);
        let order_id = order.unwrap().id;

        // Try to cancel with wrong price
        let cancelled = book.cancel_order(order_id, 101, OrderSide::Bid);
        assert!(!cancelled);

        // Order should still exist
        let level = book.bid_side.levels.get(&100).unwrap();
        assert_eq!(level.total_quantity, 10);
    }

    #[test]
    fn test_cancel_order_wrong_side() {
        let mut book = setup_book();

        // Add a bid order
        let (order, _) = book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::GTC);
        let order_id = order.unwrap().id;

        // Try to cancel with wrong side
        let cancelled = book.cancel_order(order_id, 100, OrderSide::Ask);
        assert!(!cancelled);

        // Order should still exist
        let level = book.bid_side.levels.get(&100).unwrap();
        assert_eq!(level.total_quantity, 10);
    }

    #[test]
    fn test_cancel_already_cancelled_order() {
        let mut book = setup_book();

        // Add an order
        let (order, _) = book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::GTC);
        let order_id = order.unwrap().id;

        // Cancel it
        let cancelled = book.cancel_order(order_id, 100, OrderSide::Bid);
        assert!(cancelled);

        // Try to cancel again
        let cancelled_again = book.cancel_order(order_id, 100, OrderSide::Bid);
        assert!(!cancelled_again);
    }

    #[test]
    fn test_order_id_counter_increments() {
        let mut book = setup_book();

        let (order1, _) = book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::GTC);
        let (order2, _) = book.add_order(1, 101, 5, OrderSide::Ask, TimeInForce::GTC);

        assert_eq!(order1.unwrap().id, 0);
        assert_eq!(order2.unwrap().id, 1);
        assert_eq!(book.order_id_counter, 2);
    }

    #[test]
    fn test_trade_id_counter_increments() {
        let mut book = setup_book();

        // Add a resting order
        book.add_order(1, 100, 10, OrderSide::Ask, TimeInForce::GTC);

        // Add matching orders
        let (_, trades1) = book.add_order(1, 100, 5, OrderSide::Bid, TimeInForce::GTC);
        let (_, trades2) = book.add_order(1, 100, 3, OrderSide::Bid, TimeInForce::GTC);

        assert_eq!(trades1[0].id, 0);
        assert_eq!(trades2[0].id, 1);
        assert_eq!(book.trade_id_counter, 2);
    }

    #[test]
    fn test_worst_tick_tracking() {
        let mut book = setup_book();

        // Add orders at different prices
        book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 102, 5, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 98, 3, OrderSide::Bid, TimeInForce::GTC);

        assert_eq!(book.bid_side.best_tick, Some(102)); // Highest price
        assert_eq!(book.bid_side.worst_tick, Some(98)); // Lowest price

        // Cancel the worst tick (the first order at 98)
        // We need to get the order ID of the first order at 98
        let level = book.bid_side.levels.get(&98).unwrap();
        let order_id = level.orders[0].id;
        book.cancel_order(order_id, 98, OrderSide::Bid);

        assert_eq!(book.bid_side.best_tick, Some(102));
        assert_eq!(book.bid_side.worst_tick, Some(100)); // Should update to next worst
    }

    #[test]
    fn test_zero_price_tick_limit_order() {
        let mut book = setup_book();

        // Zero price tick should not be added as limit order
        let (order, trades) = book.add_order(1, 0, 10, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_none());
        assert!(trades.is_empty());
    }

    #[test]
    fn test_partial_fill_resting_order() {
        let mut book = setup_book();

        // Add a large resting order
        book.add_order(1, 100, 100, OrderSide::Ask, TimeInForce::GTC);

        // Partially fill it
        let (order, trades) = book.add_order(1, 100, 30, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 30);

        // Check resting order state
        let level = book.ask_side.levels.get(&100).unwrap();
        assert_eq!(level.total_quantity, 70);
        assert_eq!(level.orders[0].quantity_filled, 30);
    }

    #[test]
    fn test_cross_spread_matching() {
        let mut book = setup_book();

        // Add orders that cross the spread
        book.add_order(1, 100, 10, OrderSide::Ask, TimeInForce::GTC);
        let (bid_order, trades) = book.add_order(1, 102, 5, OrderSide::Bid, TimeInForce::GTC);

        // The bid at 102 should match against the ask at 100, filling 5 units
        assert!(bid_order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 5);
        assert_eq!(trades[0].price_tick, 100); // Should match at ask price

        // Ask should be partially filled (10 - 5 = 5 remaining)
        let level = book.ask_side.levels.get(&100).unwrap();
        assert_eq!(level.total_quantity, 5);

        // The bid at 102 should be fully filled and not in the book
        assert_eq!(book.bid_side.best_tick, None);
        assert_eq!(book.ask_side.best_tick, Some(100));

        // Add an aggressive order that crosses
        let (order, trades) = book.add_order(1, 103, 8, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 5); // Should match remaining ask quantity
        assert_eq!(trades[0].price_tick, 100); // Should match at ask price

        // Ask should be fully consumed
        assert!(book.ask_side.levels.get(&100).is_none());
        assert_eq!(book.ask_side.best_tick, None);

        // The new bid at 103 should remain in the book with 3 units (8 - 5 = 3)
        assert_eq!(book.bid_side.best_tick, Some(103));
        let level = book.bid_side.levels.get(&103).unwrap();
        assert_eq!(level.total_quantity, 3);
    }

    #[test]
    fn test_get_depth_empty_book() {
        let book = setup_book();
        let depth = book.get_depth(10);

        assert!(depth.bids.is_empty());
        assert!(depth.asks.is_empty());
    }

    #[test]
    fn test_get_depth_single_levels() {
        let mut book = setup_book();

        // Add one bid and one ask
        book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 105, 5, OrderSide::Ask, TimeInForce::GTC);

        let depth = book.get_depth(10);

        assert_eq!(depth.bids.len(), 1);
        assert_eq!(depth.bids[0].price_tick, 100);
        assert_eq!(depth.bids[0].quantity, 10);

        assert_eq!(depth.asks.len(), 1);
        assert_eq!(depth.asks[0].price_tick, 105);
        assert_eq!(depth.asks[0].quantity, 5);
    }

    #[test]
    fn test_get_depth_multiple_levels() {
        let mut book = setup_book();

        // Add multiple bid levels (higher prices should come first)
        book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 102, 5, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 98, 15, OrderSide::Bid, TimeInForce::GTC);

        // Add multiple ask levels (lower prices should come first)
        book.add_order(1, 105, 8, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(1, 108, 12, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(1, 103, 3, OrderSide::Ask, TimeInForce::GTC);

        let depth = book.get_depth(10);

        // Bids should be sorted by price descending (highest first)
        assert_eq!(depth.bids.len(), 3);
        assert_eq!(depth.bids[0].price_tick, 102);
        assert_eq!(depth.bids[0].quantity, 5);
        assert_eq!(depth.bids[1].price_tick, 100);
        assert_eq!(depth.bids[1].quantity, 10);
        assert_eq!(depth.bids[2].price_tick, 98);
        assert_eq!(depth.bids[2].quantity, 15);

        // Asks should be sorted by price ascending (lowest first)
        assert_eq!(depth.asks.len(), 3);
        assert_eq!(depth.asks[0].price_tick, 103);
        assert_eq!(depth.asks[0].quantity, 3);
        assert_eq!(depth.asks[1].price_tick, 105);
        assert_eq!(depth.asks[1].quantity, 8);
        assert_eq!(depth.asks[2].price_tick, 108);
        assert_eq!(depth.asks[2].quantity, 12);
    }

    #[test]
    fn test_get_depth_limit_levels() {
        let mut book = setup_book();

        // Add 5 bid levels
        for i in 0..5 {
            book.add_order(1, 100 + i, 10, OrderSide::Bid, TimeInForce::GTC);
        }

        // Add 5 ask levels
        for i in 0..5 {
            book.add_order(1, 110 + i, 10, OrderSide::Ask, TimeInForce::GTC);
        }

        // Request only 3 levels
        let depth = book.get_depth(3);

        assert_eq!(depth.bids.len(), 3);
        assert_eq!(depth.asks.len(), 3);

        // Should get the best 3 levels
        assert_eq!(depth.bids[0].price_tick, 104); // Highest bid
        assert_eq!(depth.bids[1].price_tick, 103);
        assert_eq!(depth.bids[2].price_tick, 102);

        assert_eq!(depth.asks[0].price_tick, 110); // Lowest ask
        assert_eq!(depth.asks[1].price_tick, 111);
        assert_eq!(depth.asks[2].price_tick, 112);
    }

    #[test]
    fn test_get_depth_after_matching() {
        let mut book = setup_book();

        // Add orders
        book.add_order(1, 100, 10, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 102, 5, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(1, 105, 8, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(1, 108, 12, OrderSide::Ask, TimeInForce::GTC);

        // Match some orders - this should consume the bid at 102 and partially consume ask at 105
        book.add_order(1, 105, 3, OrderSide::Bid, TimeInForce::GTC);

        let depth = book.get_depth(10);

        // After matching: both bids should remain (102 and 100)
        assert_eq!(depth.bids.len(), 2);
        assert_eq!(depth.bids[0].price_tick, 102);
        assert_eq!(depth.bids[0].quantity, 5);
        assert_eq!(depth.bids[1].price_tick, 100);
        assert_eq!(depth.bids[1].quantity, 10);

        // Ask at 105 should be partially consumed (8 - 3 = 5), ask at 108 should remain
        assert_eq!(depth.asks.len(), 2);
        assert_eq!(depth.asks[0].price_tick, 105);
        assert_eq!(depth.asks[0].quantity, 5); // 8 - 3 = 5
        assert_eq!(depth.asks[1].price_tick, 108);
        assert_eq!(depth.asks[1].quantity, 12);
    }

    #[test]
    fn test_ask_crossing_bid_fix() {
        let mut book = setup_book();

        // Add bids at 102 (simulating your scenario with smaller numbers)
        book.add_order(1, 102, 2, OrderSide::Bid, TimeInForce::GTC);

        // Add an ask at 101 that should cross with the bids
        let (ask_order, trades) = book.add_order(2, 101, 1, OrderSide::Ask, TimeInForce::GTC);

        // The ask should be fully filled and not remain in the book
        assert!(ask_order.is_some());
        let ask_order = ask_order.unwrap();
        assert_eq!(ask_order.quantity_filled, 1);
        assert_eq!(ask_order.quantity, 1);

        // Should have 1 trade
        assert_eq!(trades.len(), 1);
        let trade = &trades[0];
        assert_eq!(trade.quantity, 1);
        assert_eq!(trade.price_tick, 102); // Should match at bid price
        assert_eq!(trade.taker_order_id, ask_order.id);
        assert_eq!(trade.maker_order_id, 0); // First bid order

        // The bid should be partially filled (2 - 1 = 1 remaining)
        let bid_level = book.bid_side.levels.get(&102).unwrap();
        assert_eq!(bid_level.total_quantity, 1);
        assert_eq!(bid_level.orders[0].quantity_filled, 1);

        // The ask should not be in the book since it was fully filled
        assert!(book.ask_side.levels.get(&101).is_none());
        assert_eq!(book.ask_side.best_tick, None);
    }
}
