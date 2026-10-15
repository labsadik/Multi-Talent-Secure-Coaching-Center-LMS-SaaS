use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use futures_util::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLessonProgressRequest {
    pub status: String,
    pub position_seconds: Option<i64>,
}

fn parse_object_id(value: &str, message: &'static str) -> AppResult<ObjectId> {
    ObjectId::parse_str(value).map_err(|_| AppError::BadRequest(message))
}

async fn require_active_enrollment(
    state: &AppState,
    auth: &AuthUser,
    course_id: &ObjectId,
) -> AppResult<()> {
    let enrolled = state
        .db
        .collection::<Document>("enrollments")
        .find_one(doc! {
            "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(),
            "learnerId": auth.user_id.clone(),
            "status": "active",
        })
        .await?
        .is_some();

    if !enrolled {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub async fn my_courses(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Vec<Value>>> {
    if auth.role != "learner" {
        return Err(AppError::Forbidden);
    }

    let mut cursor = state
        .db
        .collection::<Document>("enrollments")
        .find(doc! {
            "companyId": auth.company_id.clone(),
            "learnerId": auth.user_id.clone(),
            "status": "active",
        })
        .sort(doc! { "enrolledAt": -1 })
        .limit(100)
        .await?;

    let enrollments: Vec<Document> = cursor.try_collect().await.map_err(AppError::from_mongo)?;
    let courses = state.db.collection::<Document>("courses");
    let mut output = Vec::with_capacity(enrollments.len());

    for enrollment in enrollments {
        let Some(course_id) = enrollment.get_object_id("courseId").ok().map(|id| id.to_owned())
        else {
            tracing::warn!("enrollment document has no valid courseId");
            continue;
        };

        let Some(course) = courses
            .find_one(doc! {
                "_id": course_id.clone(),
                "companyId": auth.company_id.clone(),
                "status": "published",
                "deletedAt": { "$exists": false },
            })
            .await?
        else {
            continue;
        };

        output.push(json!({
            "courseId": course_id.to_hex(),
            "title": course.get_str("title").unwrap_or_default(),
            "slug": course.get_str("slug").unwrap_or_default(),
            "description": course.get_str("description").ok(),
            "priceInPaise": course.get_i64("priceInPaise").unwrap_or(0),
            "currency": course.get_str("currency").unwrap_or("INR"),
            "enrolledAt": enrollment
                .get_datetime("enrolledAt")
                .map(|date| date.timestamp_millis())
                .unwrap_or_default(),
            "enrollmentSource": enrollment.get_str("source").unwrap_or("unknown"),
        }));
    }

    Ok(Json(output))
}

pub async fn update_lesson_progress(
    State(state): State<AppState>,
    Path(lesson_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<UpdateLessonProgressRequest>,
) -> AppResult<Json<Value>> {
    if auth.role != "learner" {
        return Err(AppError::Forbidden);
    }
    if !matches!(input.status.as_str(), "in_progress" | "completed") {
        return Err(AppError::BadRequest("status must be in_progress or completed"));
    }

    let lesson_id = parse_object_id(&lesson_id, "invalid lesson ID")?;
    let lesson = state
        .db
        .collection::<Document>("lessons")
        .find_one(doc! {
            "_id": lesson_id.clone(),
            "companyId": auth.company_id.clone(),
            "status": "active",
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;

    let course_id = lesson
        .get_object_id("courseId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let course = state
        .db
        .collection::<Document>("courses")
        .find_one(doc! {
            "_id": course_id.clone(),
            "companyId": auth.company_id.clone(),
            "status": "published",
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;
    let _ = course;
    require_active_enrollment(&state, &auth, &course_id).await?;

    let duration = lesson.get_i64("durationSeconds").unwrap_or(0);
    let position = input.position_seconds.unwrap_or(0);
    if !(0..=86_400).contains(&position) || (duration > 0 && position > duration) {
        return Err(AppError::BadRequest("positionSeconds is outside the lesson duration"));
    }

    let collection = state.db.collection::<Document>("lesson_progress");
    let existing = collection
        .find_one(doc! {
            "companyId": auth.company_id.clone(),
            "learnerId": auth.user_id.clone(),
            "lessonId": lesson_id.clone(),
        })
        .await?;

    let already_completed = existing
        .as_ref()
        .and_then(|document| document.get_str("status").ok())
        == Some("completed");
    let status = if already_completed || input.status == "completed" {
        "completed"
    } else {
        "in_progress"
    };

    let now = DateTime::now();
    let set_fields = doc! {
        "status": status,
        "positionSeconds": position,
        "lastViewedAt": now,
        "updatedAt": now,
    };
    if status == "completed" {
        set_fields.insert(
            "completedAt",
            existing
                .as_ref()
                .and_then(|document| document.get_datetime("completedAt").ok())
                .cloned()
                .unwrap_or(now),
        );
    }

    // Identity keys exist only on insert, avoiding conflicting update paths.
    let set_on_insert = doc! {
        "_id": ObjectId::new(),
        "companyId": auth.company_id.clone(),
        "courseId": course_id.clone(),
        "lessonId": lesson_id.clone(),
        "learnerId": auth.user_id.clone(),
        "createdAt": now,
    };

    // Keep the existing completion state monotonic; resuming a completed lesson never un-completes it.

    collection
        .update_one(
            doc! {
                "companyId": auth.company_id.clone(),
                "learnerId": auth.user_id.clone(),
                "lessonId": lesson_id.clone(),
            },
            doc! {
                "$set": set_fields,
                "$setOnInsert": set_on_insert,
            },
        )
        .upsert(true)
        .await
        .map_err(AppError::from_mongo)?;

    Ok(Json(json!({
        "lessonId": lesson_id.to_hex(),
        "courseId": course_id.to_hex(),
        "status": status,
        "positionSeconds": position,
        "completed": status == "completed",
    })))
}

pub async fn my_course_progress(
    State(state): State<AppState>,
    Path(course_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<Value>> {
    if auth.role != "learner" {
        return Err(AppError::Forbidden);
    }
    let course_id = parse_object_id(&course_id, "invalid course ID")?;
    let course = state
        .db
        .collection::<Document>("courses")
        .find_one(doc! {
            "_id": course_id.clone(),
            "companyId": auth.company_id.clone(),
            "status": "published",
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;
    require_active_enrollment(&state, &auth, &course_id).await?;

    let lessons_collection = state.db.collection::<Document>("lessons");
    let total_lessons = lessons_collection
        .count_documents(doc! {
            "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(),
            "status": "active",
            "deletedAt": { "$exists": false },
        })
        .await?;

    let completed_lessons = state
        .db
        .collection::<Document>("lesson_progress")
        .count_documents(doc! {
            "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(),
            "learnerId": auth.user_id.clone(),
            "status": "completed",
        })
        .await?;

    let progress_percent = if total_lessons == 0 {
        0.0
    } else {
        ((completed_lessons as f64 / total_lessons as f64) * 100.0).round()
    };

    Ok(Json(json!({
        "courseId": course_id.to_hex(),
        "courseTitle": course.get_str("title").unwrap_or_default(),
        "totalLessons": total_lessons,
        "completedLessons": completed_lessons,
        "progressPercent": progress_percent,
    })))
}

#[cfg(test)]
mod tests {
    use super::parse_object_id;

    #[test]
    fn malformed_object_ids_are_client_errors() {
        assert!(parse_object_id("not-an-id", "invalid lesson ID").is_err());
        assert!(parse_object_id("507f1f77bcf86cd799439011", "invalid lesson ID").is_ok());
    }
}
