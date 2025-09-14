# HTTP Server

A Rust HTTP server built with Axum for the trade-engine project.

## Development Setup

### Prerequisites

- Rust (latest stable version)
- `cargo-watch` for automatic rebuilding during development

### Installing cargo-watch

If you don't have `cargo-watch` installed, install it with:

```bash
cargo install cargo-watch
```

### Running in Development Mode

To run the server with automatic rebuilding and restarting when you make changes:

```bash
# From the project root
cargo watch -x "run --bin http-server"

# Or from the http-server directory
cd http-server
cargo watch -x run
```

### Additional cargo-watch Options

```bash
# Clear screen on restart and show verbose output
cargo watch -c -x "run --bin http-server"

# Add a delay to avoid rapid restarts (2 seconds)
cargo watch -d 2 -x "run --bin http-server"

# Watch only specific files/directories
cargo watch -x "run --bin http-server" --watch src

# Run tests on file changes instead of the server
cargo watch -x test
```

### Manual Commands

```bash
# Run the server normally (without auto-reload)
cargo run

# Build the project
cargo build

# Run tests
cargo test

# Check code without building
cargo check

# Format code
cargo fmt

# Run clippy lints
cargo clippy
```

## Project Structure

- `src/main.rs` - Main server entry point
- `src/middleware.rs` - Custom middleware
- `src/models/` - Data models and database connections
- `src/routes/` - API route handlers

## Dependencies

- **axum** - Web framework
- **tokio** - Async runtime
- **serde** - Serialization/deserialization
- **tracing** - Logging
- **tower-http** - HTTP middleware (CORS)
- **matcher** - Local trading engine matcher
