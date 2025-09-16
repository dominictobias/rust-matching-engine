use criterion::{Criterion, criterion_group, criterion_main};
use matcher::orderbook::OrderBook;
use matcher::types::{OrderSide, TimeInForce};
use std::hint::black_box;

// Benchmark for adding limit orders to an empty book
fn bench_add_limit_orders(c: &mut Criterion) {
    c.bench_function("add_limit_order", |b| {
        b.iter_with_setup(
            || OrderBook::new("TEST-USD".to_string(), 100_000),
            |mut book| {
                black_box(book.add_order(1, 10100, 10, OrderSide::Bid, TimeInForce::GTC));
            },
        )
    });
}

// Benchmark for simple GTC order matching
fn bench_gtc_order_matching(c: &mut Criterion) {
    c.bench_function("immediate_match_gtc", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new("TEST-USD".to_string(), 100_000);
                book.add_order(1, 10100, 10, OrderSide::Ask, TimeInForce::GTC);
                book
            },
            |mut book| {
                black_box(book.add_order(1, 10100, 5, OrderSide::Bid, TimeInForce::GTC));
            },
        )
    });
}

// Benchmark for IOC order matching
fn bench_ioc_order_matching(c: &mut Criterion) {
    c.bench_function("immediate_match_ioc", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new("TEST-USD".to_string(), 100_000);
                book.add_order(1, 10100, 10, OrderSide::Ask, TimeInForce::GTC);
                book
            },
            |mut book| {
                black_box(book.add_order(1, 10100, 5, OrderSide::Bid, TimeInForce::IOC));
            },
        )
    });
}

// Benchmark for FOK order matching
fn bench_fok_order_matching(c: &mut Criterion) {
    c.bench_function("immediate_match_fok", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new("TEST-USD".to_string(), 100_000);
                book.add_order(1, 10100, 10, OrderSide::Ask, TimeInForce::GTC);
                book
            },
            |mut book| {
                black_box(book.add_order(1, 10100, 10, OrderSide::Bid, TimeInForce::FOK));
            },
        )
    });
}

// Benchmark for GTC market orders sweeping multiple levels
fn bench_gtc_market_orders(c: &mut Criterion) {
    c.bench_function("market_order_sweep_gtc", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new("TEST-USD".to_string(), 100_000);
                // Populate the ask side
                for i in 0..10 {
                    book.add_order(1, 10100 + i, 10, OrderSide::Ask, TimeInForce::GTC);
                }
                book
            },
            |mut book| {
                // Market buy order that will sweep some of the book
                black_box(book.add_order(1, 0, 25, OrderSide::Bid, TimeInForce::GTC));
            },
        )
    });
}

// Benchmark for IOC market orders
fn bench_ioc_market_orders(c: &mut Criterion) {
    c.bench_function("market_order_sweep_ioc", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new("TEST-USD".to_string(), 100_000);
                // Populate the ask side
                for i in 0..10 {
                    book.add_order(1, 10100 + i, 10, OrderSide::Ask, TimeInForce::GTC);
                }
                book
            },
            |mut book| {
                // Market buy order that will sweep some of the book
                black_box(book.add_order(1, 0, 25, OrderSide::Bid, TimeInForce::IOC));
            },
        )
    });
}

// Benchmark for FOK market orders
fn bench_fok_market_orders(c: &mut Criterion) {
    c.bench_function("market_order_sweep_fok", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new("TEST-USD".to_string(), 100_000);
                // Populate the ask side
                for i in 0..10 {
                    book.add_order(1, 10100 + i, 10, OrderSide::Ask, TimeInForce::GTC);
                }
                book
            },
            |mut book| {
                // Market buy order that will sweep some of the book
                black_box(book.add_order(1, 0, 100, OrderSide::Bid, TimeInForce::FOK));
            },
        )
    });
}

// Benchmark for cancelling an order
fn bench_order_cancellation(c: &mut Criterion) {
    c.bench_function("cancel_order", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new("TEST-USD".to_string(), 100_000);
                let (order, _) = book.add_order(1, 10100, 10, OrderSide::Bid, TimeInForce::GTC);
                (book, order.unwrap())
            },
            |(mut book, order_to_cancel)| {
                black_box(book.cancel_order(
                    order_to_cancel.id,
                    order_to_cancel.price_tick,
                    order_to_cancel.side,
                ));
            },
        )
    });
}

