use axum::{
    body::Bytes,
    extract::State,
    http::HeaderMap,
    Json,
};
use hmac::{Hmac, Mac};
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::Sha256;

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    integrations::razorpay::RazorpayClient,
    state::AppState,
    tenant::require_active_company,
};

type HmacSha256 = Hmac<Sha256>;
const MONTH_MILLIS: i64 = 30 * 24 * 60 * 60 * 1_000;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionOrderRequest {
    pub plan_code: String,
}

fn plan_price_paise(state: &AppState, plan_code: &str) -> AppResult<i64> {
    match plan_code {
        "starter" => Ok(state.settings.plan_starter_monthly_price_paise),
        "pro" => Ok(state.settings.plan_pro_monthly_price_paise),
        "business" => Ok(state.settings.plan_business_monthly_price_paise),
        _ => Err(AppError::BadRequest("planCode must be starter, pro, or business")),
    }
}

fn verify_signature(secret: &[u8], body: &[u8], signature_hex: &str) -> bool {
    let Ok(signature) = hex::decode(signature_hex.trim()) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(secret) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&signature).is_ok()
}

fn webhook_secret(state: &AppState) -> AppResult<&str> {
    state
        .settings
        .razorpay_webhook_secret
        .as_deref()
        .filter(|value| !value.is_empty())
        .ok_or(AppError::ServiceUnavailable("platform Razorpay webhook secret"))
}

pub async fn create_subscription_order(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<CreateSubscriptionOrderRequest>,
) -> AppResult<(axum::http::StatusCode, Json<Value>)> {
    if !matches!(auth.role.as_str(), "owner" | "admin") {
        return Err(AppError::Forbidden);
    }
    require_active_company(&state.db, &auth.company_id).await?;

    let plan_code = input.plan_code.trim().to_ascii_lowercase();
    let amount = plan_price_paise(&state, &plan_code)?;
    let razorpay = RazorpayClient::from_settings(&state.settings, state.http.clone())?;
    let order_id = ObjectId::new();
    let receipt = order_id.to_hex();
    let provider_order = razorpay.create_inr_order(amount, &receipt).await?;
    let now = DateTime::now();

    state
        .db
        .collection::<Document>("subscription_orders")
        .insert_one(doc! {
            "_id": order_id.clone(),
            "companyId": auth.company_id.clone(),
            "createdBy": auth.user_id.clone(),
            "planCode": &plan_code,
            "amountInPaise": amount,
            "currency": "INR",
            "provider": "platform_razorpay",
            "razorpayOrderId": &provider_order.id,
            "status": "created",
            "receipt": receipt,
            "createdAt": now,
            "updatedAt": now,
        })
        .await
        .map_err(AppError::from_mongo)?;

    let key_id = state
        .settings
        .razorpay_key_id
        .as_deref()
        .ok_or(AppError::ServiceUnavailable("platform Razorpay credentials"))?;

    Ok((
        axum::http::StatusCode::CREATED,
        Json(json!({
            "subscriptionOrderId": order_id.to_hex(),
            "razorpayOrderId": provider_order.id,
            "planCode": plan_code,
            "amount": provider_order.amount,
            "currency": provider_order.currency,
            "keyId": key_id,
            "status": "created",
        })),
    ))
}

