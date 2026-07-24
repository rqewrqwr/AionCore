use aionui_common::TimestampMs;
use serde::{Deserialize, Serialize};

/// Body for `POST /api/workspaces`.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct CreateWorkspaceRequest {
    pub name: String,
}

/// Public workspace metadata. The server filesystem path is intentionally
/// excluded from the API contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceResponse {
    pub id: String,
    pub name: String,
    pub created_at: TimestampMs,
    pub updated_at: TimestampMs,
}

pub type WorkspaceListResponse = Vec<WorkspaceResponse>;
