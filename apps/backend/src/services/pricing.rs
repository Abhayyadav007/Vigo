//! Bill rules. All amounts in paise.

use crate::dto::order::BillSummary;

/// Item totals at or above this (₹199) ship free.
pub const FREE_DELIVERY_ABOVE_PAISE: i64 = 19_900;
/// Otherwise delivery costs ₹30.
pub const DELIVERY_FEE_PAISE: i64 = 3_000;

pub fn bill(item_total_paise: i64, mrp_total_paise: i64) -> BillSummary {
    let delivery_fee_paise =
        if item_total_paise == 0 || item_total_paise >= FREE_DELIVERY_ABOVE_PAISE {
            0
        } else {
            DELIVERY_FEE_PAISE
        };
    BillSummary {
        item_total_paise,
        mrp_total_paise,
        delivery_fee_paise,
        total_paise: item_total_paise + delivery_fee_paise,
        free_delivery_above_paise: FREE_DELIVERY_ABOVE_PAISE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_fee_threshold() {
        assert_eq!(bill(19_899, 20_000).delivery_fee_paise, 3_000);
        assert_eq!(bill(19_899, 20_000).total_paise, 22_899);
        assert_eq!(bill(19_900, 20_000).delivery_fee_paise, 0);
        assert_eq!(bill(0, 0).total_paise, 0, "empty cart has no fee");
    }
}
