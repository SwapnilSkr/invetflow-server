use crate::domain::models::{InterviewScores, Speaker, TranscriptEntry};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct SessionResponse {
    #[ts(type = "string")]
    pub id: Uuid,
    #[ts(type = "string")]
    pub interview_id: Uuid,
    #[ts(type = "string")]
    pub candidate_id: Uuid,
    pub livekit_room: String,
    pub status: String,
    pub current_question_index: i32,
    #[ts(type = "string")]
    pub started_at: DateTime<Utc>,
    #[ts(type = "string")]
    pub ended_at: Option<DateTime<Utc>>,
    pub duration_seconds: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct TranscriptResponse {
    pub entries: Vec<TranscriptEntryResponse>,
    pub total: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct TranscriptEntryResponse {
    #[ts(type = "string")]
    pub id: Uuid,
    pub speaker: String,
    pub content: String,
    #[ts(type = "string")]
    pub timestamp: DateTime<Utc>,
    pub question_id: Option<String>,
    pub confidence: Option<f32>,
}

impl From<TranscriptEntry> for TranscriptEntryResponse {
    fn from(entry: TranscriptEntry) -> Self {
        Self {
            id: entry.id,
            speaker: match entry.speaker {
                Speaker::Candidate => "Candidate".to_string(),
                Speaker::AI => "AI".to_string(),
                Speaker::System => "System".to_string(),
            },
            content: entry.content,
            timestamp: entry.timestamp,
            question_id: entry.question_id,
            confidence: entry.confidence,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ScoresResponse {
    pub technical_accuracy: f32,
    pub communication: f32,
    pub problem_solving: f32,
    pub confidence: f32,
    pub overall: f32,
    pub reasoning: String,
    pub strengths: Vec<String>,
    pub areas_for_improvement: Vec<String>,
    #[ts(type = "string")]
    pub evaluated_at: DateTime<Utc>,
}

impl From<InterviewScores> for ScoresResponse {
    fn from(scores: InterviewScores) -> Self {
        Self {
            technical_accuracy: scores.technical_accuracy,
            communication: scores.communication,
            problem_solving: scores.problem_solving,
            confidence: scores.confidence,
            overall: scores.overall,
            reasoning: scores.reasoning,
            strengths: scores.strengths,
            areas_for_improvement: scores.areas_for_improvement,
            evaluated_at: scores.evaluated_at,
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AppendTranscriptDto {
    pub speaker: String,
    pub content: String,
    pub question_id: Option<String>,
    pub confidence: Option<f32>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct SaveScoresDto {
    pub technical_accuracy: f32,
    pub communication: f32,
    pub problem_solving: f32,
    pub confidence: f32,
    pub reasoning: String,
    pub strengths: Vec<String>,
    pub areas_for_improvement: Vec<String>,
}
