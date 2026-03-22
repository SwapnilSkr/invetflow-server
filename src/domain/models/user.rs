use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct User {
    #[ts(type = "string")]
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub email_verified: bool,
    pub image: Option<String>,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "string")]
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(email: String, name: Option<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            email,
            name,
            email_verified: false,
            image: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn from_bson_document(doc: bson::Document) -> crate::error::AppResult<Self> {
        let id_str = doc.get_str("id").or_else(|_| doc.get_str("_id"))?;
        let id = Uuid::parse_str(id_str)
            .map_err(|e| crate::error::AppError::Validation(format!("Invalid UUID: {}", e)))?;

        let email = doc.get_str("email")?.to_string();
        let name = doc.get_str("name").ok().map(|s| s.to_string());
        let email_verified = doc.get_bool("emailVerified").unwrap_or(false);
        let image = doc.get_str("image").ok().map(|s| s.to_string());

        let created_at = doc
            .get_datetime("createdAt")
            .or_else(|_| doc.get_datetime("created_at"))
            .map(|dt| chrono::DateTime::from(*dt))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = doc
            .get_datetime("updatedAt")
            .or_else(|_| doc.get_datetime("updated_at"))
            .map(|dt| chrono::DateTime::from(*dt))
            .unwrap_or_else(|_| Utc::now());

        Ok(Self {
            id,
            email,
            name,
            email_verified,
            image,
            created_at,
            updated_at,
        })
    }

    pub fn to_bson(&self) -> crate::error::AppResult<bson::Document> {
        Ok(bson::doc! {
            "id": self.id.to_string(),
            "email": &self.email,
            "name": &self.name,
            "emailVerified": self.email_verified,
            "image": &self.image,
            "createdAt": bson::DateTime::from_millis(self.created_at.timestamp_millis()),
            "updatedAt": bson::DateTime::from_millis(self.updated_at.timestamp_millis()),
        })
    }
}
