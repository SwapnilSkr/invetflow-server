use crate::api::handlers::{interview, session};
use crate::api::state::AppState;
use crate::auth::{JwtConfig, TokenResponse, UserRole};
use crate::config::Config;
use crate::domain::models::{
    Interview, InterviewScores, InterviewSession, InterviewStatus, Session, SessionStatus,
    TranscriptEntry, User,
};
use crate::domain::{
    InterviewRepository, InterviewSessionRepository, SessionRepository, UserRepository,
};
use crate::infra::LiveKitClient;
use async_trait::async_trait;
use axum::{
    Extension, Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    routing::{get, post},
};
use chrono::Utc;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tower::util::ServiceExt;
use uuid::Uuid;

#[derive(Default)]
struct MockUserRepository {
    users: Mutex<HashMap<Uuid, User>>,
}

impl MockUserRepository {
    fn insert(&self, user: User) {
        self.users.lock().unwrap().insert(user.id, user);
    }
}

#[async_trait]
impl UserRepository for MockUserRepository {
    async fn find_by_id(&self, id: Uuid) -> crate::error::AppResult<Option<User>> {
        Ok(self.users.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_email(&self, email: &str) -> crate::error::AppResult<Option<User>> {
        Ok(self
            .users
            .lock()
            .unwrap()
            .values()
            .find(|user| user.email == email)
            .cloned())
    }

    async fn create(&self, user: &User) -> crate::error::AppResult<()> {
        self.users.lock().unwrap().insert(user.id, user.clone());
        Ok(())
    }
}

#[derive(Default)]
struct MockSessionRepository;

#[async_trait]
impl SessionRepository for MockSessionRepository {
    async fn find_by_id(&self, _id: &str) -> crate::error::AppResult<Option<Session>> {
        Ok(None)
    }
}

#[derive(Default)]
struct MockInterviewRepository {
    interviews: Mutex<HashMap<Uuid, Interview>>,
}

impl MockInterviewRepository {
    fn insert(&self, interview: Interview) {
        self.interviews
            .lock()
            .unwrap()
            .insert(interview.id, interview);
    }

    fn sorted_filtered<F>(&self, predicate: F, limit: i64, offset: i64) -> Vec<Interview>
    where
        F: Fn(&Interview) -> bool,
    {
        let mut interviews = self
            .interviews
            .lock()
            .unwrap()
            .values()
            .filter(|interview| predicate(interview))
            .cloned()
            .collect::<Vec<_>>();
        interviews.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        interviews
            .into_iter()
            .skip(offset.max(0) as usize)
            .take(limit.max(0) as usize)
            .collect()
    }
}

#[async_trait]
impl InterviewRepository for MockInterviewRepository {
    async fn create(&self, interview: &Interview) -> crate::error::AppResult<Interview> {
        self.interviews
            .lock()
            .unwrap()
            .insert(interview.id, interview.clone());
        Ok(interview.clone())
    }

    async fn find_by_id(&self, id: Uuid) -> crate::error::AppResult<Option<Interview>> {
        Ok(self.interviews.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_invite_token(
        &self,
        token: &str,
    ) -> crate::error::AppResult<Option<Interview>> {
        Ok(self
            .interviews
            .lock()
            .unwrap()
            .values()
            .find(|interview| interview.invite_token.as_deref() == Some(token))
            .cloned())
    }

    async fn find_all(&self, limit: i64, offset: i64) -> crate::error::AppResult<Vec<Interview>> {
        Ok(self.sorted_filtered(|_| true, limit, offset))
    }

    async fn count_all(&self) -> crate::error::AppResult<u64> {
        Ok(self.interviews.lock().unwrap().len() as u64)
    }

    async fn find_all_for_recruiter(
        &self,
        recruiter_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> crate::error::AppResult<Vec<Interview>> {
        Ok(self.sorted_filtered(
            |interview| interview.recruiter_id == recruiter_id,
            limit,
            offset,
        ))
    }

    async fn count_for_recruiter(&self, recruiter_id: Uuid) -> crate::error::AppResult<u64> {
        Ok(self
            .interviews
            .lock()
            .unwrap()
            .values()
            .filter(|interview| interview.recruiter_id == recruiter_id)
            .count() as u64)
    }

    async fn find_all_for_candidate(
        &self,
        candidate_id: Uuid,
        candidate_email: &str,
        limit: i64,
        offset: i64,
    ) -> crate::error::AppResult<Vec<Interview>> {
        Ok(self.sorted_filtered(
            |interview| interview.is_accessible_by_candidate(candidate_id, candidate_email),
            limit,
            offset,
        ))
    }

    async fn count_for_candidate(
        &self,
        candidate_id: Uuid,
        candidate_email: &str,
    ) -> crate::error::AppResult<u64> {
        Ok(self
            .interviews
            .lock()
            .unwrap()
            .values()
            .filter(|interview| interview.is_accessible_by_candidate(candidate_id, candidate_email))
            .count() as u64)
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: InterviewStatus,
    ) -> crate::error::AppResult<()> {
        let mut interviews = self.interviews.lock().unwrap();
        let interview = interviews
            .get_mut(&id)
            .ok_or(crate::error::AppError::InterviewNotFound(id))?;
        interview.status = status;
        interview.updated_at = Utc::now();
        Ok(())
    }

    async fn update(&self, interview: &Interview) -> crate::error::AppResult<()> {
        let mut interviews = self.interviews.lock().unwrap();
        if !interviews.contains_key(&interview.id) {
            return Err(crate::error::AppError::InterviewNotFound(interview.id));
        }
        interviews.insert(interview.id, interview.clone());
        Ok(())
    }

    async fn delete(&self, id: Uuid) -> crate::error::AppResult<()> {
        self.interviews.lock().unwrap().remove(&id);
        Ok(())
    }
}

#[derive(Default)]
struct MockInterviewSessionRepository {
    sessions: Mutex<HashMap<Uuid, InterviewSession>>,
}

impl MockInterviewSessionRepository {
    fn insert(&self, session: InterviewSession) {
        self.sessions.lock().unwrap().insert(session.id, session);
    }
}

#[async_trait]
impl InterviewSessionRepository for MockInterviewSessionRepository {
    async fn create(
        &self,
        session: &InterviewSession,
    ) -> crate::error::AppResult<InterviewSession> {
        self.sessions
            .lock()
            .unwrap()
            .insert(session.id, session.clone());
        Ok(session.clone())
    }

    async fn find_by_id(&self, id: Uuid) -> crate::error::AppResult<Option<InterviewSession>> {
        Ok(self.sessions.lock().unwrap().get(&id).cloned())
    }

    async fn find_by_interview_id(
        &self,
        interview_id: Uuid,
    ) -> crate::error::AppResult<Option<InterviewSession>> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap()
            .values()
            .filter(|session| session.interview_id == interview_id)
            .cloned()
            .collect::<Vec<_>>();
        sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(sessions.into_iter().next())
    }

    async fn append_transcript(
        &self,
        id: Uuid,
        entry: &TranscriptEntry,
    ) -> crate::error::AppResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(&id)
            .ok_or(crate::error::AppError::SessionNotFound(id))?;
        session.transcript.push(entry.clone());
        session.updated_at = Utc::now();
        Ok(())
    }

    async fn update_status(&self, id: Uuid, status: SessionStatus) -> crate::error::AppResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(&id)
            .ok_or(crate::error::AppError::SessionNotFound(id))?;
        session.status = status.clone();
        session.updated_at = Utc::now();
        if status == SessionStatus::Completed {
            session.ended_at = Some(Utc::now());
        }
        Ok(())
    }

    async fn save_scores(&self, id: Uuid, scores: &InterviewScores) -> crate::error::AppResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions
            .get_mut(&id)
            .ok_or(crate::error::AppError::SessionNotFound(id))?;
        session.scores = Some(scores.clone());
        session.updated_at = Utc::now();
        Ok(())
    }
}

struct TestContext {
    app: Router,
    jwt_config: Arc<JwtConfig>,
    user_repo: Arc<MockUserRepository>,
    interview_repo: Arc<MockInterviewRepository>,
    interview_session_repo: Arc<MockInterviewSessionRepository>,
}

impl TestContext {
    fn new() -> Self {
        let config = Arc::new(test_config());
        let jwt_config = Arc::new(JwtConfig::from_config(&config));
        let user_repo = Arc::new(MockUserRepository::default());
        let interview_repo = Arc::new(MockInterviewRepository::default());
        let interview_session_repo = Arc::new(MockInterviewSessionRepository::default());

        let state = Arc::new(AppState {
            config: config.clone(),
            jwt_config: jwt_config.clone(),
            user_repo: user_repo.clone(),
            session_repo: Arc::new(MockSessionRepository),
            interview_repo: interview_repo.clone(),
            interview_session_repo: interview_session_repo.clone(),
            livekit_client: Arc::new(LiveKitClient::new(&config)),
        });

        let app = Router::new()
            .route("/api/interviews", get(interview::list_interviews))
            .route("/api/interviews/{id}", get(interview::get_interview))
            .route(
                "/api/interviews/{id}/assign",
                post(interview::assign_candidate),
            )
            .route("/api/interviews/{id}/join", post(interview::join_interview))
            .route("/api/sessions/{id}", get(session::get_session))
            .route(
                "/api/sessions/{id}/transcript",
                post(session::append_transcript),
            )
            .route(
                "/api/sessions/{id}/scores",
                get(session::get_scores).post(session::save_scores),
            )
            .route("/api/sessions/{id}/end", post(session::end_session))
            .layer(Extension(jwt_config.clone()))
            .with_state(state);

        Self {
            app,
            jwt_config,
            user_repo,
            interview_repo,
            interview_session_repo,
        }
    }

