use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use futures_util::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use serde::{Deserialize, Serialize};

use crate::{
    auth::{hash_password, valid_email, AuthUser},
    error::{AppError, AppResult},
    state::AppState,
    tenant::require_active_company,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateCompanyUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCompanyUserRequest {
    pub role: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompanyUserResponse {
    id: String,
    name: String,
    email: String,
    role: String,
    status: String,
    created_at_ms: i64,
    last_login_at_ms: Option<i64>,
}

fn user_response(document: Document) -> AppResult<CompanyUserResponse> {
    Ok(CompanyUserResponse {
        id: document
            .get_object_id("_id")
            .map_err(|_| AppError::Internal)?
            .to_hex(),
        name: document
            .get_str("name")
            .map(str::to_owned)
            .map_err(|_| AppError::Internal)?,
        email: document
            .get_str("email")
            .map(str::to_owned)
            .map_err(|_| AppError::Internal)?,
        role: document
            .get_str("role")
            .map(str::to_owned)
            .map_err(|_| AppError::Internal)?,
        status: document
            .get_str("status")
            .unwrap_or("active")
            .to_owned(),
        created_at_ms: document
            .get_datetime("createdAt")
            .map(|date| date.timestamp_millis())
            .unwrap_or_default(),
        last_login_at_ms: document
            .get_datetime("lastLoginAt")
            .ok()
            .map(|date| date.timestamp_millis()),
    })
}

fn require_admin(auth: &AuthUser) -> AppResult<()> {
    if matches!(auth.role.as_str(), "owner" | "admin") {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub async fn list_company_users(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Vec<CompanyUserResponse>>> {
    require_admin(&auth)?;
    require_active_company(&state.db, &auth.company_id).await?;

    let cursor = state
        .db
        .collection::<Document>("users")
        .find(doc! { "companyId": auth.company_id.clone() })
        .sort(doc! { "createdAt": -1 })
        .limit(500)
        .await?;

    let documents: Vec<Document> = cursor.try_collect().await.map_err(AppError::from_mongo)?;
    let users = documents
        .into_iter()
        .map(user_response)
        .collect::<AppResult<Vec<_>>>()?;
    Ok(Json(users))
}

pub async fn create_company_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateCompanyUserRequest>,
) -> AppResult<(StatusCode, Json<CompanyUserResponse>)> {
    require_admin(&auth)?;
    require_active_company(&state.db, &auth.company_id).await?;

    let name = input.name.trim().to_owned();
    let email = input.email.trim().to_ascii_lowercase();
    let role = input.role.trim().to_ascii_lowercase();

    if !(2..=80).contains(&name.chars().count())
        || !valid_email(&email)
        || !(12..=128).contains(&input.password.chars().count())
        || !matches!(role.as_str(), "admin" | "instructor" | "learner")
    {
        return Err(AppError::BadRequest(
            "name, email, password, or role does not meet validation rules",
        ));
    }
    if role == "admin" && auth.role != "owner" {
        return Err(AppError::Forbidden);
    }

    let user_id = ObjectId::new();
    let now = DateTime::now();
    let password_hash = hash_password(&input.password)?;

    let document = doc! {
        "_id": user_id,
        "companyId": auth.company_id.clone(),
        "name": name,
        "email": email,
        "passwordHash": password_hash,
        "role": role,
        "status": "active",
        "createdBy": auth.user_id.clone(),
        "createdAt": now,
        "updatedAt": now,
    };

    state
        .db
        .collection::<Document>("users")
        .insert_one(document.clone())
        .await
        .map_err(AppError::from_mongo)?;

    Ok((StatusCode::CREATED, Json(user_response(document)?)))
}

pub async fn update_company_user(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    auth: AuthUser,
    Json(input): Json<UpdateCompanyUserRequest>,
) -> AppResult<Json<CompanyUserResponse>> {
    require_admin(&auth)?;
    require_active_company(&state.db, &auth.company_id).await?;

    let user_id =
        ObjectId::parse_str(&user_id).map_err(|_| AppError::BadRequest("invalid user ID"))?;
    if user_id == auth.user_id {
        return Err(AppError::BadRequest("users cannot change their own role or status here"));
    }

    let users = state.db.collection::<Document>("users");
    let target = users
        .find_one(doc! {
            "_id": user_id.clone(),
            "companyId": auth.company_id.clone(),
        })
        .await?
        .ok_or(AppError::NotFound)?;
    let target_role = target.get_str("role").unwrap_or_default();
    if target_role == "owner" || (target_role == "admin" && auth.role != "owner") {
        return Err(AppError::Forbidden);
    }

    let mut set_fields = doc! { "updatedAt": DateTime::now() };
    if let Some(role) = input.role {
        let role = role.trim().to_ascii_lowercase();
        if !matches!(role.as_str(), "admin" | "instructor" | "learner") {
            return Err(AppError::BadRequest("role must be admin, instructor, or learner"));
        }
        if role == "admin" && auth.role != "owner" {
            return Err(AppError::Forbidden);
        }
        set_fields.insert("role", role);
    }

    if let Some(status) = input.status {
        let status = status.trim().to_ascii_lowercase();
        if !matches!(status.as_str(), "active" | "disabled") {
            return Err(AppError::BadRequest("status must be active or disabled"));
        }
        set_fields.insert("status", status);
    }

    if set_fields.len() == 1 {
        return Err(AppError::BadRequest("at least one role or status change is required"));
    }

    users
        .update_one(
            doc! {
                "_id": user_id.clone(),
                "companyId": auth.company_id.clone(),
                "role": { "$ne": "owner" },
            },
            doc! { "$set": set_fields },
        )
        .await?;

    let updated = users
        .find_one(doc! {
            "_id": user_id,
            "companyId": auth.company_id.clone(),
        })
        .await?
        .ok_or(AppError::NotFound)?;

    Ok(Json(user_response(updated)?))
}

