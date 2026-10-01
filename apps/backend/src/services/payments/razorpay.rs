use std::time::Duration;

use anyhow::Context;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::json;
use sha2::Sha256;

use super::{PaymentInit, PaymentProvider};
use crate::{
    config::RazorpayConfig,
    dto::order::{RazorpayCheckout, display_number},
    error::{AppError, AppResult},
    models::order::{Order, PaymentMethod},
};

pub struct Razorpay {
    pub key_id: String,
    webhook_secret: String,
    /// Live API access; `None` in development without a key secret.
    api: Option<Api>,
}

struct Api {
    key_id: String,
    key_secret: String,
    base: String,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct CreatedOrder {
    id: String,
}

#[derive(Deserialize)]
struct CreatedRefund {
    id: String,
}

impl Razorpay {
    pub fn new(config: &RazorpayConfig) -> anyhow::Result<Self> {
        let api = match &config.key_secret {
            Some(secret) => Some(Api {
                key_id: config.key_id.clone(),
                key_secret: secret.clone(),
                base: config.api_base.trim_end_matches('/').to_owned(),
                http: reqwest::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .build()
                    .context("building Razorpay HTTP client")?,
            }),
            None => {
                tracing::warn!(
                    "RAZORPAY_KEY_SECRET not set: gateway orders are stubbed, refunds skipped"
                );
                None
            }
        };
        Ok(Self {
            key_id: config.key_id.clone(),
            webhook_secret: config.webhook_secret.clone(),
            api,
        })
    }

    /// `X-Razorpay-Signature` is hex(HMAC-SHA256(raw body, webhook secret)).
    /// Compared in constant time.
    pub fn verify_webhook(&self, body: &[u8], signature_hex: &str) -> bool {
        hmac_matches(&self.webhook_secret, body, signature_hex)
    }

    /// The checkout's success callback signs `"{order_id}|{payment_id}"` with
    /// the key secret. False without a secret: the webhook confirms instead.
    pub fn verify_checkout(
        &self,
        gateway_order_id: &str,
        payment_id: &str,
        signature_hex: &str,
    ) -> bool {
        let Some(api) = &self.api else {
            return false;
        };
        let message = format!("{gateway_order_id}|{payment_id}");
        hmac_matches(&api.key_secret, message.as_bytes(), signature_hex)
    }

    /// Refunds `amount_paise` of a captured payment; returns the refund id, or
    /// `None` when stubbed (no key secret).
    pub async fn refund(
        &self,
        payment_id: &str,
        amount_paise: i64,
        receipt: &str,
    ) -> AppResult<Option<String>> {
        let Some(api) = &self.api else {
            tracing::warn!(
                payment_id,
                amount_paise,
                "Razorpay API not configured; refund skipped"
            );
            return Ok(None);
        };
        let refund: CreatedRefund = api
            .post(
                &format!("/v1/payments/{payment_id}/refund"),
                &json!({ "amount": amount_paise, "receipt": receipt, "speed": "normal" }),
            )
            .await?;
        Ok(Some(refund.id))
    }
}

impl Api {
    async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> AppResult<T> {
        let res = self
            .http
            .post(format!("{}{path}", self.base))
            .basic_auth(&self.key_id, Some(&self.key_secret))
            .json(body)
            .send()
            .await
            .map_err(|e| {
                AppError::ServiceUnavailable(format!("payment gateway unreachable: {e}"))
            })?;
        let status = res.status();
        if !status.is_success() {
            let text = res.text().await.unwrap_or_default();
            tracing::error!(%status, path, body = %text, "Razorpay API error");
            return Err(AppError::ServiceUnavailable(
                "the payment gateway rejected the request".into(),
            ));
        }
        res.json::<T>()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("unexpected Razorpay response: {e}")))
    }
}

fn hmac_matches(secret: &str, message: &[u8], signature_hex: &str) -> bool {
    let Ok(signature) = hex::decode(signature_hex.trim()) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(message);
    mac.verify_slice(&signature).is_ok()
}

impl PaymentProvider for Razorpay {
    fn method(&self) -> PaymentMethod {
        PaymentMethod::Online
    }

    async fn initiate(&self, order: &Order) -> AppResult<PaymentInit> {
        let gateway_order_id = match &self.api {
            // Amount is fixed on Razorpay's side, so the checkout can't be
            // tampered into paying less. The receipt ties it back to us.
            Some(api) => {
                let created: CreatedOrder = api
                    .post(
                        "/v1/orders",
                        &json!({
                            "amount": order.total_paise,
                            "currency": "INR",
                            "receipt": display_number(order.number),
                            "notes": { "order_id": order.id },
                        }),
                    )
                    .await?;
                created.id
            }
            // Development without a key secret only (refused in production).
            None => format!("order_stub{}", &order.id.simple().to_string()[..14]),
        };
        Ok(PaymentInit::Gateway(RazorpayCheckout {
            key_id: self.key_id.clone(),
            gateway_order_id,
            amount_paise: order.total_paise,
            currency: "INR".into(),
        }))
    }
}

/// The parts of a Razorpay webhook we act on.
#[derive(Debug, Deserialize)]
pub struct RazorpayWebhook {
    pub event: String,
    pub payload: WebhookPayload,
}

#[derive(Debug, Deserialize)]
pub struct WebhookPayload {
    pub payment: Option<PaymentWrapper>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentWrapper {
    pub entity: PaymentEntity,
}

#[derive(Debug, Deserialize)]
pub struct PaymentEntity {
    pub id: String,
    pub order_id: Option<String>,
    pub amount: i64,
    pub currency: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(key_secret: Option<&str>) -> RazorpayConfig {
        RazorpayConfig {
            key_id: "rzp_test_key".into(),
            key_secret: key_secret.map(Into::into),
            webhook_secret: "whsec".into(),
            api_base: "http://127.0.0.1:9".into(),
        }
    }

    #[test]
    fn verifies_checkout_signatures() {
        let rp = Razorpay::new(&config(Some("secret"))).unwrap();
        let mut mac = Hmac::<Sha256>::new_from_slice(b"secret").unwrap();
        mac.update(b"order_A|pay_B");
        let good = hex::encode(mac.finalize().into_bytes());
        assert!(rp.verify_checkout("order_A", "pay_B", &good));
        assert!(!rp.verify_checkout("order_A", "pay_C", &good));
        assert!(!rp.verify_checkout("order_X", "pay_B", &good));
        // Without a key secret nothing verifies (the webhook confirms instead).
        let stub = Razorpay::new(&config(None)).unwrap();
        assert!(!stub.verify_checkout("order_A", "pay_B", &good));
    }

    #[test]
    fn verifies_hmac_signatures() {
        let rp = Razorpay::new(&config(None)).unwrap();
        let body = br#"{"event":"payment.captured"}"#;
        let mut mac = Hmac::<Sha256>::new_from_slice(b"whsec").unwrap();
        mac.update(body);
        let good = hex::encode(mac.finalize().into_bytes());
        assert!(rp.verify_webhook(body, &good));
        assert!(!rp.verify_webhook(b"tampered", &good));
        assert!(!rp.verify_webhook(body, "deadbeef"));
        assert!(!rp.verify_webhook(body, "not-hex"));
    }
}
