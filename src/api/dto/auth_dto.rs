use crate::auth::TokenResponse;
use crate::domain::User;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Deserialize)]
pub struct ExchangeSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct AuthResponse {
    #[serde(flatten)]
    pub token: TokenResponse,
    pub user: UserResponse,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct UserResponse {
    #[ts(type = "string")]
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub email_verified: bool,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id.to_string(),
            email: user.email,
            name: user.name,
            email_verified: user.email_verified,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DemoLoginRequest {
    pub email: String,
    pub name: Option<String>,
    pub role: Option<String>,
}
