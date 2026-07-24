use crate::error::DbError;
use crate::models::McpServerRow;

/// MCP server configuration data access abstraction.
///
/// Provides CRUD operations, batch upsert, and name-based lookup
/// on the `mcp_servers` table. JSON fields (`transport_config`, `tools`)
/// are opaque strings at this layer; the service layer handles
/// serialization/deserialization.
///
/// Object-safe via `async_trait` to support `Arc<dyn IMcpServerRepository>`.
#[async_trait::async_trait]
pub trait IMcpServerRepository: Send + Sync {
    /// Returns all MCP servers, ordered by creation time ascending.
    async fn list(&self) -> Result<Vec<McpServerRow>, DbError>;

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<McpServerRow>, DbError> {
        Ok(self
            .list()
            .await?
            .into_iter()
            .filter(|row| row.owner_user_id == user_id || row.owner_user_id == crate::SHARED_RESOURCE_OWNER)
            .collect())
    }

    /// Finds an MCP server by ID, or `None` if not found.
    async fn find_by_id(&self, id: &str) -> Result<Option<McpServerRow>, DbError>;

    async fn find_by_id_for_user(&self, user_id: &str, id: &str) -> Result<Option<McpServerRow>, DbError> {
        Ok(self
            .find_by_id(id)
            .await?
            .filter(|row| row.owner_user_id == user_id || row.owner_user_id == crate::SHARED_RESOURCE_OWNER))
    }

    /// Finds an MCP server by name, or `None` if not found.
    async fn find_by_name(&self, name: &str) -> Result<Option<McpServerRow>, DbError>;

    async fn find_by_name_for_user(&self, user_id: &str, name: &str) -> Result<Option<McpServerRow>, DbError> {
        Ok(self
            .find_by_name(name)
            .await?
            .filter(|row| row.owner_user_id == user_id || row.owner_user_id == crate::SHARED_RESOURCE_OWNER))
    }

    /// Finds an MCP server by ID, including soft-deleted rows.
    async fn find_by_id_any(&self, id: &str) -> Result<Option<McpServerRow>, DbError> {
        self.find_by_id(id).await
    }

    /// Finds an MCP server by name, including soft-deleted rows.
    async fn find_by_name_any(&self, name: &str) -> Result<Option<McpServerRow>, DbError> {
        self.find_by_name(name).await
    }

    async fn find_by_name_any_for_user(&self, user_id: &str, name: &str) -> Result<Option<McpServerRow>, DbError> {
        self.find_by_name_for_user(user_id, name).await
    }

    /// Finds a set of MCP servers by ID, including soft-deleted rows.
    async fn list_by_ids_any(&self, ids: &[String]) -> Result<Vec<McpServerRow>, DbError> {
        let mut rows = Vec::with_capacity(ids.len());
        for id in ids {
            if let Some(row) = self.find_by_id_any(id).await? {
                rows.push(row);
            }
        }
        Ok(rows)
    }

    async fn list_by_ids_for_user(&self, user_id: &str, ids: &[String]) -> Result<Vec<McpServerRow>, DbError> {
        Ok(self
            .list_by_ids_any(ids)
            .await?
            .into_iter()
            .filter(|row| row.owner_user_id == user_id || row.owner_user_id == crate::SHARED_RESOURCE_OWNER)
            .collect())
    }

    /// Creates a new MCP server and returns the inserted row.
    /// Returns `DbError::Conflict` if the name already exists.
    async fn create(&self, params: CreateMcpServerParams<'_>) -> Result<McpServerRow, DbError>;

    async fn create_for_user(
        &self,
        _user_id: &str,
        params: CreateMcpServerParams<'_>,
    ) -> Result<McpServerRow, DbError> {
        self.create(params).await
    }

    /// Updates an existing MCP server. Returns `DbError::NotFound` if the ID
    /// doesn't exist, `DbError::Conflict` if the new name collides with another.
    async fn update(&self, id: &str, params: UpdateMcpServerParams<'_>) -> Result<McpServerRow, DbError>;

    async fn update_for_user(
        &self,
        user_id: &str,
        id: &str,
        params: UpdateMcpServerParams<'_>,
    ) -> Result<McpServerRow, DbError> {
        self.find_by_id_for_user(user_id, id)
            .await?
            .filter(|row| row.owner_user_id == user_id)
            .ok_or_else(|| DbError::NotFound(format!("MCP server '{id}' not found")))?;
        self.update(id, params).await
    }

    /// Soft-deletes an MCP server by ID. Returns `DbError::NotFound` if the ID
    /// doesn't exist.
    async fn delete(&self, id: &str) -> Result<(), DbError>;

    async fn delete_for_user(&self, user_id: &str, id: &str) -> Result<(), DbError> {
        self.find_by_id_for_user(user_id, id)
            .await?
            .filter(|row| row.owner_user_id == user_id)
            .ok_or_else(|| DbError::NotFound(format!("MCP server '{id}' not found")))?;
        self.delete(id).await
    }

    /// Upserts multiple servers by name: existing names are updated,
    /// new names are inserted. Returns the count of affected rows.
    async fn batch_upsert(&self, servers: &[CreateMcpServerParams<'_>]) -> Result<Vec<McpServerRow>, DbError>;

    /// Updates only the latest connection-test result status
    /// (and optionally `last_connected`).
    /// Returns `DbError::NotFound` if the ID doesn't exist.
    async fn update_status(
        &self,
        id: &str,
        status: &str,
        last_connected: Option<aionui_common::TimestampMs>,
    ) -> Result<(), DbError>;

    /// Updates only the tools JSON for a server.
    /// Returns `DbError::NotFound` if the ID doesn't exist.
    async fn update_tools(&self, id: &str, tools: Option<&str>) -> Result<(), DbError>;
}

/// Parameters for creating a new MCP server.
#[derive(Debug, Clone)]
pub struct CreateMcpServerParams<'a> {
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub enabled: bool,
    pub transport_type: &'a str,
    pub transport_config: &'a str,
    pub tools: Option<&'a str>,
    pub original_json: Option<&'a str>,
    pub builtin: bool,
}

/// Parameters for updating an existing MCP server.
///
/// All fields are optional; `None` means "keep the current value".
/// For nullable fields, `Some(None)` means "clear the value" and
/// `Some(Some(v))` means "set to v".
#[derive(Debug, Default)]
pub struct UpdateMcpServerParams<'a> {
    pub name: Option<&'a str>,
    pub description: Option<Option<&'a str>>,
    pub enabled: Option<bool>,
    pub transport_type: Option<&'a str>,
    pub transport_config: Option<&'a str>,
    pub tools: Option<Option<&'a str>>,
    pub original_json: Option<Option<&'a str>>,
    pub builtin: Option<bool>,
    pub deleted_at: Option<Option<aionui_common::TimestampMs>>,
}
