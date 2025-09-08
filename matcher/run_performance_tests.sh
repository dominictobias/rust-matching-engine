#!/bin/bash

echo "🚀 Running OrderBook Performance Tests"
echo "======================================"

# Build the project first
echo "📦 Building project..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

echo ""
echo "🏃 Running Criterion benchmarks..."
cargo bench

echo ""
echo "📊 Analyzing benchmark results..."
cargo run --bin performance_analyzer

echo ""
echo "⚡ Running load tests..."
cargo run --bin load_tester

echo ""
echo "✅ Performance testing complete!"
echo ""
echo "📈 Quick Summary:"
echo "   - Criterion benchmarks: Check target/criterion/report/index.html"
echo "   - Performance analyzer: Shows ops/sec for each benchmark"
echo "   - Load tester: Shows sustained performance under different scenarios"
