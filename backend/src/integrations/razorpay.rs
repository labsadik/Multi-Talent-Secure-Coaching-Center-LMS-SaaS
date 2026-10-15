use serde::{Deserialize, Serialize};

use crate::{
    config::Settings,
    error::{AppError, AppResult},
};

#[derive(Clone)]
pub struct RazorpayClient {
    client: reqwest::Client,
    key_id: String,
    key_secret: String,
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
    pub fn from_settings(settings: &Settings, client: reqwest::Client) -> AppResult<Self> {
        Ok(Self {
            client,
            key_id: settings
                .razorpay_key_id
                .clone()
                .ok_or(AppError::ServiceUnavailable)?,
            key_secret: settings
                .razorpay_key_secret
                .clone()
                .ok_or(AppError::ServiceUnavailable("Razorpay"))?,
        })
    }

    /// Creates a platform-side INR order. Derive amount from a server-side plan/course record.
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
