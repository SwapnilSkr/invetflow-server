use crate::api::dto::{
    AppendTranscriptDto, SaveScoresDto, ScoresResponse, SessionResponse, TranscriptEntryResponse,
    TranscriptResponse,
};
use crate::api::state::AppState;
use crate::auth::{AuthUser, UserRole};
use crate::domain::models::{
    CreateTranscriptRequest, InterviewScores, InterviewSession, SessionStatus, Speaker,
    TranscriptEntry, User,
};
use crate::error::{AppError, AppResult};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use std::sync::Arc;
use uuid::Uuid;

async fn ensure_staff_session_access(
    state: &Arc<AppState>,
    user: &User,
    role: &UserRole,
    session: &InterviewSession,
) -> AppResult<()> {
    let interview = state
        .interview_repo
        .find_by_id(session.interview_id)
        .await?
        .ok_or_else(|| AppError::InterviewNotFound(session.interview_id))?;

    match role {
        UserRole::Admin => Ok(()),
        UserRole::Recruiter if interview.recruiter_id == user.id => Ok(()),
        UserRole::Recruiter => Err(AppError::Forbidden(
            "You do not have access to this session".to_string(),
        )),
        UserRole::Candidate => Err(AppError::Forbidden(
            "Admin or Recruiter access required".to_string(),
        )),
    }
}

async fn ensure_session_access(
    state: &Arc<AppState>,
    user: &User,
    role: &UserRole,
    session: &InterviewSession,
) -> AppResult<()> {
    match role {
        UserRole::Candidate if session.is_owned_by_candidate(user.id) => Ok(()),
        UserRole::Candidate => Err(AppError::Forbidden(
            "You do not have access to this session".to_string(),
        )),
        UserRole::Admin | UserRole::Recruiter => {
            ensure_staff_session_access(state, user, role, session).await?;
            Ok(())
        }
    }
}

pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
) -> AppResult<Json<SessionResponse>> {
    let session = state
        .interview_session_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::SessionNotFound(id))?;
    ensure_session_access(&state, &user, &role, &session).await?;

    let duration_seconds = session.duration_seconds();

    Ok(Json(SessionResponse {
        id: session.id,
        interview_id: session.interview_id,
        candidate_id: session.candidate_id,
        livekit_room: session.livekit_room,
        status: match session.status {
            crate::domain::SessionStatus::Waiting => "Waiting".to_string(),
            crate::domain::SessionStatus::Active => "Active".to_string(),
            crate::domain::SessionStatus::Paused => "Paused".to_string(),
            crate::domain::SessionStatus::Completed => "Completed".to_string(),
            crate::domain::SessionStatus::Cancelled => "Cancelled".to_string(),
        },
        current_question_index: session.current_question_index,
        started_at: session.started_at,
        ended_at: session.ended_at,
        duration_seconds,
    }))
}

pub async fn get_transcript(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
) -> AppResult<Json<TranscriptResponse>> {
    let session = state
        .interview_session_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::SessionNotFound(id))?;
    ensure_session_access(&state, &user, &role, &session).await?;

    let entries: Vec<TranscriptEntryResponse> = session
        .transcript
        .into_iter()
        .map(TranscriptEntryResponse::from)
        .collect();

    Ok(Json(TranscriptResponse {
        total: entries.len() as i64,
        entries,
    }))
}

pub async fn append_transcript(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
    Json(dto): Json<AppendTranscriptDto>,
) -> AppResult<StatusCode> {
    let session = state
        .interview_session_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::SessionNotFound(id))?;
    ensure_session_access(&state, &user, &role, &session).await?;

    if session.is_terminal() {
        return Err(AppError::Conflict(
            "Cannot append transcript entries to a completed session".to_string(),
        ));
    }

    let speaker = match dto.speaker.as_str() {
        "Candidate" => Speaker::Candidate,
        "AI" => Speaker::AI,
        "System" => Speaker::System,
        _ => return Err(AppError::BadRequest("Invalid speaker".to_string())),
    };

    if role == UserRole::Candidate && speaker != Speaker::Candidate {
        return Err(AppError::Forbidden(
            "Candidates may only submit their own transcript entries".to_string(),
        ));
    }

    let entry = TranscriptEntry::new(CreateTranscriptRequest {
        session_id: id,
        speaker,
        content: dto.content,
        question_id: dto.question_id,
        confidence: dto.confidence,
    });

    state
        .interview_session_repo
        .append_transcript(id, &entry)
        .await?;

    Ok(StatusCode::CREATED)
}

pub async fn end_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
) -> AppResult<Json<SessionResponse>> {
    let mut session = state
        .interview_session_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::SessionNotFound(id))?;
    ensure_session_access(&state, &user, &role, &session).await?;

    if session.is_terminal() {
        return Err(AppError::Conflict("Session has already ended".to_string()));
    }

    session.status = SessionStatus::Completed;
    session.ended_at = Some(chrono::Utc::now());

    let duration_seconds = session.duration_seconds();
    let interview_id = session.interview_id;

    state
        .interview_session_repo
        .update_status(id, SessionStatus::Completed)
        .await?;

    state
        .interview_repo
        .update_status(interview_id, crate::domain::InterviewStatus::Completed)
        .await?;

    Ok(Json(SessionResponse {
        id: session.id,
        interview_id,
        candidate_id: session.candidate_id,
        livekit_room: session.livekit_room,
        status: "Completed".to_string(),
        current_question_index: session.current_question_index,
        started_at: session.started_at,
        ended_at: session.ended_at,
        duration_seconds,
    }))
}

pub async fn get_scores(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
) -> AppResult<Json<Option<ScoresResponse>>> {
    let session = state
        .interview_session_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::SessionNotFound(id))?;
    ensure_staff_session_access(&state, &user, &role, &session).await?;

    Ok(Json(session.scores.map(ScoresResponse::from)))
}

pub async fn save_scores(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
    Json(dto): Json<SaveScoresDto>,
) -> AppResult<Json<ScoresResponse>> {
    let session = state
        .interview_session_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::SessionNotFound(id))?;
    ensure_staff_session_access(&state, &user, &role, &session).await?;

    if !(0.0..=100.0).contains(&dto.technical_accuracy)
        || !(0.0..=100.0).contains(&dto.communication)
        || !(0.0..=100.0).contains(&dto.problem_solving)
        || !(0.0..=100.0).contains(&dto.confidence)
    {
        return Err(AppError::Validation(
            "Scores must be between 0 and 100".to_string(),
        ));
    }

    let scores = InterviewScores {
        technical_accuracy: dto.technical_accuracy,
        communication: dto.communication,
        problem_solving: dto.problem_solving,
        confidence: dto.confidence,
        overall: (dto.technical_accuracy
            + dto.communication
            + dto.problem_solving
            + dto.confidence)
            / 4.0,
        reasoning: dto.reasoning,
        strengths: dto.strengths,
        areas_for_improvement: dto.areas_for_improvement,
        evaluated_at: chrono::Utc::now(),
    };

    state
        .interview_session_repo
        .save_scores(id, &scores)
        .await?;

    Ok(Json(ScoresResponse::from(scores)))
}
