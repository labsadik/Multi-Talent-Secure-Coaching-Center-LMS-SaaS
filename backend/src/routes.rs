use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};

use crate::{auth, courses, health, learning, state::AppState};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/me", get(auth::me))
        .route(
            "/api/v1/courses",
            get(courses::list_courses).post(courses::create_course),
        )
        .route(
            "/api/v1/courses/{course_id}/publish",
            post(courses::publish_course),
        )
        .route(
            "/api/v1/courses/{course_id}/subjects",
            get(learning::list_subjects).post(learning::create_subject),
        )
        .route(
            "/api/v1/subjects/{subject_id}/topics",
            get(learning::list_topics).post(learning::create_topic),
        )
        .route(
            "/api/v1/topics/{topic_id}/lessons",
            get(learning::list_lessons).post(learning::create_lesson),
        )
        .fallback(not_found)
        .with_state(state)
}

async fn root() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({
            "service": "newage-api",
            "version": env!("CARGO_PKG_VERSION"),
            "status": "running",
        })),
    )
}

async fn not_found() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": {
                "code": "not_found",
                "message": "Route was not found"
            }
        })),
    )
}
