use axum::{Json, extract::State, http::StatusCode};
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{AppState, middleware::AuthUser, models::AuthenticatedUser};

// Login request
#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

// Login response
#[derive(Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub session_id: Option<String>,
    pub user: Option<AuthenticatedUser>,
}

// Login endpoint
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> (StatusCode, Json<LoginResponse>) {
    // Validate input
    if payload.email.is_empty() || payload.password.is_empty() {
        let response = LoginResponse {
            success: false,
            message: "Email and password are required".to_string(),
            session_id: None,
            user: None,
        };
        return (StatusCode::BAD_REQUEST, Json(response));
    }

    // Generate session_id hash from email + password
    let mut hasher = Sha256::new();
    hasher.update(payload.email.as_bytes());
    hasher.update(payload.password.as_bytes());
    let session_id = hex::encode(hasher.finalize());

    // Get or create user account with the generated session_id
    let user = state
        .storage
        .get_or_create_account_with_session(&payload.email, &session_id);
    let authenticated_user = AuthenticatedUser::from(user.clone());

    let response = LoginResponse {
        success: true,
        message: "Login successful".to_string(),
        session_id: Some(user.session_id),
        user: Some(authenticated_user),
    };
    (StatusCode::OK, Json(response))
}

// User profile response
#[derive(Serialize)]
pub struct UserProfileResponse {
    pub success: bool,
    pub user: Option<AuthenticatedUser>,
    pub message: String,
}

// Get user profile endpoint (protected route)
pub async fn get_profile(AuthUser(user): AuthUser) -> (StatusCode, Json<UserProfileResponse>) {
    let response = UserProfileResponse {
        success: true,
        user: Some(user),
        message: "Profile retrieved successfully".to_string(),
    };
    (StatusCode::OK, Json(response))
}
