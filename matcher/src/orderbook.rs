use super::types::{Order, OrderSide, TimeInForce, Trade};
use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct PriceLevel {
    orders: VecDeque<Order>,
    total_quantity: u64,
}

/// Represents one side of the orderbook (bid or ask)
/// Contains the best and worst price ticks and traversal direction
pub struct OrderbookSide {
    pub best_tick: Option<u64>,
    pub worst_tick: Option<u64>,
    /// true if higher prices are better (for bids), false if lower prices are better (for asks)
    pub higher_is_better: bool,
}

pub struct OrderBook {
    levels: Vec<Option<PriceLevel>>,

    /// Ask side of the orderbook (lower prices are better)
    ask_side: OrderbookSide,
    /// Bid side of the orderbook (higher prices are better)
    bid_side: OrderbookSide,

    /// Maximum price tick we support (determines Vec size)
    max_price_tick: u64,

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
    /// Gets a reference to the appropriate side based on OrderSide
    fn get_side(&self, side: OrderSide) -> &OrderbookSide {
        match side {
            OrderSide::Bid => &self.bid_side,
            OrderSide::Ask => &self.ask_side,
        }
    }

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

    /// Creates a new, empty OrderBook instance with specified price range
    pub fn new(max_price_tick: u64) -> Self {
        OrderBook {
            levels: vec![None; max_price_tick as usize + 1],
            ask_side: OrderbookSide {
                best_tick: None,
                worst_tick: None,
                higher_is_better: false, // Lower prices are better for asks
            },
            bid_side: OrderbookSide {
                best_tick: None,
                worst_tick: None,
                higher_is_better: true, // Higher prices are better for bids
            },
            max_price_tick,
            order_id_counter: 0,
            trade_id_counter: 0,
            total_orders: 0,
        }
    }