    fn token(&self, user: &User, role: UserRole) -> TokenResponse {
        self.jwt_config
            .generate_token(user.id, user.email.clone(), user.name.clone(), role)
            .unwrap()
    }
}

fn test_config() -> Config {
    Config {
        mongo_uri: "mongodb://localhost:27017".to_string(),
        mongo_database: "test".to_string(),
        jwt_secret: "abcdefghijklmnopqrstuvwxyz123456".to_string(),
        jwt_expiry_seconds: 3600,
        better_auth_secret: "abcdefghijklmnopqrstuvwxyz123456".to_string(),
        livekit_url: "http://127.0.0.1:65535".to_string(),
        livekit_api_key: "test-key".to_string(),
        livekit_api_secret: "test-secret".to_string(),
        server_host: "127.0.0.1".to_string(),
        server_port: 3001,
        cors_origins: vec!["http://localhost:3000".to_string()],
    }
}

fn make_user(email: &str, name: &str) -> User {
    let now = Utc::now();
    User {
        id: Uuid::new_v4(),
        email: email.to_string(),
        name: Some(name.to_string()),
        email_verified: true,
        image: None,
        created_at: now,
        updated_at: now,
    }
}

fn make_interview(
    recruiter_id: Uuid,
    candidate_id: Option<Uuid>,
    candidate_email: Option<&str>,
    status: InterviewStatus,
) -> Interview {
    let mut interview = Interview::new(
        recruiter_id,
        crate::domain::models::CreateInterviewRequest {
            title: "Backend Interview".to_string(),
            job_title: "Rust Engineer".to_string(),
            job_description: Some("Production APIs".to_string()),
            questions: Vec::new(),
            duration_minutes: 30,
        },
    );
    interview.candidate_id = candidate_id;
    interview.candidate_email = candidate_email.map(ToString::to_string);
    interview.candidate_name = candidate_email.map(|_| "Candidate".to_string());
    interview.status = status;
    interview
}

fn make_session(interview_id: Uuid, candidate_id: Uuid, status: SessionStatus) -> InterviewSession {
    let mut session = InterviewSession::new(interview_id, candidate_id, "room-1".to_string());
    session.status = status.clone();
    if status == SessionStatus::Completed {
        session.ended_at = Some(Utc::now());
    }
    session
}

async fn send_request(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

fn auth_request(method: &str, path: &str, token: &str, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("Authorization", format!("Bearer {token}"));

    if body.is_some() {
        builder = builder.header("Content-Type", "application/json");
    }

    builder
        .body(match body {
            Some(value) => Body::from(serde_json::to_vec(&value).unwrap()),
            None => Body::empty(),
        })
        .unwrap()
}

#[tokio::test]
async fn candidate_list_only_returns_assigned_interviews() {
    let ctx = TestContext::new();
    let recruiter = make_user("recruiter@example.com", "Recruiter");
    let candidate = make_user("candidate@example.com", "Candidate");
    let other_candidate = make_user("other@example.com", "Other Candidate");

    ctx.user_repo.insert(recruiter.clone());
    ctx.user_repo.insert(candidate.clone());
    ctx.user_repo.insert(other_candidate.clone());

    ctx.interview_repo.insert(make_interview(
        recruiter.id,
        Some(candidate.id),
        Some(&candidate.email),
        InterviewStatus::Scheduled,
    ));
    ctx.interview_repo.insert(make_interview(
        recruiter.id,
        None,
        Some(&candidate.email),
        InterviewStatus::Scheduled,
    ));
    ctx.interview_repo.insert(make_interview(
        recruiter.id,
        Some(other_candidate.id),
        Some(&other_candidate.email),
        InterviewStatus::Scheduled,
    ));

    let token = ctx.token(&candidate, UserRole::Candidate);
    let request = auth_request(
        "GET",
        "/api/interviews?limit=10&offset=0",
        &token.access_token,
        None,
    );

    let (status, body) = send_request(&ctx.app, request).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 2);
    assert_eq!(body["interviews"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn recruiter_list_only_returns_owned_interviews() {
    let ctx = TestContext::new();
    let recruiter = make_user("recruiter@example.com", "Recruiter");
    let other_recruiter = make_user("other-recruiter@example.com", "Other Recruiter");

    ctx.user_repo.insert(recruiter.clone());
    ctx.user_repo.insert(other_recruiter.clone());

    ctx.interview_repo.insert(make_interview(
        recruiter.id,
        None,
        None,
        InterviewStatus::Draft,
    ));
    ctx.interview_repo.insert(make_interview(
        other_recruiter.id,
        None,
        None,
        InterviewStatus::Draft,
    ));

    let token = ctx.token(&recruiter, UserRole::Recruiter);
    let request = auth_request(
        "GET",
        "/api/interviews?limit=10&offset=0",
        &token.access_token,
        None,
    );

    let (status, body) = send_request(&ctx.app, request).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["total"], 1);
}

#[tokio::test]
async fn candidate_cannot_fetch_another_candidates_interview() {
    let ctx = TestContext::new();
    let recruiter = make_user("recruiter@example.com", "Recruiter");
    let candidate = make_user("candidate@example.com", "Candidate");
    let other_candidate = make_user("other@example.com", "Other Candidate");

    ctx.user_repo.insert(recruiter.clone());
    ctx.user_repo.insert(candidate.clone());
    ctx.user_repo.insert(other_candidate.clone());

    let interview = make_interview(
        recruiter.id,
        Some(other_candidate.id),
        Some(&other_candidate.email),
        InterviewStatus::Scheduled,
    );
    let interview_id = interview.id;
    ctx.interview_repo.insert(interview);

    let token = ctx.token(&candidate, UserRole::Candidate);
    let request = auth_request(
        "GET",
        &format!("/api/interviews/{interview_id}"),
        &token.access_token,
        None,
    );

    let (status, body) = send_request(&ctx.app, request).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "You do not have access to this interview");
}

#[tokio::test]
async fn assign_candidate_uses_existing_user_id() {
    let ctx = TestContext::new();
    let recruiter = make_user("recruiter@example.com", "Recruiter");
    let candidate = make_user("candidate@example.com", "Candidate");

    ctx.user_repo.insert(recruiter.clone());
    ctx.user_repo.insert(candidate.clone());

    let interview = make_interview(recruiter.id, None, None, InterviewStatus::Draft);
    let interview_id = interview.id;
    ctx.interview_repo.insert(interview);

    let token = ctx.token(&recruiter, UserRole::Recruiter);
    let request = auth_request(
        "POST",
        &format!("/api/interviews/{interview_id}/assign"),
        &token.access_token,
        Some(json!({
            "candidate_name": "Candidate",
            "candidate_email": candidate.email,
        })),
    );

    let (status, body) = send_request(&ctx.app, request).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["candidate_id"], candidate.id.to_string());
    assert_eq!(body["status"], "Scheduled");
}

#[tokio::test]
async fn join_interview_reuses_existing_live_session() {
    let ctx = TestContext::new();
    let recruiter = make_user("recruiter@example.com", "Recruiter");
    let candidate = make_user("candidate@example.com", "Candidate");

    ctx.user_repo.insert(recruiter.clone());
    ctx.user_repo.insert(candidate.clone());

    let interview = make_interview(
        recruiter.id,
        Some(candidate.id),
        Some(&candidate.email),
        InterviewStatus::Scheduled,
    );
    let interview_id = interview.id;
    ctx.interview_repo.insert(interview);

    let session = make_session(interview_id, candidate.id, SessionStatus::Waiting);
    let session_id = session.id;
    ctx.interview_session_repo.insert(session);

    let token = ctx.token(&candidate, UserRole::Candidate);
    let request = auth_request(
        "POST",
        &format!("/api/interviews/{interview_id}/join"),
        &token.access_token,
        None,
    );

    let (status, body) = send_request(&ctx.app, request).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["session_id"], session_id.to_string());
    assert_eq!(body["interview_id"], interview_id.to_string());
}

