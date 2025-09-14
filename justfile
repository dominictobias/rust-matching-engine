# Development commands for the trade-engine project

# Start the HTTP server in development mode with auto-reload
dev:
    cargo watch -c -x "run --bin http-server"

# Start the HTTP server normally
start:
    cargo run --bin http-server

# Build the project
build:
    cargo build

# Run tests
test:
    cargo test

# Run tests for the matcher specifically
test-matcher:
    cargo test -p matcher

# Run tests for the HTTP server specifically
test-server:
    cargo test -p http-server

# Clean build artifacts
clean:
    cargo clean

# Format code
fmt:
    cargo fmt

# Check code without building
check:
    cargo check

# Run clippy lints
lint:
    cargo clippy

# Run benchmarks
bench:
    cargo bench

# Start the trading UI development server
ui-dev:
    cd trading-ui && bun run dev

# Build the trading UI
ui-build:
    cd trading-ui && bun run build

# Install UI dependencies
ui-install:
    cd trading-ui && bun install
