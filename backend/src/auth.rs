use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{FromRequestParts, State},
    http::{header, request::Parts},
    Json,
};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use mongodb::bson::{doc, oid::ObjectId, DateTime};
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    state::AppState,
    tenant::{ensure_learner_capacity, require_active_company},
};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: ObjectId,
    pub company_id: ObjectId,
    pub role: String,
}

impl AuthUser {
    pub fn can_manage_content(&self) -> bool {
        matches!(self.role.as_str(), "owner" | "admin" | "instructor")
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    #[serde(rename = "companyId")]
    company_id: String,
    role: String,
    iss: String,
    aud: String,
    iat: usize,
    exp: usize,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .filter(|value| !value.is_empty())
            .ok_or(AppError::Unauthorized)?;

        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[state.settings.jwt_issuer.as_str()]);
        validation.set_audience(&[state.settings.jwt_audience.as_str()]);
        validation.validate_exp = true;

        let decoded = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.settings.jwt_secret.as_bytes()),
            &validation,
        )
        .map_err(|_| AppError::Unauthorized)?;

        let claims = decoded.claims;
        let user_id = ObjectId::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;
        let company_id =
            ObjectId::parse_str(&claims.company_id).map_err(|_| AppError::Unauthorized)?;

        if !matches!(claims.role.as_str(), "owner" | "admin" | "instructor" | "learner") {
            return Err(AppError::Unauthorized);
        }

        // Re-check membership and role so disabled users and stale role tokens are revoked.
        require_active_company(&state.db, &company_id).await?;
        let stored_user = state
            .db
            .collection::<mongodb::bson::Document>("users")
            .find_one(doc! {
                "_id": user_id.clone(),
                "companyId": company_id.clone(),
                "status": "active",
            })
            .await?
            .ok_or(AppError::Unauthorized)?;
        let stored_role = stored_user
            .get_str("role")
            .map_err(|_| AppError::Unauthorized)?;
        if stored_role != claims.role {
            return Err(AppError::Unauthorized);
        }

        Ok(Self {
            user_id,
            company_id,
            role: stored_role.to_owned(),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    pub company_name: String,
    pub company_slug: String,
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterLearnerRequest {
    pub company_slug: String,
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub company_slug: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicCompany {
    pub id: String,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub user: PublicUser,
    pub company: PublicCompany,
}

pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterRequest>,
) -> AppResult<(axum::http::StatusCode, Json<AuthResponse>)> {
    let company_name = input.company_name.trim().to_owned();
    let company_slug = input.company_slug.trim().to_ascii_lowercase();
    let name = input.name.trim().to_owned();
    let email = input.email.trim().to_ascii_lowercase();

    if !(2..=120).contains(&company_name.chars().count())
        || !(2..=80).contains(&name.chars().count())
        || !valid_slug(&company_slug)
        || !valid_email(&email)
        || !(12..=128).contains(&input.password.chars().count())
    {
        return Err(AppError::BadRequest(
            "company, name, slug, email, or password does not meet validation rules",
        ));
    }

    let password_hash = hash_password(&input.password)?;
    let companies = state.db.collection::<mongodb::bson::Document>("companies");
    if companies
        .find_one(doc! { "slug": &company_slug })
        .await?
        .is_some()
    {
        return Err(AppError::Conflict("company slug is already in use"));
    }

    let company_id = ObjectId::new();
    let user_id = ObjectId::new();
    let now = DateTime::now();

    companies
        .insert_one(doc! {
            "_id": company_id.clone(),
            "name": &company_name,
            "slug": &company_slug,
            "status": "active",
            "planCode": "free",
            "createdAt": now,
            "updatedAt": now,
        })
        .await
        .map_err(AppError::from_mongo)?;

    let user_result = state
        .db
        .collection::<mongodb::bson::Document>("users")
        .insert_one(doc! {
            "_id": user_id.clone(),
            "companyId": company_id.clone(),
            "name": &name,
            "email": &email,
            "passwordHash": &password_hash,
            "role": "owner",
            "status": "active",
            "createdAt": now,
            "updatedAt": now,
        })
        .await;

    if let Err(error) = user_result {
        // Best-effort rollback so a failed owner insert does not leave an orphan company.
        let _ = companies.delete_one(doc! { "_id": company_id.clone() }).await;
        return Err(AppError::from_mongo(error));
    }

    let token = issue_access_token(&state, &user_id, &company_id, "owner")?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(AuthResponse {
            access_token: token,
            token_type: "Bearer",
            expires_in: state.settings.jwt_ttl_seconds,
            user: PublicUser {
                id: user_id.to_hex(),
                name,
                email,
                role: "owner".to_owned(),
            },
            company: PublicCompany {
                id: company_id.to_hex(),
                name: company_name,
                slug: company_slug,
            },
        }),
    ))
}

