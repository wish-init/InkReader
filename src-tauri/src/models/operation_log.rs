use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogRecord {
    pub id: String,
    pub operation_type: String,
    pub target: String,
    pub summary: String,
    pub reversible: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogRequest {
    pub operation_type: String,
    pub target: String,
    pub summary: String,
    pub reversible: bool,
}
