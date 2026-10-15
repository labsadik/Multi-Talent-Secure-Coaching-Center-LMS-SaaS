use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use futures_util::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    auth::{valid_slug, AuthUser},
    error::{AppError, AppResult},
    integrations::{bunny_storage::BunnyStorage, bunny_stream::BunnyStream},
    state::AppState,
    tenant::require_active_company,
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
    pub chapter_id: Option<String>,
    pub chapter_part_id: Option<String>,
    pub thumbnail_url: Option<String>,
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
    chapter_id: Option<String>,
    chapter_part_id: Option<String>,
    thumbnail_url: Option<String>,
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
        chapter_id: id_field("chapterId"),
        chapter_part_id: id_field("chapterPartId"),
        thumbnail_url: document.get_str("thumbnailUrl").ok().map(str::to_owned),
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

async fn add_lesson_thumbnails(
    state: &AppState,
    company_id: &ObjectId,
    lessons: &mut [ContentResponse],
) -> AppResult<()> {
    let asset_ids: Vec<ObjectId> = lessons
        .iter()
        .filter(|lesson| lesson.thumbnail_url.is_none())
        .filter_map(|lesson| lesson.media_asset_id.as_deref())
        .filter_map(|id| ObjectId::parse_str(id).ok())
        .collect();
    if asset_ids.is_empty() {
        return Ok(());
    }

    let cursor = state.db.collection::<Document>("assets")
        .find(doc! {
            "_id": { "$in": asset_ids },
            "companyId": company_id.clone(),
            "status": "ready",
            "deletedAt": { "$exists": false },
        })
        .await?;
    let assets: Vec<Document> = cursor.try_collect().await.map_err(AppError::from_mongo)?;
    let library_id = state.settings.bunny_stream_library_id;
    let token_key = state.settings.bunny_stream_token_auth_key.as_deref()
        .filter(|key| !key.is_empty());
    let assets_by_id: std::collections::HashMap<String, Document> = assets.into_iter()
        .filter_map(|asset| asset.get_object_id("_id").ok().map(|id| (id.to_hex(), asset)))
        .collect();

    for lesson in lessons.iter_mut() {
        if lesson.thumbnail_url.is_some() {
            continue;
        }
        let Some(asset_id) = lesson.media_asset_id.as_deref() else { continue };
        let Some(asset) = assets_by_id.get(asset_id) else { continue };
        match asset.get_str("provider").unwrap_or_default() {
            "bunny_storage" if asset.get_str("contentType").unwrap_or_default().starts_with("image/") => {
                lesson.thumbnail_url = Some(format!("/api/v1/assets/{}/download", asset_id));
            }
            "bunny_stream" if lesson.lesson_type.as_deref() == Some("video") => {
                let Some(library_id) = library_id else { continue };
                let Some(video_id) = asset.get_str("providerVideoId").ok() else { continue };
                if video_id.is_empty() || !video_id.chars().all(|ch| ch.is_ascii_alphanumeric() || "-_".contains(ch)) {
                    continue;
                }
                let raw_name = asset.get_str("thumbnailFileName").unwrap_or("thumbnail.jpg");
                let filename = if !raw_name.is_empty()
                    && raw_name.len() <= 128
                    && raw_name.chars().all(|ch| ch.is_ascii_alphanumeric() || "._-".contains(ch))
                {
                    raw_name
                } else {
                    "thumbnail.jpg"
                };
                let mut url = format!("https://vz-{library_id}.b-cdn.net/{video_id}/{filename}");
                if let Some(token_key) = token_key {
                    let now = SystemTime::now().duration_since(UNIX_EPOCH)
                        .map_err(|_| AppError::Internal)?.as_secs();
                    let expires = now.saturating_add(state.settings.bunny_playback_token_ttl_seconds);
                    let mut digest = Sha256::new();
                    digest.update(token_key.as_bytes());
                    digest.update(video_id.as_bytes());
                    digest.update(expires.to_string().as_bytes());
                    let token = hex::encode(digest.finalize());
                    url.push_str(&format!("?token={token}&expires={expires}"));
                }
                lesson.thumbnail_url = Some(url);
            }
            _ => {}
        }
    }
    Ok(())
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

    let subjects = state.db.collection::<Document>("subjects");
    if subjects
        .find_one(doc! {
            "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(),
            "slug": &slug,
            "deletedAt": { "$exists": false },
        })
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("subject slug is already in use in this course"));
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
    subjects
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

    let mut items = load_list(&state.db.collection::<Document>("lessons"), filter).await?;
    add_lesson_thumbnails(&state, &auth.company_id, &mut items).await?;
    Ok(Json(items))
}

