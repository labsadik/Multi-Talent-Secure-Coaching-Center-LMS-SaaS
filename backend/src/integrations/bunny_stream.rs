use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    config::Settings,
    error::{AppError, AppResult},
};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BunnyUploadTicket {
    pub endpoint: String,
    pub video_id: String,
    pub library_id: u64,
    pub authorization_signature: String,
    pub authorization_expire: u64,
}

#[derive(Debug)]
pub struct BunnyVideoStatus {
    pub status: i64,
    pub length_seconds: i64,
}

#[derive(Clone)]
pub struct BunnyStream {
    client: reqwest::Client,
    library_id: u64,
    api_key: String,
}

impl BunnyStream {
    pub fn from_settings(settings: &Settings, client: reqwest::Client) -> AppResult<Self> {
        Ok(Self {
            client,
            library_id: settings
                .bunny_stream_library_id
                .ok_or(AppError::ServiceUnavailable("Bunny Stream"))?,
            api_key: settings
                .bunny_stream_api_key
                .clone()
                .ok_or(AppError::ServiceUnavailable("Bunny Stream"))?,
        })
    }

    /// Creates a collection used to group one course's videos in the shared library.
    pub async fn create_collection(&self, name: &str) -> AppResult<String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 180 {
            return Err(AppError::BadRequest("collection name must be between 1 and 180 characters"));
        }

        let response: serde_json::Value = self
            .client
            .post(format!(
                "https://video.bunnycdn.com/library/{}/collections",
                self.library_id
            ))
            .header("AccessKey", &self.api_key)
            .json(&serde_json::json!({ "name": name }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        response
            .get("guid")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or(AppError::Internal)
    }

    /// Creates a video and returns a time-limited TUS upload authorization.
    /// The private library API key is never returned to the caller.
    pub async fn create_upload_ticket(
        &self,
        title: &str,
        collection_id: Option<&str>,
        ttl_seconds: u64,
        endpoint: &str,
    ) -> AppResult<BunnyUploadTicket> {
        if !(3_600..=86_400).contains(&ttl_seconds) {
            return Err(AppError::BadRequest("invalid upload token lifetime"));
        }
        if !endpoint.starts_with("https://") || endpoint.chars().any(char::is_whitespace) {
            return Err(AppError::Internal);
        }

        let video_id = self.create_video(title, collection_id).await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| AppError::Internal)?
            .as_secs();
        let expiration = now.saturating_add(ttl_seconds);
        let signing_value = format!("{}{}{}{}", self.library_id, self.api_key, expiration, video_id);
        let digest = Sha256::digest(signing_value.as_bytes());
        let signature = digest.iter().map(|byte| format!("{byte:02x}")).collect();

        Ok(BunnyUploadTicket {
            endpoint: endpoint.to_owned(),
            video_id,
            library_id: self.library_id,
            authorization_signature: signature,
            authorization_expire: expiration,
        })
    }

    pub async fn get_video_status(&self, video_id: &str) -> AppResult<BunnyVideoStatus> {
        if video_id.is_empty() || video_id.len() > 128
            || !video_id.chars().all(|ch| ch.is_ascii_alphanumeric() || "-_".contains(ch))
        {
            return Err(AppError::BadRequest("invalid Bunny video ID"));
        }
        let response: serde_json::Value = self
            .client
            .get(format!(
                "https://video.bunnycdn.com/library/{}/videos/{}",
                self.library_id, video_id
            ))
            .header("AccessKey", &self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(BunnyVideoStatus {
            status: response.get("status").and_then(serde_json::Value::as_i64).ok_or(AppError::Internal)?,
            length_seconds: response.get("length").and_then(serde_json::Value::as_i64).unwrap_or(0),
        })
    }

    /// Creates a video record in the shared Stream library.
    /// Persist the returned GUID with companyId/courseId/subjectId/topicId in MongoDB.
    pub async fn create_video(
        &self,
        title: &str,
        collection_id: Option<&str>,
    ) -> AppResult<String> {
        let title = title.trim();
        if title.is_empty() || title.chars().count() > 160 {
            return Err(AppError::BadRequest("video title must be between 1 and 160 characters"));
        }

        let mut payload = serde_json::json!({ "title": title });
        if let Some(collection_id) = collection_id {
            if collection_id.is_empty() || collection_id.chars().count() > 128 {
                return Err(AppError::BadRequest("invalid Bunny Stream collection ID"));
            }
            payload["collectionId"] = serde_json::Value::String(collection_id.to_owned());
        }

        let response: serde_json::Value = self
            .client
            .post(format!(
                "https://video.bunnycdn.com/library/{}/videos",
                self.library_id
            ))
            .header("AccessKey", &self.api_key)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        response
            .get("guid")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or(AppError::Internal)
    }
}
