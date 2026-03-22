use crate::api::dto::{
    AssignCandidateDto, CreateInterviewDto, InterviewListResponse, InterviewResponse,
    JoinInterviewDto, UpdateInterviewDto,
};
use crate::api::state::AppState;
use crate::auth::{AuthUser, UserRole};
use crate::domain::models::{
    CreateInterviewRequest, Interview, InterviewSession, InterviewStatus, User,
};
use crate::error::{AppError, AppResult};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, serde::Deserialize)]
pub struct ListQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

fn ensure_staff_role(role: &UserRole) -> AppResult<()> {
    match role {
        UserRole::Admin | UserRole::Recruiter => Ok(()),
        UserRole::Candidate => Err(AppError::Forbidden(
            "Admin or Recruiter access required".to_string(),
        )),
    }
}

fn ensure_interview_access(user: &User, role: &UserRole, interview: &Interview) -> AppResult<()> {
    match role {
        UserRole::Admin => Ok(()),
        UserRole::Recruiter => {
            if interview.is_owned_by(user.id) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "You do not have access to this interview".to_string(),
                ))
            }
        }
        UserRole::Candidate => {
            if interview.is_accessible_by_candidate(user.id, &user.email) {
                Ok(())
            } else {
                Err(AppError::Forbidden(
                    "You do not have access to this interview".to_string(),
                ))
            }
        }
    }
}

pub async fn create_interview(
    State(state): State<Arc<AppState>>,
    AuthUser(user, role): AuthUser,
    Json(dto): Json<CreateInterviewDto>,
) -> AppResult<Json<InterviewResponse>> {
    ensure_staff_role(&role)?;

    let request: CreateInterviewRequest = dto.into();
    let recruiter_id = user.id;

    let interview = Interview::new(recruiter_id, request);
    let saved = state.interview_repo.create(&interview).await?;

    Ok(Json(InterviewResponse::from(saved)))
}

pub async fn list_interviews(
    State(state): State<Arc<AppState>>,
    AuthUser(user, role): AuthUser,
    Query(query): Query<ListQuery>,
) -> AppResult<Json<InterviewListResponse>> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let offset = query.offset.unwrap_or(0).max(0);

    let (interviews, total) = match role {
        UserRole::Admin => (
            state.interview_repo.find_all(limit, offset).await?,
            state.interview_repo.count_all().await?,
        ),
        UserRole::Recruiter => (
            state
                .interview_repo
                .find_all_for_recruiter(user.id, limit, offset)
                .await?,
            state.interview_repo.count_for_recruiter(user.id).await?,
        ),
        UserRole::Candidate => (
            state
                .interview_repo
                .find_all_for_candidate(user.id, &user.email, limit, offset)
                .await?,
            state
                .interview_repo
                .count_for_candidate(user.id, &user.email)
                .await?,
        ),
    };

    Ok(Json(InterviewListResponse {
        interviews: interviews
            .into_iter()
            .map(InterviewResponse::from)
            .collect(),
        total: total as i64,
        limit,
        offset,
    }))
}

pub async fn get_interview(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
) -> AppResult<Json<InterviewResponse>> {
    let interview = state
        .interview_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::InterviewNotFound(id))?;
    ensure_interview_access(&user, &role, &interview)?;

    Ok(Json(InterviewResponse::from(interview)))
}

pub async fn update_interview(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
    Json(dto): Json<UpdateInterviewDto>,
) -> AppResult<Json<InterviewResponse>> {
    ensure_staff_role(&role)?;

    let mut interview = state
        .interview_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::InterviewNotFound(id))?;
    ensure_interview_access(&user, &role, &interview)?;

    if interview.is_terminal() {
        return Err(AppError::Conflict(
            "Cannot update an interview that has already ended".to_string(),
        ));
    }

    if let Some(title) = dto.title {
        interview.title = title;
    }
    if let Some(job_title) = dto.job_title {
        interview.job_title = job_title;
    }
    if let Some(job_description) = dto.job_description {
        interview.job_description = Some(job_description);
    }
    if let Some(duration_minutes) = dto.duration_minutes {
        interview.duration_minutes = duration_minutes;
    }

    interview.updated_at = chrono::Utc::now();

    state.interview_repo.update(&interview).await?;

    Ok(Json(InterviewResponse::from(interview)))
}

