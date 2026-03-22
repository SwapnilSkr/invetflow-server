use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Interview {
    #[ts(type = "string")]
    pub id: Uuid,
    pub title: String,
    pub job_title: String,
    pub job_description: Option<String>,
    pub questions: Vec<InterviewQuestion>,
    pub status: InterviewStatus,
    #[ts(type = "string")]
    pub recruiter_id: Uuid,
    #[ts(type = "string")]
    pub candidate_id: Option<Uuid>,
    pub candidate_name: Option<String>,
    pub candidate_email: Option<String>,
    pub livekit_room: Option<String>,
    pub duration_minutes: i32,
    #[ts(type = "string")]
    pub created_at: DateTime<Utc>,
    #[ts(type = "string")]
    pub updated_at: DateTime<Utc>,
    pub invite_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub enum InterviewStatus {
    Draft,
    Scheduled,
    Active,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct InterviewQuestion {
    pub id: String,
    pub question: String,
    pub category: QuestionCategory,
    pub time_limit_seconds: Option<i32>,
    pub follow_up_prompts: Vec<String>,
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
pub enum QuestionCategory {
    Technical,
    Behavioral,
    Situational,
    Coding,
    SystemDesign,
    SoftSkills,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateInterviewRequest {
    pub title: String,
    pub job_title: String,
    pub job_description: Option<String>,
    pub questions: Vec<CreateQuestionRequest>,
    pub duration_minutes: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateQuestionRequest {
    pub question: String,
    pub category: QuestionCategory,
    pub time_limit_seconds: Option<i32>,
    pub follow_up_prompts: Vec<String>,
}

impl Interview {
    pub fn new(recruiter_id: Uuid, request: CreateInterviewRequest) -> Self {
        let now = Utc::now();
        let questions: Vec<InterviewQuestion> = request
            .questions
            .into_iter()
            .enumerate()
            .map(|(idx, q)| InterviewQuestion {
                id: Uuid::new_v4().to_string(),
                question: q.question,
                category: q.category,
                time_limit_seconds: q.time_limit_seconds,
                follow_up_prompts: q.follow_up_prompts,
                order: idx as i32,
            })
            .collect();

        Self {
            id: Uuid::new_v4(),
            title: request.title,
            job_title: request.job_title,
            job_description: request.job_description,
            questions,
            status: InterviewStatus::Draft,
            recruiter_id,
            candidate_id: None,
            candidate_name: None,
            candidate_email: None,
            livekit_room: None,
            duration_minutes: request.duration_minutes,
            created_at: now,
            updated_at: now,
            invite_token: Some(Uuid::new_v4().to_string()),
        }
    }

    pub fn from_bson_document(doc: bson::Document) -> crate::error::AppResult<Self> {
        let id_str = doc.get_str("id").or_else(|_| doc.get_str("_id"))?;
        let id = Uuid::parse_str(id_str)
            .map_err(|e| crate::error::AppError::Validation(format!("Invalid UUID: {}", e)))?;

        let title = doc.get_str("title")?.to_string();
        let job_title = doc.get_str("jobTitle")?.to_string();
        let job_description = doc.get_str("jobDescription").ok().map(|s| s.to_string());

        let questions: Vec<InterviewQuestion> = doc
            .get_array("questions")
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        if let bson::Bson::Document(d) = v {
                            InterviewQuestion::from_bson_document(d.clone()).ok()
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        let status_str = doc.get_str("status").unwrap_or("Draft");
        let status = match status_str {
            "Draft" => InterviewStatus::Draft,
            "Scheduled" => InterviewStatus::Scheduled,
            "Active" => InterviewStatus::Active,
            "Completed" => InterviewStatus::Completed,
            "Cancelled" => InterviewStatus::Cancelled,
            _ => InterviewStatus::Draft,
        };

        let recruiter_id_str = doc.get_str("recruiterId")?;
        let recruiter_id = Uuid::parse_str(recruiter_id_str)
            .map_err(|e| crate::error::AppError::Validation(format!("Invalid UUID: {}", e)))?;

        let candidate_id = doc
            .get_str("candidateId")
            .ok()
            .and_then(|s| Uuid::parse_str(s).ok());

        let candidate_name = doc.get_str("candidateName").ok().map(|s| s.to_string());
        let candidate_email = doc.get_str("candidateEmail").ok().map(|s| s.to_string());
        let livekit_room = doc.get_str("livekitRoom").ok().map(|s| s.to_string());
        let duration_minutes = doc.get_i32("durationMinutes").unwrap_or(30);

        let created_at = doc
            .get_datetime("createdAt")
            .map(|dt| chrono::DateTime::from(*dt))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = doc
            .get_datetime("updatedAt")
            .map(|dt| chrono::DateTime::from(*dt))
            .unwrap_or_else(|_| Utc::now());

        let invite_token = doc.get_str("inviteToken").ok().map(|s| s.to_string());

        Ok(Self {
            id,
            title,
            job_title,
            job_description,
            questions,
            status,
            recruiter_id,
            candidate_id,
            candidate_name,
            candidate_email,
            livekit_room,
            duration_minutes,
            created_at,
            updated_at,
            invite_token,
        })
    }

    pub fn to_bson(&self) -> crate::error::AppResult<bson::Document> {
        let questions: Vec<bson::Document> = self
            .questions
            .iter()
            .map(|q| q.to_bson())
            .collect::<crate::error::AppResult<Vec<_>>>()?;

        let status_str = match &self.status {
            InterviewStatus::Draft => "Draft",
            InterviewStatus::Scheduled => "Scheduled",
            InterviewStatus::Active => "Active",
            InterviewStatus::Completed => "Completed",
            InterviewStatus::Cancelled => "Cancelled",
        };

        Ok(bson::doc! {
            "id": self.id.to_string(),
            "title": &self.title,
            "jobTitle": &self.job_title,
            "jobDescription": &self.job_description,
            "questions": questions,
            "status": status_str,
            "recruiterId": self.recruiter_id.to_string(),
            "candidateId": self.candidate_id.map(|id| id.to_string()),
            "candidateName": &self.candidate_name,
            "candidateEmail": &self.candidate_email,
            "livekitRoom": &self.livekit_room,
            "durationMinutes": self.duration_minutes,
            "createdAt": bson::DateTime::from_millis(self.created_at.timestamp_millis()),
            "updatedAt": bson::DateTime::from_millis(self.updated_at.timestamp_millis()),
            "inviteToken": &self.invite_token,
        })
    }

    pub fn is_owned_by(&self, recruiter_id: Uuid) -> bool {
        self.recruiter_id == recruiter_id
    }

    pub fn is_accessible_by_candidate(&self, candidate_id: Uuid, candidate_email: &str) -> bool {
        self.candidate_id == Some(candidate_id)
            || self
                .candidate_email
                .as_deref()
                .is_some_and(|email| email.eq_ignore_ascii_case(candidate_email))
    }

    pub fn can_transition_to_scheduled(&self) -> bool {
        matches!(
            self.status,
            InterviewStatus::Draft | InterviewStatus::Scheduled
        )
    }

    pub fn can_be_joined(&self) -> bool {
        matches!(
            self.status,
            InterviewStatus::Scheduled | InterviewStatus::Active
        )
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            InterviewStatus::Completed | InterviewStatus::Cancelled
        )
    }
}

impl InterviewQuestion {
    pub fn from_bson_document(doc: bson::Document) -> crate::error::AppResult<Self> {
        let id = doc.get_str("id")?.to_string();
        let question = doc.get_str("question")?.to_string();

        let category_str = doc.get_str("category").unwrap_or("Behavioral");
        let category = match category_str {
            "Technical" => QuestionCategory::Technical,
            "Behavioral" => QuestionCategory::Behavioral,
            "Situational" => QuestionCategory::Situational,
            "Coding" => QuestionCategory::Coding,
            "SystemDesign" => QuestionCategory::SystemDesign,
            "SoftSkills" => QuestionCategory::SoftSkills,
            _ => QuestionCategory::Behavioral,
        };

        let time_limit_seconds = doc.get_i32("timeLimitSeconds").ok();
        let order = doc.get_i32("order").unwrap_or(0);

        let follow_up_prompts: Vec<String> = doc
            .get_array("followUpPrompts")
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            id,
            question,
            category,
            time_limit_seconds,
            follow_up_prompts,
            order,
        })
    }

    pub fn to_bson(&self) -> crate::error::AppResult<bson::Document> {
        let category_str = match &self.category {
            QuestionCategory::Technical => "Technical",
            QuestionCategory::Behavioral => "Behavioral",
            QuestionCategory::Situational => "Situational",
            QuestionCategory::Coding => "Coding",
            QuestionCategory::SystemDesign => "SystemDesign",
            QuestionCategory::SoftSkills => "SoftSkills",
        };

        Ok(bson::doc! {
            "id": &self.id,
            "question": &self.question,
            "category": category_str,
            "timeLimitSeconds": self.time_limit_seconds,
            "followUpPrompts": &self.follow_up_prompts,
            "order": self.order,
        })
    }
}
