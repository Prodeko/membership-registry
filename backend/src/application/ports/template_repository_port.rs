use crate::domain::{EmailTemplate, EmailTemplateTranslation};

use super::repository_error::RepositoryError;

#[async_trait::async_trait]
pub trait TemplateRepositoryPort: Send + Sync {
    async fn fetch_all(&self) -> Result<Vec<EmailTemplate>, RepositoryError>;
    async fn create(&self, name: &str) -> Result<EmailTemplate, RepositoryError>;
    async fn delete(&self, name: &str) -> Result<(), RepositoryError>;

    async fn fetch_translation(
        &self,
        name: &str,
        locale: &str,
    ) -> Result<EmailTemplateTranslation, RepositoryError>;
    async fn fetch_translations(
        &self,
        name: &str,
    ) -> Result<Vec<EmailTemplateTranslation>, RepositoryError>;
    async fn upsert_translation(
        &self,
        template_name: &str,
        locale: &str,
        subject: &str,
        body_html: &str,
    ) -> Result<EmailTemplateTranslation, RepositoryError>;
    async fn delete_translation(
        &self,
        template_name: &str,
        locale: &str,
    ) -> Result<(), RepositoryError>;
}
