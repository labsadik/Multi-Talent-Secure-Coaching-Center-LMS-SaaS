use anyhow::{ensure, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::env;

#[derive(Clone)]
pub struct Settings {
    pub app_env: String,
    pub host: String,
    pub port: u16,
    pub mongodb_uri: String,
    pub mongodb_database: String,
    pub mongodb_max_pool_size: u32,
    pub jwt_secret: String,
    pub jwt_issuer: String,
    pub jwt_audience: String,
    pub jwt_ttl_seconds: u64,
    pub cors_allowed_origins: Vec<String>,
    pub max_request_body_bytes: usize,
    pub max_asset_upload_bytes: usize,
    pub http_client_timeout_seconds: u64,
    pub bunny_tus_endpoint: String,
    pub bunny_upload_signature_ttl_seconds: u64,
    pub bunny_stream_token_auth_key: Option<String>,
    pub bunny_playback_token_ttl_seconds: u64,
    pub app_encryption_key: Option<[u8; 32]>,
    pub bunny_storage_zone: Option<String>,
    pub bunny_storage_api_key: Option<String>,
    pub bunny_storage_host: String,
    pub bunny_cdn_hostname: Option<String>,
    pub bunny_stream_library_id: Option<u64>,
    pub bunny_stream_api_key: Option<String>,
    pub razorpay_key_id: Option<String>,
    pub razorpay_key_secret: Option<String>,
    pub razorpay_webhook_secret: Option<String>,
    pub plan_starter_monthly_price_paise: i64,
    pub plan_pro_monthly_price_paise: i64,
    pub plan_business_monthly_price_paise: i64,
}

fn required(key: &'static str) -> Result<String> {
    let value = env::var(key).with_context(|| format!("missing required environment variable {key}"))?;
    ensure!(!value.trim().is_empty(), "{key} must not be empty");
    Ok(value.trim().to_owned())
}

fn optional(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_or<T>(key: &'static str, default: T) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match optional(key) {
        Some(value) => value
            .parse::<T>()
            .map_err(|_| anyhow::anyhow!("{key} has an invalid value")),
        None => Ok(default),
    }
}

impl Settings {
    pub fn from_env() -> Result<Self> {
        // Existing process environment variables take precedence over values in .env.
        let _ = dotenvy::dotenv();

        let app_env = optional("APP_ENV").unwrap_or_else(|| "development".to_owned());
        ensure!(
            matches!(app_env.as_str(), "development" | "test" | "production"),
            "APP_ENV must be development, test, or production"
        );

        let host = optional("HOST").unwrap_or_else(|| "127.0.0.1".to_owned());
        let port = parse_or("PORT", 8080_u16)?;
        let mongodb_uri = required("MONGODB_URI")?;
        let mongodb_database = required("MONGODB_DATABASE")?;
        let mongodb_max_pool_size = parse_or("MONGODB_MAX_POOL_SIZE", 50_u32)?;
        ensure!(
            (1..=500).contains(&mongodb_max_pool_size),
            "MONGODB_MAX_POOL_SIZE must be between 1 and 500"
        );

        let jwt_secret = required("JWT_SECRET")?;
        ensure!(
            jwt_secret.as_bytes().len() >= 32,
            "JWT_SECRET must be at least 32 bytes"
        );
        if app_env == "production" {
            ensure!(
                !jwt_secret.to_ascii_lowercase().contains("change-this")
                    && !jwt_secret.to_ascii_lowercase().contains("development"),
                "JWT_SECRET must be replaced with a production secret"
            );
        }
        let jwt_issuer = required("JWT_ISSUER")?;
        let jwt_audience = required("JWT_AUDIENCE")?;
        let jwt_ttl_seconds = parse_or("JWT_TTL_SECONDS", 900_u64)?;
        ensure!(
            (60..=86_400).contains(&jwt_ttl_seconds),
            "JWT_TTL_SECONDS must be between 60 and 86400"
        );

        let cors_raw = required("CORS_ALLOWED_ORIGINS")?;
        let cors_allowed_origins: Vec<String> = cors_raw
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(str::to_owned)
            .collect();
        ensure!(!cors_allowed_origins.is_empty(), "CORS_ALLOWED_ORIGINS must contain at least one origin");
        for origin in &cors_allowed_origins {
            ensure!(
                !origin.contains('*')
                    && (origin.starts_with("http://") || origin.starts_with("https://"))
                    && !origin.ends_with('/'),
                "CORS origins must be exact http(s) origins without a trailing slash or wildcard"
            );
        }

        let max_request_body_bytes = parse_or("MAX_REQUEST_BODY_BYTES", 26_214_400_usize)?;
        ensure!(
            (1_024..=50 * 1_024 * 1_024).contains(&max_request_body_bytes),
            "MAX_REQUEST_BODY_BYTES must be between 1024 and 52428800"
        );
        let max_asset_upload_bytes = parse_or("MAX_ASSET_UPLOAD_BYTES", 20 * 1_024 * 1_024_usize)?;
        ensure!(
            (1_024..=50 * 1_024 * 1_024).contains(&max_asset_upload_bytes)
                && max_asset_upload_bytes <= max_request_body_bytes,
            "MAX_ASSET_UPLOAD_BYTES must be between 1024 and MAX_REQUEST_BODY_BYTES"
        );
        let http_client_timeout_seconds = parse_or("HTTP_CLIENT_TIMEOUT_SECONDS", 15_u64)?;
        ensure!(
            (1..=120).contains(&http_client_timeout_seconds),
            "HTTP_CLIENT_TIMEOUT_SECONDS must be between 1 and 120"
        );

        let app_encryption_key = match optional("APP_ENCRYPTION_KEY_BASE64") {
            Some(encoded) => {
                let decoded = STANDARD
                    .decode(encoded)
                    .context("APP_ENCRYPTION_KEY_BASE64 must be valid base64")?;
                let key: [u8; 32] = decoded
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("APP_ENCRYPTION_KEY_BASE64 must decode to exactly 32 bytes"))?;
                Some(key)
            }
            None => None,
        };
        if app_env == "production" {
            ensure!(
                app_encryption_key.is_some(),
                "APP_ENCRYPTION_KEY_BASE64 is required in production"
            );
        }

        let bunny_storage_zone = optional("BUNNY_STORAGE_ZONE");
        let bunny_storage_api_key = optional("BUNNY_STORAGE_API_KEY");
        ensure!(
            bunny_storage_zone.is_some() == bunny_storage_api_key.is_some(),
            "BUNNY_STORAGE_ZONE and BUNNY_STORAGE_API_KEY must be configured together"
        );
        if let Some(zone) = &bunny_storage_zone {
            ensure!(
                safe_single_path_segment(zone),
                "BUNNY_STORAGE_ZONE contains unsupported characters"
            );
        }

        let bunny_storage_host =
            optional("BUNNY_STORAGE_HOST").unwrap_or_else(|| "storage.bunnycdn.com".to_owned());
        ensure!(
            !bunny_storage_host.contains("://")
                && !bunny_storage_host.contains('/')
                && !bunny_storage_host.trim().is_empty()
                && !bunny_storage_host.starts_with('.')
                && bunny_storage_host.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '.' || ch == '-'),
            "BUNNY_STORAGE_HOST must be a hostname, not a URL"
        );
        let bunny_cdn_hostname = optional("BUNNY_CDN_HOSTNAME");

        let bunny_stream_library_id = match optional("BUNNY_STREAM_LIBRARY_ID") {
            Some(value) => Some(
                value
                    .parse::<u64>()
                    .map_err(|_| anyhow::anyhow!("BUNNY_STREAM_LIBRARY_ID must be a positive integer"))?,
            ),
            None => None,
        };
        ensure!(
            bunny_stream_library_id.is_some() == optional("BUNNY_STREAM_API_KEY").is_some(),
            "BUNNY_STREAM_LIBRARY_ID and BUNNY_STREAM_API_KEY must be configured together"
        );
        let bunny_stream_api_key = optional("BUNNY_STREAM_API_KEY");
        let bunny_tus_endpoint = optional("BUNNY_TUS_ENDPOINT")
            .unwrap_or_else(|| "https://video.bunnycdn.com/tusupload".to_owned());
        ensure!(
            bunny_tus_endpoint.starts_with("https://")
                && !bunny_tus_endpoint.chars().any(char::is_whitespace),
            "BUNNY_TUS_ENDPOINT must be an HTTPS endpoint"
        );
        let bunny_upload_signature_ttl_seconds =
            parse_or("BUNNY_UPLOAD_SIGNATURE_TTL_SECONDS", 21_600_u64)?;
        ensure!(
            (3_600..=86_400).contains(&bunny_upload_signature_ttl_seconds),
            "BUNNY_UPLOAD_SIGNATURE_TTL_SECONDS must be between 3600 and 86400"
        );
        let bunny_stream_token_auth_key = optional("BUNNY_STREAM_TOKEN_AUTH_KEY");
        let bunny_playback_token_ttl_seconds =
            parse_or("BUNNY_PLAYBACK_TOKEN_TTL_SECONDS", 3_600_u64)?;
        ensure!(
            (60..=86_400).contains(&bunny_playback_token_ttl_seconds),
            "BUNNY_PLAYBACK_TOKEN_TTL_SECONDS must be between 60 and 86400"
        );
        if app_env == "production" && bunny_stream_library_id.is_some() {
            ensure!(
                bunny_stream_token_auth_key.is_some(),
                "BUNNY_STREAM_TOKEN_AUTH_KEY is required in production when Bunny Stream is enabled"
            );
        }

        let razorpay_key_id = optional("RAZORPAY_KEY_ID");
        let razorpay_key_secret = optional("RAZORPAY_KEY_SECRET");
        ensure!(
            razorpay_key_id.is_some() == razorpay_key_secret.is_some(),
            "RAZORPAY_KEY_ID and RAZORPAY_KEY_SECRET must be configured together"
        );
        let razorpay_webhook_secret = optional("RAZORPAY_WEBHOOK_SECRET");
        ensure!(
            razorpay_webhook_secret.is_none() || razorpay_key_id.is_some(),
            "RAZORPAY_WEBHOOK_SECRET requires platform Razorpay credentials"
        );

        let plan_starter_monthly_price_paise =
            parse_or("PLAN_STARTER_MONTHLY_PRICE_PAISE", 99_900_i64)?;
        let plan_pro_monthly_price_paise =
            parse_or("PLAN_PRO_MONTHLY_PRICE_PAISE", 199_900_i64)?;
        let plan_business_monthly_price_paise =
            parse_or("PLAN_BUSINESS_MONTHLY_PRICE_PAISE", 499_900_i64)?;
        for (name, amount) in [
            ("PLAN_STARTER_MONTHLY_PRICE_PAISE", plan_starter_monthly_price_paise),
            ("PLAN_PRO_MONTHLY_PRICE_PAISE", plan_pro_monthly_price_paise),
            ("PLAN_BUSINESS_MONTHLY_PRICE_PAISE", plan_business_monthly_price_paise),
        ] {
            ensure!(
                (1..=100_000_000).contains(&amount),
                "{name} must be between 1 and 100000000 paise"
            );
        }

        if app_env == "production" {
            ensure!(
                cors_allowed_origins.iter().all(|origin| origin.starts_with("https://")),
                "production CORS origins must use HTTPS"
            );
            if razorpay_key_id.is_some() {
                ensure!(
                    razorpay_webhook_secret.is_some(),
                    "RAZORPAY_WEBHOOK_SECRET is required in production when platform Razorpay is enabled"
                );
            }
        }

        Ok(Self {
            app_env,
            host,
            port,
            mongodb_uri,
            mongodb_database,
            mongodb_max_pool_size,
            jwt_secret,
            jwt_issuer,
            jwt_audience,
            jwt_ttl_seconds,
            cors_allowed_origins,
            max_request_body_bytes,
            max_asset_upload_bytes,
            http_client_timeout_seconds,
            bunny_tus_endpoint,
            bunny_upload_signature_ttl_seconds,
            bunny_stream_token_auth_key,
            bunny_playback_token_ttl_seconds,
            app_encryption_key,
            bunny_storage_zone,
            bunny_storage_api_key,
            bunny_storage_host,
            bunny_cdn_hostname,
            bunny_stream_library_id,
            bunny_stream_api_key,
            razorpay_key_id,
            razorpay_key_secret,
            razorpay_webhook_secret,
            plan_starter_monthly_price_paise,
            plan_pro_monthly_price_paise,
            plan_business_monthly_price_paise,
        })
    }
}

fn safe_single_path_segment(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "-_.".contains(ch))
}
