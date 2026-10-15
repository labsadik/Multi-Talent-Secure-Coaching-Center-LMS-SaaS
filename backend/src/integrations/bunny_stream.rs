use crate::{
    config::Settings,
    error::{AppError, AppResult},
};

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
