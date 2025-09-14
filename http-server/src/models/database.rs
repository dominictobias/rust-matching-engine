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
        let new_user = User {
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
        let new_user = User {
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
}
