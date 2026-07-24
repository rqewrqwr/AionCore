use crate::error::DbError;
use crate::models::Provider;

/// Model provider data access abstraction.
///
/// Provides CRUD operations on the `providers` table.
/// API keys are stored encrypted; callers handle encryption/decryption.
#[async_trait::async_trait]
pub trait IProviderRepository: Send + Sync {
    /// Returns all providers, ordered by creation time ascending.
    async fn list(&self) -> Result<Vec<Provider>, DbError>;

    /// Returns providers owned by the user plus shared system providers.
    async fn list_for_user(&self, user_id: &str) -> Result<Vec<Provider>, DbError> {
        Ok(self
            .list()
            .await?
            .into_iter()
            .filter(|row| row.owner_user_id == user_id || row.owner_user_id == crate::SHARED_RESOURCE_OWNER)
            .collect())
    }

    /// Finds a provider by ID, or `None` if not found.
    async fn find_by_id(&self, id: &str) -> Result<Option<Provider>, DbError>;

    /// Finds a provider only when the user owns it or it is shared.
    async fn find_by_id_for_user(&self, user_id: &str, id: &str) -> Result<Option<Provider>, DbError> {
        Ok(self
            .find_by_id(id)
            .await?
            .filter(|row| row.owner_user_id == user_id || row.owner_user_id == crate::SHARED_RESOURCE_OWNER))
    }

    /// Creates a new provider and returns the inserted row.
    async fn create(&self, params: CreateProviderParams<'_>) -> Result<Provider, DbError>;

    /// Creates a provider owned by the authenticated user.
    async fn create_for_user(&self, _user_id: &str, params: CreateProviderParams<'_>) -> Result<Provider, DbError> {
        self.create(params).await
    }

    /// Updates an existing provider. Returns `DbError::NotFound` if the ID doesn't exist.
    async fn update(&self, id: &str, params: UpdateProviderParams<'_>) -> Result<Provider, DbError>;

    async fn update_for_user(
        &self,
        user_id: &str,
        id: &str,
        params: UpdateProviderParams<'_>,
    ) -> Result<Provider, DbError> {
        self.find_by_id_for_user(user_id, id)
            .await?
            .filter(|row| row.owner_user_id == user_id)
            .ok_or_else(|| DbError::NotFound(format!("Provider '{id}' not found")))?;
        self.update(id, params).await
    }

    /// Deletes a provider by ID. Returns `DbError::NotFound` if the ID doesn't exist.
    async fn delete(&self, id: &str) -> Result<(), DbError>;

    async fn delete_for_user(&self, user_id: &str, id: &str) -> Result<(), DbError> {
        self.find_by_id_for_user(user_id, id)
            .await?
            .filter(|row| row.owner_user_id == user_id)
            .ok_or_else(|| DbError::NotFound(format!("Provider '{id}' not found")))?;
        self.delete(id).await
    }
}

/// Parameters for creating a new provider.
#[derive(Debug)]
pub struct CreateProviderParams<'a> {
    /// Optional caller-supplied id. When `None`, the repository generates one.
    pub id: Option<&'a str>,
    pub platform: &'a str,
    pub name: &'a str,
    pub base_url: &'a str,
    pub api_key_encrypted: &'a str,
    pub models: &'a str,
    pub enabled: bool,
    pub capabilities: &'a str,
    pub context_limit: Option<i64>,
    pub model_protocols: Option<&'a str>,
    pub model_enabled: Option<&'a str>,
    pub model_health: Option<&'a str>,
    pub bedrock_config: Option<&'a str>,
    pub is_full_url: bool,
}

/// Parameters for updating an existing provider.
///
/// All fields are optional; `None` means "keep the current value".
#[derive(Debug, Default)]
pub struct UpdateProviderParams<'a> {
    pub platform: Option<&'a str>,
    pub name: Option<&'a str>,
    pub base_url: Option<&'a str>,
    pub api_key_encrypted: Option<&'a str>,
    pub models: Option<&'a str>,
    pub enabled: Option<bool>,
    pub capabilities: Option<&'a str>,
    pub context_limit: Option<Option<i64>>,
    pub model_protocols: Option<Option<&'a str>>,
    pub model_enabled: Option<Option<&'a str>>,
    pub model_health: Option<Option<&'a str>>,
    pub bedrock_config: Option<Option<&'a str>>,
    pub is_full_url: Option<bool>,
}