async fn organize_simple_lesson_asset(
    state: &AppState,
    company_id: &ObjectId,
    company: &Document,
    course: &Document,
    subject: &Document,
    topic: &Document,
    asset: &Document,
    lesson_id: &ObjectId,
    lesson_title: &str,
    lesson_slug: &str,
    update_stream_metadata: bool,
) -> AppResult<(Option<String>, Option<String>)> {
    let company_slug = company.get_str("slug").map_err(|_| AppError::Internal)?;
    let course_slug = course.get_str("slug").map_err(|_| AppError::Internal)?;
    let subject_slug = subject.get_str("slug").map_err(|_| AppError::Internal)?;
    let topic_slug = topic.get_str("slug").map_err(|_| AppError::Internal)?;
    let lesson_id_hex = lesson_id.to_hex();

    match asset.get_str("provider").unwrap_or_default() {
        "bunny_storage" => {
            let old_path = asset.get_str("storagePath").map(str::to_owned).map_err(|_| AppError::Internal)?;
            let filename = old_path.rsplit('/').next().filter(|part| !part.is_empty()).ok_or(AppError::Internal)?;
            let target_path = BunnyStorage::simple_lesson_content_path(
                &company_id.to_hex(), company_slug,
                &course.get_object_id("_id").map_err(|_| AppError::Internal)?.to_hex(), course_slug,
                &subject.get_object_id("_id").map_err(|_| AppError::Internal)?.to_hex(), subject_slug,
                &topic.get_object_id("_id").map_err(|_| AppError::Internal)?.to_hex(), topic_slug,
                &lesson_id_hex, lesson_slug, "assets", filename,
            )?;
            if old_path != target_path {
                let content_type = asset.get_str("contentType").map(str::to_owned).map_err(|_| AppError::Internal)?;
                let storage = BunnyStorage::from_settings(&state.settings, state.http.clone())?;
                let bytes = storage.download_bytes(&old_path, state.settings.max_asset_upload_bytes).await?;
                storage.upload_bytes(&target_path, &content_type, bytes).await?;
            }
            Ok((Some(target_path), None))
        }
        "bunny_stream" => {
            let video_id = asset.get_str("providerVideoId").map(str::to_owned).map_err(|_| AppError::Internal)?;
            let stream = BunnyStream::from_settings(&state.settings, state.http.clone())?;
            let already_attached = asset.get_object_id("lessonId").ok().is_some_and(|id| id == *lesson_id);
            let collection_id = if already_attached {
                asset.get_str("bunnyCollectionId").ok().filter(|value| !value.is_empty()).map(str::to_owned)
            } else {
                None
            };
            let collection_id = match collection_id {
                Some(existing) => existing,
                None => {
                    let collection_name = format!(
                        "c{}-r{}-s{}-h{}-l{}",
                        company_id.to_hex(),
                        course.get_object_id("_id").map_err(|_| AppError::Internal)?.to_hex(),
                        subject.get_object_id("_id").map_err(|_| AppError::Internal)?.to_hex(),
                        topic.get_object_id("_id").map_err(|_| AppError::Internal)?.to_hex(),
                        lesson_id_hex,
                    );
                    stream.create_collection(&collection_name).await?
                }
            };
            // Use the lesson title itself for Bunny's video title; the collection carries the hierarchy.
            if update_stream_metadata {
                stream.update_video(&video_id, lesson_title, &collection_id).await?;
            }
            Ok((None, Some(collection_id)))
        }
        _ => Ok((None, None)),
    }
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
    let course = visible_course(&state, &auth.company_id, &course_id, true).await?;
    let company = require_active_company(&state.db, &auth.company_id).await?;
    let subject = state.db.collection::<Document>("subjects").find_one(doc! {
        "_id": subject_id.clone(),
        "companyId": auth.company_id.clone(),
        "courseId": course_id.clone(),
        "deletedAt": { "$exists": false },
    }).await?.ok_or(AppError::NotFound)?;

    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let lesson_type = input.lesson_type.trim().to_ascii_lowercase();
    let content = input.content.map(|value| value.trim().to_owned());
    let duration_seconds = input.duration_seconds.unwrap_or(0);
    let sort_order = input.sort_order.unwrap_or(0);
    let media_asset_id = input.media_asset_id.as_deref().map(parse_id).transpose()?;
    let chapter_id = input.chapter_id.as_deref().map(parse_id).transpose()?;
    let chapter_part_id = input.chapter_part_id.as_deref().map(parse_id).transpose()?;
    let thumbnail_url = input.thumbnail_url.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(str::to_owned);
    if thumbnail_url.as_ref().is_some_and(|url| !url.starts_with("https://") || url.len() > 2048) {
        return Err(AppError::BadRequest("lesson thumbnail must be an HTTPS URL"));
    }
    if chapter_id.is_some() != chapter_part_id.is_some() {
        return Err(AppError::BadRequest("chapter and chapter-part IDs must be supplied together"));
    }
    let mut chapter_document: Option<Document> = None;
    let mut chapter_part_document: Option<Document> = None;
    if let (Some(chapter_id), Some(chapter_part_id)) = (&chapter_id, &chapter_part_id) {
        let chapter = state.db.collection::<Document>("chapters").find_one(doc! {
            "_id": chapter_id.clone(), "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(), "subjectId": subject_id.clone(),
            "topicId": topic_id.clone(), "deletedAt": { "$exists": false },
        }).await?.ok_or(AppError::BadRequest("chapter does not belong to this course part"))?;
        let chapter_part = state.db.collection::<Document>("chapter_parts").find_one(doc! {
            "_id": chapter_part_id.clone(), "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(), "subjectId": subject_id.clone(),
            "topicId": topic_id.clone(), "chapterId": chapter_id.clone(),
            "deletedAt": { "$exists": false },
        }).await?.ok_or(AppError::BadRequest("chapter part does not belong to this chapter"))?;
        chapter_document = Some(chapter);
        chapter_part_document = Some(chapter_part);
    }

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

    let mut asset_for_lesson: Option<Document> = None;
    if let Some(asset_id) = &media_asset_id {
        // PDF files and images are both valid file lesson attachments. Video lessons
        // may only reference a ready Bunny Stream video asset.
        let asset_type_filter = if lesson_type == "video" {
            doc! { "assetType": "video" }
        } else {
            doc! { "assetType": { "$in": ["file", "image"] } }
        };
        let asset_status_filter = if lesson_type == "video" {
            doc! { "status": { "$in": ["ready", "uploading", "processing"] } }
        } else {
            doc! { "status": "ready" }
        };
        let mut asset_filter = doc! {
            "_id": asset_id.clone(),
            "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(),
            "subjectId": subject_id.clone(),
            "topicId": topic_id.clone(),
            "deletedAt": { "$exists": false },
        };
        asset_filter.extend(asset_type_filter);
        asset_filter.extend(asset_status_filter);

        let asset = state
            .db
            .collection::<Document>("assets")
            .find_one(asset_filter)
            .await?
            .ok_or(AppError::BadRequest("media asset does not belong to this topic or is not ready for attachment"))?;
        if let Some(asset_part_id) = asset.get_object_id("chapterPartId").ok() {
            if chapter_part_id.as_ref() != Some(&asset_part_id) {
                return Err(AppError::BadRequest("media asset was uploaded for a different chapter part"));
            }
        }
        if asset.get_object_id("lessonId").is_ok() {
            return Err(AppError::BadRequest("this asset is already attached to a lesson; upload a separate asset for each lesson"));
        }
        asset_for_lesson = Some(asset);
    }

    let lesson_id = ObjectId::new();
    let document = doc! {
        "_id": lesson_id.clone(),
        "companyId": auth.company_id.clone(),
        "courseId": course_id.clone(),
        "subjectId": subject_id.clone(),
        "topicId": topic_id.clone(),
        "chapterId": chapter_id.clone(),
        "chapterPartId": chapter_part_id.clone(),
        "thumbnailUrl": thumbnail_url,
        "title": title.clone(),
        "slug": slug.clone(),
        "lessonType": lesson_type,
        "content": content,
        "mediaAssetId": media_asset_id.clone(),
        "durationSeconds": duration_seconds,
        "isPreview": input.is_preview.unwrap_or(false),
        "status": "active",
        "sortOrder": sort_order,
        "createdBy": auth.user_id.clone(),
        "createdAt": DateTime::now(),
        "updatedAt": DateTime::now(),
    };
    // When curriculum context is available, put storage files under the lesson's
    // own folder and move Stream videos to a lesson-specific collection. File uploads
    // are copied first; the previous path remains valid until MongoDB points to the new
    // path, avoiding a broken asset if any provider call fails.
    let mut pending_storage_move: Option<(String, String)> = None;
    let mut lesson_stream_collection: Option<String> = None;
    let mut stream_title: Option<String> = None;
    if let (Some(asset), Some(chapter), Some(chapter_part)) = (
        asset_for_lesson.as_ref(),
        chapter_document.as_ref(),
        chapter_part_document.as_ref(),
    ) {
        let company_slug = company.get_str("slug").unwrap_or_default();
        let course_slug = course.get_str("slug").unwrap_or_default();
        let subject_slug = subject.get_str("slug").unwrap_or_default();
        let topic_slug = topic.get_str("slug").unwrap_or_default();
        let chapter_slug = chapter.get_str("slug").unwrap_or_default();
        let chapter_part_slug = chapter_part.get_str("slug").unwrap_or_default();
        let chapter_id_hex = chapter.get_object_id("_id").map_err(|_| AppError::Internal)?.to_hex();
        let chapter_part_id_hex = chapter_part.get_object_id("_id").map_err(|_| AppError::Internal)?.to_hex();
        let lesson_id_hex = lesson_id.to_hex();
        let lesson_folder = (&lesson_id_hex[..], &slug[..]);

        match asset.get_str("provider").unwrap_or_default() {
            "bunny_storage" => {
                let old_path = asset.get_str("storagePath").map(str::to_owned).map_err(|_| AppError::Internal)?;
                let filename = old_path.rsplit('/').next().filter(|part| !part.is_empty()).ok_or(AppError::Internal)?;
                let target_path = BunnyStorage::course_content_path(
                    &auth.company_id.to_hex(),
                    company_slug,
                    &course_id.to_hex(),
                    course_slug,
                    &subject_id.to_hex(),
                    subject_slug,
                    &topic_id.to_hex(),
                    topic_slug,
                    Some((&chapter_id_hex, chapter_slug)),
                    Some((&chapter_part_id_hex, chapter_part_slug)),
                    Some(lesson_folder),
                    "assets",
                    filename,
                )?;
                let content_type = asset.get_str("contentType").map(str::to_owned).map_err(|_| AppError::Internal)?;
                let storage = BunnyStorage::from_settings(&state.settings, state.http.clone())?;
                let bytes = storage.download_bytes(&old_path, state.settings.max_asset_upload_bytes).await?;
                storage.upload_bytes(&target_path, &content_type, bytes).await?;
                pending_storage_move = Some((old_path, target_path));
            }
            "bunny_stream" => {
                let collection_name = format!(
                    "c{}-r{}-s{}-h{}-p{}-l{}",
                    auth.company_id.to_hex(),
                    course_id.to_hex(),
                    subject_id.to_hex(),
                    chapter_id_hex,
                    chapter_part_id_hex,
                    lesson_id_hex,
                );
                let stream = BunnyStream::from_settings(&state.settings, state.http.clone())?;
                let collection_id = stream.create_collection(&collection_name).await?;
                let path_title = format!(
                    "{}/{}/{}/{}/{}/{}/{}",
                    company_slug, course_slug, subject_slug, topic_slug,
                    chapter_slug, chapter_part_slug, title
                );
                stream_title = Some(path_title.chars().take(160).collect());
                lesson_stream_collection = Some(collection_id);
            }
            _ => {}
        }
    }

    if chapter_id.is_none() && chapter_part_id.is_none() {
        if let Some(asset) = asset_for_lesson.as_ref() {
            let (storage_path, collection_id) = organize_simple_lesson_asset(
                &state, &auth.company_id, &company, &course, &subject, &topic,
                asset, &lesson_id, &title, &slug, false,
            ).await?;
            pending_storage_move = storage_path.map(|target| (
                asset.get_str("storagePath").unwrap_or_default().to_owned(),
                target,
            ));
            lesson_stream_collection = collection_id;
        }
    }

    state
        .db
        .collection::<Document>("lessons")
        .insert_one(document.clone())
        .await
        .map_err(AppError::from_mongo)?;

    if let Some(asset) = asset_for_lesson.as_ref() {
        let assets = state.db.collection::<Document>("assets");
        let mut set_fields = doc! {
            "lessonId": lesson_id.clone(),
            "chapterId": chapter_id.clone(),
            "chapterPartId": chapter_part_id.clone(),
            "updatedAt": DateTime::now(),
        };
        if let Some((_, target_path)) = pending_storage_move.as_ref() {
            set_fields.insert("storagePath", target_path.clone());
        }
        if let Some(collection_id) = lesson_stream_collection.as_ref() {
            set_fields.insert("bunnyCollectionId", collection_id.clone());
        }

        if let Some(collection_id) = lesson_stream_collection.as_ref() {
            let video_id = asset.get_str("providerVideoId").map(str::to_owned).map_err(|_| AppError::Internal)?;
            let stream = BunnyStream::from_settings(&state.settings, state.http.clone())?;
            let title_for_stream = stream_title.as_deref().unwrap_or(&title);
            if let Err(error) = stream.update_video(&video_id, title_for_stream, collection_id).await {
                let _ = state.db.collection::<Document>("lessons").delete_one(doc! { "_id": lesson_id.clone(), "companyId": auth.company_id.clone() }).await;
                if let Some((_, target_path)) = pending_storage_move.as_ref() {
                    let storage = BunnyStorage::from_settings(&state.settings, state.http.clone())?;
                    let _ = storage.delete_object(target_path).await;
                }
                return Err(error);
            }
        }

        assets.update_one(
            doc! { "_id": asset.get_object_id("_id").map_err(|_| AppError::Internal)?.to_owned(), "companyId": auth.company_id.clone() },
            doc! { "$set": set_fields },
        ).await.map_err(AppError::from_mongo)?;

        if pending_storage_move.is_some() {
            // Keep the previous object as a compatibility copy: course covers, company
            // branding, lesson thumbnails, and links that were copied before attachment
            // may still point at the original public CDN URL. The asset record now points
            // at the lesson-specific hierarchy; an old-key cleanup requires URL migration.
        }
    }

    Ok((StatusCode::CREATED, Json(content_response(document)?)))
}



