use axum::{
    Extension, Router,
    http::{Method, header},
    routing::{delete, get, post, put},
};
use mongodb::bson::doc;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod auth;
mod config;
mod domain;
mod error;
mod infra;
#[cfg(test)]
mod tests;

use api::handlers::health;
use api::state::AppState;
use auth::JwtConfig;
use config::Config;
use domain::{InterviewRepository, InterviewSessionRepository, SessionRepository, UserRepository};
use error::AppResult;
use infra::{
    LiveKitClient, MongoInterviewRepository, MongoInterviewSessionRepository,
    MongoSessionRepository, MongoUserRepository,
};

async fn ensure_mongo_ready(client: &mongodb::Client, database_name: &str) -> AppResult<()> {
    client
        .database("admin")
        .run_command(doc! { "ping": 1 })
        .await
        .map_err(error::AppError::Database)?;

    let db = client.database(database_name);
    let existing_collections = db
        .list_collection_names()
        .await
        .map_err(error::AppError::Database)?;

    for collection in ["users", "sessions", "interviews", "answer_sessions"] {
        if !existing_collections.iter().any(|name| name == collection) {
            db.create_collection(collection)
                .await
                .map_err(error::AppError::Database)?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> AppResult<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    config.validate()?;

    tracing::info!("Connecting to MongoDB at {}", config.mongo_uri);
    let mongo_client = mongodb::Client::with_uri_str(&config.mongo_uri)
        .await
        .map_err(error::AppError::Database)?;
    ensure_mongo_ready(&mongo_client, &config.mongo_database).await?;

    let user_repo: Arc<dyn UserRepository> = Arc::new(MongoUserRepository::new(
        &mongo_client,
        &config.mongo_database,
    ));
    let session_repo: Arc<dyn SessionRepository> = Arc::new(MongoSessionRepository::new(
        &mongo_client,
        &config.mongo_database,
    ));
    let interview_repo: Arc<dyn InterviewRepository> = Arc::new(MongoInterviewRepository::new(
        &mongo_client,
        &config.mongo_database,
    ));
    let interview_session_repo: Arc<dyn InterviewSessionRepository> = Arc::new(
        MongoInterviewSessionRepository::new(&mongo_client, &config.mongo_database),
    );

    let livekit_client = Arc::new(LiveKitClient::new(&config));
    let jwt_config = Arc::new(JwtConfig::from_config(&config));
    let config = Arc::new(config);

    let state = Arc::new(AppState {
        config: config.clone(),
        jwt_config: jwt_config.clone(),
        user_repo,
        session_repo,
        interview_repo,
        interview_session_repo,
        livekit_client,
    });

    let cors_origins = config
        .cors_origins
        .iter()
        .map(|origin| {
            origin.parse::<axum::http::HeaderValue>().map_err(|error| {
                error::AppError::Config(format!("Invalid CORS origin `{origin}`: {error}"))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let cors = CorsLayer::new()
        .allow_origin(cors_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
        .allow_credentials(true);

    let app = Router::new()
        .route("/health", get(health::health_check))
        .route("/ready", get(health::readiness_check))
        .route(
            "/api/auth/exchange",
            post(api::handlers::auth::exchange_session),
        )
        .route(
            "/api/auth/demo-login",
            post(api::handlers::auth::demo_login),
        )
        .route("/api/auth/me", get(api::handlers::auth::get_current_user))
        .route(
            "/api/auth/refresh",
            post(api::handlers::auth::refresh_token),
        )
        .route(
            "/api/interviews",
            post(api::handlers::interview::create_interview),
        )
        .route(
            "/api/interviews",
            get(api::handlers::interview::list_interviews),
        )
        .route(
            "/api/interviews/{id}",
            get(api::handlers::interview::get_interview),
        )
        .route(
            "/api/interviews/{id}",
            put(api::handlers::interview::update_interview),
        )
        .route(
            "/api/interviews/{id}",
            delete(api::handlers::interview::delete_interview),
        )
        .route(
            "/api/interviews/{id}/assign",
            post(api::handlers::interview::assign_candidate),
        )
        .route(
            "/api/interviews/{id}/schedule",
            post(api::handlers::interview::schedule_interview),
        )
        .route(
            "/api/interviews/{id}/join",
            post(api::handlers::interview::join_interview),
        )
        .route(
            "/api/interviews/token/{token}",
            get(api::handlers::interview::get_interview_by_token),
        )
        .route(
            "/api/sessions/{id}",
            get(api::handlers::session::get_session),
        )
        .route(
            "/api/sessions/{id}/transcript",
            get(api::handlers::session::get_transcript),
        )
        .route(
            "/api/sessions/{id}/transcript",
            post(api::handlers::session::append_transcript),
        )
        .route(
            "/api/sessions/{id}/end",
            post(api::handlers::session::end_session),
        )
        .route(
            "/api/sessions/{id}/scores",
            get(api::handlers::session::get_scores),
        )
        .route(
            "/api/sessions/{id}/scores",
            post(api::handlers::session::save_scores),
        )
        .layer(cors)
        .layer(Extension(jwt_config))
        .with_state(state);

    let addr: SocketAddr = config
        .server_address()
        .parse()
        .expect("Failed to parse server address");

    tracing::info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");

    Ok(())
}
