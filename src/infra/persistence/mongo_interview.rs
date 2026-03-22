use crate::domain::models::{
    Interview, InterviewScores, InterviewSession, InterviewStatus, SessionStatus, TranscriptEntry,
};
use crate::domain::{InterviewRepository, InterviewSessionRepository};
use crate::error::{AppError, AppResult};
use async_trait::async_trait;
use mongodb::bson::{Document, doc};
use mongodb::{Client, Collection};
use uuid::Uuid;

const INTERVIEWS_COLLECTION: &str = "interviews";
const SESSIONS_COLLECTION: &str = "answer_sessions";

pub struct MongoInterviewRepository {
    interviews: Collection<Document>,
}

impl MongoInterviewRepository {
    pub fn new(client: &Client, database: &str) -> Self {
        let db = client.database(database);
        Self {
            interviews: db.collection(INTERVIEWS_COLLECTION),
        }
    }

    fn candidate_filter(candidate_id: Uuid, candidate_email: &str) -> Document {
        doc! {
            "$or": [
                { "candidateId": candidate_id.to_string() },
                { "candidateEmail": candidate_email }
            ]
        }
    }
}

#[async_trait]
impl InterviewRepository for MongoInterviewRepository {
    async fn create(&self, interview: &Interview) -> AppResult<Interview> {
        let doc = interview.to_bson()?;
        self.interviews.insert_one(doc).await?;
        Ok(interview.clone())
    }

    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<Interview>> {
        let filter = doc! { "id": id.to_string() };
        let result = self.interviews.find_one(filter).await?;

        match result {
            Some(doc) => {
                let interview = Interview::from_bson_document(doc)?;
                Ok(Some(interview))
            }
            None => Ok(None),
        }
    }

    async fn find_by_invite_token(&self, token: &str) -> AppResult<Option<Interview>> {
        let filter = doc! { "inviteToken": token };
        let result = self.interviews.find_one(filter).await?;

        match result {
            Some(doc) => {
                let interview = Interview::from_bson_document(doc)?;
                Ok(Some(interview))
            }
            None => Ok(None),
        }
    }

    async fn find_all(&self, limit: i64, offset: i64) -> AppResult<Vec<Interview>> {
        let mut cursor = self
            .interviews
            .find(doc! {})
            .sort(doc! { "createdAt": -1 })
            .skip(offset as u64)
            .limit(limit)
            .await?;

        let mut interviews = Vec::new();
        while cursor.advance().await? {
            let doc = cursor.deserialize_current()?;
            let interview = Interview::from_bson_document(doc)?;
            interviews.push(interview);
        }

        Ok(interviews)
    }

    async fn count_all(&self) -> AppResult<u64> {
        Ok(self.interviews.count_documents(doc! {}).await?)
    }

