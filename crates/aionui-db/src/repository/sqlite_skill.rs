use sqlx::SqlitePool;

use crate::error::DbError;
use crate::models::{SkillImportRecordRow, SkillRow};
#[cfg(test)]
use crate::repository::skill::DEFAULT_SKILL_OWNER;
use crate::repository::skill::{
    CreateSkillImportRecordParams, ISkillRepository, SHARED_SKILL_OWNER, UpsertSkillParams,
};

const SKILL_ROW_COLUMNS: &str = "id, COALESCE(display_name, name) AS name, owner_user_id, description, path, source, enabled, deleted_at, created_at, updated_at";

fn skill_storage_key(owner_user_id: &str, name: &str) -> String {
    format!("{owner_user_id}\u{1f}{name}")
}

/// SQLite-backed implementation of [`ISkillRepository`].
#[derive(Clone, Debug)]
pub struct SqliteSkillRepository {
    pool: SqlitePool,
}

impl SqliteSkillRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ISkillRepository for SqliteSkillRepository {
    async fn list(&self, owner_user_id: &str) -> Result<Vec<SkillRow>, DbError> {
        let query = format!(
            "SELECT {SKILL_ROW_COLUMNS} FROM skills \
             WHERE owner_user_id IN (?, ?) AND deleted_at IS NULL AND enabled = 1 \
             ORDER BY CASE WHEN owner_user_id = ? THEN 0 ELSE 1 END, updated_at DESC, display_name ASC"
        );
        let rows = sqlx::query_as::<_, SkillRow>(&query)
            .bind(owner_user_id)
            .bind(SHARED_SKILL_OWNER)
            .bind(owner_user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn find_by_name(&self, owner_user_id: &str, name: &str) -> Result<Option<SkillRow>, DbError> {
        let query = format!(
            "SELECT {SKILL_ROW_COLUMNS} FROM skills \
             WHERE owner_user_id IN (?, ?) AND COALESCE(display_name, name) = ? AND deleted_at IS NULL AND enabled = 1 \
             ORDER BY CASE WHEN owner_user_id = ? THEN 0 ELSE 1 END LIMIT 1"
        );
        let row = sqlx::query_as::<_, SkillRow>(&query)
            .bind(owner_user_id)
            .bind(SHARED_SKILL_OWNER)
            .bind(name)
            .bind(owner_user_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn find_by_name_any(&self, owner_user_id: &str, name: &str) -> Result<Option<SkillRow>, DbError> {
        let query = format!(
            "SELECT {SKILL_ROW_COLUMNS} FROM skills WHERE owner_user_id = ? AND COALESCE(display_name, name) = ? LIMIT 1"
        );
        let row = sqlx::query_as::<_, SkillRow>(&query)
            .bind(owner_user_id)
            .bind(name)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn upsert(&self, params: UpsertSkillParams<'_>) -> Result<SkillRow, DbError> {
        let now = aionui_common::now_ms();
        let existing = self.find_by_name_any(params.owner_user_id, params.name).await?;
        let id = existing
            .as_ref()
            .map(|row| row.id.clone())
            .unwrap_or_else(|| aionui_common::generate_prefixed_id("skill"));
        let created_at = existing.as_ref().map(|row| row.created_at).unwrap_or(now);

        let storage_key = skill_storage_key(params.owner_user_id, params.name);
        sqlx::query(
            "INSERT INTO skills \
                (id, name, display_name, owner_user_id, description, path, source, enabled, deleted_at, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?) \
             ON CONFLICT(name) DO UPDATE SET \
                display_name = excluded.display_name, \
                owner_user_id = excluded.owner_user_id, \
                description = excluded.description, \
                path = excluded.path, \
                source = excluded.source, \
                enabled = excluded.enabled, \
                deleted_at = NULL, \
                updated_at = excluded.updated_at",
        )
        .bind(&id)
        .bind(&storage_key)
        .bind(params.name)
        .bind(params.owner_user_id)
        .bind(params.description)
        .bind(params.path)
        .bind(params.source)
        .bind(params.enabled)
        .bind(created_at)
        .bind(now)
        .execute(&self.pool)
        .await?;

        self.find_by_name_any(params.owner_user_id, params.name)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("skill '{}' was not found after upsert", params.name)))
    }

    async fn delete_by_name(&self, owner_user_id: &str, name: &str) -> Result<SkillRow, DbError> {
        let now = aionui_common::now_ms();
        let result = sqlx::query(
            "UPDATE skills SET enabled = 0, deleted_at = ?, updated_at = ? \
             WHERE owner_user_id = ? AND display_name = ? AND source = 'user' AND deleted_at IS NULL",
        )
        .bind(now)
        .bind(now)
        .bind(owner_user_id)
        .bind(name)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("skill '{name}'")));
        }

        self.find_by_name_any(owner_user_id, name)
            .await?
            .ok_or_else(|| DbError::NotFound(format!("skill '{name}'")))
    }

