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

## Screenshots

<img width="1504" height="556" alt="Screenshot 2025-09-17 at 1 27 59 PM" src="https://github.com/user-attachments/assets/d39c84ae-534c-4fc2-801c-230542cb81f9" />
<img width="1512" height="855" alt="Screenshot 2025-09-17 at 1 27 50 PM" src="https://github.com/user-attachments/assets/58b72446-5fe5-41d1-911c-706c410f682c" />
