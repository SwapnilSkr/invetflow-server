use crate::config::Config;
use crate::error::{AppError, AppResult};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    secret: String,
    expiry_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    Recruiter,
    Candidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiClaims {
    pub sub: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub role: UserRole,
    pub iat: u64,
    pub exp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
}

impl JwtConfig {
    pub fn from_config(config: &Config) -> Self {
        Self {
            secret: config.jwt_secret.clone(),
            expiry_seconds: config.jwt_expiry_seconds,
        }
    }

    pub fn generate_token(
        &self,
        user_id: Uuid,
        email: String,
        name: Option<String>,
        role: UserRole,
    ) -> AppResult<TokenResponse> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.expiry_seconds as i64);

        let claims = ApiClaims {
            sub: user_id,
            email,
            name,
            role,
            iat: now.timestamp() as u64,
            exp: exp.timestamp() as u64,
        };

        let header = Header::new(jsonwebtoken::Algorithm::HS256);
        let encoding_key = EncodingKey::from_secret(self.secret.as_bytes());

        let token = jsonwebtoken::encode(&header, &claims, &encoding_key)
            .map_err(|e| AppError::InvalidToken(format!("Failed to encode token: {}", e)))?;

        Ok(TokenResponse {
            access_token: token,
            token_type: "Bearer".to_string(),
            expires_in: self.expiry_seconds,
        })
    }

    pub fn validate_token(&self, token: &str) -> AppResult<ApiClaims> {
        let decoding_key = DecodingKey::from_secret(self.secret.as_bytes());
        let validation = Validation::new(jsonwebtoken::Algorithm::HS256);

        let claims =
            jsonwebtoken::decode::<ApiClaims>(token, &decoding_key, &validation).map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
                    _ => AppError::InvalidToken(format!("Token validation failed: {}", e)),
                }
            })?;

        Ok(claims.claims)
    }
}

impl std::fmt::Display for UserRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserRole::Admin => write!(f, "Admin"),
            UserRole::Recruiter => write!(f, "Recruiter"),
            UserRole::Candidate => write!(f, "Candidate"),
        }
    }
}

impl std::str::FromStr for UserRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(UserRole::Admin),
            "recruiter" => Ok(UserRole::Recruiter),
            "candidate" => Ok(UserRole::Candidate),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }
}
