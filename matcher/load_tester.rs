use matcher::orderbook::OrderBook;
use matcher::types::{OrderSide, TimeInForce};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// Load testing tool for sustained performance measurement
fn main() {
    println!("=== OrderBook Load Testing ===\n");

    // Test different scenarios
    test_sustained_add_orders();
    test_mixed_workload();
    test_concurrent_access();
    test_memory_usage();
}

fn test_sustained_add_orders() {
    println!("📈 Testing sustained add orders...");

    let duration = Duration::from_secs(10);
    let start = Instant::now();
    let mut book = OrderBook::new("TEST-USD".to_string(), 100_000, 100);
    let mut operations = 0;

    while start.elapsed() < duration {
        let price = 10100 + (operations % 1000);
        let side = if operations % 2 == 0 {
            OrderSide::Bid
        } else {
            OrderSide::Ask
        };
        let quantity = 1 + (operations % 100);

        book.add_order(1, price, quantity, side, TimeInForce::GTC);
        operations += 1;
    }

    let elapsed = start.elapsed();
    let ops_per_second = operations as f64 / elapsed.as_secs_f64();

    println!("   Operations: {}", operations);
    println!("   Duration: {:.2}s", elapsed.as_secs_f64());
    println!("   Throughput: {:.0} ops/sec", ops_per_second);
    println!("   Orders in book: {}\n", book.total_orders());
}

fn test_mixed_workload() {
    println!("🔄 Testing mixed workload...");

    let duration = Duration::from_secs(10);
    let start = Instant::now();
    let mut book = OrderBook::new("TEST-USD".to_string(), 100_000, 100);
    let mut order_ids = Vec::new();

    // Pre-populate with some orders and track their IDs
    for i in 0..1000 {
        let (order, _) = book.add_order(1, 10100 + i, 10, OrderSide::Bid, TimeInForce::GTC);
        if let Some(order) = order {
            order_ids.push((order.id, 10100 + i, OrderSide::Bid));
        }
        let (order, _) = book.add_order(1, 10200 + i, 10, OrderSide::Ask, TimeInForce::GTC);
        if let Some(order) = order {
            order_ids.push((order.id, 10200 + i, OrderSide::Ask));
        }
    }

    let mut operations = 0;
    let mut matches = 0;
    let mut cancellations = 0;

    while start.elapsed() < duration {
        match operations % 5 {
            0 => {
                // Add new limit order
                let price = 10300 + (operations % 500);
                let (order, _) = book.add_order(1, price, 5, OrderSide::Bid, TimeInForce::GTC);
                if let Some(order) = order {
                    order_ids.push((order.id, price, OrderSide::Bid));
                }
            }
            1 => {
                // Try to match with IOC
                let (_, trades) = book.add_order(1, 10150, 5, OrderSide::Ask, TimeInForce::IOC);
                if !trades.is_empty() {
                    matches += 1;
                }
            }
            2 => {
                // Market order
                book.add_order(1, 0, 5, OrderSide::Bid, TimeInForce::GTC);
            }
            3 => {
                // FOK order
                let (_, trades) = book.add_order(1, 10100, 10, OrderSide::Ask, TimeInForce::FOK);
                if !trades.is_empty() {
                    matches += 1;
                }
            }
            _ => {
                // Cancel an existing order
                if !order_ids.is_empty() {
                    let (order_id, price_tick, side) =
                        order_ids[(operations as usize) % order_ids.len()];
                    let cancelled = book.cancel_order(order_id, price_tick, side);
                    if cancelled {
                        cancellations += 1;
                        // Remove from our tracking list
                        order_ids.retain(|&(id, _, _)| id != order_id);
                    }
                }
            }
        }
        operations += 1;
    }

    let elapsed = start.elapsed();
    let ops_per_second = operations as f64 / elapsed.as_secs_f64();

    println!("   Operations: {}", operations);
    println!("   Matches: {}", matches);
    println!("   Cancellations: {}", cancellations);
    println!("   Duration: {:.2}s", elapsed.as_secs_f64());
    println!("   Throughput: {:.0} ops/sec", ops_per_second);
    println!("   Orders in book: {}\n", book.total_orders());
}

fn test_concurrent_access() {
    println!("⚡ Testing concurrent access simulation...");

    let book = Arc::new(std::sync::Mutex::new(OrderBook::new(
        "TEST-USD".to_string(),
        100_000,
        100,
    )));
    let operations = Arc::new(AtomicU64::new(0));
    let duration = Duration::from_secs(5);

    let num_threads = 4;
    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let book_clone = Arc::clone(&book);
        let ops_clone = Arc::clone(&operations);

        let handle = thread::spawn(move || {
            let start = Instant::now();
            let mut local_ops = 0;

            while start.elapsed() < duration {
                {
                    let mut book = book_clone.lock().unwrap();
                    let price = 10100 + ((thread_id * 1000) + local_ops) % 1000;
                    let side = if local_ops % 2 == 0 {
                        OrderSide::Bid
                    } else {
                        OrderSide::Ask
                    };

                    book.add_order(1, price, 10, side, TimeInForce::GTC);
                }
                local_ops += 1;
            }

            ops_clone.fetch_add(local_ops, Ordering::Relaxed);
        });

        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    let total_ops = operations.load(Ordering::Relaxed);
    let ops_per_second = total_ops as f64 / duration.as_secs_f64();

    println!("   Threads: {}", num_threads);
    println!("   Total operations: {}", total_ops);
    println!("   Duration: {:.2}s", duration.as_secs_f64());
    println!("   Throughput: {:.0} ops/sec", ops_per_second);

    let book = book.lock().unwrap();
    println!("   Orders in book: {}\n", book.total_orders());
}

fn test_memory_usage() {
    println!("💾 Testing memory usage under load...");

    let mut book = OrderBook::new("TEST-USD".to_string(), 100_000, 100);
    let start = Instant::now();

    // Add a large number of orders
    for i in 0..50_000 {
        let price = 10100 + (i % 1000);
        let side = if i % 2 == 0 {
            OrderSide::Bid
        } else {
            OrderSide::Ask
        };
        let quantity = 1 + (i % 1000);

        book.add_order(1, price, quantity, side, TimeInForce::GTC);

        if i % 10_000 == 0 && i > 0 {
            let elapsed = start.elapsed();
            let ops_per_second = i as f64 / elapsed.as_secs_f64();
            println!("   {} orders: {:.0} ops/sec", i, ops_per_second);
        }
    }

    let elapsed = start.elapsed();
    let ops_per_second = 50_000.0 / elapsed.as_secs_f64();

    println!("   Final: 50,000 orders in {:.2}s", elapsed.as_secs_f64());
    println!("   Average throughput: {:.0} ops/sec", ops_per_second);
    println!("   Orders in book: {}\n", book.total_orders());
}
