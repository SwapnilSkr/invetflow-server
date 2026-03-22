use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TranscriptEntry {
    #[ts(type = "string")]
    pub id: Uuid,
    #[ts(type = "string")]
    pub session_id: Uuid,
    pub speaker: Speaker,
    pub content: String,
    #[ts(type = "string")]
    pub timestamp: DateTime<Utc>,
    pub question_id: Option<String>,
    pub confidence: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub enum Speaker {
    Candidate,
    AI,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateTranscriptRequest {
    #[ts(type = "string")]
    pub session_id: Uuid,
    pub speaker: Speaker,
    pub content: String,
    pub question_id: Option<String>,
    pub confidence: Option<f32>,
}

impl TranscriptEntry {
    pub fn new(request: CreateTranscriptRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            session_id: request.session_id,
            speaker: request.speaker,
            content: request.content,
            timestamp: Utc::now(),
            question_id: request.question_id,
            confidence: request.confidence,
        }
    }

    pub fn from_bson_document(doc: bson::Document) -> crate::error::AppResult<Self> {
        let id_str = doc.get_str("id")?;
        let id = Uuid::parse_str(id_str)
            .map_err(|e| crate::error::AppError::Validation(format!("Invalid UUID: {}", e)))?;

        let session_id_str = doc.get_str("sessionId")?;
        let session_id = Uuid::parse_str(session_id_str)
            .map_err(|e| crate::error::AppError::Validation(format!("Invalid UUID: {}", e)))?;

        let speaker_str = doc.get_str("speaker").unwrap_or("Candidate");
        let speaker = match speaker_str {
            "Candidate" => Speaker::Candidate,
            "AI" => Speaker::AI,
            "System" => Speaker::System,
            _ => Speaker::Candidate,
        };

        let content = doc.get_str("content")?.to_string();

        let timestamp = doc
            .get_datetime("timestamp")
            .map(|dt| chrono::DateTime::from(*dt))
            .unwrap_or_else(|_| Utc::now());

        let question_id = doc.get_str("questionId").ok().map(|s| s.to_string());
        let confidence = doc.get_f64("confidence").map(|f| f as f32).ok();

        Ok(Self {
            id,
            session_id,
            speaker,
            content,
            timestamp,
            question_id,
            confidence,
        })
    }

    pub fn to_bson(&self) -> crate::error::AppResult<bson::Document> {
        let speaker_str = match &self.speaker {
            Speaker::Candidate => "Candidate",
            Speaker::AI => "AI",
            Speaker::System => "System",
        };

        Ok(bson::doc! {
            "id": self.id.to_string(),
            "sessionId": self.session_id.to_string(),
            "speaker": speaker_str,
            "content": &self.content,
            "timestamp": bson::DateTime::from_millis(self.timestamp.timestamp_millis()),
            "questionId": &self.question_id,
            "confidence": self.confidence.map(|f| f as f64),
        })
    }
}