    async fn create_import_record(
        &self,
        params: CreateSkillImportRecordParams<'_>,
    ) -> Result<SkillImportRecordRow, DbError> {
        let id = aionui_common::generate_prefixed_id("skill_import");
        let now = aionui_common::now_ms();

        sqlx::query(
            "INSERT INTO skill_import_records \
                (id, operation_id, source_label, source_path, source_name, owner_user_id, skill_id, skill_name, \
                 status, error_code, error_path, actual_bytes, limit_bytes, line, column, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(params.operation_id)
        .bind(params.source_label)
        .bind(params.source_path)
        .bind(params.source_name)
        .bind(params.owner_user_id)
        .bind(params.skill_id)
        .bind(params.skill_name)
        .bind(params.status)
        .bind(params.error_code)
        .bind(params.error_path)
        .bind(params.actual_bytes)
        .bind(params.limit_bytes)
        .bind(params.line)
        .bind(params.column)
        .bind(now)
        .execute(&self.pool)
        .await?;

        let row = sqlx::query_as::<_, SkillImportRecordRow>("SELECT * FROM skill_import_records WHERE id = ?")
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_import_records(&self, owner_user_id: &str, limit: i64) -> Result<Vec<SkillImportRecordRow>, DbError> {
        let rows = sqlx::query_as::<_, SkillImportRecordRow>(
            "SELECT * FROM skill_import_records WHERE owner_user_id = ? ORDER BY created_at DESC, id DESC LIMIT ?",
        )
        .bind(owner_user_id)
        .bind(limit.max(0))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_database_memory;

    async fn setup() -> (SqliteSkillRepository, crate::Database) {
        let db = init_database_memory().await.unwrap();
        let repo = SqliteSkillRepository::new(db.pool().clone());
        (repo, db)
    }

    #[tokio::test]
    async fn upsert_restores_soft_deleted_skill() {
        let (repo, _db) = setup().await;

        let created = repo
            .upsert(UpsertSkillParams {
                owner_user_id: DEFAULT_SKILL_OWNER,
                name: "sample",
                description: Some("Old"),
                path: "/tmp/old",
                source: "user",
                enabled: true,
            })
            .await
            .unwrap();
        repo.delete_by_name(DEFAULT_SKILL_OWNER, "sample").await.unwrap();

        let restored = repo
            .upsert(UpsertSkillParams {
                owner_user_id: DEFAULT_SKILL_OWNER,
                name: "sample",
                description: Some("New"),
                path: "/tmp/new",
                source: "user",
                enabled: true,
            })
            .await
            .unwrap();

        assert_eq!(restored.id, created.id);
        assert_eq!(restored.description.as_deref(), Some("New"));
        assert_eq!(restored.path, "/tmp/new");
        assert_eq!(restored.deleted_at, None);
        assert!(
            repo.find_by_name(DEFAULT_SKILL_OWNER, "sample")
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn list_filters_soft_deleted_skills() {
        let (repo, _db) = setup().await;

        repo.upsert(UpsertSkillParams {
            owner_user_id: DEFAULT_SKILL_OWNER,
            name: "active",
            description: None,
            path: "/tmp/active",
            source: "user",
            enabled: true,
        })
        .await
        .unwrap();
        repo.upsert(UpsertSkillParams {
            owner_user_id: DEFAULT_SKILL_OWNER,
            name: "deleted",
            description: None,
            path: "/tmp/deleted",
            source: "user",
            enabled: true,
        })
        .await
        .unwrap();
        repo.delete_by_name(DEFAULT_SKILL_OWNER, "deleted").await.unwrap();

        let names: Vec<_> = repo
            .list(DEFAULT_SKILL_OWNER)
            .await
            .unwrap()
            .into_iter()
            .map(|row| row.name)
            .collect();
        assert_eq!(names, vec!["active"]);
        assert!(
            repo.find_by_name_any(DEFAULT_SKILL_OWNER, "deleted")
                .await
                .unwrap()
                .is_some()
        );
    }

    #[tokio::test]
    async fn same_visible_name_is_isolated_between_users() {
        let (repo, _db) = setup().await;

        for (owner, path) in [("user-a", "/tmp/a"), ("user-b", "/tmp/b")] {
            repo.upsert(UpsertSkillParams {
                owner_user_id: owner,
                name: "same-name",
                description: Some(owner),
                path,
                source: "user",
                enabled: true,
            })
            .await
            .unwrap();
        }

        let user_a = repo.find_by_name("user-a", "same-name").await.unwrap().unwrap();
        let user_b = repo.find_by_name("user-b", "same-name").await.unwrap().unwrap();
        assert_eq!(user_a.path, "/tmp/a");
        assert_eq!(user_b.path, "/tmp/b");

        repo.delete_by_name("user-a", "same-name").await.unwrap();
        assert!(repo.find_by_name("user-a", "same-name").await.unwrap().is_none());
        assert!(repo.find_by_name("user-b", "same-name").await.unwrap().is_some());
    }

    #[tokio::test]
    async fn import_records_keep_structured_error_details() {
        let (repo, _db) = setup().await;

        let row = repo
            .create_import_record(CreateSkillImportRecordParams {
                operation_id: "import_1",
                source_label: "parent-pack",
                source_path: Some("/tmp/parent-pack"),
                source_name: "beta-skill",
                owner_user_id: DEFAULT_SKILL_OWNER,
                skill_id: None,
                skill_name: None,
                status: "failed",
                error_code: Some("SKILL_IMPORT_FILE_TOO_LARGE"),
                error_path: Some("assets/movie.mp4"),
                actual_bytes: Some(73_400_320),
                limit_bytes: Some(10_485_760),
                line: None,
                column: None,
            })
            .await
            .unwrap();

        assert_eq!(row.operation_id, "import_1");
        assert_eq!(row.error_path.as_deref(), Some("assets/movie.mp4"));
        assert_eq!(row.actual_bytes, Some(73_400_320));
        assert_eq!(row.limit_bytes, Some(10_485_760));
        let records = repo.list_import_records(DEFAULT_SKILL_OWNER, 10).await.unwrap();
        assert_eq!(records.len(), 1);
    }
}
