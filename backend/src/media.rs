use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
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
    let (topic, subject, course, _company) =
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

    let stream = BunnyStream::from_settings(&state.settings, state.http.clone())?;
    let courses = state.db.collection::<Document>("courses");

    let collection_id = match course.get_str("bunnyCollectionId").ok() {
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
    let company_slug = required_slug(&company, "slug")?;
    let course_slug = required_slug(&course, "slug")?;
    let subject_slug = required_slug(&subject, "slug")?;
    let topic_slug = required_slug(&topic, "slug")?;

    let asset_id = ObjectId::new();
    let filename = format!("{}.{}", asset_id.to_hex(), extension);
    let relative_path = BunnyStorage::course_topic_path(
        &auth.company_id.to_hex(),
        &company_slug,
        &course_id.to_hex(),
        &course_slug,
        &subject_id.to_hex(),
        &subject_slug,
        &topic_id.to_hex(),
        &topic_slug,
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