pub async fn assign_candidate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
    Json(dto): Json<AssignCandidateDto>,
) -> AppResult<Json<InterviewResponse>> {
    ensure_staff_role(&role)?;

    let mut interview = state
        .interview_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::InterviewNotFound(id))?;
    ensure_interview_access(&user, &role, &interview)?;

    if interview.status == InterviewStatus::Active {
        return Err(AppError::Conflict(
            "Cannot reassign the candidate while the interview is active".to_string(),
        ));
    }

    if interview.is_terminal() {
        return Err(AppError::Conflict(
            "Cannot assign a candidate to an interview that has already ended".to_string(),
        ));
    }

    let candidate_user = state.user_repo.find_by_email(&dto.candidate_email).await?;

    interview.candidate_name = Some(dto.candidate_name);
    interview.candidate_email = Some(dto.candidate_email);
    interview.candidate_id = candidate_user.map(|candidate| candidate.id);
    interview.status = InterviewStatus::Scheduled;
    interview.updated_at = chrono::Utc::now();

    state.interview_repo.update(&interview).await?;

    Ok(Json(InterviewResponse::from(interview)))
}

pub async fn schedule_interview(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
) -> AppResult<Json<InterviewResponse>> {
    ensure_staff_role(&role)?;

    let mut interview = state
        .interview_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::InterviewNotFound(id))?;
    ensure_interview_access(&user, &role, &interview)?;

    if interview.candidate_email.is_none() {
        return Err(AppError::BadRequest(
            "Candidate must be assigned first".to_string(),
        ));
    }

    if !interview.can_transition_to_scheduled() {
        return Err(AppError::Conflict(
            "Interview cannot be scheduled from its current status".to_string(),
        ));
    }

    interview.status = InterviewStatus::Scheduled;
    interview.updated_at = chrono::Utc::now();

    state.interview_repo.update(&interview).await?;

    Ok(Json(InterviewResponse::from(interview)))
}

pub async fn delete_interview(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
) -> AppResult<StatusCode> {
    ensure_staff_role(&role)?;

    let interview = state
        .interview_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::InterviewNotFound(id))?;
    ensure_interview_access(&user, &role, &interview)?;

    if interview.status == InterviewStatus::Active {
        return Err(AppError::Conflict(
            "Cannot delete an active interview".to_string(),
        ));
    }

    state.interview_repo.delete(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_interview_by_token(
    State(state): State<Arc<AppState>>,
    Path(token): Path<String>,
) -> AppResult<Json<InterviewResponse>> {
    let interview = state
        .interview_repo
        .find_by_invite_token(&token)
        .await?
        .ok_or_else(|| AppError::NotFound("Interview not found".to_string()))?;

    Ok(Json(InterviewResponse::from(interview)))
}

pub async fn join_interview(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    AuthUser(user, role): AuthUser,
) -> AppResult<Json<JoinInterviewDto>> {
    if role != UserRole::Candidate {
        return Err(AppError::Forbidden(
            "Only candidates can join an interview".to_string(),
        ));
    }

    let mut interview = state
        .interview_repo
        .find_by_id(id)
        .await?
        .ok_or_else(|| AppError::InterviewNotFound(id))?;

    if !interview.can_be_joined() {
        return Err(AppError::Conflict(
            "Interview is not available to join".to_string(),
        ));
    }

    if !interview.is_accessible_by_candidate(user.id, &user.email) {
        return Err(AppError::Forbidden(
            "You are not the assigned candidate for this interview".to_string(),
        ));
    }

    if interview.candidate_id != Some(user.id) {
        interview.candidate_id = Some(user.id);
    }

    let room_name = format!("interview-{}", interview.id);
    let saved_session = match state
        .interview_session_repo
        .find_by_interview_id(interview.id)
        .await?
    {
        Some(existing_session)
            if existing_session.is_owned_by_candidate(user.id) && existing_session.is_live() =>
        {
            interview.livekit_room = Some(existing_session.livekit_room.clone());
            existing_session
        }
        Some(existing_session) if !existing_session.is_owned_by_candidate(user.id) => {
            return Err(AppError::Forbidden(
                "An interview session already exists for another candidate".to_string(),
            ));
        }
        Some(_existing_session) => {
            return Err(AppError::Conflict(
                "This interview session has already ended".to_string(),
            ));
        }
        None => {
            state.livekit_client.get_or_create_room(&room_name).await?;
            interview.livekit_room = Some(room_name.clone());
            interview.status = InterviewStatus::Active;
            interview.updated_at = chrono::Utc::now();

            let session = InterviewSession::new(interview.id, user.id, room_name.clone());
            state.interview_session_repo.create(&session).await?
        }
    };

    interview.status = InterviewStatus::Active;
    interview.updated_at = chrono::Utc::now();
    state.interview_repo.update(&interview).await?;

    let livekit_token = state.livekit_client.generate_candidate_token(
        &saved_session.livekit_room,
        &user.name.unwrap_or_else(|| "Candidate".to_string()),
    )?;

    Ok(Json(JoinInterviewDto {
        interview_id: interview.id,
        session_id: saved_session.id,
        livekit_token,
        livekit_url: state.config.livekit_url.clone(),
        interview: InterviewResponse::from(interview),
    }))
}
