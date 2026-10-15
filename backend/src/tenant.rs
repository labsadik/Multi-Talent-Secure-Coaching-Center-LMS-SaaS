use mongodb::{bson::{doc, oid::ObjectId, Document}, Database};

use crate::error::{AppError, AppResult};

pub async fn require_active_company(db: &Database, company_id: &ObjectId) -> AppResult<Document> {
    let company = db
        .collection::<Document>("companies")
        .find_one(doc! { "_id": company_id.clone() })
        .await?
        .ok_or(AppError::Forbidden)?;

    if company.get_str("status").unwrap_or_default() != "active" {
        return Err(AppError::Forbidden);
    }

    Ok(company)
}
