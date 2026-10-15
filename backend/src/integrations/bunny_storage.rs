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
                .ok_or(AppError::ServiceUnavailable("Bunny Storage"))?,
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


    /// Downloads a validated file through the server so private media does not need a public CDN URL.
    pub async fn download_bytes(
        &self,
        relative_path: &str,
        max_bytes: usize,
    ) -> AppResult<Bytes> {
        if !valid_relative_path(relative_path) {
            return Err(AppError::BadRequest("invalid storage path"));
        }
        let url = format!("https://{}/{}/{}", self.host, self.zone, relative_path);
        let response = self
            .client
            .get(url)
            .header("AccessKey", &self.api_key)
            .send()
            .await?
            .error_for_status()?;

        if response
            .content_length()
            .is_some_and(|length| length > max_bytes as u64)
        {
            return Err(AppError::BadRequest("stored asset exceeds download limit"));
        }
        let bytes = response.bytes().await?;
        if bytes.len() > max_bytes {
            return Err(AppError::BadRequest("stored asset exceeds download limit"));
        }
        Ok(bytes)
    }

    /// Deletes an object from the configured storage zone. Server-only; AccessKey is never exposed.
    pub async fn delete_object(&self, relative_path: &str) -> AppResult<()> {
        if !valid_relative_path(relative_path) {
            return Err(AppError::BadRequest("invalid storage path"));
        }
        let url = format!("https://{}/{}/{}", self.host, self.zone, relative_path);
        self.client
            .delete(url)
            .header("AccessKey", &self.api_key)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    /// Builds a tenant-scoped storage path with optional chapter, chapter-part and lesson folders.
    /// IDs provide stable isolation; slugs make the Bunny Storage zone easy to browse.
    pub fn course_content_path(
        company_id: &str,
        company_slug: &str,
        course_id: &str,
        course_slug: &str,
        subject_id: &str,
        subject_slug: &str,
        topic_id: &str,
        topic_slug: &str,
        chapter: Option<(&str, &str)>,
        chapter_part: Option<(&str, &str)>,
        lesson: Option<(&str, &str)>,
        category: &str,
        filename: &str,
    ) -> AppResult<String> {
        let base_parts = [
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
        if !base_parts.iter().all(|part| valid_path_segment(part)) {
            return Err(AppError::BadRequest("invalid storage path segment"));
        }
        if chapter_part.is_some() && chapter.is_none() {
            return Err(AppError::BadRequest("chapter-part folder requires a chapter"));
        }
        if lesson.is_some() && chapter_part.is_none() {
            return Err(AppError::BadRequest("lesson folder requires a chapter part"));
        }

        let mut path = format!(
            "companies/{}-{}/courses/{}-{}/subjects/{}-{}/parts/{}-{}/",
            company_id, company_slug, course_id, course_slug,
            subject_id, subject_slug, topic_id, topic_slug
        );
        if let Some((id, slug)) = chapter {
            if !valid_path_segment(id) || !valid_path_segment(slug) {
                return Err(AppError::BadRequest("invalid chapter path segment"));
            }
            path.push_str(&format!("chapters/{}-{}/", id, slug));
        }
        if let Some((id, slug)) = chapter_part {
            if !valid_path_segment(id) || !valid_path_segment(slug) {
                return Err(AppError::BadRequest("invalid chapter-part path segment"));
            }
            path.push_str(&format!("chapter-parts/{}-{}/", id, slug));
        }
        if let Some((id, slug)) = lesson {
            if !valid_path_segment(id) || !valid_path_segment(slug) {
                return Err(AppError::BadRequest("invalid lesson path segment"));
            }
            path.push_str(&format!("lessons/{}-{}/", id, slug));
        }
        path.push_str(category);
        path.push('/');
        path.push_str(filename);
        if !valid_relative_path(&path) {
            return Err(AppError::BadRequest("invalid storage path"));
        }
        Ok(path)
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


#[cfg(test)]
mod tests {
    use super::BunnyStorage;

    #[test]
    fn lesson_storage_path_contains_company_and_curriculum_hierarchy() {
        let path = BunnyStorage::course_content_path(
            "company123",
            "acme",
            "course123",
            "foundations",
            "subject123",
            "math",
            "part123",
            "algebra",
            Some(("chapter123", "equations")),
            Some(("chapterpart123", "worked-examples")),
            Some(("lesson123", "linear-equations")),
            "assets",
            "asset123.pdf",
        )
        .expect("valid hierarchy should create a path");

        assert_eq!(
            path,
            "companies/company123-acme/courses/course123-foundations/subjects/subject123-math/parts/part123-algebra/chapters/chapter123-equations/chapter-parts/chapterpart123-worked-examples/lessons/lesson123-linear-equations/assets/asset123.pdf"
        );
    }

    #[test]
    fn lesson_path_requires_a_chapter_part_parent() {
        let result = BunnyStorage::course_content_path(
            "company123", "acme", "course123", "foundations",
            "subject123", "math", "part123", "algebra",
            None, None, Some(("lesson123", "linear-equations")),
            "assets", "asset123.pdf",
        );
        assert!(result.is_err());
    }
}
