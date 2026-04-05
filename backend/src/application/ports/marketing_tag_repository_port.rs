use crate::domain::MarketingTag;

use super::repository_error::RepositoryError;

#[async_trait::async_trait]
pub trait MarketingTagRepositoryPort: Send + Sync {
    /// Fetch all marketing tags ordered by `display_order` ascending,
    /// tie-broken by `label` for stability.
    async fn fetch_all(&self) -> Result<Vec<MarketingTag>, RepositoryError>;

    async fn create(&self, tag: &MarketingTag) -> Result<MarketingTag, RepositoryError>;

    async fn update(&self, tag: &MarketingTag) -> Result<MarketingTag, RepositoryError>;

    async fn delete(&self, label: &str) -> Result<(), RepositoryError>;
}
