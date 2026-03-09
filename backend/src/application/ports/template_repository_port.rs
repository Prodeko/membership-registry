use crate::domain::EmailTemplate;

use super::repository_error::RepositoryError;

#[async_trait::async_trait]
pub trait TemplateRepositoryPort: Send + Sync {
    async fn fetch_all(&self) -> Result<Vec<EmailTemplate>, RepositoryError>;
    async fn fetch_one(&self, name: &str) -> Result<EmailTemplate, RepositoryError>;
    async fn create(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> Result<EmailTemplate, RepositoryError>;
    async fn update(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> Result<EmailTemplate, RepositoryError>;
    async fn delete(&self, name: &str) -> Result<(), RepositoryError>;
}
