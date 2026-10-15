use axum::http::HeaderValue;
use bytes::Bytes;
use reqwest::header::CONTENT_TYPE;

use crate::{
    config::Settings,
    error::{AppError, AppResult},
};

#[derive(Clone)]
pub struct BunnyStorage {
    client: reqwest::Client,
    zone: String,
    api_key: String,
    host: String,
}

impl BunnyStorage {
    pub fn from_settings(settings: &Settings, client: reqwest::Client) -> AppResult<Self> {
        Ok(Self {
            client,
            zone: settings
                .bunny_storage_zone
                .clone()
                .ok_or(AppError::ServiceUnavailable)?,
            api_key: settings
                .bunny_storage_api_key
                .clone()
                .ok_or(AppError::ServiceUnavailable("Bunny Storage"))?,
            host: settings.bunny_storage_host.clone(),
        })
    }

    /// Uploads bytes to a validated relative path. This is server-only; never expose the AccessKey.
    pub async fn upload_bytes(
        &self,
        relative_path: &str,
        content_type: &str,
        body: Bytes,
    ) -> AppResult<()> {
        if !valid_relative_path(relative_path) {
            return Err(AppError::BadRequest("invalid storage path"));
        }
        let content_type = HeaderValue::from_str(content_type)
            .map_err(|_| AppError::BadRequest("invalid content type"))?;
        let url = format!("https://{}/{}/{}", self.host, self.zone, relative_path);

        self.client
            .put(url)
            .header("AccessKey", &self.api_key)
            .header(CONTENT_TYPE, content_type)
            .body(body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Builds a folder-style path from trusted IDs and validated slugs.
    pub fn course_topic_path(
        company_id: &str,
        company_slug: &str,
        course_id: &str,
        course_slug: &str,
        subject_id: &str,
        subject_slug: &str,
        topic_id: &str,
        topic_slug: &str,
        category: &str,
        filename: &str,
    ) -> AppResult<String> {
        let parts = [
            company_id,
            company_slug,
            course_id,
            course_slug,
            subject_id,
            subject_slug,
            topic_id,
            topic_slug,
            category,
            filename,
        ];
        if !parts.iter().all(|part| valid_path_segment(part)) {
            return Err(AppError::BadRequest("invalid media path segment"));
        }

        Ok(format!(
            "companies/{}-{}/courses/{}-{}/subjects/{}-{}/topics/{}-{}/{}/{}",
            company_id,
            company_slug,
            course_id,
            course_slug,
            subject_id,
            subject_slug,
            topic_id,
            topic_slug,
            category,
            filename
        ))
    }
}

fn valid_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && valid_path_segment(segment))
}

fn valid_path_segment(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value.len() <= 180
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "-_.".contains(ch))
}
