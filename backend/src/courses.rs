use axum::{extract::State, http::StatusCode, Json};
use futures_util::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    auth::valid_slug,
    error::{AppError, AppResult},
    state::AppState,
    tenant::require_active_company,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCourseRequest {
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub price_in_paise: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseResponse {
    pub id: String,
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub price_in_paise: i64,
    pub currency: String,
    pub status: String,
    pub created_at_ms: i64,
}

pub async fn list_courses(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Vec<CourseResponse>>> {
    require_active_company(&state.db, &auth.company_id).await?;

    let cursor = state
        .db
        .collection::<Document>("courses")
        .find(doc! {
            "companyId": auth.company_id.clone(),
            "deletedAt": { "$exists": false },
        })
        .sort(doc! { "createdAt": -1 })
        .limit(100)
        .await
        .map_err(AppError::from_mongo)?;

    let documents: Vec<Document> = cursor
        .try_collect()
        .await
        .map_err(AppError::from_mongo)?;
    let courses = documents
        .into_iter()
        .map(course_response)
        .collect::<AppResult<Vec<_>>>()?;
    Ok(Json(courses))
}

pub async fn create_course(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateCourseRequest>,
) -> AppResult<(StatusCode, Json<CourseResponse>)> {
    if !auth.can_manage_content() {
        return Err(AppError::Forbidden);
    }
    require_active_company(&state.db, &auth.company_id).await?;

    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let description = input.description.map(|value| value.trim().to_owned());
    let price_in_paise = input.price_in_paise.unwrap_or(0);

    if !(3..=140).contains(&title.chars().count())
        || !valid_slug(&slug)
        || price_in_paise < 0
        || description
            .as_ref()
            .is_some_and(|value| value.chars().count() > 10_000)
    {
        return Err(AppError::BadRequest(
            "title, slug, description, or price does not meet validation rules",
        ));
    }

    let courses = state.db.collection::<Document>("courses");
    if courses
        .find_one(doc! {
            "companyId": auth.company_id.clone(),
            "slug": &slug,
        })
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("course slug is already in use for this company"));
    }

    let id = ObjectId::new();
    let now = DateTime::now();
    let document = doc! {
        "_id": id.clone(),
        "companyId": auth.company_id.clone(),
        "title": &title,
        "slug": &slug,
        "description": description.clone(),
        "priceInPaise": price_in_paise,
        "currency": "INR",
        "status": "draft",
        "createdBy": auth.user_id.clone(),
        "createdAt": now,
        "updatedAt": now,
    };
    courses
        .insert_one(document.clone())
        .await
        .map_err(AppError::from_mongo)?;

    Ok((StatusCode::CREATED, Json(course_response(document)?)))
}

fn course_response(document: Document) -> AppResult<CourseResponse> {
    let id = document
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_hex();
    let title = document
        .get_str("title")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let slug = document
        .get_str("slug")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let description = document.get_str("description").ok().map(str::to_owned);
    let currency = document.get_str("currency").unwrap_or("INR").to_owned();
    let status = document.get_str("status").unwrap_or("draft").to_owned();
    let price_in_paise = document.get_i64("priceInPaise").unwrap_or(0);
    let created_at_ms = document
        .get_datetime("createdAt")
        .map(|value| value.timestamp_millis())
        .unwrap_or_default();

    Ok(CourseResponse {
        id,
        title,
        slug,
        description,
        price_in_paise,
        currency,
        status,
        created_at_ms,
    })
}
