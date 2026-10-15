use crate::config::Settings;
use mongodb::Database;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub settings: Arc<Settings>,
    pub http: reqwest::Client,
}
