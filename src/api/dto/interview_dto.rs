use crate::domain::models::{
    CreateInterviewRequest, Interview, InterviewQuestion, InterviewStatus, QuestionCategory,
};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct InterviewResponse {
    #[ts(type = "string")]
    pub id: Uuid,
    pub title: String,
    pub job_title: String,
    pub job_description: Option<String>,
    pub questions: Vec<QuestionResponse>,
    pub status: String,
    #[ts(type = "string")]
    pub recruiter_id: Uuid,
    #[ts(type = "string")]
    pub candidate_id: Option<Uuid>,
    pub candidate_name: Option<String>,
    pub candidate_email: Option<String>,
    pub livekit_room: Option<String>,
    pub duration_minutes: i32,
    #[ts(type = "string")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[ts(type = "string")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub invite_token: Option<String>,
    pub invite_link: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct QuestionResponse {
    pub id: String,
    pub question: String,
    pub category: String,
    pub time_limit_seconds: Option<i32>,
    pub follow_up_prompts: Vec<String>,
    pub order: i32,
}

impl From<Interview> for InterviewResponse {
    fn from(interview: Interview) -> Self {
        let invite_link = interview
            .invite_token
            .as_ref()
            .map(|token| format!("/interview/join/{}", token));

        Self {
            id: interview.id,
            title: interview.title,
            job_title: interview.job_title,
            job_description: interview.job_description,
            questions: interview
                .questions
                .into_iter()
                .map(QuestionResponse::from)
                .collect(),
            status: match interview.status {
                InterviewStatus::Draft => "Draft".to_string(),
                InterviewStatus::Scheduled => "Scheduled".to_string(),
                InterviewStatus::Active => "Active".to_string(),
                InterviewStatus::Completed => "Completed".to_string(),
                InterviewStatus::Cancelled => "Cancelled".to_string(),
            },
            recruiter_id: interview.recruiter_id,
            candidate_id: interview.candidate_id,
            candidate_name: interview.candidate_name,
            candidate_email: interview.candidate_email,
            livekit_room: interview.livekit_room,
            duration_minutes: interview.duration_minutes,
            created_at: interview.created_at,
            updated_at: interview.updated_at,
            invite_token: interview.invite_token.clone(),
            invite_link,
        }
    }
}

impl From<InterviewQuestion> for QuestionResponse {
    fn from(q: InterviewQuestion) -> Self {
        Self {
            id: q.id,
            question: q.question,
            category: match q.category {
                QuestionCategory::Technical => "Technical".to_string(),
                QuestionCategory::Behavioral => "Behavioral".to_string(),
                QuestionCategory::Situational => "Situational".to_string(),
                QuestionCategory::Coding => "Coding".to_string(),
                QuestionCategory::SystemDesign => "SystemDesign".to_string(),
                QuestionCategory::SoftSkills => "SoftSkills".to_string(),
            },
            time_limit_seconds: q.time_limit_seconds,
            follow_up_prompts: q.follow_up_prompts,
            order: q.order,
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateInterviewDto {
    pub title: String,
    pub job_title: String,
    pub job_description: Option<String>,
    pub questions: Vec<CreateQuestionDto>,
    pub duration_minutes: Option<i32>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateQuestionDto {
    pub question: String,
    pub category: String,
    pub time_limit_seconds: Option<i32>,
    pub follow_up_prompts: Vec<String>,
}

impl From<CreateInterviewDto> for CreateInterviewRequest {
    fn from(dto: CreateInterviewDto) -> Self {
        Self {
            title: dto.title,
            job_title: dto.job_title,
            job_description: dto.job_description,
            questions: dto
                .questions
                .into_iter()
                .map(|q| crate::domain::models::CreateQuestionRequest {
                    question: q.question,
                    category: match q.category.as_str() {
                        "Technical" => QuestionCategory::Technical,
                        "Behavioral" => QuestionCategory::Behavioral,
                        "Situational" => QuestionCategory::Situational,
                        "Coding" => QuestionCategory::Coding,
                        "SystemDesign" => QuestionCategory::SystemDesign,
                        "SoftSkills" => QuestionCategory::SoftSkills,
                        _ => QuestionCategory::Behavioral,
                    },
                    time_limit_seconds: q.time_limit_seconds,
                    follow_up_prompts: q.follow_up_prompts,
                })
                .collect(),
            duration_minutes: dto.duration_minutes.unwrap_or(30),
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateInterviewDto {
    pub title: Option<String>,
    pub job_title: Option<String>,
    pub job_description: Option<String>,
    pub duration_minutes: Option<i32>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct AssignCandidateDto {
    pub candidate_name: String,
    pub candidate_email: String,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct InterviewListResponse {
    pub interviews: Vec<InterviewResponse>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct JoinInterviewDto {
    #[ts(type = "string")]
    pub interview_id: Uuid,
    #[ts(type = "string")]
    pub session_id: Uuid,
    pub livekit_token: String,
    pub livekit_url: String,
    pub interview: InterviewResponse,
}
