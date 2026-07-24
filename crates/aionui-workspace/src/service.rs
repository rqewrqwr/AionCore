use std::path::{Path, PathBuf};
use std::sync::Arc;

use aionui_api_types::{WorkspaceListResponse, WorkspaceResponse};
use aionui_common::{generate_id, now_ms};
use aionui_db::IWorkspaceRepository;
use aionui_db::models::WorkspaceRow;
use async_trait::async_trait;

use crate::error::WorkspaceError;

pub const WORKSPACE_REFERENCE_PREFIX: &str = "workspace://";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceAccess {
    pub id: String,
    pub root_path: PathBuf,
}

#[async_trait]
pub trait WorkspaceAccessPort: Send + Sync {
    async fn prepare_conversation_workspace(
        &self,
        owner_user_id: &str,
        conversation_id: &str,
        name: &str,
        requested_reference: Option<&str>,
    ) -> Result<WorkspaceAccess, WorkspaceError>;

    async fn bind_conversation(
        &self,
        owner_user_id: &str,
        conversation_id: &str,
        workspace_id: &str,
    ) -> Result<(), WorkspaceError>;

    async fn resolve_owned(&self, owner_user_id: &str, reference: &str) -> Result<WorkspaceAccess, WorkspaceError>;
}

#[derive(Clone)]
pub struct WorkspaceService {
    repo: Arc<dyn IWorkspaceRepository>,
    work_dir: PathBuf,
    allow_external_paths: bool,
}

impl WorkspaceService {
    pub fn new(repo: Arc<dyn IWorkspaceRepository>, work_dir: PathBuf, allow_external_paths: bool) -> Self {
        Self {
            repo,
            work_dir,
            allow_external_paths,
        }
    }

    pub async fn create_personal(&self, owner_user_id: &str, name: &str) -> Result<WorkspaceResponse, WorkspaceError> {
        let row = self.create_managed_row(owner_user_id, name).await?;
        Ok(to_response(row))
    }

    pub async fn list_personal(&self, owner_user_id: &str) -> Result<WorkspaceListResponse, WorkspaceError> {
        let rows = self.repo.list_owned(owner_user_id).await?;
        Ok(rows.into_iter().map(to_response).collect())
    }

    async fn create_managed_row(&self, owner_user_id: &str, name: &str) -> Result<WorkspaceRow, WorkspaceError> {
        let id = generate_id();
        let shard = id.get(..2).unwrap_or("00");
        let root_path = self.work_dir.join("workspaces").join(shard).join(&id).join("root");
        tokio::fs::create_dir_all(&root_path)
            .await
            .map_err(WorkspaceError::Storage)?;
        let now = now_ms();
        let row = WorkspaceRow {
            id,
            owner_user_id: owner_user_id.to_owned(),
            name: normalized_name(name),
            root_path: root_path.to_string_lossy().into_owned(),
            storage_type: "managed".to_owned(),
            status: "active".to_owned(),
            created_at: now,
            updated_at: now,
        };
        if let Err(error) = self.repo.create(&row).await {
            let _ = tokio::fs::remove_dir_all(&root_path).await;
            return Err(error.into());
        }
        tracing::info!(
            workspace_id = %row.id,
            owner_user_id,
            storage_type = "managed",
            "Personal workspace created"
        );
        Ok(row)
    }

    async fn adopt_external_row(
        &self,
        owner_user_id: &str,
        name: &str,
        root_path: &Path,
    ) -> Result<WorkspaceRow, WorkspaceError> {
        let canonical = root_path.canonicalize().map_err(WorkspaceError::Storage)?;
        if !canonical.is_dir() {
            return Err(WorkspaceError::Invalid(
                "Workspace must reference an existing directory".to_owned(),
            ));
        }
        let root = canonical.to_string_lossy().into_owned();
        if let Some(existing) = self.repo.find_owned_by_root(owner_user_id, &root).await? {
            return Ok(existing);
        }
        let now = now_ms();
        let row = WorkspaceRow {
            id: generate_id(),
            owner_user_id: owner_user_id.to_owned(),
            name: normalized_name(name),
            root_path: root,
            storage_type: "local_external".to_owned(),
            status: "active".to_owned(),
            created_at: now,
            updated_at: now,
        };
        self.repo.create(&row).await?;
        Ok(row)
    }

