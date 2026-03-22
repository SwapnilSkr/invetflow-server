use crate::auth::JwtConfig;
use crate::config::Config;
use crate::domain::{
    InterviewRepository, InterviewSessionRepository, SessionRepository, UserRepository,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub jwt_config: Arc<JwtConfig>,
    pub user_repo: Arc<dyn UserRepository>,
    pub session_repo: Arc<dyn SessionRepository>,
    pub interview_repo: Arc<dyn InterviewRepository>,
    pub interview_session_repo: Arc<dyn InterviewSessionRepository>,
    pub livekit_client: Arc<crate::infra::LiveKitClient>,
}
