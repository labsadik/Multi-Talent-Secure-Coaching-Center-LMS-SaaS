use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use futures_util::TryStreamExt;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    integrations::{
        bunny_storage::BunnyStorage,
        bunny_stream::{BunnyStream, BunnyUploadTicket},
    },
    state::AppState,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVideoTicketRequest {
    pub title: String,
    pub chapter_id: Option<String>,
    pub chapter_part_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AssetUploadContext {
    pub chapter_id: Option<String>,
    pub chapter_part_id: Option<String>,
    pub filename: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVideoTicketResponse {
    pub asset_id: String,
    pub status: String,
    pub upload: BunnyUploadTicket,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoStatusResponse {
    pub asset_id: String,
    pub status: String,
    pub bunny_status: i64,
    pub duration_seconds: i64,
}

async fn topic_context(
    state: &AppState,
    company_id: &ObjectId,
    topic_id: &ObjectId,
) -> AppResult<(Document, Document, Document, Document)> {
    let topic = state
        .db
        .collection::<Document>("topics")
        .find_one(doc! {
            "_id": topic_id.clone(),
            "companyId": company_id.clone(),
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;

    let subject_id = topic
        .get_object_id("subjectId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let course_id = topic
        .get_object_id("courseId")
        .map_err(|_| AppError::Internal)?
        .to_owned();

    let subject = state
        .db
        .collection::<Document>("subjects")
        .find_one(doc! {
            "_id": subject_id,
            "companyId": company_id.clone(),
            "courseId": course_id.clone(),
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;

    let course = state
        .db
        .collection::<Document>("courses")
        .find_one(doc! {
            "_id": course_id,
            "companyId": company_id.clone(),
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;

    let company = state
        .db
        .collection::<Document>("companies")
        .find_one(doc! {
            "_id": company_id.clone(),
            "status": "active",
        })
        .await?
        .ok_or(AppError::Forbidden)?;

    Ok((topic, subject, course, company))
}

fn required_slug(document: &Document, field: &'static str) -> AppResult<String> {
    document
        .get_str(field)
        .map(str::to_owned)
        .map_err(|_| AppError::Internal)
}

type ChapterUploadContext = (ObjectId, Document, ObjectId, Document);

async fn resolve_chapter_upload_context(
    state: &AppState,
    company_id: &ObjectId,
    course_id: &ObjectId,
    subject_id: &ObjectId,
    topic_id: &ObjectId,
    chapter_id: Option<&str>,
    chapter_part_id: Option<&str>,
) -> AppResult<Option<ChapterUploadContext>> {
    match (chapter_id, chapter_part_id) {
        (None, None) => Ok(None),
        (Some(chapter_id), Some(part_id)) => {
            let chapter_id = parse_path_id(chapter_id)?;
            let part_id = parse_path_id(part_id)?;
            let chapter = state.db.collection::<Document>("chapters").find_one(doc! {
                "_id": chapter_id.clone(),
                "companyId": company_id.clone(),
                "courseId": course_id.clone(),
                "subjectId": subject_id.clone(),
                "topicId": topic_id.clone(),
                "deletedAt": { "$exists": false },
            }).await?.ok_or(AppError::BadRequest("chapter does not belong to this company, course and part"))?;
            let chapter_part = state.db.collection::<Document>("chapter_parts").find_one(doc! {
                "_id": part_id.clone(),
                "companyId": company_id.clone(),
                "courseId": course_id.clone(),
                "subjectId": subject_id.clone(),
                "topicId": topic_id.clone(),
                "chapterId": chapter_id.clone(),
                "deletedAt": { "$exists": false },
            }).await?.ok_or(AppError::BadRequest("chapter part does not belong to the selected chapter"))?;
            Ok(Some((chapter_id, chapter, part_id, chapter_part)))
        }
        _ => Err(AppError::BadRequest("chapterId and chapterPartId must be supplied together")),
    }
}

fn safe_original_filename(value: Option<&str>, fallback: &str) -> String {
    let name = value.unwrap_or(fallback).rsplit(['/', '\\\\']).next().unwrap_or(fallback);
    let cleaned: String = name.chars().filter(|ch| !ch.is_control()).take(160).collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() { fallback.to_owned() } else { cleaned.to_owned() }
}

fn parse_path_id(value: &str) -> AppResult<ObjectId> {
    ObjectId::parse_str(value).map_err(|_| AppError::BadRequest("invalid resource ID"))
}

pub async fn create_video_ticket(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<CreateVideoTicketRequest>,
) -> AppResult<(StatusCode, Json<CreateVideoTicketResponse>)> {
    if !auth.can_manage_content() {
        return Err(AppError::Forbidden);
    }

    let topic_id = parse_path_id(&topic_id)?;
    let (topic, subject, course, company) =
        topic_context(&state, &auth.company_id, &topic_id).await?;

    let course_id = course
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let subject_id = subject
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let topic_id = topic
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let folder_context = resolve_chapter_upload_context(
        &state,
        &auth.company_id,
        &course_id,
        &subject_id,
        &topic_id,
        input.chapter_id.as_deref(),
        input.chapter_part_id.as_deref(),
    ).await?;

    let stream = BunnyStream::from_settings(&state.settings, state.http.clone())?;
    let courses = state.db.collection::<Document>("courses");

    let collection_id = if let Some((chapter_id, _chapter, chapter_part_id, chapter_part)) = folder_context.as_ref() {
        if let Some(existing) = chapter_part.get_str("bunnyCollectionId").ok().filter(|value| !value.is_empty()) {
            existing.to_owned()
        } else {
            let company_id_hex = auth.company_id.to_hex();
            let subject_id_hex = subject_id.to_hex();
            let topic_id_hex = topic_id.to_hex();
            let chapter_id_hex = chapter_id.to_hex();
            let chapter_part_id_hex = chapter_part_id.to_hex();
            let collection_name = format!(
                "c-{}-cr-{}-s-{}-t-{}-ch-{}-cp-{}",
                company_id_hex, course_id.to_hex(), subject_id_hex, topic_id_hex,
                chapter_id_hex, chapter_part_id_hex
            );
            let created = stream.create_collection(&collection_name).await?;
            let parts = state.db.collection::<Document>("chapter_parts");
            let update = parts.update_one(
                doc! {
                    "_id": chapter_part_id.clone(),
                    "companyId": auth.company_id.clone(),
                    "bunnyCollectionId": { "$exists": false },
                },
                doc! { "$set": { "bunnyCollectionId": &created, "updatedAt": DateTime::now() } },
            ).await?;
            if update.matched_count == 1 {
                created
            } else {
                let current = parts.find_one(doc! {
                    "_id": chapter_part_id.clone(),
                    "companyId": auth.company_id.clone(),
                }).await?.ok_or(AppError::NotFound)?;
                current.get_str("bunnyCollectionId").map(str::to_owned).map_err(|_| AppError::Internal)?
            }
        }
    } else {
        match course.get_str("bunnyCollectionId").ok() {
        Some(existing) if !existing.is_empty() => existing.to_owned(),
        _ => {
            let company_id_hex = auth.company_id.to_hex();
            let course_id_hex = course_id.to_hex();
            let collection_name = format!(
                "company_{}_course_{}",
                company_id_hex, course_id_hex
            );
            let created = stream.create_collection(&collection_name).await?;

            let update = courses
                .update_one(
                    doc! {
                        "_id": course_id.clone(),
                        "companyId": auth.company_id.clone(),
                        "bunnyCollectionId": { "$exists": false },
                    },
                    doc! {
                        "$set": {
                            "bunnyCollectionId": &created,
                            "updatedAt": DateTime::now(),
                        }
                    },
                )
                .await?;

            if update.matched_count == 1 {
                created
            } else {
                // Another concurrent request may have provisioned the collection first.
                let current = courses
                    .find_one(doc! {
                        "_id": course_id.clone(),
                        "companyId": auth.company_id.clone(),
                    })
                    .await?
                    .ok_or(AppError::NotFound)?;
                current
                    .get_str("bunnyCollectionId")
                    .map(str::to_owned)
                    .map_err(|_| AppError::Internal)?
            }
        }
    }
    };

    let upload = stream
        .create_upload_ticket(
            &input.title,
            Some(&collection_id),
            state.settings.bunny_upload_signature_ttl_seconds,
            &state.settings.bunny_tus_endpoint,
        )
        .await?;

    let asset_id = ObjectId::new();
    let now = DateTime::now();
    let library_id = i64::try_from(upload.library_id).map_err(|_| AppError::Internal)?;
    let asset = doc! {
        "_id": asset_id.clone(),
        "companyId": auth.company_id.clone(),
        "courseId": course_id,
        "subjectId": subject_id,
        "topicId": topic_id,
        "chapterId": folder_context.as_ref().map(|context| context.0.clone()),
        "chapterPartId": folder_context.as_ref().map(|context| context.2.clone()),
        "title": input.title.trim(),
        "assetType": "video",
        "provider": "bunny_stream",
        "providerVideoId": &upload.video_id,
        "bunnyLibraryId": library_id,
        "bunnyCollectionId": &collection_id,
        "status": "uploading",
        "createdBy": auth.user_id.clone(),
        "createdAt": now,
        "updatedAt": now,
    };

    state
        .db
        .collection::<Document>("assets")
        .insert_one(asset)
        .await
        .map_err(AppError::from_mongo)?;

    Ok((
        StatusCode::CREATED,
        Json(CreateVideoTicketResponse {
            asset_id: asset_id.to_hex(),
            status: "uploading".to_owned(),
            upload,
        }),
    ))
}

fn file_signature_matches(mime: &str, bytes: &[u8]) -> bool {
    match mime {
        "image/jpeg" => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/webp" => bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "application/pdf" => bytes.starts_with(b"%PDF-"),
        _ => false,
    }
}

pub async fn upload_topic_file(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
    auth: AuthUser,
    Query(upload_context): Query<AssetUploadContext>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<(StatusCode, Json<Value>)> {
    if !auth.can_manage_content() {
        return Err(AppError::Forbidden);
    }
    if body.is_empty() {
        return Err(AppError::BadRequest("uploaded file is empty"));
    }
    if body.len() > state.settings.max_asset_upload_bytes {
        return Err(AppError::BadRequest("uploaded file exceeds MAX_ASSET_UPLOAD_BYTES"));
    }

    let raw_content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::BadRequest("Content-Type is required"))?;
    let mime = raw_content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    let (asset_type, category, extension) = match mime.as_str() {
        "image/jpeg" => ("image", "images", "jpg"),
        "image/png" => ("image", "images", "png"),
        "image/webp" => ("image", "images", "webp"),
        "image/gif" => ("image", "images", "gif"),
        "application/pdf" => ("file", "files", "pdf"),
        _ => {
            return Err(AppError::BadRequest(
                "supported file types are JPEG, PNG, WebP, GIF, and PDF",
            ));
        }
    };

    if !file_signature_matches(&mime, body.as_ref()) {
        return Err(AppError::BadRequest("file content does not match its declared Content-Type"));
    }

    let topic_id = parse_path_id(&topic_id)?;
    let (topic, subject, course, company) =
        topic_context(&state, &auth.company_id, &topic_id).await?;

    let course_id = course
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let subject_id = subject
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let topic_id = topic
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let folder_context = resolve_chapter_upload_context(
        &state,
        &auth.company_id,
        &course_id,
        &subject_id,
        &topic_id,
        upload_context.chapter_id.as_deref(),
        upload_context.chapter_part_id.as_deref(),
    ).await?;
    let company_slug = required_slug(&company, "slug")?;
    let course_slug = required_slug(&course, "slug")?;
    let subject_slug = required_slug(&subject, "slug")?;
    let topic_slug = required_slug(&topic, "slug")?;

    let chapter_id_hex = folder_context.as_ref().map(|context| context.0.to_hex());
    let chapter_slug = folder_context.as_ref().map(|context| required_slug(&context.1, "slug")).transpose()?;
    let chapter_part_id_hex = folder_context.as_ref().map(|context| context.2.to_hex());
    let chapter_part_slug = folder_context.as_ref().map(|context| required_slug(&context.3, "slug")).transpose()?;
    let chapter_path = chapter_id_hex.as_deref().zip(chapter_slug.as_deref());
    let chapter_part_path = chapter_part_id_hex.as_deref().zip(chapter_part_slug.as_deref());

    let asset_id = ObjectId::new();
    let filename = format!("{}.{}", asset_id.to_hex(), extension);
    let title = safe_original_filename(upload_context.filename.as_deref(), &filename);
    let relative_path = BunnyStorage::course_content_path(
        &auth.company_id.to_hex(),
        &company_slug,
        &course_id.to_hex(),
        &course_slug,
        &subject_id.to_hex(),
        &subject_slug,
        &topic_id.to_hex(),
        &topic_slug,
        chapter_path,
        chapter_part_path,
        None,
        category,
        &filename,
    )?;

    let storage = BunnyStorage::from_settings(&state.settings, state.http.clone())?;
    storage
        .upload_bytes(&relative_path, &mime, body.clone())
        .await?;

    let now = DateTime::now();
    let asset = doc! {
        "_id": asset_id.clone(),
        "companyId": auth.company_id.clone(),
        "courseId": course_id,
        "subjectId": subject_id,
        "topicId": topic_id,
        "chapterId": folder_context.as_ref().map(|context| context.0.clone()),
        "chapterPartId": folder_context.as_ref().map(|context| context.2.clone()),
        "title": &title,
        "assetType": asset_type,
        "provider": "bunny_storage",
        "storagePath": &relative_path,
        "contentType": &mime,
        "sizeBytes": i64::try_from(body.len()).map_err(|_| AppError::Internal)?,
        "status": "ready",
        "createdBy": auth.user_id.clone(),
        "createdAt": now,
        "updatedAt": now,
    };
    state
        .db
        .collection::<Document>("assets")
        .insert_one(asset)
        .await
        .map_err(AppError::from_mongo)?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "assetId": asset_id.to_hex(),
            "assetType": asset_type,
            "provider": "bunny_storage",
            "storagePath": relative_path,
            "contentType": mime,
            "sizeBytes": body.len(),
            "status": "ready"
        })),
    ))
}

pub async fn refresh_video_status(
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<VideoStatusResponse>> {
    if !auth.can_manage_content() {
        return Err(AppError::Forbidden);
    }
    let asset_id = parse_path_id(&asset_id)?;
    let assets = state.db.collection::<Document>("assets");
    let asset = assets
        .find_one(doc! {
            "_id": asset_id.clone(),
            "companyId": auth.company_id.clone(),
            "assetType": "video",
            "provider": "bunny_stream",
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;

    let expected_library_id = state
        .settings
        .bunny_stream_library_id
        .ok_or(AppError::ServiceUnavailable("Bunny Stream"))?;
    if asset.get_i64("bunnyLibraryId").ok()
        != i64::try_from(expected_library_id).ok()
    {
        return Err(AppError::Forbidden);
    }
    let video_id = asset
        .get_str("providerVideoId")
        .map(str::to_owned)
        .map_err(|_| AppError::Internal)?;

    let stream = BunnyStream::from_settings(&state.settings, state.http.clone())?;
    let video = stream.get_video_status(&video_id).await?;
    let status = match video.status {
        3 | 4 | 9 | 10 => "ready",
        5 | 8 => "failed",
        6 => "uploading",
        7 => "processing",
        _ => "processing",
    };

    assets
        .update_one(
            doc! { "_id": asset_id.clone(), "companyId": auth.company_id.clone() },
            doc! {
                "$set": {
                    "status": status,
                    "bunnyStatus": video.status,
                    "durationSeconds": video.length_seconds,
                    "updatedAt": DateTime::now(),
                }
            },
        )
        .await?;

    Ok(Json(VideoStatusResponse {
        asset_id: asset_id.to_hex(),
        status: status.to_owned(),
        bunny_status: video.status,
        duration_seconds: video.length_seconds,
    }))
}

#[cfg(test)]
mod tests {
    use super::file_signature_matches;

    #[test]
    fn rejects_mime_spoofing_and_accepts_known_signatures() {
        assert!(file_signature_matches("image/png", b"\x89PNG\r\n\x1a\nrest"));
        assert!(file_signature_matches("application/pdf", b"%PDF-1.7"));
        assert!(!file_signature_matches("image/png", b"<script>alert(1)</script>"));
        assert!(!file_signature_matches("application/pdf", b"not a pdf"));
    }
}

async fn ensure_lesson_access(
    state: &AppState,
    auth: &AuthUser,
    course_id: &ObjectId,
    lesson: &Document,
) -> AppResult<()> {
    let course = state
        .db
        .collection::<Document>("courses")
        .find_one(doc! {
            "_id": course_id.clone(),
            "companyId": auth.company_id.clone(),
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;

    if !auth.can_manage_content() && course.get_str("status").unwrap_or_default() != "published" {
        return Err(AppError::NotFound);
    }

    if !auth.can_manage_content() {
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
        if !enrolled && !lesson.get_bool("isPreview").unwrap_or(false) {
            // Use the same response for unentitled lessons and non-existent lessons.
            return Err(AppError::NotFound);
        }
    }
    Ok(())
}

pub async fn lesson_playback(
    State(state): State<AppState>,
    Path(lesson_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<Value>> {
    let lesson_id = parse_path_id(&lesson_id)?;
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
    ensure_lesson_access(&state, &auth, &course_id, &lesson).await?;

    let Some(asset_id) = lesson.get_object_id("mediaAssetId").ok().map(|id| id.to_owned()) else {
        if lesson.get_str("lessonType").ok() == Some("text") {
            return Ok(Json(json!({
                "lessonId": lesson_id.to_hex(),
                "lessonType": "text",
                "content": lesson.get_str("content").unwrap_or_default()
            })));
        }
        return Err(AppError::NotFound);
    };

    let asset = state
        .db
        .collection::<Document>("assets")
        .find_one(doc! {
            "_id": asset_id.clone(),
            "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(),
            "topicId": lesson.get_object_id("topicId").map_err(|_| AppError::Internal)?.to_owned(),
            "status": "ready",
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;

    match asset.get_str("provider").unwrap_or_default() {
        "bunny_stream" => {
            let video_id = asset
                .get_str("providerVideoId")
                .map(str::to_owned)
                .map_err(|_| AppError::Internal)?;
            if video_id.is_empty()
                || video_id.len() > 128
                || !video_id.chars().all(|ch| ch.is_ascii_alphanumeric() || "-_".contains(ch))
            {
                return Err(AppError::Internal);
            }
            let library_id = state
                .settings
                .bunny_stream_library_id
                .ok_or(AppError::ServiceUnavailable("Bunny Stream"))?;
            if asset.get_i64("bunnyLibraryId").ok()
                != i64::try_from(library_id).ok()
            {
                return Err(AppError::Forbidden);
            }
            let token_key = state
                .settings
                .bunny_stream_token_auth_key
                .as_deref()
                .filter(|key| !key.is_empty())
                .ok_or(AppError::ServiceUnavailable("Bunny Stream token authentication"))?;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| AppError::Internal)?
                .as_secs();
            let expires = now.saturating_add(state.settings.bunny_playback_token_ttl_seconds);
            let mut digest = Sha256::new();
            digest.update(token_key.as_bytes());
            digest.update(video_id.as_bytes());
            digest.update(expires.to_string().as_bytes());
            let token = hex::encode(digest.finalize());
            let url = format!(
                "https://iframe.mediadelivery.net/embed/{}/{}?token={}&expires={}",
                library_id, video_id, token, expires
            );
            Ok(Json(json!({
                "lessonId": lesson_id.to_hex(),
                "assetId": asset_id.to_hex(),
                "lessonType": "video",
                "provider": "bunny_stream",
                "playbackUrl": url,
                "expiresAt": expires
            })))
        }
        "bunny_storage" => {
            let content_type = asset
                .get_str("contentType")
                .map(str::to_owned)
                .map_err(|_| AppError::Internal)?;
            if !matches!(
                content_type.as_str(),
                "image/jpeg" | "image/png" | "image/webp" | "image/gif" | "application/pdf"
            ) {
                return Err(AppError::Internal);
            }
            Ok(Json(json!({
                "lessonId": lesson_id.to_hex(),
                "assetId": asset_id.to_hex(),
                "lessonType": lesson.get_str("lessonType").unwrap_or("file"),
                "provider": "bunny_storage",
                "downloadUrl": format!("/api/v1/assets/{}/download", asset_id.to_hex()),
                "contentType": content_type
            })))
        }
        _ => Err(AppError::NotFound),
    }
}

pub async fn download_asset(
    State(state): State<AppState>,
    Path(asset_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Response> {
    let asset_id = parse_path_id(&asset_id)?;
    let asset = state
        .db
        .collection::<Document>("assets")
        .find_one(doc! {
            "_id": asset_id.clone(),
            "companyId": auth.company_id.clone(),
            "provider": "bunny_storage",
            "status": "ready",
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;

    let course_id = asset
        .get_object_id("courseId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let topic_id = asset
        .get_object_id("topicId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let lesson = state
        .db
        .collection::<Document>("lessons")
        .find_one(doc! {
            "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(),
            "topicId": topic_id,
            "mediaAssetId": asset_id.clone(),
            "status": "active",
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;
    ensure_lesson_access(&state, &auth, &course_id, &lesson).await?;

    let declared_size = asset.get_i64("sizeBytes").unwrap_or(-1);
    if declared_size < 0 || declared_size as usize > state.settings.max_asset_upload_bytes {
        return Err(AppError::Internal);
    }
    let content_type = asset
        .get_str("contentType")
        .map(str::to_owned)
        .map_err(|_| AppError::Internal)?;
    if !matches!(
        content_type.as_str(),
        "image/jpeg" | "image/png" | "image/webp" | "image/gif" | "application/pdf"
    ) {
        return Err(AppError::Internal);
    }
    let path = asset
        .get_str("storagePath")
        .map(str::to_owned)
        .map_err(|_| AppError::Internal)?;
    let storage = BunnyStorage::from_settings(&state.settings, state.http.clone())?;
    let body = storage
        .download_bytes(&path, state.settings.max_asset_upload_bytes)
        .await?;

    let extension = match content_type.as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "application/pdf" => "pdf",
        _ => "bin",
    };
    let content_type_header =
        HeaderValue::from_str(&content_type).map_err(|_| AppError::Internal)?;
    let disposition = HeaderValue::from_str(&format!(
        "attachment; filename=\"asset-{}.{}\"",
        asset_id.to_hex(),
        extension
    ))
    .map_err(|_| AppError::Internal)?;

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type_header)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::CACHE_CONTROL, "private, no-store")
        .header("x-content-type-options", "nosniff")
        .body(Body::from(body))
        .map_err(|_| AppError::Internal)
}



#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopicAssetResponse {
    pub id: String,
    pub title: String,
    pub asset_type: String,
    pub provider: String,
    pub content_type: Option<String>,
    pub status: String,
    pub size_bytes: i64,
    pub duration_seconds: Option<i64>,
    pub created_at_ms: i64,
    pub public_url: Option<String>,
    pub chapter_id: Option<String>,
    pub chapter_part_id: Option<String>,
    pub lesson_id: Option<String>,
}

pub async fn list_topic_assets(
    State(state): State<AppState>,
    Path(topic_id): Path<String>,
    auth: AuthUser,
) -> AppResult<Json<Vec<TopicAssetResponse>>> {
    if !auth.can_manage_content() {
        return Err(AppError::Forbidden);
    }
    let topic_id = parse_path_id(&topic_id)?;
    let _ = topic_context(&state, &auth.company_id, &topic_id).await?;
    let cursor = state.db.collection::<Document>("assets")
        .find(doc! {
            "companyId": auth.company_id.clone(),
            "topicId": topic_id,
            "deletedAt": { "$exists": false },
        })
        .sort(doc! { "createdAt": -1 })
        .limit(200)
        .await?;
    let documents: Vec<Document> = cursor.try_collect().await.map_err(AppError::from_mongo)?;
    let cdn_host = state.settings.bunny_cdn_hostname.as_deref()
        .map(|host| host.trim().trim_end_matches('/'))
        .filter(|host| !host.is_empty());
    let assets = documents.into_iter().filter_map(|document| {
        let id = document.get_object_id("_id").ok()?.to_hex();
        let asset_type = document.get_str("assetType").unwrap_or("asset").to_owned();
        let provider = document.get_str("provider").unwrap_or("unknown").to_owned();
        let title = document.get_str("title").ok().map(str::to_owned).unwrap_or_else(|| {
            document.get_str("storagePath").ok()
                .and_then(|path| path.rsplit('/').next())
                .unwrap_or("Uploaded asset")
                .to_owned()
        });
        let storage_path = document.get_str("storagePath").ok();
        let public_url = match (cdn_host, storage_path) {
            (Some(host), Some(path)) if path.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'/' || b == b'-' || b == b'_' || b == b'.') => {
                Some(format!("https://{host}/{}", path.trim_start_matches('/')))
            }
            _ => None,
        };
        Some(TopicAssetResponse {
            id,
            title,
            asset_type,
            provider,
            content_type: document.get_str("contentType").ok().map(str::to_owned),
            status: document.get_str("status").unwrap_or("unknown").to_owned(),
            size_bytes: document.get_i64("sizeBytes").unwrap_or(0),
            duration_seconds: document.get_i64("durationSeconds").ok(),
            created_at_ms: document.get_datetime("createdAt").map(|date| date.timestamp_millis()).unwrap_or_default(),
            public_url,
            chapter_id: document.get_object_id("chapterId").ok().map(|id| id.to_hex()),
            chapter_part_id: document.get_object_id("chapterPartId").ok().map(|id| id.to_hex()),
            lesson_id: document.get_object_id("lessonId").ok().map(|id| id.to_hex()),
        })
    }).collect();
    Ok(Json(assets))
}
