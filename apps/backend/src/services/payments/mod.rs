//! Payment providers. Cash on delivery works today; Razorpay is stubbed behind
//! the same trait (real Orders API call pending) with working webhook
//! signature verification, so a live gateway plugs in without touching orders.

mod cod;
mod razorpay;

use std::future::Future;

pub use cod::CashOnDelivery;
pub use razorpay::{Razorpay, RazorpayWebhook};

use crate::{
    dto::order::RazorpayCheckout,
    error::AppResult,
    models::order::{Order, PaymentMethod},
};

/// What the client must do next after an order is placed.
#[derive(Debug)]
pub enum PaymentInit {
    /// Nothing to pay now; confirm the order immediately.
    CollectOnDelivery,
    /// Open the gateway checkout; the order confirms on the payment webhook.
    Gateway(RazorpayCheckout),
}

pub trait PaymentProvider: Send + Sync {
    fn method(&self) -> PaymentMethod;

    /// Starts payment for a freshly placed (PLACED) order.
    fn initiate(&self, order: &Order) -> impl Future<Output = AppResult<PaymentInit>> + Send;
}

/// Configured providers. Online payments are off unless Razorpay is configured.
pub struct Payments {
    pub cod: CashOnDelivery,
    pub razorpay: Option<Razorpay>,
}
