use crate::config::Config;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveKitRoom {
    pub name: String,
    pub sid: String,
    pub empty_timeout: u32,
    pub max_participants: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveKitTokenOptions {
    pub identity: String,
    pub name: String,
    pub room: String,
    pub can_publish: bool,
    pub can_subscribe: bool,
}

#[derive(Debug, Clone)]
pub struct LiveKitClient {
    url: String,
    api_key: String,
    api_secret: String,
    http_client: reqwest::Client,
}

impl LiveKitClient {
    pub fn new(config: &Config) -> Self {
        Self {
            url: config.livekit_url.clone(),
            api_key: config.livekit_api_key.clone(),
            api_secret: config.livekit_api_secret.clone(),
            http_client: reqwest::Client::new(),
        }
    }

    pub async fn get_or_create_room(&self, name: &str) -> AppResult<LiveKitRoom> {
        let create_room_url = format!(
            "{}/twirp/livekit.Room/CreateRoom",
            self.url
                .replace("wss://", "https://")
                .replace("ws://", "http://")
        );

        let body = serde_json::json!({
            "name": name,
            "empty_timeout": 600,
            "max_participants": 2,
        });

        let response = self
            .http_client
            .post(&create_room_url)
            .header(
                "Authorization",
                format!("Bearer {}", self.generate_api_token()?),
            )
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AppError::LiveKit(format!("Failed to create room: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::LiveKit(format!(
                "Room creation failed: {} - {}",
                status, body
            )));
        }

        let room: LiveKitRoom = response
            .json()
            .await
            .map_err(|e| AppError::LiveKit(format!("Failed to parse room response: {}", e)))?;

        Ok(room)
    }

    pub fn generate_candidate_token(&self, room: &str, name: &str) -> AppResult<String> {
        self.generate_token(&LiveKitTokenOptions {
            identity: name.to_string(),
            name: name.to_string(),
            room: room.to_string(),
            can_publish: true,
            can_subscribe: true,
        })
    }

    fn generate_api_token(&self) -> AppResult<String> {
        use chrono::{Duration, Utc};
        use jsonwebtoken::{EncodingKey, Header};

        #[derive(Debug, Serialize, Deserialize)]
        struct ApiTokenClaims {
            iss: String,
            nbf: u64,
            exp: u64,
        }

        let now = Utc::now();
        let exp = now + Duration::hours(1);

        let claims = ApiTokenClaims {
            iss: self.api_key.clone(),
            nbf: now.timestamp() as u64,
            exp: exp.timestamp() as u64,
        };

        let header = Header::new(jsonwebtoken::Algorithm::HS256);
        let encoding_key = EncodingKey::from_secret(self.api_secret.as_bytes());

        jsonwebtoken::encode(&header, &claims, &encoding_key)
            .map_err(|e| AppError::LiveKit(format!("Failed to generate API token: {}", e)))
    }

    fn generate_token(&self, options: &LiveKitTokenOptions) -> AppResult<String> {
        use chrono::{Duration, Utc};
        use jsonwebtoken::{EncodingKey, Header};

        #[derive(Debug, Serialize, Deserialize)]
        struct AccessTokenClaims {
            sub: String,
            name: String,
            iss: String,
            nbf: u64,
            exp: u64,
            video: VideoGrant,
        }

        #[derive(Debug, Serialize, Deserialize)]
        struct VideoGrant {
            room: String,
            room_create: bool,
            room_join: bool,
            can_publish: bool,
            can_subscribe: bool,
            can_publish_data: bool,
        }

        let now = Utc::now();
        let exp = now + Duration::hours(24);

        let claims = AccessTokenClaims {
            sub: options.identity.clone(),
            name: options.name.clone(),
            iss: self.api_key.clone(),
            nbf: now.timestamp() as u64,
            exp: exp.timestamp() as u64,
            video: VideoGrant {
                room: options.room.clone(),
                room_create: false,
                room_join: true,
                can_publish: options.can_publish,
                can_subscribe: options.can_subscribe,
                can_publish_data: true,
            },
        };

        let header = Header::new(jsonwebtoken::Algorithm::HS256);
        let encoding_key = EncodingKey::from_secret(self.api_secret.as_bytes());

        jsonwebtoken::encode(&header, &claims, &encoding_key)
            .map_err(|e| AppError::LiveKit(format!("Failed to generate token: {}", e)))
    }
}