    async fn find_all_for_recruiter(
        &self,
        recruiter_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Interview>> {
        let mut cursor = self
            .interviews
            .find(doc! { "recruiterId": recruiter_id.to_string() })
            .sort(doc! { "createdAt": -1 })
            .skip(offset as u64)
            .limit(limit)
            .await?;

        let mut interviews = Vec::new();
        while cursor.advance().await? {
            let doc = cursor.deserialize_current()?;
            interviews.push(Interview::from_bson_document(doc)?);
        }

        Ok(interviews)
    }

    async fn count_for_recruiter(&self, recruiter_id: Uuid) -> AppResult<u64> {
        Ok(self
            .interviews
            .count_documents(doc! { "recruiterId": recruiter_id.to_string() })
            .await?)
    }

    async fn find_all_for_candidate(
        &self,
        candidate_id: Uuid,
        candidate_email: &str,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<Interview>> {
        let mut cursor = self
            .interviews
            .find(Self::candidate_filter(candidate_id, candidate_email))
            .sort(doc! { "createdAt": -1 })
            .skip(offset as u64)
            .limit(limit)
            .await?;

        let mut interviews = Vec::new();
        while cursor.advance().await? {
            let doc = cursor.deserialize_current()?;
            interviews.push(Interview::from_bson_document(doc)?);
        }

        Ok(interviews)
    }

    async fn count_for_candidate(
        &self,
        candidate_id: Uuid,
        candidate_email: &str,
    ) -> AppResult<u64> {
        Ok(self
            .interviews
            .count_documents(Self::candidate_filter(candidate_id, candidate_email))
            .await?)
    }

    async fn update_status(&self, id: Uuid, status: InterviewStatus) -> AppResult<()> {
        let status_str = match status {
            InterviewStatus::Draft => "Draft",
            InterviewStatus::Scheduled => "Scheduled",
            InterviewStatus::Active => "Active",
            InterviewStatus::Completed => "Completed",
            InterviewStatus::Cancelled => "Cancelled",
        };

        let filter = doc! { "id": id.to_string() };
        let update = doc! {
            "$set": {
                "status": status_str,
                "updatedAt": bson::DateTime::now()
            }
        };

        let result = self.interviews.update_one(filter, update).await?;

        if result.matched_count == 0 {
            return Err(AppError::InterviewNotFound(id));
        }

        Ok(())
    }

    async fn update(&self, interview: &Interview) -> AppResult<()> {
        let filter = doc! { "id": interview.id.to_string() };
        let update_doc = interview.to_bson()?;
        let update = doc! { "$set": update_doc };

        let result = self.interviews.update_one(filter, update).await?;

        if result.matched_count == 0 {
            return Err(AppError::InterviewNotFound(interview.id));
        }

        Ok(())
    }

    async fn delete(&self, id: Uuid) -> AppResult<()> {
        let filter = doc! { "id": id.to_string() };
        self.interviews.delete_one(filter).await?;
        Ok(())
    }
}

pub struct MongoInterviewSessionRepository {
    sessions: Collection<Document>,
}

impl MongoInterviewSessionRepository {
    pub fn new(client: &Client, database: &str) -> Self {
        let db = client.database(database);
        Self {
            sessions: db.collection(SESSIONS_COLLECTION),
        }
    }
}

#[async_trait]
impl InterviewSessionRepository for MongoInterviewSessionRepository {
    async fn create(&self, session: &InterviewSession) -> AppResult<InterviewSession> {
        let doc = session.to_bson()?;
        self.sessions.insert_one(doc).await?;
        Ok(session.clone())
    }

    async fn find_by_id(&self, id: Uuid) -> AppResult<Option<InterviewSession>> {
        let filter = doc! { "id": id.to_string() };
        let result = self.sessions.find_one(filter).await?;

        match result {
            Some(doc) => {
                let session = InterviewSession::from_bson_document(doc)?;
                Ok(Some(session))
            }
            None => Ok(None),
        }
    }

    async fn find_by_interview_id(
        &self,
        interview_id: Uuid,
    ) -> AppResult<Option<InterviewSession>> {
        let filter = doc! { "interviewId": interview_id.to_string() };
        let result = self
            .sessions
            .find_one(filter)
            .sort(doc! { "createdAt": -1 })
            .await?;

        match result {
            Some(doc) => Ok(Some(InterviewSession::from_bson_document(doc)?)),
            None => Ok(None),
        }
    }

    async fn append_transcript(&self, id: Uuid, entry: &TranscriptEntry) -> AppResult<()> {
        let filter = doc! { "id": id.to_string() };
        let entry_doc = entry.to_bson()?;

        let update = doc! {
            "$push": { "transcript": entry_doc },
            "$set": { "updatedAt": bson::DateTime::now() }
        };

        let result = self.sessions.update_one(filter, update).await?;

        if result.matched_count == 0 {
            return Err(AppError::SessionNotFound(id));
        }

        Ok(())
    }

    async fn update_status(&self, id: Uuid, status: SessionStatus) -> AppResult<()> {
        let status_str = match status {
            SessionStatus::Waiting => "Waiting",
            SessionStatus::Active => "Active",
            SessionStatus::Paused => "Paused",
            SessionStatus::Completed => "Completed",
            SessionStatus::Cancelled => "Cancelled",
        };

        let now = bson::DateTime::now();
        let filter = doc! { "id": id.to_string() };

        let update = if status == SessionStatus::Completed {
            doc! {
                "$set": {
                    "status": status_str,
                    "updatedAt": now,
                    "endedAt": now
                }
            }
        } else {
            doc! {
                "$set": {
                    "status": status_str,
                    "updatedAt": now
                }
            }
        };

        let result = self.sessions.update_one(filter, update).await?;

        if result.matched_count == 0 {
            return Err(AppError::SessionNotFound(id));
        }

        Ok(())
    }

    async fn save_scores(&self, id: Uuid, scores: &InterviewScores) -> AppResult<()> {
        let scores_doc = doc! {
            "technicalAccuracy": scores.technical_accuracy as f64,
            "communication": scores.communication as f64,
            "problemSolving": scores.problem_solving as f64,
            "confidence": scores.confidence as f64,
            "overall": scores.overall as f64,
            "reasoning": &scores.reasoning,
            "strengths": &scores.strengths,
            "areasForImprovement": &scores.areas_for_improvement,
            "evaluatedAt": bson::DateTime::from_millis(scores.evaluated_at.timestamp_millis()),
        };

        let filter = doc! { "id": id.to_string() };
        let update = doc! {
            "$set": {
                "scores": scores_doc,
                "updatedAt": bson::DateTime::now()
            }
        };

        let result = self.sessions.update_one(filter, update).await?;

        if result.matched_count == 0 {
            return Err(AppError::SessionNotFound(id));
        }

        Ok(())
    }
}
