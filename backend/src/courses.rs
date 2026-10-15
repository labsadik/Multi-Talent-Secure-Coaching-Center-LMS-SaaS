use axum::{extract::{Path, State}, http::StatusCode, Json};
use futures_util::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use serde::{Deserialize, Serialize};

use crate::{
    auth::AuthUser,
    auth::valid_slug,
    error::{AppError, AppResult},
    state::AppState,
    tenant::{limits_for_plan, require_active_company},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCourseRequest {
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub price_in_paise: Option<i64>,
    pub thumbnail_url: Option<String>,
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
    pub thumbnail_url: Option<String>,
    pub status: String,
    pub created_at_ms: i64,
}

pub async fn list_courses(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Vec<CourseResponse>>> {
    // Learners may discover only published courses; staff can also see drafts.
    let filter = if auth.can_manage_content() {
        doc! {
            "companyId": auth.company_id.clone(),
            "deletedAt": { "$exists": false },
        }
    } else {
        doc! {
            "companyId": auth.company_id.clone(),
            "status": "published",
            "deletedAt": { "$exists": false },
        }
    };

    let cursor = state
        .db
        .collection::<Document>("courses")
        .find(filter)
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
    let company = require_active_company(&state.db, &auth.company_id).await?;
    let limits = limits_for_plan(
        &state.settings,
        company.get_str("planCode").unwrap_or("free"),
    )?;
    let existing_courses = state
        .db
        .collection::<Document>("courses")
        .count_documents(doc! {
            "companyId": auth.company_id.clone(),
            "deletedAt": { "$exists": false },
        })
        .await?;
    if existing_courses >= limits.max_courses {
        return Err(AppError::Conflict("course limit reached for the current plan"));
    }
    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let description = input.description.map(|value| value.trim().to_owned());
    let price_in_paise = input.price_in_paise.unwrap_or(0);
    let thumbnail_url = input.thumbnail_url.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(str::to_owned);

    if !(3..=140).contains(&title.chars().count())
        || !valid_slug(&slug)
        || !(0..=100_000_000).contains(&price_in_paise)
        || description
            .as_ref()
            .is_some_and(|value| value.chars().count() > 10_000)
        || thumbnail_url.as_ref().is_some_and(|url| !url.starts_with("https://") || url.len() > 2048)
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
        "thumbnailUrl": thumbnail_url,
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
    let thumbnail_url = document.get_str("thumbnailUrl").ok().map(str::to_owned);
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
        thumbnail_url,
        status,
        created_at_ms,
    })
}



pub async fn update_course(
    State(state): State<AppState>,
    Path(course_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateCourseRequest>,
) -> AppResult<Json<CourseResponse>> {
    if !auth.can_manage_content() {
        return Err(AppError::Forbidden);
    }
    let course_id = ObjectId::parse_str(&course_id)
        .map_err(|_| AppError::BadRequest("invalid course ID"))?;
    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let description = input.description.map(|value| value.trim().to_owned());
    let price_in_paise = input.price_in_paise.unwrap_or(0);
    let thumbnail_url = input.thumbnail_url.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(str::to_owned);
    if !(3..=140).contains(&title.chars().count())
        || !valid_slug(&slug)
        || !(0..=100_000_000).contains(&price_in_paise)
        || description.as_ref().is_some_and(|value| value.chars().count() > 10_000)
        || thumbnail_url.as_ref().is_some_and(|url| !url.starts_with("https://") || url.len() > 2048)
    {
        return Err(AppError::BadRequest(
            "title, slug, description, price, or thumbnail URL does not meet validation rules",
        ));
    }

    let courses = state.db.collection::<Document>("courses");
    let filter = doc! {
        "_id": course_id.clone(),
        "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    if courses.find_one(filter.clone()).await?.is_none() {
        return Err(AppError::NotFound);
    }
    if courses.find_one(doc! {
        "companyId": auth.company_id.clone(),
        "slug": &slug,
        "_id": { "$ne": course_id.clone() },
        "deletedAt": { "$exists": false },
    }).await?.is_some() {
        return Err(AppError::Conflict("course slug is already in use for this company"));
    }

    courses.update_one(
        filter.clone(),
        doc! { "$set": {
            "title": &title,
            "slug": &slug,
            "description": description.clone(),
            "priceInPaise": price_in_paise,
            "thumbnailUrl": thumbnail_url,
            "updatedAt": DateTime::now(),
        } },
    ).await.map_err(AppError::from_mongo)?;
    let document = courses.find_one(filter).await?.ok_or(AppError::NotFound)?;
    Ok(Json(course_response(document)?))
}

pub async fn delete_course(
    State(state): State<AppState>,
    Path(course_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<CourseResponse>> {
    if !auth.can_manage_content() {
        return Err(AppError::Forbidden);
    }
    let course_id = ObjectId::parse_str(&course_id)
        .map_err(|_| AppError::BadRequest("invalid course ID"))?;
    let courses = state.db.collection::<Document>("courses");
    let filter = doc! {
        "_id": course_id.clone(),
        "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    let now = DateTime::now();
    let result = courses.update_one(
        filter,
        doc! { "$set": {
            "status": "archived",
            "deletedAt": now,
            "updatedAt": now,
        } },
    ).await?;
    if result.matched_count != 1 {
        return Err(AppError::NotFound);
    }

    // Keep enrolment and payment history while hiding the archived course hierarchy.
    for collection in ["subjects", "topics", "lessons", "assets"] {
        state.db.collection::<Document>(collection).update_many(
            doc! {
                "companyId": auth.company_id.clone(),
                "courseId": course_id.clone(),
                "deletedAt": { "$exists": false },
            },
            doc! { "$set": {
                "status": "archived",
                "deletedAt": now,
                "updatedAt": now,
            } },
        ).await?;
    }
    let document = courses.find_one(doc! {
        "_id": course_id,
        "companyId": auth.company_id.clone(),
    }).await?.ok_or(AppError::NotFound)?;
    Ok(Json(course_response(document)?))
}

pub async fn publish_course(
    State(state): State<AppState>,
    Path(course_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<CourseResponse>> {
    if !auth.can_manage_content() {
        return Err(AppError::Forbidden);
    }
    let course_id = ObjectId::parse_str(&course_id)
        .map_err(|_| AppError::BadRequest("invalid course ID"))?;
    let now = DateTime::now();
    let result = state
        .db
        .collection::<Document>("courses")
        .update_one(
            doc! {
                "_id": course_id.clone(),
                "companyId": auth.company_id.clone(),
                "deletedAt": { "$exists": false },
            },
            doc! { "$set": { "status": "published", "updatedAt": now } },
        )
        .await?;

    if result.matched_count != 1 {
        return Err(AppError::NotFound);
    }
    let document = state
        .db
        .collection::<Document>("courses")
        .find_one(doc! {
            "_id": course_id,
            "companyId": auth.company_id.clone(),
        })
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(course_response(document)?))
}
