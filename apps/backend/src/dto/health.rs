use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum ComponentStatus {
    Ok,
    Down,
}

#[derive(Debug, Serialize, TS)]
#[ts(export)]
pub struct HealthResponse {
    pub status: ComponentStatus,
    pub database: ComponentStatus,
    pub redis: ComponentStatus,
    pub version: String,
}
