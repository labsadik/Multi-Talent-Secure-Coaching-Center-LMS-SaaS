mod auth;
mod config;
mod courses;
mod crypto;
mod db;
mod error;
mod health;
mod integrations;
mod media;
mod payments;
mod progress;
mod learning;
mod routes;
mod security;
mod state;
mod tenant;

use anyhow::Context;
use axum::{
    extract::DefaultBodyLimit,
    http::{header, HeaderValue, Method},
    middleware,
};
use mongodb::{options::ClientOptions, Client};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::{config::Settings, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Arc::new(Settings::from_env().context("configuration loading failed")?);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(if settings.app_env == "production" {
            "info,tower_http=info"
        } else {
            "debug,tower_http=info"
        })
    });
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(filter)
        .with_target(true)
        .init();

    let mut mongo_options = ClientOptions::parse(&settings.mongodb_uri)
        .await
        .context("could not parse MONGODB_URI")?;
    mongo_options.app_name = Some("newage-rust-api".to_owned());
    mongo_options.max_pool_size = Some(settings.mongodb_max_pool_size);

    let mongo_client = Client::with_options(mongo_options).context("MongoDB client setup failed")?;
    let db = mongo_client.database(&settings.mongodb_database);
    db.run_command(mongodb::bson::doc! { "ping": 1 })
        .await
        .context("MongoDB readiness check failed")?;
    db::ensure_indexes(&db)
        .await
        .context("MongoDB index initialization failed")?;

    let http_client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(settings.http_client_timeout_seconds))
        .timeout(Duration::from_secs(settings.http_client_timeout_seconds))
        .pool_idle_timeout(Duration::from_secs(90))
        .user_agent(concat!("newage-api/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("HTTP client setup failed")?;

    let state = AppState {
        db,
        settings: Arc::clone(&settings),
        http: http_client,
    };

    let origins = settings
        .cors_allowed_origins
        .iter()
        .map(|origin| HeaderValue::from_str(origin))
        .collect::<Result<Vec<_>, _>>()
        .context("CORS_ALLOWED_ORIGINS contains an invalid HTTP header value")?;

    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_credentials(true)
        .max_age(Duration::from_secs(600));

    let app = routes::router(state)
        .layer(DefaultBodyLimit::max(settings.max_request_body_bytes))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            security::RateLimiter::default(),
            security::rate_limit,
        ))
        .layer(middleware::from_fn(security::secure_headers));

    let listener = TcpListener::bind((settings.host.as_str(), settings.port))
        .await
        .with_context(|| format!("could not bind API listener on {}:{}", settings.host, settings.port))?;

    tracing::info!(
        host = %settings.host,
        port = settings.port,
        environment = %settings.app_env,
        "Newage Rust API listening"
    );

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutdown signal received");
        })
        .await
        .context("HTTP server failed")?;

    Ok(())
}
