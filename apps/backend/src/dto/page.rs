use serde::Serialize;
use ts_rs::TS;

/// One page of a list endpoint.
#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct Page<T: TS> {
    pub items: Vec<T>,
    #[ts(type = "number")]
    pub total: i64,
    #[ts(type = "number")]
    pub limit: i64,
    #[ts(type = "number")]
    pub offset: i64,
}