#[tokio::test]
async fn scores_are_visible_to_owner_recruiter_but_not_candidate() {
    let ctx = TestContext::new();
    let recruiter = make_user("recruiter@example.com", "Recruiter");
    let candidate = make_user("candidate@example.com", "Candidate");

    ctx.user_repo.insert(recruiter.clone());
    ctx.user_repo.insert(candidate.clone());

    let interview = make_interview(
        recruiter.id,
        Some(candidate.id),
        Some(&candidate.email),
        InterviewStatus::Completed,
    );
    let interview_id = interview.id;
    ctx.interview_repo.insert(interview);

    let mut session = make_session(interview_id, candidate.id, SessionStatus::Completed);
    session.scores = Some(InterviewScores {
        technical_accuracy: 80.0,
        communication: 85.0,
        problem_solving: 90.0,
        confidence: 75.0,
        overall: 82.5,
        reasoning: "Strong reasoning".to_string(),
        strengths: vec!["Communication".to_string()],
        areas_for_improvement: vec!["Speed".to_string()],
        evaluated_at: Utc::now(),
    });
    let session_id = session.id;
    ctx.interview_session_repo.insert(session);

    let candidate_token = ctx.token(&candidate, UserRole::Candidate);
    let candidate_request = auth_request(
        "GET",
        &format!("/api/sessions/{session_id}/scores"),
        &candidate_token.access_token,
        None,
    );
    let (candidate_status, _) = send_request(&ctx.app, candidate_request).await;
    assert_eq!(candidate_status, StatusCode::FORBIDDEN);

    let recruiter_token = ctx.token(&recruiter, UserRole::Recruiter);
    let recruiter_request = auth_request(
        "GET",
        &format!("/api/sessions/{session_id}/scores"),
        &recruiter_token.access_token,
        None,
    );
    let (recruiter_status, recruiter_body) = send_request(&ctx.app, recruiter_request).await;

    assert_eq!(recruiter_status, StatusCode::OK);
    assert_eq!(recruiter_body["overall"], 82.5);
}

