use axum::{
    http::StatusCode,
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use serde_json::{json, Value};

use crate::{auth, company_users, courses, health, learning, media, payments, platform_billing, progress, state::AppState};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health/live", get(health::live))
        .route("/health/ready", get(health::ready))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/register-learner", post(auth::register_learner))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/refresh", post(auth::refresh))
        .route("/api/v1/auth/logout", post(auth::logout))
        .route("/api/v1/auth/me", get(auth::me))
        .route("/api/v1/me/courses", get(progress::my_courses))
        .route(
            "/api/v1/company/users",
            get(company_users::list_company_users).post(company_users::create_company_user),
        )
        .route(
            "/api/v1/company/users/{user_id}",
            patch(company_users::update_company_user),
        )
        .route(
            "/api/v1/courses",
            get(courses::list_courses).post(courses::create_course),
        )
        .route(
            "/api/v1/courses/{course_id}",
            patch(courses::update_course).delete(courses::delete_course),
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
            "/api/v1/subjects/{subject_id}",
            patch(learning::update_subject).delete(learning::delete_subject),
        )
        .route(
            "/api/v1/subjects/{subject_id}/topics",
            get(learning::list_topics).post(learning::create_topic),
        )
        .route(
            "/api/v1/topics/{topic_id}",
            patch(learning::update_topic).delete(learning::delete_topic),
        )
        .route(
            "/api/v1/topics/{topic_id}/lessons",
            get(learning::list_lessons).post(learning::create_lesson),
        )
        .route(
            "/api/v1/lessons/{lesson_id}",
            patch(learning::update_lesson).delete(learning::delete_lesson),
        )
        .route(
            "/api/v1/topics/{topic_id}/video-upload-ticket",
            post(media::create_video_ticket),
        )
        .route(
            "/api/v1/topics/{topic_id}/assets",
            put(media::upload_topic_file),
        )
        .route(
            "/api/v1/assets/{asset_id}/refresh-status",
            post(media::refresh_video_status),
        )
        .route(
            "/api/v1/lessons/{lesson_id}/playback",
            get(media::lesson_playback),
        )
        .route(
            "/api/v1/lessons/{lesson_id}/progress",
            put(progress::update_lesson_progress),
        )
        .route(
            "/api/v1/courses/{course_id}/my-progress",
            get(progress::my_course_progress),
        )
        .route(
            "/api/v1/assets/{asset_id}/download",
            get(media::download_asset),
        )
        .route(
            "/api/v1/company/integrations/razorpay",
            get(payments::get_razorpay_settings).put(payments::configure_razorpay),
        )
        .route(
            "/api/v1/courses/{course_id}/orders",
            post(payments::create_course_order),
        )
        .route(
            "/api/v1/company/subscription/orders",
            post(platform_billing::create_subscription_order),
        )
        .route(
            "/api/v1/payments/verify",
            post(payments::verify_course_payment),
        )
        .route(
            "/api/v1/webhooks/razorpay",
            post(payments::razorpay_webhook),
        )
        .route(
            "/api/v1/webhooks/platform-razorpay",
            post(platform_billing::razorpay_webhook),
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
