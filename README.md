# Rust matching engine

This project consists of:

- **matcher**: A Rust matching engine. The main data structures are a balancing tree map (`BTreeMap`) for price levels, and a ring buffer (`VecDeque`) for prices.

- **http-server**: A Rust/Axum HTTP server with in-memory state and a naive auth mechanism to test out the orderbook.

- **trading-ui**: A React front end to play around with placing orders.

## Installation

- Install Rust and [Bun](https://bun.sh/).
- Run `bun install` in `trading-ui`.

## Running

Start all three services vwith `cargo run .` or `bun dev`.
