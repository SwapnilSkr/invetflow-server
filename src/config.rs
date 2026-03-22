use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub mongo_uri: String,
    pub mongo_database: String,
    pub jwt_secret: String,
    pub jwt_expiry_seconds: u64,
    pub better_auth_secret: String,
    pub livekit_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub server_host: String,
    pub server_port: u16,
    pub cors_origins: Vec<String>,
}

impl Config {
    pub fn from_env() -> crate::error::AppResult<Self> {
        dotenvy::dotenv().ok();

        fn required_env(key: &str) -> crate::error::AppResult<String> {
            env::var(key).map_err(|_| crate::error::AppError::Config(format!("{key} must be set")))
        }

        let cors_origins = env::var("CORS_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:3000".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        Ok(Config {
            mongo_uri: required_env("MONGO_URI")?,
            mongo_database: env::var("MONGO_DATABASE").unwrap_or_else(|_| "invetflow".to_string()),
            jwt_secret: required_env("JWT_SECRET")?,
            jwt_expiry_seconds: env::var("JWT_EXPIRY_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            better_auth_secret: required_env("BETTER_AUTH_SECRET")?,
            livekit_url: required_env("LIVEKIT_URL")?,
            livekit_api_key: required_env("LIVEKIT_API_KEY")?,
            livekit_api_secret: required_env("LIVEKIT_API_SECRET")?,
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: env::var("SERVER_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3001),
            cors_origins,
        })
    }

    pub fn validate(&self) -> crate::error::AppResult<()> {
        if self.jwt_secret.len() < 32 {
            return Err(crate::error::AppError::Config(
                "JWT_SECRET must be at least 32 characters".to_string(),
            ));
        }
        if self.better_auth_secret.len() < 32 {
            return Err(crate::error::AppError::Config(
                "BETTER_AUTH_SECRET must be at least 32 characters".to_string(),
            ));
        }
        Ok(())
    }

    pub fn server_address(&self) -> String {
        format!("{}:{}", self.server_host, self.server_port)
    }
}