    pub fn add_order(
        &mut self,
        price_tick: u64,
        quantity: u64,
        side: OrderSide,
        time_in_force: TimeInForce,
    ) -> (Option<Order>, Vec<Trade>) {
        // Check if price tick is within bounds (except for market orders)
        if price_tick > 0 && price_tick > self.max_price_tick {
            return (None, Vec::new());
        }

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

    // Assume it's being called when we have a price
    fn get_tick_iter_bounds(&self, order: &Order) -> (usize, usize) {
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
                    self.ask_side.worst_tick.unwrap_or(self.max_price_tick)
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

        (start_tick as usize, end_tick as usize)
    }

    fn can_fill_fok(&self, order: &Order) -> bool {
        let best_tick = self.get_opposite_best_tick(order.side);

        // If there are no orders on the matching side, we can't match
        if best_tick.is_none() {
            return false;
        }

        let (start_tick, end_tick) = self.get_tick_iter_bounds(order);

        let mut qty_till_price: u64 = 0;

        for tick in start_tick..=end_tick {
            let level_option = &self.levels[tick as usize];
            if let Some(level) = level_option {
                qty_till_price += level.total_quantity;
            }
            if qty_till_price >= order.quantity {
                return true;
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

        // For market orders, we need to handle the iteration direction correctly
        let tick_range = if order.price_tick == 0 && order.side == OrderSide::Ask {
            // Market sell order: iterate from best_bid down to worst_bid
            (end_tick..=start_tick).rev().collect::<Vec<_>>()
        } else if order.price_tick == 0 && order.side == OrderSide::Bid {
            // Market buy order: iterate from best_ask up to worst_ask
            (start_tick..=end_tick).collect::<Vec<_>>()
        } else {
            // Limit orders: normal iteration
            (start_tick..=end_tick).collect::<Vec<_>>()
        };

        'outer: for tick in tick_range {
            let level_option = &mut self.levels[tick as usize];

            if let Some(level) = level_option {
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
                        // Set the level to None if it's empty before breaking
                        if level.total_quantity == 0 {
                            self.levels[tick as usize] = None;
                        }
                        break 'outer;
                    }
                }
            }

            // Set the level to None if it's empty (after processing all orders in the level)
            if let Some(level) = &self.levels[tick as usize] {
                if level.total_quantity == 0 {
                    self.levels[tick as usize] = None;
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
        // First, get all the values we need without holding any mutable references
        let (current_best, current_worst, higher_is_better) = {
            let side_ref = self.get_side(side);
            (
                side_ref.best_tick,
                side_ref.worst_tick,
                side_ref.higher_is_better,
            )
        };

        // If we don't have any ticks, nothing to update
        if current_best.is_none() || current_worst.is_none() {
            return;
        }

        let best_tick = current_best.unwrap();
        let worst_tick = current_worst.unwrap();

        // Check if best tick was consumed
        let best_consumed = self.levels[best_tick as usize].is_none();

        // Check if worst tick was consumed
        let worst_consumed = self.levels[worst_tick as usize].is_none();

        if best_consumed && worst_consumed {
            // Both consumed - find new range
            let (new_best, new_worst) = self.find_new_tick_range(side, higher_is_better);
            let side_mut = self.get_side_mut(side);
            side_mut.best_tick = new_best;
            side_mut.worst_tick = new_worst;
        } else if best_consumed {
            // Only best consumed - find new best
            let new_best = self.find_new_best_tick(side, worst_tick, higher_is_better);
            let side_mut = self.get_side_mut(side);
            side_mut.best_tick = new_best;
        } else if worst_consumed {
            // Only worst consumed - find new worst
            let new_worst = self.find_new_worst_tick(side, best_tick, higher_is_better);
            let side_mut = self.get_side_mut(side);
            side_mut.worst_tick = new_worst;
        }
        // If neither consumed, no update needed
    }

    /// Finds new best and worst ticks when both are consumed
    fn find_new_tick_range(
        &self,
        _side: OrderSide,
        higher_is_better: bool,
    ) -> (Option<u64>, Option<u64>) {
        let mut best_tick = None;
        let mut worst_tick = None;

        if higher_is_better {
            // For bids: find lowest and highest prices with orders
            for tick in 0..=self.max_price_tick {
                if self.levels[tick as usize].is_some() {
                    if best_tick.is_none() {
                        worst_tick = Some(tick); // First found is worst (lowest)
                    }
                    best_tick = Some(tick); // Last found is best (highest)
                }
            }
        } else {
            // For asks: find highest and lowest prices with orders
            for tick in (0..=self.max_price_tick).rev() {
                if self.levels[tick as usize].is_some() {
                    if best_tick.is_none() {
                        worst_tick = Some(tick); // First found is worst (highest)
                    }
                    best_tick = Some(tick); // Last found is best (lowest)
                }
            }
        }

        (best_tick, worst_tick)
    }

    /// Finds new best tick when only best is consumed
    fn find_new_best_tick(
        &self,
        _side: OrderSide,
        current_worst: u64,
        higher_is_better: bool,
    ) -> Option<u64> {
        if higher_is_better {
            // For bids: find highest price between current worst and max_price_tick
            for tick in (current_worst..=self.max_price_tick).rev() {
                if self.levels[tick as usize].is_some() {
                    return Some(tick);
                }
            }
        } else {
            // For asks: find lowest price between 0 and current worst
            for tick in 0..=current_worst {
                if self.levels[tick as usize].is_some() {
                    return Some(tick);
                }
            }
        }
        None
    }

    /// Finds new worst tick when only worst is consumed
    fn find_new_worst_tick(
        &self,
        _side: OrderSide,
        current_best: u64,
        higher_is_better: bool,
    ) -> Option<u64> {
        if higher_is_better {
            // For bids: find lowest price between best and current worst
            for tick in 0..=current_best {
                if self.levels[tick as usize].is_some() {
                    return Some(tick);
                }
            }
        } else {
            // For asks: find highest price between best and current worst
            for tick in (current_best..=self.max_price_tick).rev() {
                if self.levels[tick as usize].is_some() {
                    return Some(tick);
                }
            }
        }
        None
    }

    fn add_limit_order(&mut self, order: Order) {
        let price_tick = order.price_tick;
        let order_side = order.side;

        if (price_tick as usize) >= self.levels.len() {
            return;
        }
        let level = self.levels[price_tick as usize].get_or_insert_with(|| PriceLevel {
            orders: VecDeque::new(),
            total_quantity: 0,
        });

        level.orders.push_back(order.clone());
        level.total_quantity += order.quantity - order.quantity_filled;

        // Update best and worst ticks for the appropriate side
        let side_mut = self.get_side_mut(order_side);
        let higher_is_better = side_mut.higher_is_better;

        match side_mut.best_tick {
            None => {
                // First order on this side
                side_mut.best_tick = Some(price_tick);
                side_mut.worst_tick = Some(price_tick);
            }
            Some(current_best) => {
                // Check if this is a better price
                let is_better = if higher_is_better {
                    price_tick > current_best
                } else {
                    price_tick < current_best
                };

                if is_better {
                    side_mut.best_tick = Some(price_tick);
                }

                // Update worst tick if this is a worse price
                match side_mut.worst_tick {
                    None => side_mut.worst_tick = Some(price_tick),
                    Some(current_worst) => {
                        let is_worse = if higher_is_better {
                            price_tick < current_worst
                        } else {
                            price_tick > current_worst
                        };

                        if is_worse {
                            side_mut.worst_tick = Some(price_tick);
                        }
                    }
                }
            }
        }

        self.total_orders += 1;
    }

    pub fn cancel_order(&mut self, order_id: u64, price_tick: u64, side: OrderSide) -> bool {
        if (price_tick as usize) >= self.levels.len() {
            return false;
        }

        if let Some(level) = &mut self.levels[price_tick as usize] {
            if let Some(index) = level.orders.iter().position(|o| o.id == order_id) {
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
                        self.total_orders -= 1;

                        // If the level is now empty, set it to None and update ticks
                        if level.total_quantity == 0 {
                            self.levels[price_tick as usize] = None;
                        }
                    }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OrderSide, TimeInForce};

    fn setup_book() -> OrderBook {
        OrderBook::new(1000)
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
        assert_eq!(book.max_price_tick, 1000);
    }

    #[test]
    fn test_add_gtc_limit_order() {
        let mut book = setup_book();
        let price_tick = 101;
        let quantity = 10;

        // Add a buy order
        let (order, trades) =
            book.add_order(price_tick, quantity, OrderSide::Bid, TimeInForce::GTC);

        assert!(order.is_some());
        let order = order.unwrap();
        assert_eq!(order.price_tick, price_tick);
        assert_eq!(order.quantity, quantity);
        assert_eq!(order.side, OrderSide::Bid);
        assert!(trades.is_empty());

        assert_eq!(book.bid_side.best_tick, Some(price_tick));
        assert_eq!(book.bid_side.worst_tick, Some(price_tick));
        assert!(book.ask_side.best_tick.is_none());
        let level = book.levels[price_tick as usize].as_ref().unwrap();
        assert_eq!(level.total_quantity, quantity);
        assert_eq!(level.orders.len(), 1);
        assert_eq!(book.total_orders, 1);

        // Add a sell order
        let sell_price_tick = 102;
        let (sell_order, trades) =
            book.add_order(sell_price_tick, 5, OrderSide::Ask, TimeInForce::GTC);
        assert!(sell_order.is_some());
        assert!(trades.is_empty());
        assert_eq!(book.ask_side.best_tick, Some(sell_price_tick));
        assert_eq!(book.ask_side.worst_tick, Some(sell_price_tick));
        let ask_level = book.levels[sell_price_tick as usize].as_ref().unwrap();
        assert_eq!(ask_level.total_quantity, 5);
        assert_eq!(book.total_orders, 2);
    }

    #[test]
    fn test_simple_order_match() {
        let mut book = setup_book();

        // Add a resting sell order
        book.add_order(101, 10, OrderSide::Ask, TimeInForce::GTC);

        // Add a matching buy order
        let (buy_order, trades) = book.add_order(101, 5, OrderSide::Bid, TimeInForce::GTC);

        assert!(buy_order.is_some());
        let buy_order = buy_order.unwrap();
        assert_eq!(buy_order.quantity_filled, 5);
        assert_eq!(trades.len(), 1);

        let trade = &trades[0];
        assert_eq!(trade.quantity, 5);
        assert_eq!(trade.price_tick, 101);
        assert_eq!(trade.taker_order_id, buy_order.id);

        // Check the state of the resting order
        let ask_level = book.levels[101 as usize].as_ref().unwrap();
        assert_eq!(ask_level.total_quantity, 5);
        assert_eq!(ask_level.orders[0].quantity_filled, 5);
    }

    #[test]
    fn test_market_order_full_fill() {
        let mut book = setup_book();
        book.add_order(101, 10, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(102, 10, OrderSide::Ask, TimeInForce::GTC);

        // Market buy order, price_tick = 0
        let (market_order, trades) = book.add_order(0, 15, OrderSide::Bid, TimeInForce::GTC);

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
        let level = book.levels[102 as usize].as_ref().unwrap();
        assert_eq!(level.total_quantity, 5);
    }

    #[test]
    fn test_cancel_order() {
        let mut book = setup_book();
        let (order, _) = book.add_order(101, 10, OrderSide::Bid, TimeInForce::GTC);
        let order_id = order.unwrap().id;

        let cancelled = book.cancel_order(order_id, 101, OrderSide::Bid);
        assert!(cancelled);

        // After cancelling the only order in the level, the level should be None
        assert!(book.levels[101 as usize].is_none());

        // Try to cancel again
        let cancelled_again = book.cancel_order(order_id, 101, OrderSide::Bid);
        assert!(!cancelled_again);
    }

    #[test]
    fn test_ioc_order_partial_fill() {
        let mut book = setup_book();
        book.add_order(101, 5, OrderSide::Ask, TimeInForce::GTC);

        // IOC order for 10, only 5 available
        let (order, trades) = book.add_order(102, 10, OrderSide::Bid, TimeInForce::IOC);

        // IOC orders are not added to the book, so we get None
        assert!(order.is_none());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 5);

        // The resting order should be gone
        assert!(book.levels[101 as usize].is_none());
        assert!(book.ask_side.best_tick.is_none());
        assert_eq!(book.total_orders, 0);
    }

    #[test]
    fn test_fok_order_success() {
        let mut book = setup_book();
        book.add_order(101, 10, OrderSide::Ask, TimeInForce::GTC);

        // FOK order that can be filled
        let (order, trades) = book.add_order(101, 10, OrderSide::Bid, TimeInForce::FOK);

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
        book.add_order(101, 5, OrderSide::Ask, TimeInForce::GTC);

        // FOK order that cannot be fully filled
        let (order, trades) = book.add_order(101, 10, OrderSide::Bid, TimeInForce::FOK);

        // Order should be rejected
        assert!(order.is_none());
        assert!(trades.is_empty());

        // Book should be unchanged
        let level = book.levels[101 as usize].as_ref().unwrap();
        assert_eq!(level.total_quantity, 5);
        assert_eq!(book.total_orders, 1);
    }

    #[test]
    fn test_cancel_order_updates_best_tick() {
        let mut book = setup_book();
        // Add two orders on the buy side
        let (order1, _) = book.add_order(101, 10, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(100, 10, OrderSide::Bid, TimeInForce::GTC);
        let order1_id = order1.unwrap().id;

        assert_eq!(book.bid_side.best_tick, Some(101));

        // Cancel the order at the best tick
        let cancelled = book.cancel_order(order1_id, 101, OrderSide::Bid);
        assert!(cancelled);

        // The best tick should be updated to the next best price
        assert_eq!(book.bid_side.best_tick, Some(100));

        // After cancelling the only order in the level, the level should be None
        assert!(book.levels[101 as usize].is_none());
    }

    #[test]
    fn test_add_order_updates_best_tick() {
        let mut book = setup_book();

        // Test buy side - higher prices should become new best tick
        book.add_order(100, 10, OrderSide::Bid, TimeInForce::GTC);
        assert_eq!(book.bid_side.best_tick, Some(100));

        book.add_order(101, 5, OrderSide::Bid, TimeInForce::GTC);
        assert_eq!(book.bid_side.best_tick, Some(101)); // Higher price becomes best

        book.add_order(99, 5, OrderSide::Bid, TimeInForce::GTC);
        assert_eq!(book.bid_side.best_tick, Some(101)); // Lower price doesn't change best

        // Test sell side - lower prices should become new best tick
        book.add_order(110, 10, OrderSide::Ask, TimeInForce::GTC);
        assert_eq!(book.ask_side.best_tick, Some(110));

        book.add_order(109, 5, OrderSide::Ask, TimeInForce::GTC);
        assert_eq!(book.ask_side.best_tick, Some(109)); // Lower price becomes best

        book.add_order(111, 5, OrderSide::Ask, TimeInForce::GTC);
        assert_eq!(book.ask_side.best_tick, Some(109)); // Higher price doesn't change best
    }

    #[test]
    fn test_match_order_updates_best_tick() {
        let mut book = setup_book();

        // Set up sell side with multiple price levels
        book.add_order(101, 10, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(102, 10, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(103, 10, OrderSide::Ask, TimeInForce::GTC);
        assert_eq!(book.ask_side.best_tick, Some(101));

        // Market buy order that fully consumes the best ask level
        let (order, trades) = book.add_order(0, 10, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price_tick, 101);
        assert_eq!(trades[0].quantity, 10);

        // Best tick should be updated to the next best price
        assert_eq!(book.ask_side.best_tick, Some(102));

        // Another market buy that consumes the next level partially
        let (order, trades) = book.add_order(0, 5, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price_tick, 102);
        assert_eq!(trades[0].quantity, 5);

        // Best tick should remain the same since level still has quantity
        assert_eq!(book.ask_side.best_tick, Some(102));
        let level = book.levels[102 as usize].as_ref().unwrap();
        assert_eq!(level.total_quantity, 5);

        // Final market buy that fully consumes the 102 level
        let (order, trades) = book.add_order(0, 5, OrderSide::Bid, TimeInForce::GTC);
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
        book.add_order(103, 10, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(102, 10, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(101, 10, OrderSide::Bid, TimeInForce::GTC);
        assert_eq!(book.bid_side.best_tick, Some(103));

        // Market sell order that fully consumes the best bid level
        let (order, trades) = book.add_order(0, 10, OrderSide::Ask, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price_tick, 103);
        assert_eq!(trades[0].quantity, 10);

        // Best tick should be updated to the next best price
        assert_eq!(book.bid_side.best_tick, Some(102));

        // Another market sell that fully consumes two levels
        let (order, trades) = book.add_order(0, 20, OrderSide::Ask, TimeInForce::GTC);
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
        book.add_order(101, 3, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(101, 3, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(101, 4, OrderSide::Ask, TimeInForce::GTC);
        book.add_order(102, 20, OrderSide::Ask, TimeInForce::GTC);

        assert_eq!(book.ask_side.best_tick, Some(101));
        let level = book.levels[101 as usize].as_ref().unwrap();
        assert_eq!(level.total_quantity, 10);
        assert_eq!(level.orders.len(), 3);

        // Large buy order that consumes all orders at 101 and moves to 102
        let (order, trades) = book.add_order(0, 15, OrderSide::Bid, TimeInForce::GTC);
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
        assert!(book.levels[101 as usize].is_none());

        // Verify remaining quantity at 102
        let level = book.levels[102 as usize].as_ref().unwrap();
        assert_eq!(level.total_quantity, 15);
    }

    // Additional comprehensive tests
    #[test]
    fn test_market_order_no_liquidity() {
        let mut book = setup_book();

        // Market order with no liquidity should be cancelled
        let (order, trades) = book.add_order(0, 10, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_none());
        assert!(trades.is_empty());
    }

    #[test]
    fn test_ioc_order_no_liquidity() {
        let mut book = setup_book();

        // IOC order with no liquidity should be rejected
        let (order, trades) = book.add_order(100, 10, OrderSide::Bid, TimeInForce::IOC);
        assert!(order.is_none());
        assert!(trades.is_empty());
    }

    #[test]
    fn test_fok_order_no_liquidity() {
        let mut book = setup_book();

        // FOK order with no liquidity should be rejected
        let (order, trades) = book.add_order(100, 10, OrderSide::Bid, TimeInForce::FOK);
        assert!(order.is_none());
        assert!(trades.is_empty());
    }

    #[test]
    fn test_limit_order_improvement() {
        let mut book = setup_book();

        // Add a sell order at 100
        book.add_order(100, 10, OrderSide::Ask, TimeInForce::GTC);

        // Add a buy order at 99 (should not match)
        let (order, trades) = book.add_order(99, 5, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert!(trades.is_empty());

        // Add a buy order at 100 (should match)
        let (order, trades) = book.add_order(100, 5, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 5);
    }

    #[test]
    fn test_multiple_orders_same_price() {
        let mut book = setup_book();

        // Add multiple orders at the same price
        book.add_order(100, 5, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(100, 3, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(100, 2, OrderSide::Bid, TimeInForce::GTC);

        let level = book.levels[100 as usize].as_ref().unwrap();
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
        let (order, _) = book.add_order(100, 10, OrderSide::Bid, TimeInForce::GTC);
        let order_id = order.unwrap().id;

        // Try to cancel with wrong price
        let cancelled = book.cancel_order(order_id, 101, OrderSide::Bid);
        assert!(!cancelled);

        // Order should still exist
        let level = book.levels[100 as usize].as_ref().unwrap();
        assert_eq!(level.total_quantity, 10);
    }

    #[test]
    fn test_cancel_order_wrong_side() {
        let mut book = setup_book();

        // Add a bid order
        let (order, _) = book.add_order(100, 10, OrderSide::Bid, TimeInForce::GTC);
        let order_id = order.unwrap().id;

        // Try to cancel with wrong side
        let cancelled = book.cancel_order(order_id, 100, OrderSide::Ask);
        assert!(!cancelled);

        // Order should still exist
        let level = book.levels[100 as usize].as_ref().unwrap();
        assert_eq!(level.total_quantity, 10);
    }

    #[test]
    fn test_cancel_already_cancelled_order() {
        let mut book = setup_book();

        // Add an order
        let (order, _) = book.add_order(100, 10, OrderSide::Bid, TimeInForce::GTC);
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

        let (order1, _) = book.add_order(100, 10, OrderSide::Bid, TimeInForce::GTC);
        let (order2, _) = book.add_order(101, 5, OrderSide::Ask, TimeInForce::GTC);

        assert_eq!(order1.unwrap().id, 0);
        assert_eq!(order2.unwrap().id, 1);
        assert_eq!(book.order_id_counter, 2);
    }

    #[test]
    fn test_trade_id_counter_increments() {
        let mut book = setup_book();

        // Add a resting order
        book.add_order(100, 10, OrderSide::Ask, TimeInForce::GTC);

        // Add matching orders
        let (_, trades1) = book.add_order(100, 5, OrderSide::Bid, TimeInForce::GTC);
        let (_, trades2) = book.add_order(100, 3, OrderSide::Bid, TimeInForce::GTC);

        assert_eq!(trades1[0].id, 0);
        assert_eq!(trades2[0].id, 1);
        assert_eq!(book.trade_id_counter, 2);
    }

    #[test]
    fn test_worst_tick_tracking() {
        let mut book = setup_book();

        // Add orders at different prices
        book.add_order(100, 10, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(102, 5, OrderSide::Bid, TimeInForce::GTC);
        book.add_order(98, 3, OrderSide::Bid, TimeInForce::GTC);

        assert_eq!(book.bid_side.best_tick, Some(102)); // Highest price
        assert_eq!(book.bid_side.worst_tick, Some(98)); // Lowest price

        // Cancel the worst tick (the first order at 98)
        // We need to get the order ID of the first order at 98
        let level = book.levels[98 as usize].as_ref().unwrap();
        let order_id = level.orders[0].id;
        book.cancel_order(order_id, 98, OrderSide::Bid);

        assert_eq!(book.bid_side.best_tick, Some(102));
        assert_eq!(book.bid_side.worst_tick, Some(100)); // Should update to next worst
    }

    #[test]
    fn test_price_tick_bounds() {
        let mut book = setup_book();

        // Try to add order beyond max price tick
        let (order, trades) = book.add_order(1001, 10, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_none());
        assert!(trades.is_empty());

        // Try to cancel order beyond max price tick
        let cancelled = book.cancel_order(0, 1001, OrderSide::Bid);
        assert!(!cancelled);
    }

    #[test]
    fn test_zero_price_tick_limit_order() {
        let mut book = setup_book();

        // Zero price tick should not be added as limit order
        let (order, trades) = book.add_order(0, 10, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_none());
        assert!(trades.is_empty());
    }

    #[test]
    fn test_partial_fill_resting_order() {
        let mut book = setup_book();

        // Add a large resting order
        book.add_order(100, 100, OrderSide::Ask, TimeInForce::GTC);

        // Partially fill it
        let (order, trades) = book.add_order(100, 30, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 30);

        // Check resting order state
        let level = book.levels[100 as usize].as_ref().unwrap();
        assert_eq!(level.total_quantity, 70);
        assert_eq!(level.orders[0].quantity_filled, 30);
    }

    #[test]
    fn test_cross_spread_matching() {
        let mut book = setup_book();

        // Add orders that cross the spread
        book.add_order(100, 10, OrderSide::Ask, TimeInForce::GTC);
        let (bid_order, trades) = book.add_order(102, 5, OrderSide::Bid, TimeInForce::GTC);

        // The bid at 102 should match against the ask at 100, filling 5 units
        assert!(bid_order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 5);
        assert_eq!(trades[0].price_tick, 100); // Should match at ask price

        // Ask should be partially filled (10 - 5 = 5 remaining)
        let level = book.levels[100 as usize].as_ref().unwrap();
        assert_eq!(level.total_quantity, 5);

        // The bid at 102 should be fully filled and not in the book
        assert_eq!(book.bid_side.best_tick, None);
        assert_eq!(book.ask_side.best_tick, Some(100));

        // Add an aggressive order that crosses
        let (order, trades) = book.add_order(103, 8, OrderSide::Bid, TimeInForce::GTC);
        assert!(order.is_some());
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, 5); // Should match remaining ask quantity
        assert_eq!(trades[0].price_tick, 100); // Should match at ask price

        // Ask should be fully consumed
        assert!(book.levels[100 as usize].is_none());
        assert_eq!(book.ask_side.best_tick, None);

        // The new bid at 103 should remain in the book with 3 units (8 - 5 = 3)
        assert_eq!(book.bid_side.best_tick, Some(103));
        let level = book.levels[103 as usize].as_ref().unwrap();
        assert_eq!(level.total_quantity, 3);
    }
}
