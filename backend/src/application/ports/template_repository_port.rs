use crate::domain::EmailTemplate;

#[derive(Debug)]
pub enum TemplateRepositoryError {
    NotFound,
    AlreadyExists,
    Constraint(String),
    Unexpected(String),
}

#[async_trait::async_trait]
pub trait TemplateRepositoryPort: Send + Sync {
    async fn fetch_all(&self) -> Result<Vec<EmailTemplate>, TemplateRepositoryError>;
    async fn fetch_one(&self, name: &str) -> Result<EmailTemplate, TemplateRepositoryError>;
    async fn create(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> Result<EmailTemplate, TemplateRepositoryError>;
    async fn update(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> Result<EmailTemplate, TemplateRepositoryError>;
    async fn delete(&self, name: &str) -> Result<(), TemplateRepositoryError>;
}
