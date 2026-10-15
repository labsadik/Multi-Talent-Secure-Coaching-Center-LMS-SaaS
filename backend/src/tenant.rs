use mongodb::{
    bson::{doc, oid::ObjectId, DateTime, Document},
    Database,
};

use crate::{
    config::Settings,
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Clone, Copy)]
pub struct PlanLimits {
    pub max_courses: u64,
    pub max_learners: u64,
}

pub fn limits_for_plan(settings: &Settings, plan_code: &str) -> AppResult<PlanLimits> {
    let (max_courses, max_learners) = match plan_code {
        "free" => (settings.plan_free_max_courses, settings.plan_free_max_learners),
        "starter" => (settings.plan_starter_max_courses, settings.plan_starter_max_learners),
        "pro" => (settings.plan_pro_max_courses, settings.plan_pro_max_learners),
        "business" => (settings.plan_business_max_courses, settings.plan_business_max_learners),
        _ => return Err(AppError::Internal),
    };

    Ok(PlanLimits { max_courses, max_learners })
}

pub async fn require_active_company(db: &Database, company_id: &ObjectId) -> AppResult<Document> {
    let companies = db.collection::<Document>("companies");
    let mut company = companies
        .find_one(doc! { "_id": company_id.clone() })
        .await?
        .ok_or(AppError::Forbidden)?;

    if company.get_str("status").unwrap_or_default() != "active" {
        return Err(AppError::Forbidden);
    }

    let plan_code = company.get_str("planCode").unwrap_or("free");
    let expiry = company.get_datetime("subscriptionExpiresAt").ok().cloned();
    if plan_code != "free"
        && expiry.is_some_and(|date| date.timestamp_millis() <= DateTime::now().timestamp_millis())
    {
        companies
            .update_one(
                doc! { "_id": company_id.clone(), "planCode": plan_code },
                doc! {
                    "$set": {
                        "planCode": "free",
                        "subscriptionStatus": "expired",
                        "updatedAt": DateTime::now(),
                    }
                },
            )
            .await?;

        company = companies
            .find_one(doc! { "_id": company_id.clone(), "status": "active" })
            .await?
            .ok_or(AppError::Forbidden)?;
    }

    Ok(company)
}

pub async fn ensure_learner_capacity(
    state: &AppState,
    company: &Document,
    company_id: &ObjectId,
) -> AppResult<()> {
    let plan_code = company.get_str("planCode").unwrap_or("free");
    let limits = limits_for_plan(&state.settings, plan_code)?;
    let active_learners = state
        .db
        .collection::<Document>("users")
        .count_documents(doc! {
            "companyId": company_id.clone(),
            "role": "learner",
            "status": "active",
        })
        .await?;

    if active_learners >= limits.max_learners {
        return Err(AppError::Conflict("learner limit reached for the current plan"));
    }
    Ok(())
}