#[tokio::test]
async fn candidate_cannot_submit_ai_transcript_entries() {
    let ctx = TestContext::new();
    let recruiter = make_user("recruiter@example.com", "Recruiter");
    let candidate = make_user("candidate@example.com", "Candidate");

    ctx.user_repo.insert(recruiter.clone());
    ctx.user_repo.insert(candidate.clone());

    let interview = make_interview(
        recruiter.id,
        Some(candidate.id),
        Some(&candidate.email),
        InterviewStatus::Active,
    );
    let interview_id = interview.id;
    ctx.interview_repo.insert(interview);

    let session = make_session(interview_id, candidate.id, SessionStatus::Active);
    let session_id = session.id;
    ctx.interview_session_repo.insert(session);

    let token = ctx.token(&candidate, UserRole::Candidate);
    let request = auth_request(
        "POST",
        &format!("/api/sessions/{session_id}/transcript"),
        &token.access_token,
        Some(json!({
            "speaker": "AI",
            "content": "system generated",
            "question_id": null,
            "confidence": 0.9
        })),
    );

    let (status, body) = send_request(&ctx.app, request).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(
        body["error"],
        "Candidates may only submit their own transcript entries"
    );
}

#[tokio::test]
async fn completed_sessions_reject_new_transcript_entries() {
    let ctx = TestContext::new();
    let recruiter = make_user("recruiter@example.com", "Recruiter");
    let candidate = make_user("candidate@example.com", "Candidate");

    ctx.user_repo.insert(recruiter.clone());
    ctx.user_repo.insert(candidate.clone());

    let interview = make_interview(
        recruiter.id,
        Some(candidate.id),
        Some(&candidate.email),
        InterviewStatus::Completed,
    );
    let interview_id = interview.id;
    ctx.interview_repo.insert(interview);

    let session = make_session(interview_id, candidate.id, SessionStatus::Completed);
    let session_id = session.id;
    ctx.interview_session_repo.insert(session);

    let token = ctx.token(&candidate, UserRole::Candidate);
    let request = auth_request(
        "POST",
        &format!("/api/sessions/{session_id}/transcript"),
        &token.access_token,
        Some(json!({
            "speaker": "Candidate",
            "content": "late answer",
            "question_id": null,
            "confidence": 0.9
        })),
    );

    let (status, body) = send_request(&ctx.app, request).await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        body["error"],
        "Cannot append transcript entries to a completed session"
    );
}

