use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ErrorBody {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct ErrorDetail {
    /// Stable machine-readable code, e.g. `NOT_FOUND`, `CONFLICT`.
    pub code: String,
    pub message: String,
}
