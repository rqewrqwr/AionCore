use crate::error::DbError;
use crate::models::WorkspaceRow;

/// Workspace persistence and ownership abstraction.
#[async_trait::async_trait]
pub trait IWorkspaceRepository: Send + Sync {
    async fn create(&self, row: &WorkspaceRow) -> Result<(), DbError>;

    async fn get_owned(&self, owner_user_id: &str, workspace_id: &str) -> Result<Option<WorkspaceRow>, DbError>;

    async fn find_owned_by_root(&self, owner_user_id: &str, root_path: &str) -> Result<Option<WorkspaceRow>, DbError>;

    async fn list_owned(&self, owner_user_id: &str) -> Result<Vec<WorkspaceRow>, DbError>;

    async fn bind_conversation(
        &self,
        owner_user_id: &str,
        conversation_id: &str,
        workspace_id: &str,
    ) -> Result<(), DbError>;

    async fn get_for_conversation(
        &self,
        owner_user_id: &str,
        conversation_id: &str,
    ) -> Result<Option<WorkspaceRow>, DbError>;

    async fn bind_team(&self, owner_user_id: &str, team_id: &str, workspace_id: &str) -> Result<(), DbError>;
}
