use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub session_id: String,
    pub email: String,
    pub funds: UserFunds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFunds {
    pub btc: f64,
    pub sol: f64,
    pub usd: f64,
}

impl Default for UserFunds {
    fn default() -> Self {
        Self {
            btc: 100.0,     // Give users 100 BTC to start
            sol: 10_000.0,  // Give users 10000 SOL to start
            usd: 100_000.0, // Give users $100,000 USD to start
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub session_id: String,
    pub email: String,
    pub funds: UserFunds,
}

impl From<User> for AuthenticatedUser {
    fn from(user: User) -> Self {
        Self {
            session_id: user.session_id,
            email: user.email,
            funds: user.funds,
        }
    }
}
