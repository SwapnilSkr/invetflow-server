use crate::api::dto::{AuthResponse, DemoLoginRequest, ExchangeSessionRequest, UserResponse};
use crate::api::state::AppState;
use crate::auth::UserRole;
use crate::domain::models::User;
use crate::error::{AppError, AppResult};
use axum::{Json, extract::State};
use std::sync::Arc;

pub async fn exchange_session(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExchangeSessionRequest>,
) -> AppResult<Json<AuthResponse>> {
    let session = state
        .session_repo
        .find_by_id(&req.session_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("Invalid session".to_string()))?;

    if session.is_expired() {
        return Err(AppError::SessionExpired);
    }

    let user = state
        .user_repo
        .find_by_id(session.user_id)
        .await?
        .ok_or_else(|| AppError::UserNotFound(session.user_id))?;

    let token = state.jwt_config.generate_token(
        user.id,
        user.email.clone(),
        user.name.clone(),
        UserRole::Candidate,
    )?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse::from(user),
    }))
}

pub async fn demo_login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DemoLoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let existing_user = state.user_repo.find_by_email(&req.email).await?;

    let user = if let Some(user) = existing_user {
        user
    } else {
        let new_user = User::new(req.email, req.name);
        state.user_repo.create(&new_user).await?;
        new_user
    };

    let role = req
        .role
        .as_ref()
        .and_then(|r| r.parse::<UserRole>().ok())
        .unwrap_or(UserRole::Candidate);

    let token =
        state
            .jwt_config
            .generate_token(user.id, user.email.clone(), user.name.clone(), role)?;

    Ok(Json(AuthResponse {
        token,
        user: UserResponse::from(user),
    }))
}

pub async fn get_current_user(
    State(state): State<Arc<AppState>>,
    crate::auth::AuthUser(user, _role): crate::auth::AuthUser,
) -> AppResult<Json<UserResponse>> {
    let user = state
        .user_repo
        .find_by_id(user.id)
        .await?
        .ok_or_else(|| AppError::UserNotFound(user.id))?;

    Ok(Json(UserResponse::from(user)))
}

pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    crate::auth::AuthUser(user, role): crate::auth::AuthUser,
) -> AppResult<Json<AuthResponse>> {
    let user = state
        .user_repo
        .find_by_id(user.id)
        .await?
        .ok_or_else(|| AppError::UserNotFound(user.id))?;

    Ok(Json(AuthResponse {
        token: state.jwt_config.generate_token(
            user.id,
            user.email.clone(),
            user.name.clone(),
            role,
        )?,
        user: UserResponse::from(user),
    }))
}
