use hex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::{User, UserFunds};

// Simple in-memory storage implementation
#[derive(Clone)]
pub struct InMemoryStorage {
    pub accounts: Arc<Mutex<HashMap<String, User>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Create a hash of the email to use as user ID
    fn hash_email(email: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(email.as_bytes());
        hex::encode(hasher.finalize())
    }

    // Get or create a user account
    pub fn get_or_create_account(&self, email: &str) -> User {
        let session_id = Self::hash_email(email);

        let mut accounts = self.accounts.lock().unwrap();

        if let Some(user) = accounts.get(&session_id) {
            return user.clone();
        }

        // Create new account with default funds
        let user_id = rand::random::<u64>();
        let new_user = User {
            user_id,
            session_id: session_id.clone(),
            email: email.to_string(),
            funds: UserFunds::default(),
        };

        accounts.insert(session_id, new_user.clone());
        new_user
    }

    // Get or create a user account with a specific session_id
    pub fn get_or_create_account_with_session(&self, email: &str, session_id: &str) -> User {
        let mut accounts = self.accounts.lock().unwrap();

        if let Some(user) = accounts.get(session_id) {
            return user.clone();
        }

        // Create new account with the provided session_id and default funds
        let user_id = rand::random::<u64>();
        let new_user = User {
            user_id,
            session_id: session_id.to_string(),
            email: email.to_string(),
            funds: UserFunds::default(),
        };

        accounts.insert(session_id.to_string(), new_user.clone());
        new_user
    }

    // Get user by session ID
    pub fn get_user_by_session_id(&self, session_id: &str) -> Option<User> {
        let accounts = self.accounts.lock().unwrap();
        accounts.get(session_id).cloned()
    }

