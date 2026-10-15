use mongodb::{
    bson::{doc, oid::ObjectId, Document},
    Database,
};

use crate::error::{AppError, AppResult};

/// Resolve a tenant workspace and reject access when it has been disabled.
/// Course/learner access no longer depends on a paid platform plan.
pub async fn require_active_company(db: &Database, company_id: &ObjectId) -> AppResult<Document> {
    db.collection::<Document>("companies")
        .find_one(doc! { "_id": company_id.clone(), "status": "active" })
        .await?
        .ok_or(AppError::Forbidden)
}