async fn archive_content_collection(
    state: &AppState,
    collection_name: &str,
    filter: Document,
    now: DateTime,
) -> AppResult<()> {
    state.db.collection::<Document>(collection_name).update_many(
        filter,
        doc! { "$set": {
            "status": "archived",
            "deletedAt": now,
            "updatedAt": now,
        } },
    ).await?;
    Ok(())
}

pub async fn update_subject(
    State(state): State<AppState>,
    Path(subject_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateSubjectRequest>,
) -> AppResult<Json<ContentResponse>> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let subject_id = parse_id(&subject_id)?;
    let subjects = state.db.collection::<Document>("subjects");
    let filter = doc! {
        "_id": subject_id.clone(),
        "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    let current = subjects.find_one(filter.clone()).await?.ok_or(AppError::NotFound)?;
    let course_id = current.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;

    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let description = input.description.map(|value| value.trim().to_owned());
    let sort_order = input.sort_order.unwrap_or(current.get_i32("sortOrder").unwrap_or(0));
    if !valid_title_slug(&title, &slug) || !validate_optional_fields(&description, sort_order) {
        return Err(AppError::BadRequest("invalid subject title, slug, description, or order"));
    }
    if subjects.find_one(doc! {
        "companyId": auth.company_id.clone(), "courseId": course_id.clone(),
        "slug": &slug, "_id": { "$ne": subject_id.clone() },
        "deletedAt": { "$exists": false },
    }).await?.is_some() {
        return Err(AppError::Conflict("subject slug is already in use in this course"));
    }

    subjects.update_one(filter.clone(), doc! { "$set": {
        "title": &title, "slug": &slug, "description": description,
        "sortOrder": sort_order, "updatedAt": DateTime::now(),
    } }).await?;
    Ok(Json(content_response(subjects.find_one(filter).await?.ok_or(AppError::NotFound)?)?))
}

pub async fn delete_subject(
    State(state): State<AppState>,
    Path(subject_id): Path<String>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let subject_id = parse_id(&subject_id)?;
    let filter = doc! {
        "_id": subject_id.clone(), "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    let subject = state.db.collection::<Document>("subjects")
        .find_one(filter.clone()).await?.ok_or(AppError::NotFound)?;
    let course_id = subject.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;
    let now = DateTime::now();
    state.db.collection::<Document>("subjects").update_one(
        filter, doc! { "$set": { "status": "archived", "deletedAt": now, "updatedAt": now } }
    ).await?;
    let child_filter = doc! {
        "companyId": auth.company_id.clone(), "courseId": course_id.clone(),
        "subjectId": subject_id.clone(), "deletedAt": { "$exists": false },
    };
    archive_content_collection(&state, "topics", child_filter.clone(), now).await?;
    archive_content_collection(&state, "chapters", child_filter.clone(), now).await?;
    archive_content_collection(&state, "chapter_parts", child_filter.clone(), now).await?;
    archive_content_collection(&state, "lessons", child_filter.clone(), now).await?;
    archive_content_collection(&state, "assets", child_filter, now).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_topic(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateTopicRequest>,
) -> AppResult<Json<ContentResponse>> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let topic_id = parse_id(&topic_id)?;
    let topics = state.db.collection::<Document>("topics");
    let filter = doc! {
        "_id": topic_id.clone(), "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    let current = topics.find_one(filter.clone()).await?.ok_or(AppError::NotFound)?;
    let course_id = current.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    let subject_id = current.get_object_id("subjectId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;

    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let description = input.description.map(|value| value.trim().to_owned());
    let sort_order = input.sort_order.unwrap_or(current.get_i32("sortOrder").unwrap_or(0));
    if !valid_title_slug(&title, &slug) || !validate_optional_fields(&description, sort_order) {
        return Err(AppError::BadRequest("invalid topic title, slug, description, or order"));
    }
    if topics.find_one(doc! {
        "companyId": auth.company_id.clone(), "subjectId": subject_id.clone(),
        "slug": &slug, "_id": { "$ne": topic_id.clone() },
        "deletedAt": { "$exists": false },
    }).await?.is_some() {
        return Err(AppError::Conflict("topic slug is already in use in this subject"));
    }

    topics.update_one(filter.clone(), doc! { "$set": {
        "title": &title, "slug": &slug, "description": description,
        "sortOrder": sort_order, "updatedAt": DateTime::now(),
    } }).await?;
    Ok(Json(content_response(topics.find_one(filter).await?.ok_or(AppError::NotFound)?)?))
}

pub async fn delete_topic(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let topic_id = parse_id(&topic_id)?;
    let filter = doc! {
        "_id": topic_id.clone(), "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    let topic = state.db.collection::<Document>("topics")
        .find_one(filter.clone()).await?.ok_or(AppError::NotFound)?;
    let course_id = topic.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;
    let now = DateTime::now();
    state.db.collection::<Document>("topics").update_one(
        filter, doc! { "$set": { "status": "archived", "deletedAt": now, "updatedAt": now } }
    ).await?;
    let child_filter = doc! {
        "companyId": auth.company_id.clone(), "courseId": course_id.clone(),
        "topicId": topic_id.clone(), "deletedAt": { "$exists": false },
    };
    archive_content_collection(&state, "chapters", child_filter.clone(), now).await?;
    archive_content_collection(&state, "chapter_parts", child_filter.clone(), now).await?;
    archive_content_collection(&state, "lessons", child_filter.clone(), now).await?;
    archive_content_collection(&state, "assets", child_filter, now).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_lesson(
    State(state): State<AppState>,
    Path(lesson_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateLessonRequest>,
) -> AppResult<Json<ContentResponse>> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let lesson_id = parse_id(&lesson_id)?;
    let lessons = state.db.collection::<Document>("lessons");
    let filter = doc! {
        "_id": lesson_id.clone(), "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    let current = lessons.find_one(filter.clone()).await?.ok_or(AppError::NotFound)?;
    let course_id = current.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    let subject_id = current.get_object_id("subjectId").map_err(|_| AppError::Internal)?.to_owned();
    let topic_id = current.get_object_id("topicId").map_err(|_| AppError::Internal)?.to_owned();
    let course = visible_course(&state, &auth.company_id, &course_id, true).await?;
    let company = require_active_company(&state.db, &auth.company_id).await?;
    let subject = state.db.collection::<Document>("subjects").find_one(doc! {
        "_id": subject_id.clone(), "companyId": auth.company_id.clone(),
        "courseId": course_id.clone(), "deletedAt": { "$exists": false },
    }).await?.ok_or(AppError::NotFound)?;
    let topic = state.db.collection::<Document>("topics").find_one(doc! {
        "_id": topic_id.clone(), "companyId": auth.company_id.clone(),
        "courseId": course_id.clone(), "subjectId": subject_id.clone(),
        "deletedAt": { "$exists": false },
    }).await?.ok_or(AppError::NotFound)?;

    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let lesson_type = input.lesson_type.trim().to_ascii_lowercase();
    let content = input.content.map(|value| value.trim().to_owned());
    let duration_seconds = input.duration_seconds.unwrap_or(0);
    let sort_order = input.sort_order.unwrap_or(current.get_i32("sortOrder").unwrap_or(0));
    let media_asset_id = input.media_asset_id.as_deref().map(parse_id).transpose()?;
    let chapter_id = input.chapter_id.as_deref().map(parse_id).transpose()?
        .or_else(|| current.get_object_id("chapterId").ok());
    let chapter_part_id = input.chapter_part_id.as_deref().map(parse_id).transpose()?
        .or_else(|| current.get_object_id("chapterPartId").ok());
    let thumbnail_url = input.thumbnail_url.as_deref().map(str::trim).filter(|value| !value.is_empty()).map(str::to_owned)
        .or_else(|| current.get_str("thumbnailUrl").ok().map(str::to_owned));
    if thumbnail_url.as_ref().is_some_and(|url| !url.starts_with("https://") || url.len() > 2048) {
        return Err(AppError::BadRequest("lesson thumbnail must be an HTTPS URL"));
    }
    if chapter_id.is_some() != chapter_part_id.is_some() {
        return Err(AppError::BadRequest("chapter and chapter-part IDs must be supplied together"));
    }
    if let (Some(chapter_id), Some(chapter_part_id)) = (&chapter_id, &chapter_part_id) {
        state.db.collection::<Document>("chapters").find_one(doc! {
            "_id": chapter_id.clone(), "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(), "subjectId": subject_id.clone(),
            "topicId": topic_id.clone(), "deletedAt": { "$exists": false },
        }).await?.ok_or(AppError::BadRequest("chapter does not belong to this course part"))?;
        state.db.collection::<Document>("chapter_parts").find_one(doc! {
            "_id": chapter_part_id.clone(), "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(), "subjectId": subject_id.clone(),
            "topicId": topic_id.clone(), "chapterId": chapter_id.clone(),
            "deletedAt": { "$exists": false },
        }).await?.ok_or(AppError::BadRequest("chapter part does not belong to this chapter"))?;
    }
    if !valid_title_slug(&title, &slug)
        || !matches!(lesson_type.as_str(), "video" | "text" | "file")
        || !(0..=86_400).contains(&duration_seconds)
        || !(0..=1_000_000).contains(&sort_order)
        || content.as_ref().is_some_and(|value| value.chars().count() > 50_000)
    {
        return Err(AppError::BadRequest("invalid lesson title, slug, type, content, duration, or order"));
    }
    match lesson_type.as_str() {
        "text" if content.as_ref().is_none_or(|value| value.is_empty()) =>
            return Err(AppError::BadRequest("text lessons require content")),
        "text" if media_asset_id.is_some() =>
            return Err(AppError::BadRequest("text lessons cannot attach a media asset")),
        "video" | "file" if media_asset_id.is_none() =>
            return Err(AppError::BadRequest("video and file lessons require a media asset")),
        _ => {}
    }
    let mut target_asset: Option<Document> = None;
    if let Some(asset_id) = &media_asset_id {
        let asset_type_filter = if lesson_type == "video" {
            doc! { "assetType": "video", "status": { "$in": ["ready", "uploading", "processing"] } }
        } else {
            doc! { "assetType": { "$in": ["file", "image"] }, "status": "ready" }
        };
        let mut asset_filter = doc! {
            "_id": asset_id.clone(), "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(), "subjectId": subject_id.clone(),
            "topicId": topic_id.clone(), "deletedAt": { "$exists": false },
        };
        asset_filter.extend(asset_type_filter);
        let asset = state.db.collection::<Document>("assets").find_one(asset_filter).await?
            .ok_or(AppError::BadRequest("media asset does not belong to this topic or is not ready for attachment"))?;
        if asset.get_object_id("lessonId").ok().is_some_and(|attached| attached != lesson_id) {
            return Err(AppError::BadRequest("this asset is already attached to another lesson"));
        }
        target_asset = Some(asset);
    }
    if lessons.find_one(doc! {
        "companyId": auth.company_id.clone(), "topicId": topic_id.clone(),
        "slug": &slug, "_id": { "$ne": lesson_id.clone() },
        "deletedAt": { "$exists": false },
    }).await?.is_some() {
        return Err(AppError::Conflict("lesson slug is already in use in this topic"));
    }

    let mut target_storage_path: Option<String> = None;
    let mut target_collection_id: Option<String> = None;
    if chapter_id.is_none() && chapter_part_id.is_none() {
        if let Some(asset) = target_asset.as_ref() {
            let (storage_path, collection_id) = organize_simple_lesson_asset(
                &state, &auth.company_id, &company, &course, &subject, &topic,
                asset, &lesson_id, &title, &slug, true,
            ).await?;
            target_storage_path = storage_path;
            target_collection_id = collection_id;
        }
    }

    lessons.update_one(filter.clone(), doc! { "$set": {
        "title": &title, "slug": &slug, "lessonType": &lesson_type,
        "content": content, "mediaAssetId": media_asset_id,
        "chapterId": chapter_id, "chapterPartId": chapter_part_id,
        "thumbnailUrl": thumbnail_url,
        "durationSeconds": duration_seconds, "isPreview": input.is_preview.unwrap_or(false),
        "sortOrder": sort_order, "updatedAt": DateTime::now(),
    } }).await?;

    let assets = state.db.collection::<Document>("assets");
    let previous_asset_id = current.get_object_id("mediaAssetId").ok().map(|id| id.to_owned());
    if previous_asset_id.as_ref() != media_asset_id.as_ref() {
        if let Some(previous_id) = previous_asset_id {
            assets.update_one(
                doc! { "_id": previous_id, "companyId": auth.company_id.clone(), "lessonId": lesson_id.clone() },
                doc! { "$unset": { "lessonId": "" }, "$set": { "updatedAt": DateTime::now() } },
            ).await?;
        }
    }
    if let Some(asset) = target_asset.as_ref() {
        let mut set_fields = doc! {
            "lessonId": lesson_id.clone(),
            "updatedAt": DateTime::now(),
        };
        if let Some(path) = target_storage_path {
            set_fields.insert("storagePath", path);
        }
        if let Some(collection_id) = target_collection_id {
            set_fields.insert("bunnyCollectionId", collection_id);
        }
        assets.update_one(
            doc! { "_id": asset.get_object_id("_id").map_err(|_| AppError::Internal)?.to_owned(), "companyId": auth.company_id.clone() },
            doc! { "$set": set_fields },
        ).await?;
    }
    Ok(Json(content_response(lessons.find_one(filter).await?.ok_or(AppError::NotFound)?)?))
}

pub async fn delete_lesson(
    State(state): State<AppState>,
    Path(lesson_id): Path<String>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let lesson_id = parse_id(&lesson_id)?;
    let filter = doc! {
        "_id": lesson_id, "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    let lesson = state.db.collection::<Document>("lessons")
        .find_one(filter.clone()).await?.ok_or(AppError::NotFound)?;
    let course_id = lesson.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;
    let now = DateTime::now();
    state.db.collection::<Document>("lessons").update_one(
        filter, doc! { "$set": {
            "status": "archived", "deletedAt": now, "updatedAt": now,
        } },
    ).await?;
    Ok(StatusCode::NO_CONTENT)
}


async fn permanently_remove_assets(
    state: &AppState,
    company_id: &ObjectId,
    filter: Document,
) -> AppResult<()> {
    let cursor = state.db.collection::<Document>("assets").find(filter).await?;
    let assets: Vec<Document> = cursor.try_collect().await.map_err(AppError::from_mongo)?;
    for asset in assets {
        let id = asset.get_object_id("_id").map_err(|_| AppError::Internal)?.to_owned();
        crate::media::permanently_delete_asset_record(state, company_id, &id).await?;
    }
    Ok(())
}

async fn permanently_remove_lesson_progress(
    state: &AppState,
    company_id: &ObjectId,
    lesson_filter: Document,
) -> AppResult<()> {
    let cursor = state.db.collection::<Document>("lessons")
        .find(lesson_filter).projection(doc! { "_id": 1 }).await?;
    let lessons: Vec<Document> = cursor.try_collect().await.map_err(AppError::from_mongo)?;
    let ids: Vec<ObjectId> = lessons.iter()
        .filter_map(|lesson| lesson.get_object_id("_id").ok().map(|id| id.to_owned()))
        .collect();
    if !ids.is_empty() {
        state.db.collection::<Document>("lesson_progress").delete_many(doc! {
            "companyId": company_id.clone(),
            "lessonId": { "$in": ids },
        }).await?;
    }
    Ok(())
}

/// Permanently deletes a subject and its curriculum/media. Payment and enrollment
/// history is deliberately retained; lesson progress for deleted lessons is removed.
pub async fn permanently_delete_subject(
    State(state): State<AppState>,
    Path(subject_id): Path<String>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let subject_id = parse_id(&subject_id)?;
    let subject = state.db.collection::<Document>("subjects").find_one(doc! {
        "_id": subject_id.clone(), "companyId": auth.company_id.clone(),
    }).await?.ok_or(AppError::NotFound)?;
    let course_id = subject.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    if state.db.collection::<Document>("courses").find_one(doc! {
        "_id": course_id.clone(), "companyId": auth.company_id.clone(),
    }).await?.is_none() {
        return Err(AppError::NotFound);
    }

    let subtree = doc! {
        "companyId": auth.company_id.clone(),
        "courseId": course_id.clone(),
        "subjectId": subject_id.clone(),
    };
    permanently_remove_lesson_progress(&state, &auth.company_id, subtree.clone()).await?;
    permanently_remove_assets(&state, &auth.company_id, subtree.clone()).await?;
    for collection in ["lessons", "chapter_parts", "chapters", "topics"] {
        state.db.collection::<Document>(collection).delete_many(subtree.clone()).await?;
    }
    state.db.collection::<Document>("subjects").delete_one(doc! {
        "_id": subject_id, "companyId": auth.company_id.clone(),
    }).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Permanently deletes a chapter and its lessons/media while retaining course history.
pub async fn permanently_delete_topic(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let topic_id = parse_id(&topic_id)?;
    let topic = state.db.collection::<Document>("topics").find_one(doc! {
        "_id": topic_id.clone(), "companyId": auth.company_id.clone(),
    }).await?.ok_or(AppError::NotFound)?;
    let course_id = topic.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    let subtree = doc! {
        "companyId": auth.company_id.clone(),
        "courseId": course_id,
        "topicId": topic_id.clone(),
    };
    permanently_remove_lesson_progress(&state, &auth.company_id, subtree.clone()).await?;
    permanently_remove_assets(&state, &auth.company_id, subtree.clone()).await?;
    for collection in ["lessons", "chapter_parts", "chapters"] {
        state.db.collection::<Document>(collection).delete_many(subtree.clone()).await?;
    }
    state.db.collection::<Document>("topics").delete_one(doc! {
        "_id": topic_id, "companyId": auth.company_id.clone(),
    }).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Permanently deletes a lesson, its attached media and its progress records.
pub async fn permanently_delete_lesson(
    State(state): State<AppState>,
    Path(lesson_id): Path<String>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let lesson_id = parse_id(&lesson_id)?;
    let lesson = state.db.collection::<Document>("lessons").find_one(doc! {
        "_id": lesson_id.clone(), "companyId": auth.company_id.clone(),
    }).await?.ok_or(AppError::NotFound)?;

    let mut asset_ids: Vec<ObjectId> = Vec::new();
    if let Ok(id) = lesson.get_object_id("mediaAssetId") {
        asset_ids.push(id.to_owned());
    }
    let cursor = state.db.collection::<Document>("assets").find(doc! {
        "companyId": auth.company_id.clone(), "lessonId": lesson_id.clone(),
    }).await?;
    let linked_assets: Vec<Document> = cursor.try_collect().await.map_err(AppError::from_mongo)?;
    for asset in linked_assets {
        if let Ok(id) = asset.get_object_id("_id") {
            if !asset_ids.iter().any(|existing| *existing == id) {
                asset_ids.push(id.to_owned());
            }
        }
    }
    for asset_id in asset_ids {
        crate::media::permanently_delete_asset_record(&state, &auth.company_id, &asset_id).await?;
    }
    state.db.collection::<Document>("lesson_progress").delete_many(doc! {
        "companyId": auth.company_id.clone(), "lessonId": lesson_id.clone(),
    }).await?;
    state.db.collection::<Document>("lessons").delete_one(doc! {
        "_id": lesson_id, "companyId": auth.company_id.clone(),
    }).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_chapters(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<Vec<ContentResponse>>> {
    let topic_id = parse_id(&topic_id)?;
    let topic = state.db.collection::<Document>("topics").find_one(doc! {
        "_id": topic_id.clone(), "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    }).await?.ok_or(AppError::NotFound)?;
    let course_id = topic.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, auth.can_manage_content()).await?;
    let items = load_list(
        &state.db.collection::<Document>("chapters"),
        doc! {
            "companyId": auth.company_id.clone(), "topicId": topic_id,
            "status": "active", "deletedAt": { "$exists": false },
        },
    ).await?;
    Ok(Json(items))
}

pub async fn create_chapter(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateTopicRequest>,
) -> AppResult<(StatusCode, Json<ContentResponse>)> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let topic_id = parse_id(&topic_id)?;
    let topic = state.db.collection::<Document>("topics").find_one(doc! {
        "_id": topic_id.clone(), "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    }).await?.ok_or(AppError::NotFound)?;
    let course_id = topic.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    let subject_id = topic.get_object_id("subjectId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;

    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let description = input.description.map(|value| value.trim().to_owned());
    let sort_order = input.sort_order.unwrap_or(0);
    if !valid_title_slug(&title, &slug) || !validate_optional_fields(&description, sort_order) {
        return Err(AppError::BadRequest("invalid chapter title, slug, description, or order"));
    }
    let chapters = state.db.collection::<Document>("chapters");
    if chapters.find_one(doc! {
        "companyId": auth.company_id.clone(), "topicId": topic_id.clone(),
        "slug": &slug, "deletedAt": { "$exists": false },
    }).await?.is_some() {
        return Err(AppError::Conflict("chapter slug is already in use in this part"));
    }
    let now = DateTime::now();
    let document = doc! {
        "_id": ObjectId::new(), "companyId": auth.company_id.clone(),
        "courseId": course_id, "subjectId": subject_id, "topicId": topic_id,
        "title": title, "slug": slug, "description": description,
        "status": "active", "sortOrder": sort_order,
        "createdBy": auth.user_id.clone(), "createdAt": now, "updatedAt": now,
    };
    chapters.insert_one(document.clone()).await.map_err(AppError::from_mongo)?;
    Ok((StatusCode::CREATED, Json(content_response(document)?)))
}

pub async fn update_chapter(
    State(state): State<AppState>,
    Path(chapter_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateTopicRequest>,
) -> AppResult<Json<ContentResponse>> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let chapter_id = parse_id(&chapter_id)?;
    let chapters = state.db.collection::<Document>("chapters");
    let filter = doc! {
        "_id": chapter_id.clone(), "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    let current = chapters.find_one(filter.clone()).await?.ok_or(AppError::NotFound)?;
    let course_id = current.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    let topic_id = current.get_object_id("topicId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;
    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let description = input.description.map(|value| value.trim().to_owned());
    let sort_order = input.sort_order.unwrap_or(current.get_i32("sortOrder").unwrap_or(0));
    if !valid_title_slug(&title, &slug) || !validate_optional_fields(&description, sort_order) {
        return Err(AppError::BadRequest("invalid chapter title, slug, description, or order"));
    }
    if chapters.find_one(doc! {
        "companyId": auth.company_id.clone(), "topicId": topic_id.clone(),
        "slug": &slug, "_id": { "$ne": chapter_id.clone() },
        "deletedAt": { "$exists": false },
    }).await?.is_some() {
        return Err(AppError::Conflict("chapter slug is already in use in this part"));
    }
    chapters.update_one(filter.clone(), doc! { "$set": {
        "title": &title, "slug": &slug, "description": description,
        "sortOrder": sort_order, "updatedAt": DateTime::now(),
    } }).await?;
    Ok(Json(content_response(chapters.find_one(filter).await?.ok_or(AppError::NotFound)?)?))
}

pub async fn delete_chapter(
    State(state): State<AppState>,
    Path(chapter_id): Path<String>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let chapter_id = parse_id(&chapter_id)?;
    let chapters = state.db.collection::<Document>("chapters");
    let filter = doc! {
        "_id": chapter_id.clone(), "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    let chapter = chapters.find_one(filter.clone()).await?.ok_or(AppError::NotFound)?;
    let course_id = chapter.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;
    let now = DateTime::now();
    chapters.update_one(filter, doc! { "$set": {
        "status": "archived", "deletedAt": now, "updatedAt": now,
    } }).await?;
    let child_filter = doc! {
        "companyId": auth.company_id.clone(), "courseId": course_id,
        "chapterId": chapter_id.clone(), "deletedAt": { "$exists": false },
    };
    archive_content_collection(&state, "chapter_parts", child_filter.clone(), now).await?;
    archive_content_collection(&state, "lessons", child_filter, now).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_chapter_parts(
    State(state): State<AppState>,
    Path(chapter_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<Vec<ContentResponse>>> {
    let chapter_id = parse_id(&chapter_id)?;
    let chapter = state.db.collection::<Document>("chapters").find_one(doc! {
        "_id": chapter_id.clone(), "companyId": auth.company_id.clone(),
        "status": "active", "deletedAt": { "$exists": false },
    }).await?.ok_or(AppError::NotFound)?;
    let course_id = chapter.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, auth.can_manage_content()).await?;
    let items = load_list(
        &state.db.collection::<Document>("chapter_parts"),
        doc! {
            "companyId": auth.company_id.clone(), "chapterId": chapter_id,
            "status": "active", "deletedAt": { "$exists": false },
        },
    ).await?;
    Ok(Json(items))
}

pub async fn create_chapter_part(
    State(state): State<AppState>,
    Path(chapter_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateTopicRequest>,
) -> AppResult<(StatusCode, Json<ContentResponse>)> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let chapter_id = parse_id(&chapter_id)?;
    let chapter = state.db.collection::<Document>("chapters").find_one(doc! {
        "_id": chapter_id.clone(), "companyId": auth.company_id.clone(),
        "status": "active", "deletedAt": { "$exists": false },
    }).await?.ok_or(AppError::NotFound)?;
    let course_id = chapter.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    let subject_id = chapter.get_object_id("subjectId").map_err(|_| AppError::Internal)?.to_owned();
    let topic_id = chapter.get_object_id("topicId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;
    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let description = input.description.map(|value| value.trim().to_owned());
    let sort_order = input.sort_order.unwrap_or(0);
    if !valid_title_slug(&title, &slug) || !validate_optional_fields(&description, sort_order) {
        return Err(AppError::BadRequest("invalid content-part title, slug, description, or order"));
    }
    let parts = state.db.collection::<Document>("chapter_parts");
    if parts.find_one(doc! {
        "companyId": auth.company_id.clone(), "chapterId": chapter_id.clone(),
        "slug": &slug, "deletedAt": { "$exists": false },
    }).await?.is_some() {
        return Err(AppError::Conflict("part slug is already in use in this chapter"));
    }
    let now = DateTime::now();
    let document = doc! {
        "_id": ObjectId::new(), "companyId": auth.company_id.clone(),
        "courseId": course_id, "subjectId": subject_id, "topicId": topic_id,
        "chapterId": chapter_id, "title": title, "slug": slug,
        "description": description, "status": "active", "sortOrder": sort_order,
        "createdBy": auth.user_id.clone(), "createdAt": now, "updatedAt": now,
    };
    parts.insert_one(document.clone()).await.map_err(AppError::from_mongo)?;
    Ok((StatusCode::CREATED, Json(content_response(document)?)))
}

pub async fn update_chapter_part(
    State(state): State<AppState>,
    Path(part_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateTopicRequest>,
) -> AppResult<Json<ContentResponse>> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let part_id = parse_id(&part_id)?;
    let parts = state.db.collection::<Document>("chapter_parts");
    let filter = doc! {
        "_id": part_id.clone(), "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    let current = parts.find_one(filter.clone()).await?.ok_or(AppError::NotFound)?;
    let course_id = current.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    let chapter_id = current.get_object_id("chapterId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;
    let title = input.title.trim().to_owned();
    let slug = input.slug.trim().to_ascii_lowercase();
    let description = input.description.map(|value| value.trim().to_owned());
    let sort_order = input.sort_order.unwrap_or(current.get_i32("sortOrder").unwrap_or(0));
    if !valid_title_slug(&title, &slug) || !validate_optional_fields(&description, sort_order) {
        return Err(AppError::BadRequest("invalid content-part title, slug, description, or order"));
    }
    if parts.find_one(doc! {
        "companyId": auth.company_id.clone(), "chapterId": chapter_id.clone(),
        "slug": &slug, "_id": { "$ne": part_id.clone() },
        "deletedAt": { "$exists": false },
    }).await?.is_some() {
        return Err(AppError::Conflict("part slug is already in use in this chapter"));
    }
    parts.update_one(filter.clone(), doc! { "$set": {
        "title": &title, "slug": &slug, "description": description,
        "sortOrder": sort_order, "updatedAt": DateTime::now(),
    } }).await?;
    Ok(Json(content_response(parts.find_one(filter).await?.ok_or(AppError::NotFound)?)?))
}

pub async fn delete_chapter_part(
    State(state): State<AppState>,
    Path(part_id): Path<String>,
    auth: AuthUser,
) -> AppResult<StatusCode> {
    if !auth.can_manage_content() { return Err(AppError::Forbidden); }
    let part_id = parse_id(&part_id)?;
    let parts = state.db.collection::<Document>("chapter_parts");
    let filter = doc! {
        "_id": part_id.clone(), "companyId": auth.company_id.clone(),
        "deletedAt": { "$exists": false },
    };
    let part = parts.find_one(filter.clone()).await?.ok_or(AppError::NotFound)?;
    let course_id = part.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_owned();
    visible_course(&state, &auth.company_id, &course_id, true).await?;
    let now = DateTime::now();
    parts.update_one(filter, doc! { "$set": {
        "status": "archived", "deletedAt": now, "updatedAt": now,
    } }).await?;
    archive_content_collection(&state, "lessons", doc! {
        "companyId": auth.company_id.clone(), "courseId": course_id,
        "chapterPartId": part_id, "deletedAt": { "$exists": false },
    }, now).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_chapter_part_lessons(
    State(state): State<AppState>,
    Path(part_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<Vec<ContentResponse>>> {
    let part_id = parse_id(&part_id)?;
    let part = state.db.collection::<Document>("chapter_parts").find_one(doc! {
        "_id": part_id.clone(), "companyId": auth.company_id.clone(),
        "status": "active", "deletedAt": { "$exists": false },
    }).await?.ok_or(AppError::NotFound)?;
    let topic_id = part.get_object_id("topicId").map_err(|_| AppError::Internal)?.to_owned();
    let Json(mut lessons) = list_lessons(State(state), Path(topic_id.to_hex()), auth).await?;
    let part_hex = part_id.to_hex();
    lessons.retain(|lesson| lesson.chapter_part_id.as_deref() == Some(part_hex.as_str()));
    Ok(Json(lessons))
}

pub async fn create_chapter_part_lesson(
    State(state): State<AppState>,
    Path(part_id): Path<String>,
    auth: AuthUser,
    Json(mut input): Json<CreateLessonRequest>,
) -> AppResult<(StatusCode, Json<ContentResponse>)> {
    let part_id = parse_id(&part_id)?;
    let part = state.db.collection::<Document>("chapter_parts").find_one(doc! {
        "_id": part_id.clone(), "companyId": auth.company_id.clone(),
        "status": "active", "deletedAt": { "$exists": false },
    }).await?.ok_or(AppError::NotFound)?;
    let topic_id = part.get_object_id("topicId").map_err(|_| AppError::Internal)?.to_owned();
    let chapter_id = part.get_object_id("chapterId").map_err(|_| AppError::Internal)?.to_owned();
    input.chapter_id = Some(chapter_id.to_hex());
    input.chapter_part_id = Some(part_id.to_hex());
    create_lesson(State(state), Path(topic_id.to_hex()), auth, Json(input)).await
}
