use axum::{extract::State, http::StatusCode, Json};
use mongodb::bson::doc;
use serde_json::{json, Value};

use crate::state::AppState;

pub async fn live() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({ "status": "live" })))
}

pub async fn ready(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match state.db.run_command(doc! { "ping": 1 }).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ready", "database": "ok" }))),
        Err(error) => {
            tracing::error!(error = ?error, "readiness check failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "status": "not_ready", "database": "unavailable" })),
            )
        }
    }
}
