use axum::{
    extract::FromRequestParts,
    http::{StatusCode, header::AUTHORIZATION, request::Parts},
    response::{IntoResponse, Response},
};

use crate::{AppState, models::AuthenticatedUser};

// Axum extractor for authenticated users
#[derive(Debug, Clone)]
pub struct AuthUser(pub AuthenticatedUser);

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract the Authorization header
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|header| header.to_str().ok())
            .ok_or_else(|| {
                (StatusCode::UNAUTHORIZED, "Missing Authorization header").into_response()
            })?;

        // Check if it's a Bearer token
        if !auth_header.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid Authorization header format",
            )
                .into_response());
        }

        // Extract the token (user ID)
        let token = &auth_header[7..]; // Remove "Bearer " prefix

        // Get user from storage
        match state.storage.get_user_by_session_id(token) {
            Some(user) => Ok(AuthUser(AuthenticatedUser::from(user))),
            None => Err((StatusCode::UNAUTHORIZED, "Invalid token").into_response()),
        }
    }
}
