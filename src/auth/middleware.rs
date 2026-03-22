use crate::auth::{JwtConfig, UserRole};
use crate::domain::models::User;
use crate::error::AppError;
use axum::extract::FromRequestParts;
use std::sync::Arc;

pub struct AuthUser(pub User, pub UserRole);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let jwt_config = parts
            .extensions
            .get::<Arc<JwtConfig>>()
            .ok_or_else(|| AppError::Internal("JWT config not found in extensions".to_string()))?;

        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::Unauthorized("Missing Authorization header".to_string()))?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            AppError::Unauthorized("Invalid Authorization header format".to_string())
        })?;

        let claims = jwt_config.validate_token(token)?;

        let user = User {
            id: claims.sub,
            email: claims.email,
            name: claims.name,
            email_verified: true,
            image: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        Ok(AuthUser(user, claims.role))
    }
}
