use std::fs;
use std::path::Path;

/// Simple performance analyzer to convert Criterion benchmark results to ops/sec
fn main() {
    let benchmark_dir = "target/criterion";

    if !Path::new(benchmark_dir).exists() {
        println!("No benchmark results found. Run 'cargo bench' first.");
        return;
    }

    println!("=== OrderBook Performance Analysis ===\n");

    // Analyze each benchmark
    let benchmarks = [
        "add_limit_order",
        "immediate_match_gtc",
        "immediate_match_ioc",
        "immediate_match_fok",
        "market_order_sweep_gtc",
        "market_order_sweep_ioc",
        "market_order_sweep_fok",
        "cancel_order",
    ];

    for benchmark in &benchmarks {
        analyze_benchmark(benchmark_dir, benchmark);
    }

    // Check for throughput benchmarks
    let throughput_dir = format!("{}/throughput", benchmark_dir);
    if Path::new(&throughput_dir).exists() {
        println!("\n=== Throughput Benchmarks ===");
        analyze_benchmark(&throughput_dir, "add_orders_throughput");
        analyze_benchmark(&throughput_dir, "mixed_operations_throughput");
    }

    let sustained_dir = format!("{}/sustained_load", benchmark_dir);
    if Path::new(&sustained_dir).exists() {
        println!("\n=== Sustained Load Benchmarks ===");
        analyze_benchmark(&sustained_dir, "sustained_add_orders");
    }
}

fn analyze_benchmark(base_dir: &str, benchmark_name: &str) {
    let estimates_path = format!("{}/{}/new/estimates.json", base_dir, benchmark_name);

    if !Path::new(&estimates_path).exists() {
        println!("❌ {} - No results found", benchmark_name);
        return;
    }

    match fs::read_to_string(&estimates_path) {
        Ok(content) => {
            if let Ok(estimates) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(mean) = estimates.get("mean") {
                    if let Some(point_estimate) = mean.get("point_estimate") {
                        if let Some(nanoseconds) = point_estimate.as_f64() {
                            let microseconds = nanoseconds / 1000.0;
                            let ops_per_second = 1_000_000.0 / microseconds;

                            println!("📊 {}:", benchmark_name);
                            println!("   Time per operation: {:.2} μs", microseconds);
                            println!("   Operations per second: {:.0} ops/sec", ops_per_second);

                            // Add confidence interval if available
                            if let Some(ci) = mean.get("confidence_interval") {
                                if let (Some(lower), Some(upper)) = (
                                    ci.get("lower_bound").and_then(|v| v.as_f64()),
                                    ci.get("upper_bound").and_then(|v| v.as_f64()),
                                ) {
                                    let lower_ops = 1_000_000.0 / (lower / 1000.0);
                                    let upper_ops = 1_000_000.0 / (upper / 1000.0);
                                    println!(
                                        "   95% CI: {:.0} - {:.0} ops/sec",
                                        lower_ops, upper_ops
                                    );
                                }
                            }
                            println!();
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ {} - Error reading results: {}", benchmark_name, e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ops_per_second_calculation() {
        // Test with your current benchmark result: ~37,494 nanoseconds
        let nanoseconds = 37494.0;
        let microseconds = nanoseconds / 1000.0;
        let ops_per_second = 1_000_000.0 / microseconds;

        // Should be around 26,670 ops/sec
        assert!((ops_per_second - 26670.0_f64).abs() < 100.0);
    }
}
