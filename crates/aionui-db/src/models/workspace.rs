use aionui_common::TimestampMs;
use serde::{Deserialize, Serialize};

/// Database row for a personal server workspace.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Eq)]
pub struct WorkspaceRow {
    pub id: String,
    pub owner_user_id: String,
    pub name: String,
    pub root_path: String,
    pub storage_type: String,
    pub status: String,
    pub created_at: TimestampMs,
    pub updated_at: TimestampMs,
}
