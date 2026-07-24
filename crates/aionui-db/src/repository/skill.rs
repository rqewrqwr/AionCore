use crate::error::DbError;
use crate::models::{SkillImportRecordRow, SkillRow};

pub const DEFAULT_SKILL_OWNER: &str = "system_default_user";
pub const SHARED_SKILL_OWNER: &str = "__shared__";

/// Skill metadata and import-history data access abstraction.
#[async_trait::async_trait]
pub trait ISkillRepository: Send + Sync {
    /// Returns active skills visible to one user (their own plus shared).
    async fn list(&self, owner_user_id: &str) -> Result<Vec<SkillRow>, DbError>;

    /// Finds an active skill by name.
    async fn find_by_name(&self, owner_user_id: &str, name: &str) -> Result<Option<SkillRow>, DbError>;

    /// Finds a skill by name, including soft-deleted rows.
    async fn find_by_name_any(&self, owner_user_id: &str, name: &str) -> Result<Option<SkillRow>, DbError>;

    /// Creates or updates a user skill by name and clears soft-delete state.
    async fn upsert(&self, params: UpsertSkillParams<'_>) -> Result<SkillRow, DbError>;

    /// Soft-deletes an active skill by name.
    async fn delete_by_name(&self, owner_user_id: &str, name: &str) -> Result<SkillRow, DbError>;

    /// Appends one import record.
    async fn create_import_record(
        &self,
        params: CreateSkillImportRecordParams<'_>,
    ) -> Result<SkillImportRecordRow, DbError>;

    /// Lists recent import records ordered by creation time descending.
    async fn list_import_records(&self, owner_user_id: &str, limit: i64) -> Result<Vec<SkillImportRecordRow>, DbError>;
}

/// Parameters for creating or updating a skill row.
#[derive(Debug, Clone)]
pub struct UpsertSkillParams<'a> {
    pub owner_user_id: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub path: &'a str,
    pub source: &'a str,
    pub enabled: bool,
}

/// Parameters for appending a skill import history row.
#[derive(Debug, Clone)]
pub struct CreateSkillImportRecordParams<'a> {
    pub operation_id: &'a str,
    pub source_label: &'a str,
    pub source_path: Option<&'a str>,
    pub source_name: &'a str,
    pub owner_user_id: &'a str,
    pub skill_id: Option<&'a str>,
    pub skill_name: Option<&'a str>,
    pub status: &'a str,
    pub error_code: Option<&'a str>,
    pub error_path: Option<&'a str>,
    pub actual_bytes: Option<i64>,
    pub limit_bytes: Option<i64>,
    pub line: Option<i64>,
    pub column: Option<i64>,
}
