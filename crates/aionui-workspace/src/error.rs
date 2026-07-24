use aionui_common::ApiError;
use aionui_db::DbError;

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error("Workspace not found")]
    NotFound,
    #[error("Workspace access is forbidden")]
    Forbidden,
    #[error("Invalid workspace: {0}")]
    Invalid(String),
    #[error("Workspace storage failed")]
    Storage(#[source] std::io::Error),
    #[error("Workspace persistence failed")]
    Database(#[source] DbError),
}

impl From<DbError> for WorkspaceError {
    fn from(error: DbError) -> Self {
        Self::Database(error)
    }
}

impl From<WorkspaceError> for ApiError {
    fn from(error: WorkspaceError) -> Self {
        match error {
            WorkspaceError::NotFound => ApiError::NotFound("Workspace not found".to_owned()),
            WorkspaceError::Forbidden => ApiError::NotFound("Workspace not found".to_owned()),
            WorkspaceError::Invalid(message) => ApiError::BadRequest(message),
            WorkspaceError::Storage(_) | WorkspaceError::Database(_) => {
                ApiError::Internal("Workspace operation failed".to_owned())
            }
        }
    }
}
