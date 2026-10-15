use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Clone)]
pub struct RazorpayClient {
    client: reqwest::Client,
    key_id: String,
    key_secret: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RazorpayPayment {
    pub id: String,
    pub order_id: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateOrderPayload<'a> {
    amount: i64,
    currency: &'static str,
    receipt: &'a str,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RazorpayOrder {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub receipt: Option<String>,
}

impl RazorpayClient {
    /// Creates an INR order from the owning company's encrypted Razorpay credentials.
    pub fn from_credentials(
        client: reqwest::Client,
        key_id: String,
        key_secret: String,
    ) -> AppResult<Self> {
        if key_id.trim().is_empty() || key_secret.trim().is_empty() {
            return Err(AppError::ServiceUnavailable("Razorpay credentials"));
        }
        Ok(Self {
            client,
            key_id,
            key_secret,
        })
    }

    pub async fn get_payment(&self, payment_id: &str) -> AppResult<RazorpayPayment> {
        if payment_id.is_empty()
            || payment_id.len() > 80
            || !payment_id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "-_".contains(ch))
        {
            return Err(AppError::BadRequest("invalid Razorpay payment ID"));
        }

        let response = self
            .client
            .get(format!("https://api.razorpay.com/v1/payments/{payment_id}"))
            .basic_auth(&self.key_id, Some(&self.key_secret))
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json::<RazorpayPayment>().await?)
    }

    pub async fn create_inr_order(
        &self,
        amount_in_paise: i64,
        receipt: &str,
    ) -> AppResult<RazorpayOrder> {
        if amount_in_paise <= 0
            || amount_in_paise > 100_000_000
            || receipt.is_empty()
            || receipt.len() > 40
            || !receipt
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || "-_".contains(ch))
        {
            return Err(AppError::BadRequest("invalid order amount or receipt"));
        }

        let response = self
            .client
            .post("https://api.razorpay.com/v1/orders")
            .basic_auth(&self.key_id, Some(&self.key_secret))
            .json(&CreateOrderPayload {
                amount: amount_in_paise,
                currency: "INR",
                receipt,
            })
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json::<RazorpayOrder>().await?)
    }
}
