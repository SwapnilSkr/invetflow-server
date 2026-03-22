use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Session {
    pub id: String,
    #[ts(type = "string")]
    pub user_id: Uuid,
    #[ts(type = "string")]
    pub expires_at: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl Session {
    pub fn is_expired(&self) -> bool {
        self.expires_at < Utc::now()
    }

    pub fn from_bson_document(doc: bson::Document) -> crate::error::AppResult<Self> {
        let id = doc
            .get_str("id")
            .or_else(|_| doc.get_str("_id"))?
            .to_string();

        let user_id_str = doc.get_str("userId").or_else(|_| doc.get_str("user_id"))?;
        let user_id = Uuid::parse_str(user_id_str)
            .map_err(|e| crate::error::AppError::Validation(format!("Invalid UUID: {}", e)))?;

        let expires_at = doc
            .get_datetime("expiresAt")
            .or_else(|_| doc.get_datetime("expires_at"))
            .map(|dt| chrono::DateTime::from(*dt))
            .unwrap_or_else(|_| Utc::now());

        let ip_address = doc.get_str("ipAddress").ok().map(|s| s.to_string());
        let user_agent = doc.get_str("userAgent").ok().map(|s| s.to_string());

        Ok(Self {
            id,
            user_id,
            expires_at,
            ip_address,
            user_agent,
        })
    }
}
