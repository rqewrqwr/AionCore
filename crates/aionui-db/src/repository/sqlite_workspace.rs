use sqlx::SqlitePool;

use crate::error::DbError;
use crate::models::WorkspaceRow;
use crate::repository::workspace::IWorkspaceRepository;

#[derive(Clone, Debug)]
pub struct SqliteWorkspaceRepository {
    pool: SqlitePool,
}

impl SqliteWorkspaceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl IWorkspaceRepository for SqliteWorkspaceRepository {
    async fn create(&self, row: &WorkspaceRow) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO workspaces (
                id, owner_user_id, name, root_path, storage_type, status,
                created_at, updated_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&row.id)
        .bind(&row.owner_user_id)
        .bind(&row.name)
        .bind(&row.root_path)
        .bind(&row.storage_type)
        .bind(&row.status)
        .bind(row.created_at)
        .bind(row.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_owned(&self, owner_user_id: &str, workspace_id: &str) -> Result<Option<WorkspaceRow>, DbError> {
        let row = sqlx::query_as::<_, WorkspaceRow>(
            "SELECT * FROM workspaces
             WHERE id = ? AND owner_user_id = ? AND status = 'active'",
        )
        .bind(workspace_id)
        .bind(owner_user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn find_owned_by_root(&self, owner_user_id: &str, root_path: &str) -> Result<Option<WorkspaceRow>, DbError> {
        let row = sqlx::query_as::<_, WorkspaceRow>(
            "SELECT * FROM workspaces
             WHERE owner_user_id = ? AND root_path = ? AND status = 'active'",
        )
        .bind(owner_user_id)
        .bind(root_path)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_owned(&self, owner_user_id: &str) -> Result<Vec<WorkspaceRow>, DbError> {
        let rows = sqlx::query_as::<_, WorkspaceRow>(
            "SELECT * FROM workspaces
             WHERE owner_user_id = ? AND status = 'active'
             ORDER BY updated_at DESC, id DESC",
        )
        .bind(owner_user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn bind_conversation(
        &self,
        owner_user_id: &str,
        conversation_id: &str,
        workspace_id: &str,
    ) -> Result<(), DbError> {
        let result = sqlx::query(
            "UPDATE conversations
             SET workspace_id = ?
             WHERE id = ? AND user_id = ?",
        )
        .bind(workspace_id)
        .bind(conversation_id)
        .bind(owner_user_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Conversation '{conversation_id}' not found")));
        }
        Ok(())
    }

    async fn get_for_conversation(
        &self,
        owner_user_id: &str,
        conversation_id: &str,
    ) -> Result<Option<WorkspaceRow>, DbError> {
        let row = sqlx::query_as::<_, WorkspaceRow>(
            "SELECT w.*
             FROM workspaces w
             INNER JOIN conversations c ON c.workspace_id = w.id
             WHERE c.id = ? AND c.user_id = ? AND w.owner_user_id = ?
               AND w.status = 'active'",
        )
        .bind(conversation_id)
        .bind(owner_user_id)
        .bind(owner_user_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn bind_team(&self, owner_user_id: &str, team_id: &str, workspace_id: &str) -> Result<(), DbError> {
        let result = sqlx::query(
            "UPDATE teams
             SET workspace_id = ?
             WHERE id = ? AND user_id = ?",
        )
        .bind(workspace_id)
        .bind(team_id)
        .bind(owner_user_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound(format!("Team '{team_id}' not found")));
        }
        Ok(())
    }
}
