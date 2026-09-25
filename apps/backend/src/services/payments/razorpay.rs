use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;

use super::{PaymentInit, PaymentProvider};
use crate::{
    dto::order::RazorpayCheckout,
    error::AppResult,
    models::order::{Order, PaymentMethod},
};

pub struct Razorpay {
    pub key_id: String,
    webhook_secret: String,
}

impl Razorpay {
    pub fn new(key_id: String, webhook_secret: String) -> Self {
        Self {
            key_id,
            webhook_secret,
        }
    }

    /// `X-Razorpay-Signature` is hex(HMAC-SHA256(raw body, webhook secret)).
    /// Compared in constant time.
    pub fn verify_webhook(&self, body: &[u8], signature_hex: &str) -> bool {
        let Ok(signature) = hex::decode(signature_hex.trim()) else {
            return false;
        };
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(self.webhook_secret.as_bytes()) else {
            return false;
        };
        mac.update(body);
        mac.verify_slice(&signature).is_ok()
    }
}

impl PaymentProvider for Razorpay {
    fn method(&self) -> PaymentMethod {
        PaymentMethod::Online
    }

    async fn initiate(&self, order: &Order) -> AppResult<PaymentInit> {
        // TODO(phase-8): create the order with Razorpay's Orders API
        // (POST https://api.razorpay.com/v1/orders, amount in paise, receipt =
        // order number, idempotent on our order id) and use its id here.
        let gateway_order_id = format!("order_stub{}", &order.id.simple().to_string()[..14]);
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

    #[test]
    fn verifies_hmac_signatures() {
        let rp = Razorpay::new("rzp_test_key".into(), "whsec".into());
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