    async fn resolve_requested(
        &self,
        owner_user_id: &str,
        name: &str,
        reference: &str,
    ) -> Result<WorkspaceRow, WorkspaceError> {
        let reference = reference.trim();
        if let Some(id) = reference.strip_prefix(WORKSPACE_REFERENCE_PREFIX) {
            return self
                .repo
                .get_owned(owner_user_id, id)
                .await?
                .ok_or(WorkspaceError::NotFound);
        }
        if let Some(existing) = self.repo.find_owned_by_root(owner_user_id, reference).await? {
            return Ok(existing);
        }
        if !self.allow_external_paths {
            tracing::warn!(owner_user_id, "External workspace path rejected in multi-user mode");
            return Err(WorkspaceError::Forbidden);
        }
        self.adopt_external_row(owner_user_id, name, Path::new(reference)).await
    }
}

#[async_trait]
impl WorkspaceAccessPort for WorkspaceService {
    async fn prepare_conversation_workspace(
        &self,
        owner_user_id: &str,
        conversation_id: &str,
        name: &str,
        requested_reference: Option<&str>,
    ) -> Result<WorkspaceAccess, WorkspaceError> {
        if let Some(existing) = self.repo.get_for_conversation(owner_user_id, conversation_id).await? {
            return Ok(to_access(existing));
        }
        let row = match requested_reference.map(str::trim).filter(|value| !value.is_empty()) {
            Some(reference) => self.resolve_requested(owner_user_id, name, reference).await?,
            None => self.create_managed_row(owner_user_id, name).await?,
        };
        Ok(to_access(row))
    }

    async fn bind_conversation(
        &self,
        owner_user_id: &str,
        conversation_id: &str,
        workspace_id: &str,
    ) -> Result<(), WorkspaceError> {
        self.repo
            .bind_conversation(owner_user_id, conversation_id, workspace_id)
            .await?;
        Ok(())
    }

    async fn resolve_owned(&self, owner_user_id: &str, reference: &str) -> Result<WorkspaceAccess, WorkspaceError> {
        let row = self.resolve_requested(owner_user_id, "Workspace", reference).await?;
        Ok(to_access(row))
    }
}

fn normalized_name(name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        "Workspace".to_owned()
    } else {
        name.chars().take(120).collect()
    }
}

fn to_access(row: WorkspaceRow) -> WorkspaceAccess {
    WorkspaceAccess {
        id: row.id,
        root_path: PathBuf::from(row.root_path),
    }
}

fn to_response(row: WorkspaceRow) -> WorkspaceResponse {
    WorkspaceResponse {
        id: row.id,
        name: row.name,
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aionui_db::{SqliteWorkspaceRepository, init_database_memory};

    #[tokio::test]
    async fn managed_workspaces_are_private_and_server_backed() {
        let database = init_database_memory().await.expect("database");
        let temp = tempfile::tempdir().expect("temp dir");
        let repo = Arc::new(SqliteWorkspaceRepository::new(database.pool().clone()));
        let service = WorkspaceService::new(repo, temp.path().to_path_buf(), false);

        let created = service
            .create_personal("user-a", "Private project")
            .await
            .expect("create workspace");
        let access = service
            .resolve_owned("user-a", &format!("{WORKSPACE_REFERENCE_PREFIX}{}", created.id))
            .await
            .expect("owner can resolve");

        assert!(access.root_path.starts_with(temp.path().join("workspaces")));
        assert!(access.root_path.is_dir());
        assert_eq!(service.list_personal("user-a").await.expect("owner list").len(), 1);
        assert!(service.list_personal("user-b").await.expect("other list").is_empty());
        assert!(matches!(
            service
                .resolve_owned("user-b", &format!("{WORKSPACE_REFERENCE_PREFIX}{}", created.id))
                .await,
            Err(WorkspaceError::NotFound)
        ));
    }

    #[tokio::test]
    async fn multi_user_mode_rejects_host_paths() {
        let database = init_database_memory().await.expect("database");
        let temp = tempfile::tempdir().expect("temp dir");
        let repo = Arc::new(SqliteWorkspaceRepository::new(database.pool().clone()));
        let service = WorkspaceService::new(repo, temp.path().to_path_buf(), false);

        assert!(matches!(
            service.resolve_owned("user-a", &temp.path().to_string_lossy()).await,
            Err(WorkspaceError::Forbidden)
        ));
    }
}