pub async fn register_learner(
    State(state): State<AppState>,
    Json(input): Json<RegisterLearnerRequest>,
) -> AppResult<(axum::http::StatusCode, Json<AuthResponse>)> {
    let company_slug = input.company_slug.trim().to_ascii_lowercase();
    let name = input.name.trim().to_owned();
    let email = input.email.trim().to_ascii_lowercase();

    if !valid_slug(&company_slug)
        || !(2..=80).contains(&name.chars().count())
        || !valid_email(&email)
        || !(12..=128).contains(&input.password.chars().count())
    {
        return Err(AppError::BadRequest(
            "company slug, name, email, or password does not meet validation rules",
        ));
    }

    let company = state
        .db
        .collection::<mongodb::bson::Document>("companies")
        .find_one(doc! { "slug": &company_slug, "status": "active" })
        .await?
        .ok_or(AppError::NotFound)?;

    if !company.get_bool("studentRegistrationEnabled").unwrap_or(true) {
        return Err(AppError::Forbidden);
    }
    let company_id = company
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let company = require_active_company(&state.db, &company_id).await?;
    ensure_learner_capacity(&state, &company, &company_id).await?;
    let user_id = ObjectId::new();
    let now = DateTime::now();
    let password_hash = hash_password(&input.password)?;

    state
        .db
        .collection::<mongodb::bson::Document>("users")
        .insert_one(doc! {
            "_id": user_id.clone(),
            "companyId": company_id.clone(),
            "name": &name,
            "email": &email,
            "passwordHash": &password_hash,
            "role": "learner",
            "status": "active",
            "createdAt": now,
            "updatedAt": now,
        })
        .await
        .map_err(AppError::from_mongo)?;

    let token = issue_access_token(&state, &user_id, &company_id, "learner")?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(AuthResponse {
            access_token: token,
            token_type: "Bearer",
            expires_in: state.settings.jwt_ttl_seconds,
            user: PublicUser {
                id: user_id.to_hex(),
                name,
                email,
                role: "learner".to_owned(),
            },
            company: PublicCompany {
                id: company_id.to_hex(),
                name: company.get_str("name").unwrap_or_default().to_owned(),
                slug: company_slug,
            },
        }),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    let company_slug = input.company_slug.trim().to_ascii_lowercase();
    let email = input.email.trim().to_ascii_lowercase();
    if !valid_slug(&company_slug) || !valid_email(&email) || input.password.is_empty() {
        return Err(AppError::Unauthorized);
    }

    let companies = state.db.collection::<mongodb::bson::Document>("companies");
    let company = companies
        .find_one(doc! { "slug": &company_slug, "status": "active" })
        .await?
        .ok_or(AppError::Unauthorized)?;
    let company_id = company
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_owned();

    let user = state
        .db
        .collection::<mongodb::bson::Document>("users")
        .find_one(doc! {
            "companyId": company_id.clone(),
            "email": &email,
            "status": "active",
        })
        .await?
        .ok_or(AppError::Unauthorized)?;

    let stored_hash = user
        .get_str("passwordHash")
        .map_err(|_| AppError::Internal)?;
    if !verify_password(&input.password, stored_hash)? {
        return Err(AppError::Unauthorized);
    }

    let user_id = user
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let role = user
        .get_str("role")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    if !matches!(role.as_str(), "owner" | "admin" | "instructor" | "learner") {
        return Err(AppError::Unauthorized);
    }

    let token = issue_access_token(&state, &user_id, &company_id, &role)?;
    Ok(Json(AuthResponse {
        access_token: token,
        token_type: "Bearer",
        expires_in: state.settings.jwt_ttl_seconds,
        user: PublicUser {
            id: user_id.to_hex(),
            name: user.get_str("name").unwrap_or_default().to_owned(),
            email,
            role,
        },
        company: PublicCompany {
            id: company_id.to_hex(),
            name: company.get_str("name").unwrap_or_default().to_owned(),
            slug: company.get_str("slug").unwrap_or_default().to_owned(),
        },
    }))
}

pub async fn me(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let company = require_active_company(&state.db, &auth.company_id).await?;
    let user = state
        .db
        .collection::<mongodb::bson::Document>("users")
        .find_one(doc! {
            "_id": auth.user_id.clone(),
            "companyId": auth.company_id.clone(),
            "status": "active",
        })
        .await?
        .ok_or(AppError::Unauthorized)?;

    Ok(Json(serde_json::json!({
        "user": {
            "id": auth.user_id.to_hex(),
            "name": user.get_str("name").unwrap_or_default(),
            "email": user.get_str("email").unwrap_or_default(),
            "role": auth.role,
        },
        "company": {
            "id": auth.company_id.to_hex(),
            "name": company.get_str("name").unwrap_or_default(),
            "slug": company.get_str("slug").unwrap_or_default(),
            "status": company.get_str("status").unwrap_or_default(),
        }
    })))
}

fn issue_access_token(
    state: &AppState,
    user_id: &ObjectId,
    company_id: &ObjectId,
    role: &str,
) -> AppResult<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| AppError::Internal)?
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_hex(),
        company_id: company_id.to_hex(),
        role: role.to_owned(),
        iss: state.settings.jwt_issuer.clone(),
        aud: state.settings.jwt_audience.clone(),
        iat: now,
        exp: now.saturating_add(state.settings.jwt_ttl_seconds as usize),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(state.settings.jwt_secret.as_bytes()),
    )
    .map_err(|_| AppError::Internal)
}

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::Internal)
}

fn verify_password(password: &str, encoded_hash: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(encoded_hash).map_err(|_| AppError::Internal)?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn valid_slug(value: &str) -> bool {
    (2..=80).contains(&value.len())
        && value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        && !value.starts_with('-')
        && !value.ends_with('-')
        && !value.contains("--")
}

pub fn valid_email(value: &str) -> bool {
    value.len() <= 254
        && value
            .split_once('@')
            .is_some_and(|(local, domain)| {
                !local.is_empty()
                    && domain.contains('.')
                    && !domain.starts_with('.')
                    && !domain.ends_with('.')
            })
}

#[cfg(test)]
mod tests {
    use super::{valid_email, valid_slug};

    #[test]
    fn slug_validation_rejects_path_and_case_tricks() {
        assert!(valid_slug("alpha-academy"));
        assert!(!valid_slug("../alpha"));
        assert!(!valid_slug("Alpha"));
        assert!(!valid_slug("alpha--academy"));
    }

    #[test]
    fn email_validation_rejects_obvious_invalid_inputs() {
        assert!(valid_email("owner@example.com"));
        assert!(!valid_email("not-an-email"));
        assert!(!valid_email("@example.com"));
    }
}
