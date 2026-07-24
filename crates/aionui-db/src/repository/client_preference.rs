use crate::error::DbError;
use crate::models::ClientPreference;

/// Client preference data access abstraction.
///
/// Provides CRUD operations on the generic key-value `client_preferences` table.
#[async_trait::async_trait]
pub trait IClientPreferenceRepository: Send + Sync {
    /// Returns all client preferences.
    async fn get_all(&self) -> Result<Vec<ClientPreference>, DbError>;

    async fn get_all_for_user(&self, user_id: &str) -> Result<Vec<ClientPreference>, DbError> {
        let _ = user_id;
        self.get_all().await
    }

    /// Returns preferences for the given keys only.
    /// Keys that don't exist are simply omitted from the result.
    async fn get_by_keys(&self, keys: &[&str]) -> Result<Vec<ClientPreference>, DbError>;

    async fn get_by_keys_for_user(&self, user_id: &str, keys: &[&str]) -> Result<Vec<ClientPreference>, DbError> {
        let _ = user_id;
        self.get_by_keys(keys).await
    }

    /// Inserts or updates a batch of key-value pairs.
    async fn upsert_batch(&self, entries: &[(&str, &str)]) -> Result<(), DbError>;

    async fn upsert_batch_for_user(&self, user_id: &str, entries: &[(&str, &str)]) -> Result<(), DbError> {
        let _ = user_id;
        self.upsert_batch(entries).await
    }

    /// Deletes the given keys.
    async fn delete_keys(&self, keys: &[&str]) -> Result<(), DbError>;

    async fn delete_keys_for_user(&self, user_id: &str, keys: &[&str]) -> Result<(), DbError> {
        let _ = user_id;
        self.delete_keys(keys).await
    }
}
