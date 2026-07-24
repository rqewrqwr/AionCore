#![warn(clippy::disallowed_types)]

//! SQLite database layer: init, migrations, repository traits, and implementations.
mod agent_binding;
mod database;
mod error;
mod legacy_handoff;
pub mod models;
mod repository;

pub use agent_binding::{
    AgentBindingResolution, binding_resolution_for_agent, resolve_agent_binding, resolve_agent_binding_from_rows,
    runtime_backend_for_agent,
};
pub use database::{
    Database, DatabaseInitError, DatabaseInitOptions, init_database, init_database_memory, init_database_staged,
    init_database_staged_with_options, init_database_with_options, maybe_copy_legacy_database,
};
pub use error::DbError;
pub use models::{
    AgentMetadataRow, AssistantDefinitionRow, AssistantOverlayRow, AssistantOverrideRow, AssistantPreferenceRow,
    AssistantRow, ConversationArtifactRow, ConversationAssistantSnapshotRow, CreateAssistantParams,
    SkillImportRecordRow, SkillRow, UpdateAgentAvailabilitySnapshotParams, UpdateAgentHandshakeParams,
    UpdateAssistantParams, UpsertAgentMetadataParams, UpsertAssistantDefinitionParams, UpsertAssistantOverlayParams,
    UpsertAssistantPreferenceParams, UpsertConversationAssistantSnapshotParams, UpsertOverrideParams, WorkspaceRow,
};
pub use repository::channel::UpdatePluginStatusParams;
pub use repository::conversation::{
    ConversationFilters, ConversationRowUpdate, MessagePageCursor, MessagePageDirection, MessagePageParams,
    MessagePageResult, MessageRowUpdate, MessageSearchRow,
};
pub use repository::cron::{
    ClaimCronRunParams, CronRunClaimResult, FinishCronRunParams, RecoverableCronRun, UpdateCronJobParams,
};
pub use repository::mcp_server::{CreateMcpServerParams, UpdateMcpServerParams};
pub use repository::oauth_token::UpsertOAuthTokenParams;
pub use repository::provider::{CreateProviderParams, UpdateProviderParams};
pub use repository::remote_agent::{CreateRemoteAgentParams, UpdateRemoteAgentParams};
pub use repository::skill::{
    CreateSkillImportRecordParams, DEFAULT_SKILL_OWNER, SHARED_SKILL_OWNER, UpsertSkillParams,
};
pub use repository::team::{UpdateTaskParams, UpdateTeamParams};
pub use repository::{
    CreateAcpSessionParams, FeedbackDiagnosticsDbContext, FeedbackDiagnosticsProfile, FeedbackDiagnosticsProfileResult,
    FeedbackDiagnosticsRequest, FeedbackDiagnosticsResult, IAcpSessionRepository, IAgentMetadataRepository,
    IAssistantDefinitionRepository, IAssistantOverlayRepository, IAssistantOverrideRepository,
    IAssistantPreferenceRepository, IAssistantRepository, IChannelRepository, IClientPreferenceRepository,
    IConversationRepository, ICronRepository, IFeedbackDiagnosticsRepository, IMcpServerRepository,
    IOAuthTokenRepository, IProviderRepository, IRemoteAgentRepository, ISettingsRepository, ISkillRepository,
    ITeamRepository, IUserRepository, IWorkspaceRepository, PersistedSessionState, SaveRuntimeStateParams,
    SqliteAcpSessionRepository, SqliteAgentMetadataRepository, SqliteAssistantDefinitionRepository,
    SqliteAssistantOverlayRepository, SqliteAssistantOverrideRepository, SqliteAssistantPreferenceRepository,
    SqliteAssistantRepository, SqliteChannelRepository, SqliteClientPreferenceRepository, SqliteConversationRepository,
    SqliteCronRepository, SqliteFeedbackDiagnosticsRepository, SqliteMcpServerRepository, SqliteOAuthTokenRepository,
    SqliteProviderRepository, SqliteRemoteAgentRepository, SqliteSettingsRepository, SqliteSkillRepository,
    SqliteTeamRepository, SqliteUserRepository, SqliteWorkspaceRepository,
};

// Re-export sqlx pool type for downstream crates
pub use sqlx::SqlitePool;

/// Owner used by trusted single-user/local mode and legacy rows.
pub const DEFAULT_RESOURCE_OWNER: &str = "system_default_user";
/// Owner marker for built-in resources readable by every authenticated user.
pub const SHARED_RESOURCE_OWNER: &str = "__shared__";

/// Stable, non-reversible subject key used by the enterprise ownership table.
pub fn resource_owner_subject(user_id: &str) -> String {
    use sha2::{Digest, Sha256};

    hex::encode(Sha256::digest(user_id.as_bytes()))[..24].to_owned()
}