// Throughput benchmarks - measure operations per second
fn bench_throughput_add_orders(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.measurement_time(std::time::Duration::from_secs(10));
    group.sample_size(100);

    group.bench_function("add_orders_throughput", |b| {
        b.iter_with_setup(
            || OrderBook::new("TEST-USD".to_string(), 100_000),
            |mut book| {
                // Perform multiple operations to get better throughput measurement
                for i in 0..1000 {
                    let price = 10100 + (i % 100);
                    let side = if i % 2 == 0 {
                        OrderSide::Bid
                    } else {
                        OrderSide::Ask
                    };
                    black_box(book.add_order(1, price, 10, side, TimeInForce::GTC));
                }
            },
        )
    });
}

fn bench_throughput_mixed_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    group.measurement_time(std::time::Duration::from_secs(10));
    group.sample_size(100);

    group.bench_function("mixed_operations_throughput", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new("TEST-USD".to_string(), 100_000);
                let mut order_ids = Vec::new();

                // Pre-populate with some orders and track their IDs
                for i in 0..100 {
                    let (order, _) =
                        book.add_order(1, 10100 + i, 10, OrderSide::Bid, TimeInForce::GTC);
                    if let Some(order) = order {
                        order_ids.push((order.id, 10100 + i, OrderSide::Bid));
                    }
                    let (order, _) =
                        book.add_order(1, 10200 + i, 10, OrderSide::Ask, TimeInForce::GTC);
                    if let Some(order) = order {
                        order_ids.push((order.id, 10200 + i, OrderSide::Ask));
                    }
                }
                (book, order_ids)
            },
            |(mut book, mut order_ids)| {
                // Mix of operations: adds, matches, cancels
                for i in 0..500 {
                    match i % 4 {
                        0 => {
                            // Add new order
                            let price = 10300 + (i % 50);
                            let (order, _) =
                                book.add_order(1, price, 5, OrderSide::Bid, TimeInForce::GTC);
                            if let Some(order) = order {
                                order_ids.push((order.id, price, OrderSide::Bid));
                            }
                        }
                        1 => {
                            // Try to match with IOC
                            black_box(book.add_order(
                                1,
                                10150,
                                5,
                                OrderSide::Ask,
                                TimeInForce::IOC,
                            ));
                        }
                        2 => {
                            // Market order
                            black_box(book.add_order(1, 0, 5, OrderSide::Bid, TimeInForce::GTC));
                        }
                        _ => {
                            // Cancel an existing order
                            if !order_ids.is_empty() {
                                let (order_id, price_tick, side) =
                                    order_ids[(i as usize) % order_ids.len()];
                                let cancelled = book.cancel_order(order_id, price_tick, side);
                                if cancelled {
                                    // Remove from our tracking list
                                    order_ids.retain(|&(id, _, _)| id != order_id);
                                }
                            }
                        }
                    }
                }
            },
        )
    });
}

fn bench_sustained_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_load");
    group.measurement_time(std::time::Duration::from_secs(30)); // Longer measurement
    group.sample_size(50);

    group.bench_function("sustained_add_orders", |b| {
        b.iter_with_setup(
            || OrderBook::new("TEST-USD".to_string(), 100_000),
            |mut book| {
                // Simulate sustained load with 10,000 operations
                for i in 0..10_000 {
                    let price = 10100 + (i % 1000);
                    let side = if i % 2 == 0 {
                        OrderSide::Bid
                    } else {
                        OrderSide::Ask
                    };
                    let quantity = 1 + (i % 100);
                    black_box(book.add_order(1, price, quantity, side, TimeInForce::GTC));
                }
            },
        )
    });
}

criterion_group!(
    benches,
    bench_add_limit_orders,
    bench_gtc_order_matching,
    bench_ioc_order_matching,
    bench_fok_order_matching,
    bench_gtc_market_orders,
    bench_ioc_market_orders,
    bench_fok_market_orders,
    bench_order_cancellation,
    bench_throughput_add_orders,
    bench_throughput_mixed_operations,
    bench_sustained_load
);
criterion_main!(benches);