#[tokio::test]
async fn non_owner_recruiter_cannot_access_session() {
    let ctx = TestContext::new();
    let owner = make_user("owner@example.com", "Owner");
    let other_recruiter = make_user("other@example.com", "Other Recruiter");
    let candidate = make_user("candidate@example.com", "Candidate");

    ctx.user_repo.insert(owner.clone());
    ctx.user_repo.insert(other_recruiter.clone());
    ctx.user_repo.insert(candidate.clone());

    let interview = make_interview(
        owner.id,
        Some(candidate.id),
        Some(&candidate.email),
        InterviewStatus::Active,
    );
    let interview_id = interview.id;
    ctx.interview_repo.insert(interview);

    let session = make_session(interview_id, candidate.id, SessionStatus::Active);
    let session_id = session.id;
    ctx.interview_session_repo.insert(session);

    let token = ctx.token(&other_recruiter, UserRole::Recruiter);
    let request = auth_request(
        "GET",
        &format!("/api/sessions/{session_id}"),
        &token.access_token,
        None,
    );

    let (status, body) = send_request(&ctx.app, request).await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"], "You do not have access to this session");
}

#[tokio::test]
async fn invalid_scores_are_rejected() {
    let ctx = TestContext::new();
    let recruiter = make_user("recruiter@example.com", "Recruiter");
    let candidate = make_user("candidate@example.com", "Candidate");

    ctx.user_repo.insert(recruiter.clone());
    ctx.user_repo.insert(candidate.clone());

    let interview = make_interview(
        recruiter.id,
        Some(candidate.id),
        Some(&candidate.email),
        InterviewStatus::Completed,
    );
    let interview_id = interview.id;
    ctx.interview_repo.insert(interview);

    let session = make_session(interview_id, candidate.id, SessionStatus::Completed);
    let session_id = session.id;
    ctx.interview_session_repo.insert(session);

    let token = ctx.token(&recruiter, UserRole::Recruiter);
    let request = auth_request(
        "POST",
        &format!("/api/sessions/{session_id}/scores"),
        &token.access_token,
        Some(json!({
            "technical_accuracy": 110.0,
            "communication": 80.0,
            "problem_solving": 70.0,
            "confidence": 60.0,
            "reasoning": "ok",
            "strengths": ["Rust"],
            "areas_for_improvement": ["Speed"]
        })),
    );

    let (status, body) = send_request(&ctx.app, request).await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["error"], "Scores must be between 0 and 100");
}
