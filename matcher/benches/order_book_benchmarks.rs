use criterion::{criterion_group, criterion_main, Criterion};
use matcher::orderbook::OrderBook;
use matcher::types::{OrderSide, TimeInForce};
use std::hint::black_box;

// Benchmark for adding limit orders to an empty book
fn bench_add_limit_orders(c: &mut Criterion) {
    c.bench_function("add_limit_order", |b| {
        b.iter_with_setup(
            || OrderBook::new(100_000),
            |mut book| {
                black_box(book.add_order(10100, 10, OrderSide::Bid, TimeInForce::GTC));
            },
        )
    });
}

// Benchmark for simple GTC order matching
fn bench_gtc_order_matching(c: &mut Criterion) {
    c.bench_function("immediate_match_gtc", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new(100_000);
                book.add_order(10100, 10, OrderSide::Ask, TimeInForce::GTC);
                book
            },
            |mut book| {
                black_box(book.add_order(10100, 5, OrderSide::Bid, TimeInForce::GTC));
            },
        )
    });
}

// Benchmark for IOC order matching
fn bench_ioc_order_matching(c: &mut Criterion) {
    c.bench_function("immediate_match_ioc", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new(100_000);
                book.add_order(10100, 10, OrderSide::Ask, TimeInForce::GTC);
                book
            },
            |mut book| {
                black_box(book.add_order(10100, 5, OrderSide::Bid, TimeInForce::IOC));
            },
        )
    });
}

// Benchmark for FOK order matching
fn bench_fok_order_matching(c: &mut Criterion) {
    c.bench_function("immediate_match_fok", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new(100_000);
                book.add_order(10100, 10, OrderSide::Ask, TimeInForce::GTC);
                book
            },
            |mut book| {
                black_box(book.add_order(10100, 10, OrderSide::Bid, TimeInForce::FOK));
            },
        )
    });
}

// Benchmark for GTC market orders sweeping multiple levels
fn bench_gtc_market_orders(c: &mut Criterion) {
    c.bench_function("market_order_sweep_gtc", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new(100_000);
                // Populate the ask side
                for i in 0..10 {
                    book.add_order(10100 + i, 10, OrderSide::Ask, TimeInForce::GTC);
                }
                book
            },
            |mut book| {
                // Market buy order that will sweep some of the book
                black_box(book.add_order(0, 25, OrderSide::Bid, TimeInForce::GTC));
            },
        )
    });
}

// Benchmark for IOC market orders
fn bench_ioc_market_orders(c: &mut Criterion) {
    c.bench_function("market_order_sweep_ioc", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new(100_000);
                // Populate the ask side
                for i in 0..10 {
                    book.add_order(10100 + i, 10, OrderSide::Ask, TimeInForce::GTC);
                }
                book
            },
            |mut book| {
                // Market buy order that will sweep some of the book
                black_box(book.add_order(0, 25, OrderSide::Bid, TimeInForce::IOC));
            },
        )
    });
}

// Benchmark for FOK market orders
fn bench_fok_market_orders(c: &mut Criterion) {
    c.bench_function("market_order_sweep_fok", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new(100_000);
                // Populate the ask side
                for i in 0..10 {
                    book.add_order(10100 + i, 10, OrderSide::Ask, TimeInForce::GTC);
                }
                book
            },
            |mut book| {
                // Market buy order that will sweep some of the book
                black_box(book.add_order(0, 100, OrderSide::Bid, TimeInForce::FOK));
            },
        )
    });
}

// Benchmark for cancelling an order
fn bench_order_cancellation(c: &mut Criterion) {
    c.bench_function("cancel_order", |b| {
        b.iter_with_setup(
            || {
                let mut book = OrderBook::new(100_000);
                let (order, _) = book.add_order(10100, 10, OrderSide::Bid, TimeInForce::GTC);
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

criterion_group!(
    benches,
    bench_add_limit_orders,
    bench_gtc_order_matching,
    bench_ioc_order_matching,
    bench_fok_order_matching,
    bench_gtc_market_orders,
    bench_ioc_market_orders,
    bench_fok_market_orders,
    bench_order_cancellation
);
criterion_main!(benches);