    // Update user funds
    pub fn update_user_funds(&self, session_id: &str, funds: &UserFunds) -> Result<(), String> {
        let mut accounts = self.accounts.lock().unwrap();

        if let Some(user) = accounts.get_mut(session_id) {
            user.funds = funds.clone();
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    // Get user by user_id
    pub fn get_user_by_id(&self, user_id: u64) -> Option<User> {
        let accounts = self.accounts.lock().unwrap();
        accounts
            .values()
            .find(|user| user.user_id == user_id)
            .cloned()
    }

    // Debit user funds for order placement
    pub fn debit_funds_for_order(
        &self,
        user_id: u64,
        symbol: &str,
        side: matcher::types::OrderSide,
        quantity: u64,
        price_tick: u64,
        tick_multiplier: u64,
    ) -> Result<(), String> {
        let mut accounts = self.accounts.lock().unwrap();

        // Find user by user_id
        let user = accounts
            .values_mut()
            .find(|user| user.user_id == user_id)
            .ok_or("User not found")?;

        // Convert quantity from ticks to actual amount
        let quantity_amount = quantity as f64 / (tick_multiplier as f64);
        let price_amount = price_tick as f64 / (tick_multiplier as f64);
        let cost_amount = quantity_amount * price_amount;

        match side {
            matcher::types::OrderSide::Bid => {
                // Buying crypto with USD - debit USD
                if user.funds.usd < cost_amount {
                    return Err("Insufficient USD funds".to_string());
                }
                user.funds.usd -= cost_amount;
            }
            matcher::types::OrderSide::Ask => {
                // Selling crypto for USD - debit crypto
                match symbol {
                    "BTC-USD" => {
                        if user.funds.btc < quantity_amount {
                            return Err("Insufficient BTC funds".to_string());
                        }
                        user.funds.btc -= quantity_amount;
                    }
                    "SOL-USD" => {
                        if user.funds.sol < quantity_amount {
                            return Err("Insufficient SOL funds".to_string());
                        }
                        user.funds.sol -= quantity_amount;
                    }
                    _ => return Err("Unsupported symbol".to_string()),
                }
            }
        }

        Ok(())
    }

    // Credit funds back to user (for rejected orders or partial fills)
    pub fn credit_funds_back(
        &self,
        user_id: u64,
        symbol: &str,
        side: matcher::types::OrderSide,
        quantity: u64,
        price_tick: u64,
        tick_multiplier: u64,
    ) -> Result<(), String> {
        let mut accounts = self.accounts.lock().unwrap();

        // Find user by user_id
        let user = accounts
            .values_mut()
            .find(|user| user.user_id == user_id)
            .ok_or("User not found")?;

        // Convert quantity from ticks to actual amount
        let quantity_amount = quantity as f64 / (tick_multiplier as f64);
        let price_amount = price_tick as f64 / (tick_multiplier as f64);
        let refund_amount = quantity_amount * price_amount;

        match side {
            matcher::types::OrderSide::Bid => {
                // Refunding USD for rejected buy order
                user.funds.usd += refund_amount;
            }
            matcher::types::OrderSide::Ask => {
                // Refunding crypto for rejected sell order
                match symbol {
                    "BTC-USD" => {
                        user.funds.btc += quantity_amount;
                    }
                    "SOL-USD" => {
                        user.funds.sol += quantity_amount;
                    }
                    _ => return Err("Unsupported symbol".to_string()),
                }
            }
        }

        Ok(())
    }

    // Settle a trade between two users
    pub fn settle_trade(
        &self,
        trade: &matcher::types::Trade,
        symbol: &str,
        taker_user_id: u64,
        maker_user_id: u64,
        tick_multiplier: u64,
    ) -> Result<(), String> {
        let mut accounts = self.accounts.lock().unwrap();

        // Find both users - handle the case where they might be the same user
        let mut taker_user = None;
        let mut maker_user = None;

        for user in accounts.values_mut() {
            if user.user_id == taker_user_id {
                taker_user = Some(user);
            } else if user.user_id == maker_user_id {
                maker_user = Some(user);
            }
        }

        // If taker and maker are the same user, we need to handle this differently
        if taker_user_id == maker_user_id {
            let user = taker_user.ok_or("User not found")?;
            // Self-trade: reverse the debits that were made during order placement
            // The order placement already debited the appropriate funds, so we need to credit them back
            tracing::info!("Self-trade detected for user {}", taker_user_id);

            let quantity = trade.quantity as f64;
            let price_tick = trade.price_tick as f64;

            // Convert from ticks to actual amounts
            let quantity_amount = quantity / (tick_multiplier as f64);
            let price_amount = price_tick / (tick_multiplier as f64);
            let usd_amount = quantity_amount * price_amount;

            match symbol {
                "BTC-USD" => {
                    // Credit back the BTC that was debited for the ask order
                    user.funds.btc += quantity_amount;
                    // Credit back the USD that was debited for the bid order
                    user.funds.usd += usd_amount;
                }
                "SOL-USD" => {
                    // Credit back the SOL that was debited for the ask order
                    user.funds.sol += quantity_amount;
                    // Credit back the USD that was debited for the bid order
                    user.funds.usd += usd_amount;
                }
                _ => return Err("Unsupported symbol".to_string()),
            }

            return Ok(());
        }

        let taker_user = taker_user.ok_or("Taker user not found")?;
        let maker_user = maker_user.ok_or("Maker user not found")?;

        let quantity = trade.quantity as f64;
        let price_tick = trade.price_tick as f64;

        // Convert from ticks to actual amounts
        let quantity_amount = quantity / (tick_multiplier as f64);
        let price_amount = price_tick / (tick_multiplier as f64);
        let usd_amount = quantity_amount * price_amount;

        match symbol {
            "BTC-USD" => {
                // Taker is buying BTC (gets BTC, pays USD)
                // Maker is selling BTC (gets USD, pays BTC)
                taker_user.funds.btc += quantity_amount;
                taker_user.funds.usd -= usd_amount;
                maker_user.funds.btc -= quantity_amount;
                maker_user.funds.usd += usd_amount;
            }
            "SOL-USD" => {
                // Taker is buying SOL (gets SOL, pays USD)
                // Maker is selling SOL (gets USD, pays SOL)
                taker_user.funds.sol += quantity_amount;
                taker_user.funds.usd -= usd_amount;
                maker_user.funds.sol -= quantity_amount;
                maker_user.funds.usd += usd_amount;
            }
            _ => return Err("Unsupported symbol".to_string()),
        }

        Ok(())
    }

    // Handle partial fill refunds
    pub fn handle_partial_fill_refund(
        &self,
        user_id: u64,
        symbol: &str,
        side: matcher::types::OrderSide,
        unfilled_quantity: u64,
        price_tick: u64,
        tick_multiplier: u64,
    ) -> Result<(), String> {
        let mut accounts = self.accounts.lock().unwrap();

        let user = accounts
            .values_mut()
            .find(|user| user.user_id == user_id)
            .ok_or("User not found")?;

        // Convert from ticks to actual amounts
        let quantity_amount = unfilled_quantity as f64 / (tick_multiplier as f64);
        let price_amount = price_tick as f64 / (tick_multiplier as f64);
        let refund_amount = quantity_amount * price_amount;

        match side {
            matcher::types::OrderSide::Bid => {
                // Refund USD for unfilled buy order
                user.funds.usd += refund_amount;
            }
            matcher::types::OrderSide::Ask => {
                // Refund crypto for unfilled sell order
                match symbol {
                    "BTC-USD" => {
                        user.funds.btc += quantity_amount;
                    }
                    "SOL-USD" => {
                        user.funds.sol += quantity_amount;
                    }
                    _ => return Err("Unsupported symbol".to_string()),
                }
            }
        }

        Ok(())
    }
}
