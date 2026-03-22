use super::TranscriptEntry;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InterviewSession {
    #[ts(type = "string")]
    pub id: Uuid,
    #[ts(type = "string")]
    pub interview_id: Uuid,
    #[ts(type = "string")]
    pub candidate_id: Uuid,
    pub livekit_room: String,
    pub status: SessionStatus,
    pub transcript: Vec<TranscriptEntry>,
    pub scores: Option<InterviewScores>,
    pub current_question_index: i32,
    #[ts(type = "string")]
    pub started_at: DateTime<Utc>,
    #[ts(type = "string")]
    pub ended_at: Option<DateTime<Utc>>,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "string")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub enum SessionStatus {
    Waiting,
    Active,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InterviewScores {
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

impl InterviewSession {
    pub fn new(interview_id: Uuid, candidate_id: Uuid, livekit_room: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            interview_id,
            candidate_id,
            livekit_room,
            status: SessionStatus::Waiting,
            transcript: Vec::new(),
            scores: None,
            current_question_index: 0,
            started_at: now,
            ended_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn from_bson_document(doc: bson::Document) -> crate::error::AppResult<Self> {
        let id_str = doc.get_str("id")?;
        let id = Uuid::parse_str(id_str)
            .map_err(|e| crate::error::AppError::Validation(format!("Invalid UUID: {}", e)))?;

        let interview_id_str = doc.get_str("interviewId")?;
        let interview_id = Uuid::parse_str(interview_id_str)
            .map_err(|e| crate::error::AppError::Validation(format!("Invalid UUID: {}", e)))?;

        let candidate_id_str = doc.get_str("candidateId")?;
        let candidate_id = Uuid::parse_str(candidate_id_str)
            .map_err(|e| crate::error::AppError::Validation(format!("Invalid UUID: {}", e)))?;

        let livekit_room = doc.get_str("livekitRoom")?.to_string();

        let status_str = doc.get_str("status").unwrap_or("Waiting");
        let status = match status_str {
            "Waiting" => SessionStatus::Waiting,
            "Active" => SessionStatus::Active,
            "Paused" => SessionStatus::Paused,
            "Completed" => SessionStatus::Completed,
            "Cancelled" => SessionStatus::Cancelled,
            _ => SessionStatus::Waiting,
        };

        let transcript: Vec<TranscriptEntry> = doc
            .get_array("transcript")
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        if let bson::Bson::Document(d) = v {
                            TranscriptEntry::from_bson_document(d.clone()).ok()
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let scores = doc.get_document("scores").ok().and_then(|d| {
            Some(InterviewScores {
                technical_accuracy: d.get_f64("technicalAccuracy").ok()? as f32,
                communication: d.get_f64("communication").ok()? as f32,
                problem_solving: d.get_f64("problemSolving").ok()? as f32,
                confidence: d.get_f64("confidence").ok()? as f32,
                overall: d.get_f64("overall").ok()? as f32,
                reasoning: d.get_str("reasoning").ok()?.to_string(),
                strengths: d
                    .get_array("strengths")
                    .ok()?
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
                areas_for_improvement: d
                    .get_array("areasForImprovement")
                    .ok()?
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect(),
                evaluated_at: d.get_datetime("evaluatedAt").ok()?.to_chrono(),
            })
        });

        let current_question_index = doc.get_i32("currentQuestionIndex").unwrap_or(0);

        let started_at = doc
            .get_datetime("startedAt")
            .map(|dt| chrono::DateTime::from(*dt))
            .unwrap_or_else(|_| Utc::now());

        let ended_at = doc
            .get_datetime("endedAt")
            .ok()
            .map(|dt| chrono::DateTime::from(*dt));

        let created_at = doc
            .get_datetime("createdAt")
            .map(|dt| chrono::DateTime::from(*dt))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = doc
            .get_datetime("updatedAt")
            .map(|dt| chrono::DateTime::from(*dt))
            .unwrap_or_else(|_| Utc::now());

        Ok(Self {
            id,
            interview_id,
            candidate_id,
            livekit_room,
            status,
            transcript,
            scores,
            current_question_index,
            started_at,
            ended_at,
            created_at,
            updated_at,
        })
    }

    pub fn to_bson(&self) -> crate::error::AppResult<bson::Document> {
        let status_str = match &self.status {
            SessionStatus::Waiting => "Waiting",
            SessionStatus::Active => "Active",
            SessionStatus::Paused => "Paused",
            SessionStatus::Completed => "Completed",
            SessionStatus::Cancelled => "Cancelled",
        };

        let transcript: Vec<bson::Document> = self
            .transcript
            .iter()
            .map(|t| t.to_bson())
            .collect::<crate::error::AppResult<Vec<_>>>()?;

        let scores_doc = self.scores.as_ref().map(|s| {
            bson::doc! {
                "technicalAccuracy": s.technical_accuracy as f64,
                "communication": s.communication as f64,
                "problemSolving": s.problem_solving as f64,
                "confidence": s.confidence as f64,
                "overall": s.overall as f64,
                "reasoning": &s.reasoning,
                "strengths": &s.strengths,
                "areasForImprovement": &s.areas_for_improvement,
                "evaluatedAt": bson::DateTime::from_millis(s.evaluated_at.timestamp_millis()),
            }
        });

        Ok(bson::doc! {
            "id": self.id.to_string(),
            "interviewId": self.interview_id.to_string(),
            "candidateId": self.candidate_id.to_string(),
            "livekitRoom": &self.livekit_room,
            "status": status_str,
            "transcript": transcript,
            "scores": scores_doc,
            "currentQuestionIndex": self.current_question_index,
            "startedAt": bson::DateTime::from_millis(self.started_at.timestamp_millis()),
            "endedAt": self.ended_at.map(|dt| bson::DateTime::from_millis(dt.timestamp_millis())),
            "createdAt": bson::DateTime::from_millis(self.created_at.timestamp_millis()),
            "updatedAt": bson::DateTime::from_millis(self.updated_at.timestamp_millis()),
        })
    }

    pub fn duration_seconds(&self) -> i64 {
        let end = self.ended_at.unwrap_or_else(Utc::now);
        (end - self.started_at).num_seconds()
    }

    pub fn is_owned_by_candidate(&self, candidate_id: Uuid) -> bool {
        self.candidate_id == candidate_id
    }

    pub fn is_live(&self) -> bool {
        matches!(self.status, SessionStatus::Waiting | SessionStatus::Active)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            SessionStatus::Completed | SessionStatus::Cancelled
        )
    }
}