pub async fn razorpay_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<Json<Value>> {
    let secret = webhook_secret(&state)?;
    let signature = headers
        .get("x-razorpay-signature")
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;
    if !verify_signature(secret.as_bytes(), &body, signature) {
        return Err(AppError::Unauthorized);
    }

    let payload: Value =
        serde_json::from_slice(&body).map_err(|_| AppError::BadRequest("invalid webhook JSON"))?;
    let event = payload.get("event").and_then(Value::as_str).unwrap_or_default();
    if event != "payment.captured" {
        return Ok(Json(json!({ "received": true, "ignored": true })));
    }

    let payment = payload
        .pointer("/payload/payment/entity")
        .ok_or(AppError::BadRequest("missing payment entity"))?;
    let provider_order_id = payment
        .get("order_id")
        .and_then(Value::as_str)
        .ok_or(AppError::BadRequest("missing order ID"))?;
    let payment_id = payment
        .get("id")
        .and_then(Value::as_str)
        .ok_or(AppError::BadRequest("missing payment ID"))?;
    let payment_status = payment
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let amount = payment
        .get("amount")
        .and_then(Value::as_i64)
        .ok_or(AppError::BadRequest("missing payment amount"))?;
    let currency = payment
        .get("currency")
        .and_then(Value::as_str)
        .ok_or(AppError::BadRequest("missing payment currency"))?;

    if payment_status != "captured" || currency != "INR" {
        return Ok(Json(json!({ "received": true, "ignored": true })));
    }

    let orders = state.db.collection::<Document>("subscription_orders");
    let order = orders
        .find_one(doc! {
            "razorpayOrderId": provider_order_id,
            "provider": "platform_razorpay",
        })
        .await?;

    let Some(order) = order else {
        return Ok(Json(json!({ "received": true, "ignored": true })));
    };

    let order_id = order
        .get_object_id("_id")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let company_id = order
        .get_object_id("companyId")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let plan_code = order
        .get_str("planCode")
        .map(str::to_owned)
        .map_err(|_| AppError::Internal)?;
    let expected_amount = order
        .get_i64("amountInPaise")
        .map_err(|_| AppError::Internal)?;
    if amount != expected_amount {
        return Err(AppError::Conflict("platform payment amount does not match subscription order"));
    }

    if order.get_str("status").ok() == Some("paid") {
        if order.get_str("razorpayPaymentId").ok() == Some(payment_id) {
            return Ok(Json(json!({ "received": true, "activated": true, "idempotent": true })));
        }
        return Err(AppError::Conflict("subscription order has already been paid with another payment"));
    }

    // Claim the order with fixed activation dates. Concurrent webhook retries reuse these dates,
    // so a retry cannot extend the same subscription repeatedly.
    if order.get_str("status").ok() == Some("created") {
        let starts_at = DateTime::now();
        let expires_at = DateTime::from_millis(starts_at.timestamp_millis() + MONTH_MILLIS);
        let claim = orders
            .update_one(
                doc! { "_id": order_id.clone(), "status": "created" },
                doc! {
                    "$set": {
                        "status": "processing",
                        "razorpayPaymentId": payment_id,
                        "activationStartsAt": starts_at,
                        "activationExpiresAt": expires_at,
                        "updatedAt": starts_at,
                    }
                },
            )
            .await?;
        let _ = claim;
    }

    let current_order = orders
        .find_one(doc! { "_id": order_id.clone() })
        .await?
        .ok_or(AppError::NotFound)?;
    if current_order.get_str("status").ok() == Some("paid") {
        if current_order.get_str("razorpayPaymentId").ok() == Some(payment_id) {
            return Ok(Json(json!({ "received": true, "activated": true, "idempotent": true })));
        }
        return Err(AppError::Conflict("subscription order has already been paid with another payment"));
    }
    if current_order.get_str("status").ok() != Some("processing")
        || current_order.get_str("razorpayPaymentId").ok() != Some(payment_id)
    {
        return Err(AppError::Conflict("subscription order is being processed by another payment"));
    }

    let subscription_payments = state.db.collection::<Document>("subscription_payments");
    if let Some(existing) = subscription_payments
        .find_one(doc! { "razorpayPaymentId": payment_id })
        .await?
    {
        if existing.get_object_id("orderId").ok() != Some(order_id.clone()) {
            return Err(AppError::Conflict("payment ID is linked to another subscription order"));
        }
    } else {
        let insert_result = subscription_payments
            .insert_one(doc! {
                "_id": ObjectId::new(),
                "companyId": company_id.clone(),
                "orderId": order_id.clone(),
                "razorpayOrderId": provider_order_id,
                "razorpayPaymentId": payment_id,
                "amountInPaise": amount,
                "currency": currency,
                "status": "captured",
                "capturedAt": DateTime::now(),
            })
            .await;

        if insert_result.is_err() {
            let existing = subscription_payments
                .find_one(doc! { "razorpayPaymentId": payment_id })
                .await?
                .ok_or(AppError::Conflict("could not record subscription payment idempotently"))?;
            if existing.get_object_id("orderId").ok() != Some(order_id.clone()) {
                return Err(AppError::Conflict("payment ID is linked to another subscription order"));
            }
        }
    }

    let starts_at = current_order
        .get_datetime("activationStartsAt")
        .map_err(|_| AppError::Internal)?
        .to_owned();
    let expires_at = current_order
        .get_datetime("activationExpiresAt")
        .map_err(|_| AppError::Internal)?
        .to_owned();

    let company_update = state
        .db
        .collection::<Document>("companies")
        .update_one(
            doc! { "_id": company_id.clone(), "status": "active" },
            doc! {
                "$set": {
                    "planCode": &plan_code,
                    "subscriptionStatus": "active",
                    "subscriptionStartsAt": starts_at,
                    "subscriptionExpiresAt": expires_at,
                    "updatedAt": DateTime::now(),
                }
            },
        )
        .await?;
    if company_update.matched_count != 1 {
        return Err(AppError::NotFound);
    }

    orders
        .update_one(
            doc! {
                "_id": order_id,
                "status": "processing",
                "razorpayPaymentId": payment_id,
            },
            doc! {
                "$set": {
                    "status": "paid",
                    "paidAt": DateTime::now(),
                    "updatedAt": DateTime::now(),
                }
            },
        )
        .await?;

    Ok(Json(json!({
        "received": true,
        "activated": true,
        "planCode": plan_code,
        "subscriptionExpiresAt": expires_at.timestamp_millis(),
    })))
}

#[cfg(test)]
mod tests {
    use super::verify_signature;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    #[test]
    fn signature_verification_uses_raw_body() {
        let secret = b"webhook-secret";
        let body = br#"{"event":"payment.captured"}"#;
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC accepts any key length");
        mac.update(body);
        let signature = hex::encode(mac.finalize().into_bytes());

        assert!(verify_signature(secret, body, &signature));
        assert!(!verify_signature(secret, br#"{"event":"payment.failed"}"#, &signature));
        assert!(!verify_signature(secret, body, "not-hex"));
    }

    #[test]
    fn webhook_signature_is_not_accepted_for_modified_body() {
        let secret = b"another-secret";
        assert!(!verify_signature(secret, b"payload", "00"));
    }
}
