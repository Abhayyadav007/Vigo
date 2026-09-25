use super::{PaymentInit, PaymentProvider};
use crate::{
    error::AppResult,
    models::order::{Order, PaymentMethod},
};

/// Cash (or UPI to the rider) on delivery: nothing to collect up front.
pub struct CashOnDelivery;

impl PaymentProvider for CashOnDelivery {
    fn method(&self) -> PaymentMethod {
        PaymentMethod::Cod
    }

    async fn initiate(&self, _order: &Order) -> AppResult<PaymentInit> {
        Ok(PaymentInit::CollectOnDelivery)
    }
}
