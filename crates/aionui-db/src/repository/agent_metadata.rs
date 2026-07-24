//! Repository trait for the `agent_metadata` catalog.

use crate::error::DbError;
use crate::models::{
    AgentMetadataRow, UpdateAgentAvailabilitySnapshotParams, UpdateAgentHandshakeParams, UpsertAgentMetadataParams,
};

/// CRUD access for agent metadata rows.
///
/// The table is the single source of truth for how each agent is spawned
/// and what static capabilities it exposes. Handshake-derived fields
/// (`agent_capabilities`, `auth_methods`, `config_options`,
/// `available_modes`, `available_models`, `available_commands`) are
/// refreshed separately via [`IAgentMetadataRepository::apply_handshake`].
#[async_trait::async_trait]
pub trait IAgentMetadataRepository: Send + Sync {
    /// Return every row, in insertion order.
    async fn list_all(&self) -> Result<Vec<AgentMetadataRow>, DbError>;

    /// Return the catalog visible to one user. Shared agent definitions remain
    /// visible; custom definitions require an ownership record.
    async fn list_all_for_user(&self, user_id: &str) -> Result<Vec<AgentMetadataRow>, DbError> {
        let _ = user_id;
        self.list_all().await
    }

    /// Look up by primary key.
    async fn get(&self, id: &str) -> Result<Option<AgentMetadataRow>, DbError>;

    /// User-scoped lookup used at API and runtime trust boundaries.
    async fn get_for_user(&self, user_id: &str, id: &str) -> Result<Option<AgentMetadataRow>, DbError> {
        let _ = user_id;
        self.get(id).await
    }

    /// Persist ownership for a newly-created custom agent.
    async fn assign_to_user(&self, _user_id: &str, _id: &str) -> Result<(), DbError> {
        Ok(())
    }

    /// Look up by the unique `(agent_source, name)` pair.
    async fn find_by_source_and_name(
        &self,
        agent_source: &str,
        name: &str,
    ) -> Result<Option<AgentMetadataRow>, DbError>;

    /// Look up the first `builtin` row whose vendor label matches.
    /// Useful when the caller only has the legacy `backend` string and
    /// not a full agent id.
    async fn find_builtin_by_backend(&self, backend: &str) -> Result<Option<AgentMetadataRow>, DbError>;

    /// Insert or replace a row. Returns the row as stored.
    async fn upsert(&self, params: &UpsertAgentMetadataParams<'_>) -> Result<AgentMetadataRow, DbError>;

    /// Apply handshake-derived fields on top of an existing row.
    /// Returns `Ok(None)` if no row matches `id`.
    async fn apply_handshake(
        &self,
        id: &str,
        params: &UpdateAgentHandshakeParams<'_>,
    ) -> Result<Option<AgentMetadataRow>, DbError>;

    /// Persist the latest availability snapshot for an existing row.
    /// Returns `Ok(None)` if no row matches `id`.
    async fn update_availability_snapshot(
        &self,
        id: &str,
        params: &UpdateAgentAvailabilitySnapshotParams<'_>,
    ) -> Result<Option<AgentMetadataRow>, DbError>;

    /// Write only the self-repair override columns for an agent, leaving all
    /// other columns (seed truth + availability snapshot) untouched. Kept
    /// separate from the full-row upsert so startup reconcile never clobbers
    /// user overrides.
    async fn update_agent_overrides(
        &self,
        id: &str,
        command_override: Option<&str>,
        env_override: Option<&str>,
    ) -> Result<(), DbError>;

    /// Toggle the `enabled` flag. Returns `true` if a row was updated.
    async fn set_enabled(&self, id: &str, enabled: bool) -> Result<bool, DbError>;

    /// Delete a row. Returns `true` if a row was removed.
    async fn delete(&self, id: &str) -> Result<bool, DbError>;
}
