use crate::domain::models::*;
use crate::error::AppResult;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<User>>;
    async fn find_by_email(&self, email: &str) -> AppResult<Option<User>>;
    async fn create(&self, user: &User) -> AppResult<()>;
}

#[async_trait]
pub trait SessionRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> AppResult<Option<Session>>;
}

#[async_trait]
pub trait InterviewRepository: Send + Sync {
    async fn create(&self, interview: &Interview) -> AppResult<Interview>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Interview>>;
    async fn find_by_invite_token(&self, token: &str) -> AppResult<Option<Interview>>;
    async fn find_all(&self, limit: i64, offset: i64) -> AppResult<Vec<Interview>>;
    async fn count_all(&self) -> AppResult<u64>;
    async fn find_all_for_recruiter(
        &self,
        recruiter_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Interview>>;
    async fn count_for_recruiter(&self, recruiter_id: Uuid) -> AppResult<u64>;
    async fn find_all_for_candidate(
        &self,
        candidate_id: Uuid,
        candidate_email: &str,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Interview>>;
    async fn count_for_candidate(
        &self,
        candidate_id: Uuid,
        candidate_email: &str,
    ) -> AppResult<u64>;
    async fn update_status(&self, id: Uuid, status: InterviewStatus) -> AppResult<()>;
    async fn update(&self, interview: &Interview) -> AppResult<()>;
    async fn delete(&self, id: Uuid) -> AppResult<()>;
}

#[async_trait]
pub trait InterviewSessionRepository: Send + Sync {
    async fn create(&self, session: &InterviewSession) -> AppResult<InterviewSession>;
    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<InterviewSession>>;
    async fn find_by_interview_id(&self, interview_id: Uuid)
    -> AppResult<Option<InterviewSession>>;
    async fn append_transcript(&self, id: Uuid, entry: &TranscriptEntry) -> AppResult<()>;
    async fn update_status(&self, id: Uuid, status: SessionStatus) -> AppResult<()>;
    async fn save_scores(&self, id: Uuid, scores: &InterviewScores) -> AppResult<()>;
}
