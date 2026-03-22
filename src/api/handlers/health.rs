use crate::api::state::AppState;
use crate::error::AppResult;
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    status: String,
    version: String,
}

pub async fn health_check() -> AppResult<Json<HealthResponse>> {
    Ok(Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

pub async fn readiness_check(State(_state): State<Arc<AppState>>) -> AppResult<StatusCode> {
    Ok(StatusCode::OK)
}
