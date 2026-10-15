use axum::{
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use hmac::{Hmac, Mac};
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;

use crate::{
    auth::AuthUser,
    crypto::{decrypt_tenant_secret, encrypt_tenant_secret},
    error::{AppError, AppResult},
    integrations::razorpay::RazorpayClient,
    state::AppState,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureRazorpayRequest {
    pub key_id: String,
    pub key_secret: String,
    pub webhook_secret: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyPaymentRequest {
    pub razorpay_order_id: String,
    pub razorpay_payment_id: String,
    pub razorpay_signature: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RazorpaySettingsResponse {
    pub configured: bool,
    pub key_id: Option<String>,
}

fn encryption_key(state: &AppState) -> AppResult<&[u8; 32]> {
    state
        .settings
        .app_encryption_key
        .as_ref()
        .ok_or(AppError::ServiceUnavailable("application encryption key"))
}

fn validate_key_id(value: &str) -> bool {
    (value.starts_with("rzp_test_") || value.starts_with("rzp_live_"))
        && (12..=100).contains(&value.len())
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "_-".contains(ch))
}

fn reveal_secret(company: &Document, field: &str, company_id: &ObjectId, key: &[u8; 32]) -> AppResult<String> {
    let encoded = company.get_str(field).map_err(|_| AppError::ServiceUnavailable("company Razorpay settings"))?;
    let plaintext = decrypt_tenant_secret(encoded, &company_id.to_hex(), key)?;
    String::from_utf8(plaintext).map_err(|_| AppError::Internal)
}

async fn load_company(state: &AppState, company_id: &ObjectId) -> AppResult<Document> {
    state
        .db
        .collection::<Document>("companies")
        .find_one(doc! { "_id": company_id.clone(), "status": "active" })
        .await?
        .ok_or(AppError::Forbidden)
}

fn hmac_matches(secret: &[u8], message: &[u8], signature_hex: &str) -> bool {
    let Ok(signature) = hex::decode(signature_hex.trim()) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(secret) else {
        return false;
    };
    mac.update(message);
    mac.verify_slice(&signature).is_ok()
}

pub async fn configure_razorpay(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<ConfigureRazorpayRequest>,
) -> AppResult<Json<RazorpaySettingsResponse>> {
    if auth.role != "owner" && auth.role != "admin" {
        return Err(AppError::Forbidden);
    }

    let key_id = input.key_id.trim().to_owned();
    let key_secret = input.key_secret.trim();
    let webhook_secret = input.webhook_secret.trim();
    if !validate_key_id(&key_id)
        || !(8..=256).contains(&key_secret.len())
        || !(8..=256).contains(&webhook_secret.len())
    {
        return Err(AppError::BadRequest("invalid Razorpay key or webhook secret"));
    }

    let key = encryption_key(&state)?;
    let company_id = auth.company_id.to_hex();
    let encrypted_key_secret = encrypt_tenant_secret(key_secret.as_bytes(), &company_id, key)?;
    let encrypted_webhook_secret =
        encrypt_tenant_secret(webhook_secret.as_bytes(), &company_id, key)?;

    state
        .db
        .collection::<Document>("companies")
        .update_one(
            doc! { "_id": auth.company_id.clone(), "status": "active" },
            doc! {
                "$set": {
                    "razorpayKeyId": &key_id,
                    "razorpaySecretEncrypted": encrypted_key_secret,
                    "razorpayWebhookSecretEncrypted": encrypted_webhook_secret,
                    "razorpayConfiguredAt": DateTime::now(),
                    "updatedAt": DateTime::now(),
                }
            },
        )
        .await?;

    Ok(Json(RazorpaySettingsResponse {
        configured: true,
        key_id: Some(key_id),
    }))
}

pub async fn get_razorpay_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<RazorpaySettingsResponse>> {
    if auth.role != "owner" && auth.role != "admin" {
        return Err(AppError::Forbidden);
    }
    let company = load_company(&state, &auth.company_id).await?;
    let key_id = company.get_str("razorpayKeyId").ok().map(str::to_owned);
    let configured = key_id.is_some()
        && company.get_str("razorpaySecretEncrypted").is_ok()
        && company.get_str("razorpayWebhookSecretEncrypted").is_ok();

    Ok(Json(RazorpaySettingsResponse { configured, key_id }))
}

pub async fn create_course_order(
    State(state): State<AppState>,
    Path(course_id): Path<String>,
    auth: AuthUser,
) -> AppResult<(axum::http::StatusCode, Json<Value>)> {
    if auth.role != "learner" {
        return Err(AppError::Forbidden);
    }
    let course_id =
        ObjectId::parse_str(&course_id).map_err(|_| AppError::BadRequest("invalid course ID"))?;
    let course = state
        .db
        .collection::<Document>("courses")
        .find_one(doc! {
            "_id": course_id.clone(),
            "companyId": auth.company_id.clone(),
            "status": "published",
            "deletedAt": { "$exists": false },
        })
        .await?
        .ok_or(AppError::NotFound)?;

    let amount = course.get_i64("priceInPaise").unwrap_or(0);
    if !(0..=100_000_000).contains(&amount) {
        return Err(AppError::BadRequest("course price is outside the supported range"));
    }

    if amount == 0 {
        enroll(
            &state,
            &auth.company_id,
            &course_id,
            &auth.user_id,
            "free",
            None,
        )
        .await?;
        return Ok((
            axum::http::StatusCode::OK,
            Json(json!({
                "enrolled": true,
                "paymentRequired": false,
                "courseId": course_id.to_hex()
            })),
        ));
    }

    let company = load_company(&state, &auth.company_id).await?;
    let key_id = company
        .get_str("razorpayKeyId")
        .map(str::to_owned)
        .map_err(|_| AppError::ServiceUnavailable("company Razorpay settings"))?;
    let key = encryption_key(&state)?;
    let key_secret = reveal_secret(&company, "razorpaySecretEncrypted", &auth.company_id, key)?;
    let razorpay = RazorpayClient::from_credentials(state.http.clone(), key_id.clone(), key_secret)?;
    let internal_order_id = ObjectId::new();
    let receipt = internal_order_id.to_hex();
    let provider_order = razorpay.create_inr_order(amount, &receipt).await?;
    let now = DateTime::now();

    state
        .db
        .collection::<Document>("orders")
        .insert_one(doc! {
            "_id": internal_order_id.clone(),
            "companyId": auth.company_id.clone(),
            "courseId": course_id.clone(),
            "learnerId": auth.user_id.clone(),
            "amountInPaise": amount,
            "currency": "INR",
            "provider": "razorpay",
            "razorpayOrderId": &provider_order.id,
            "status": "created",
            "receipt": receipt,
            "createdAt": now,
            "updatedAt": now,
        })
        .await
        .map_err(AppError::from_mongo)?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(json!({
            "enrolled": false,
            "paymentRequired": true,
            "orderId": internal_order_id.to_hex(),
            "razorpayOrderId": provider_order.id,
            "amount": provider_order.amount,
            "currency": provider_order.currency,
            "keyId": key_id,
        })),
    ))
}

pub async fn verify_course_payment(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<VerifyPaymentRequest>,
) -> AppResult<Json<Value>> {
    if auth.role != "learner" {
        return Err(AppError::Forbidden);
    }
    if input.razorpay_order_id.is_empty() || input.razorpay_payment_id.is_empty() {
        return Err(AppError::BadRequest("Razorpay order and payment IDs are required"));
    }

    let order = state
        .db
        .collection::<Document>("orders")
        .find_one(doc! {
            "companyId": auth.company_id.clone(),
            "learnerId": auth.user_id.clone(),
            "razorpayOrderId": &input.razorpay_order_id,
            "provider": "razorpay",
        })
        .await?
        .ok_or(AppError::NotFound)?;

    let company = load_company(&state, &auth.company_id).await?;
    let key = encryption_key(&state)?;
    let key_secret = reveal_secret(&company, "razorpaySecretEncrypted", &auth.company_id, key)?;
    let message = format!("{}|{}", input.razorpay_order_id, input.razorpay_payment_id);
    if !hmac_matches(key_secret.as_bytes(), message.as_bytes(), &input.razorpay_signature) {
        return Err(AppError::Unauthorized);
    }

    let key_id = company
        .get_str("razorpayKeyId")
        .map(str::to_owned)
        .map_err(|_| AppError::ServiceUnavailable("company Razorpay settings"))?;
    let razorpay = RazorpayClient::from_credentials(state.http.clone(), key_id, key_secret)?;
    let payment = razorpay.get_payment(&input.razorpay_payment_id).await?;
    let expected_amount = order.get_i64("amountInPaise").map_err(|_| AppError::Internal)?;
    if payment.order_id != input.razorpay_order_id
        || payment.amount != expected_amount
        || payment.currency != "INR"
    {
        return Err(AppError::Conflict("payment does not match this order"));
    }
    if payment.status != "captured" {
        return Err(AppError::Conflict("payment is not captured yet; await payment webhook"));
    }

    fulfill_order(&state, &order, &payment.id, payment.amount, &payment.currency).await?;
    Ok(Json(json!({
        "verified": true,
        "enrolled": true,
        "courseId": order.get_object_id("courseId").map_err(|_| AppError::Internal)?.to_hex(),
        "paymentId": payment.id
    })))
}

pub async fn razorpay_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<Value>> {
    let payload: Value = serde_json::from_slice(&body)
        .map_err(|_| AppError::BadRequest("invalid webhook JSON"))?;
    let event = payload.get("event").and_then(Value::as_str).unwrap_or_default();
    if event != "payment.captured" {
        return Ok(Json(json!({ "received": true, "ignored": true })));
    }

    let payment_entity = payload
        .pointer("/payload/payment/entity")
        .ok_or(AppError::BadRequest("missing payment entity"))?;
    let razorpay_order_id = payment_entity
        .get("order_id")
        .and_then(Value::as_str)
        .ok_or(AppError::BadRequest("missing order ID"))?;
    let payment_id = payment_entity
        .get("id")
        .and_then(Value::as_str)
        .ok_or(AppError::BadRequest("missing payment ID"))?;
    let payment_status = payment_entity
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let amount = payment_entity
        .get("amount")
        .and_then(Value::as_i64)
        .ok_or(AppError::BadRequest("missing payment amount"))?;
    let currency = payment_entity
        .get("currency")
        .and_then(Value::as_str)
        .ok_or(AppError::BadRequest("missing payment currency"))?;

    let order = state
        .db
        .collection::<Document>("orders")
        .find_one(doc! {
            "razorpayOrderId": razorpay_order_id,
            "provider": "razorpay",
        })
        .await?;

    let Some(order) = order else {
        // Acknowledging unknown order IDs avoids leaking order existence to webhook senders.
        return Ok(Json(json!({ "received": true, "ignored": true })));
    };

    let company_id = order
        .get_object_id("companyId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let company = load_company(&state, &company_id).await?;
    let key = encryption_key(&state)?;
    let webhook_secret = reveal_secret(
        &company,
        "razorpayWebhookSecretEncrypted",
        &company_id,
        key,
    )?;
    let signature = headers
        .get("x-razorpay-signature")
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    if !hmac_matches(webhook_secret.as_bytes(), &body, signature) {
        return Err(AppError::Unauthorized);
    }

    if payment_status != "captured" {
        return Ok(Json(json!({ "received": true, "ignored": true })));
    }
    let expected_amount = order.get_i64("amountInPaise").map_err(|_| AppError::Internal)?;
    if amount != expected_amount || currency != "INR" {
        return Err(AppError::Conflict("webhook payment does not match order amount"));
    }

    fulfill_order(&state, &order, payment_id, amount, currency).await?;
    Ok(Json(json!({ "received": true, "enrolled": true })))
}

async fn fulfill_order(
    state: &AppState,
    order: &Document,
    payment_id: &str,
    amount: i64,
    currency: &str,
) -> AppResult<()> {
    let company_id = order
        .get_object_id("companyId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let course_id = order
        .get_object_id("courseId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let learner_id = order
        .get_object_id("learnerId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let order_id = order
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let external_order_id = order
        .get_str("razorpayOrderId")
        .map(str::to_owned)
        .map_err(|_| AppError::Internal)?;
    let expected_amount = order.get_i64("amountInPaise").map_err(|_| AppError::Internal)?;

    if amount != expected_amount || currency != "INR" || payment_id.is_empty() {
        return Err(AppError::Conflict("payment details do not match order"));
    }

    if order.get_str("status").ok() == Some("paid") {
        if order.get_str("razorpayPaymentId").ok() != Some(payment_id) {
            return Err(AppError::Conflict("order was already paid with another payment"));
        }
    }

    let payments = state.db.collection::<Document>("payments");
    if let Some(existing) = payments
        .find_one(doc! {
            "companyId": company_id.clone(),
            "razorpayPaymentId": payment_id,
        })
        .await?
    {
        if existing.get_object_id("orderId").ok() != Some(order_id.clone()) {
            return Err(AppError::Conflict("payment ID is already linked to another order"));
        }
    } else {
        let insert = payments
            .insert_one(doc! {
                "_id": ObjectId::new(),
                "companyId": company_id.clone(),
                "courseId": course_id.clone(),
                "learnerId": learner_id.clone(),
                "orderId": order_id.clone(),
                "razorpayOrderId": external_order_id,
                "razorpayPaymentId": payment_id,
                "amountInPaise": amount,
                "currency": currency,
                "status": "captured",
                "capturedAt": DateTime::now(),
            })
            .await;
        if insert.is_err() {
            // A concurrent webhook may have inserted the same payment; verify its binding.
            let existing = payments
                .find_one(doc! {
                    "companyId": company_id.clone(),
                    "razorpayPaymentId": payment_id,
                })
                .await?
                .ok_or(AppError::Conflict("could not record payment idempotently"))?;
            if existing.get_object_id("orderId").ok() != Some(order_id.clone()) {
                return Err(AppError::Conflict("payment ID is already linked to another order"));
            }
        }
    }

    enroll(
        state,
        &company_id,
        &course_id,
        &learner_id,
        "purchase",
        Some(&order_id),
    )
    .await?;

    state
        .db
        .collection::<Document>("orders")
        .update_one(
            doc! { "_id": order_id, "companyId": company_id },
            doc! {
                "$set": {
                    "status": "paid",
                    "razorpayPaymentId": payment_id,
                    "paidAt": DateTime::now(),
                    "updatedAt": DateTime::now(),
                }
            },
        )
        .await?;

    Ok(())
}

async fn enroll(
    state: &AppState,
    company_id: &ObjectId,
    course_id: &ObjectId,
    learner_id: &ObjectId,
    source: &str,
    order_id: Option<&ObjectId>,
) -> AppResult<()> {
    let mut inserted = doc! {
        "_id": ObjectId::new(),
        "companyId": company_id.clone(),
        "courseId": course_id.clone(),
        "learnerId": learner_id.clone(),
        "status": "active",
        "source": source,
        "enrolledAt": DateTime::now(),
        "createdAt": DateTime::now(),
        "updatedAt": DateTime::now(),
    };
    if let Some(order_id) = order_id {
        inserted.insert("orderId", order_id.clone());
    }

    state
        .db
        .collection::<Document>("enrollments")
        .update_one(
            doc! {
                "companyId": company_id.clone(),
                "courseId": course_id.clone(),
                "learnerId": learner_id.clone(),
            },
            doc! { "$setOnInsert": inserted },
        )
        .upsert(true)
        .await?;

    Ok(())
}
