use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct AuthProviderMapping {
    pub user_id: Uuid,
    pub provider_name: String,
    pub provider_user_id: String,
    pub linked_at: DateTime<Utc>,
}

#[derive(Debug)]
pub enum AuthProviderRepoError {
    NotFound,
    AlreadyExists,
    Unavailable(String),
}

#[async_trait::async_trait]
pub trait AuthProviderRepositoryPort: Send + Sync {
    async fn find_by_provider(
        &self,
        provider_name: &str,
        provider_user_id: &str,
    ) -> Result<Option<AuthProviderMapping>, AuthProviderRepoError>;

    async fn find_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<AuthProviderMapping>, AuthProviderRepoError>;

    async fn create(
        &self,
        user_id: &Uuid,
        provider_name: &str,
        provider_user_id: &str,
    ) -> Result<AuthProviderMapping, AuthProviderRepoError>;

    async fn delete(
        &self,
        user_id: &Uuid,
        provider_name: &str,
    ) -> Result<bool, AuthProviderRepoError>;

    async fn count_by_user_id(&self, user_id: &Uuid) -> Result<usize, AuthProviderRepoError>;
}
