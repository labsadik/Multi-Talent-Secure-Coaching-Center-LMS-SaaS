use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use futures_util::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{valid_slug, AuthUser},
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubjectRequest {
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTopicRequest {
    pub title: String,
    pub slug: String,
    pub description: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateLessonRequest {
    pub title: String,
    pub slug: String,
    pub lesson_type: String,
    pub content: Option<String>,
    pub media_asset_id: Option<String>,
    pub duration_seconds: Option<i64>,
    pub is_preview: Option<bool>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentResponse {
    id: String,
    title: String,
    slug: String,
    description: Option<String>,
    status: String,
    sort_order: i32,
    created_at_ms: i64,
    course_id: Option<String>,
    subject_id: Option<String>,
    topic_id: Option<String>,
    lesson_type: Option<String>,
    content: Option<String>,
    media_asset_id: Option<String>,
    duration_seconds: Option<i64>,
    is_preview: Option<bool>,
}

fn parse_id(value: &str) -> AppResult<ObjectId> {
    ObjectId::parse_str(value).map_err(|_| AppError::BadRequest("invalid resource ID"))
}

fn valid_title_slug(title: &str, slug: &str) -> bool {
    (2..=140).contains(&title.trim().chars().count()) && valid_slug(slug)
}

fn validate_optional_fields(description: &Option<String>, sort_order: i32) -> bool {
    sort_order >= 0
        && sort_order <= 1_000_000
        && description
            .as_ref()
            .is_none_or(|value| value.chars().count() <= 10_000)
}

async fn visible_course(
    state: &AppState,
    company_id: &ObjectId,
    course_id: &ObjectId,
    can_manage: bool,
) -> AppResult<Document> {
    let course = state
        .db
        .collection::<Document>("courses")
        .find_one(doc! {
            "_id": course_id.clone(),
            "companyId": company_id.clone(),
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;

    if !can_manage && course.get_str("status").unwrap_or_default() != "published" {
        return Err(AppError::NotFound);
    }

    Ok(course)
}

fn content_response(document: Document) -> AppResult<ContentResponse> {
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
    let status = document.get_str("status").unwrap_or("active").to_owned();
    let sort_order = document.get_i32("sortOrder").unwrap_or(0);
    let created_at_ms = document
        .get_datetime("createdAt")
        .map(|value| value.timestamp_millis())
        .unwrap_or_default();

    let id_field = |key: &str| {
        document
            .get_object_id(key)
            .ok()
            .map(|value| value.to_hex())
    };

    Ok(ContentResponse {
        id,
        title,
        slug,
        description,
        status,
        sort_order,
        created_at_ms,
        course_id: id_field("courseId"),
        subject_id: id_field("subjectId"),
        topic_id: id_field("topicId"),
        lesson_type: document.get_str("lessonType").ok().map(str::to_owned),
        content: document.get_str("content").ok().map(str::to_owned),
        media_asset_id: id_field("mediaAssetId"),
        duration_seconds: document.get_i64("durationSeconds").ok(),
        is_preview: document.get_bool("isPreview").ok(),
    })
}

async fn load_list(
    collection: &mongodb::Collection<Document>,
    filter: Document,
) -> AppResult<Vec<ContentResponse>> {
    let cursor = collection
        .find(filter)
        .sort(doc! { "sortOrder": 1, "createdAt": 1 })
        .limit(500)
        .await?;

    let documents: Vec<Document> = cursor.try_collect().await.map_err(AppError::from_mongo)?;
    documents.into_iter().map(content_response).collect()
}

pub async fn list_subjects(
    State(state): State<AppState>,
    Path(course_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<Vec<ContentResponse>>> {
    let course_id = parse_id(&course_id)?;
    visible_course(&state, &auth.company_id, &course_id, auth.can_manage_content()).await?;

    let items = load_list(
        &state.db.collection::<Document>("subjects"),
        doc! {
            "companyId": auth.company_id.clone(),
            "courseId": course_id,
            "status": "active",
            "deletedAt": { "$exists": false },
        },
    )
    .await?;
    Ok(Json(items))
}

pub async fn create_subject(
    State(state): State<AppState>,
    Path(course_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateSubjectRequest>,
) -> AppResult<(StatusCode, Json<ContentResponse>)> {
    if !auth.can_manage_content() {
        return Err(AppError::Forbidden);
    }
    let course_id = parse_id(&course_id)?;
    visible_course(&state, &auth.company_id, &course_id, true).await?;

    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let description = input.description.map(|value| value.trim().to_owned());
    let sort_order = input.sort_order.unwrap_or(0);
    if !valid_title_slug(&title, &slug) || !validate_optional_fields(&description, sort_order) {
        return Err(AppError::BadRequest("invalid subject title, slug, description, or order"));
    }

    let id = ObjectId::new();
    let now = DateTime::now();
    let document = doc! {
        "_id": id,
        "companyId": auth.company_id.clone(),
        "courseId": course_id,
        "title": title,
        "slug": slug,
        "description": description,
        "status": "active",
        "sortOrder": sort_order,
        "createdBy": auth.user_id.clone(),
        "createdAt": now,
        "updatedAt": now,
    };
    state
        .db
        .collection::<Document>("subjects")
        .insert_one(document.clone())
        .await
        .map_err(AppError::from_mongo)?;

    Ok((StatusCode::CREATED, Json(content_response(document)?)))
}

pub async fn list_topics(
    State(state): State<AppState>,
    Path(subject_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<Vec<ContentResponse>>> {
    let subject_id = parse_id(&subject_id)?;
    let subject = state
        .db
        .collection::<Document>("subjects")
        .find_one(doc! {
            "_id": subject_id.clone(),
            "companyId": auth.company_id.clone(),
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;
    let course_id = subject
        .get_object_id("courseId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    visible_course(&state, &auth.company_id, &course_id, auth.can_manage_content()).await?;

    let items = load_list(
        &state.db.collection::<Document>("topics"),
        doc! {
            "companyId": auth.company_id.clone(),
            "subjectId": subject_id,
            "status": "active",
            "deletedAt": { "$exists": false },
        },
    )
    .await?;
    Ok(Json(items))
}

pub async fn create_topic(
    State(state): State<AppState>,
    Path(subject_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateTopicRequest>,
) -> AppResult<(StatusCode, Json<ContentResponse>)> {
    if !auth.can_manage_content() {
        return Err(AppError::Forbidden);
    }
    let subject_id = parse_id(&subject_id)?;
    let subject = state
        .db
        .collection::<Document>("subjects")
        .find_one(doc! {
            "_id": subject_id.clone(),
            "companyId": auth.company_id.clone(),
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;
    let course_id = subject
        .get_object_id("courseId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;

    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let description = input.description.map(|value| value.trim().to_owned());
    let sort_order = input.sort_order.unwrap_or(0);
    if !valid_title_slug(&title, &slug) || !validate_optional_fields(&description, sort_order) {
        return Err(AppError::BadRequest("invalid topic title, slug, description, or order"));
    }

    let document = doc! {
        "_id": ObjectId::new(),
        "companyId": auth.company_id.clone(),
        "courseId": course_id,
        "subjectId": subject_id,
        "title": title,
        "slug": slug,
        "description": description,
        "status": "active",
        "sortOrder": sort_order,
        "createdBy": auth.user_id.clone(),
        "createdAt": DateTime::now(),
        "updatedAt": DateTime::now(),
    };
    state
        .db
        .collection::<Document>("topics")
        .insert_one(document.clone())
        .await
        .map_err(AppError::from_mongo)?;

    Ok((StatusCode::CREATED, Json(content_response(document)?)))
}

pub async fn list_lessons(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<Vec<ContentResponse>>> {
    let topic_id = parse_id(&topic_id)?;
    let topic = state
        .db
        .collection::<Document>("topics")
        .find_one(doc! {
            "_id": topic_id.clone(),
            "companyId": auth.company_id.clone(),
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;
    let course_id = topic
        .get_object_id("courseId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let _course =
        visible_course(&state, &auth.company_id, &course_id, auth.can_manage_content()).await?;

    let mut filter = doc! {
        "companyId": auth.company_id.clone(),
        "topicId": topic_id,
        "status": "active",
        "deletedAt": { "$exists": false },
    };

    // Published content is discoverable without enrollment, but the lesson body/media is not.
    // Non-enrolled learners can retrieve only lessons explicitly marked as previews.
    if !auth.can_manage_content() {
        let enrolled = state
            .db
            .collection::<Document>("enrollments")
            .find_one(doc! {
                "companyId": auth.company_id.clone(),
                "courseId": course_id,
                "learnerId": auth.user_id.clone(),
                "status": "active",
            })
            .await?
            .is_some();

        if !enrolled {
            filter.insert("isPreview", true);
        }
    }

    let items = load_list(&state.db.collection::<Document>("lessons"), filter).await?;
    Ok(Json(items))
}

pub async fn create_lesson(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateLessonRequest>,
) -> AppResult<(StatusCode, Json<ContentResponse>)> {
    if !auth.can_manage_content() {
        return Err(AppError::Forbidden);
    }
    let topic_id = parse_id(&topic_id)?;
    let topic = state
        .db
        .collection::<Document>("topics")
        .find_one(doc! {
            "_id": topic_id.clone(),
            "companyId": auth.company_id.clone(),
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;
    let course_id = topic
        .get_object_id("courseId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let subject_id = topic
        .get_object_id("subjectId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;

    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let lesson_type = input.lesson_type.trim().to_ascii_lowercase();
    let content = input.content.map(|value| value.trim().to_owned());
    let duration_seconds = input.duration_seconds.unwrap_or(0);
    let sort_order = input.sort_order.unwrap_or(0);
    let media_asset_id = input.media_asset_id.as_deref().map(parse_id).transpose()?;

    if !valid_title_slug(&title, &slug)
        || !matches!(lesson_type.as_str(), "video" | "text" | "file")
        || !(0..=86_400).contains(&duration_seconds)
        || !(0..=1_000_000).contains(&sort_order)
        || content.as_ref().is_some_and(|value| value.chars().count() > 50_000)
    {
        return Err(AppError::BadRequest("invalid lesson title, slug, type, content, duration, or order"));
    }

    match lesson_type.as_str() {
        "text" if content.as_ref().is_none_or(|value| value.is_empty()) => {
            return Err(AppError::BadRequest("text lessons require content"));
        }
        "text" if media_asset_id.is_some() => {
            return Err(AppError::BadRequest("text lessons cannot attach a media asset"));
        }
        "video" | "file" if media_asset_id.is_none() => {
            return Err(AppError::BadRequest("video and file lessons require a media asset"));
        }
        _ => {}
    }

    if let Some(asset_id) = &media_asset_id {
        let asset_type = if lesson_type == "video" { "video" } else { "file" };
        let asset = state
            .db
            .collection::<Document>("assets")
            .find_one(doc! {
                "_id": asset_id.clone(),
                "companyId": auth.company_id.clone(),
                "courseId": course_id.clone(),
                "subjectId": subject_id.clone(),
                "topicId": topic_id.clone(),
                "assetType": asset_type,
                "status": "ready",
                "deletedAt": { "$exists": false },
            })
            .await?
            .ok_or(AppError::BadRequest("media asset does not belong to this topic or is not ready"))?;
        let _ = asset;
    }

    let document = doc! {
        "_id": ObjectId::new(),
        "companyId": auth.company_id.clone(),
        "courseId": course_id,
        "subjectId": subject_id,
        "topicId": topic_id,
        "title": title,
        "slug": slug,
        "lessonType": lesson_type,
        "content": content,
        "mediaAssetId": media_asset_id,
        "durationSeconds": duration_seconds,
        "isPreview": input.is_preview.unwrap_or(false),
        "status": "active",
        "sortOrder": sort_order,
        "createdBy": auth.user_id.clone(),
        "createdAt": DateTime::now(),
        "updatedAt": DateTime::now(),
    };
    state
        .db
        .collection::<Document>("lessons")
        .insert_one(document.clone())
        .await
        .map_err(AppError::from_mongo)?;

    Ok((StatusCode::CREATED, Json(content_response(document)?)))
}

