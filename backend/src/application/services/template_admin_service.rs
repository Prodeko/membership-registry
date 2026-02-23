use std::sync::{Arc, LazyLock};

use regex::Regex;

use crate::application::ports::template_repository_port::{
    TemplateRepositoryError, TemplateRepositoryPort,
};
use crate::domain::EmailTemplate;

const ALLOWED_PLACEHOLDERS: &[&str] = &["name", "role_name"];
#[allow(clippy::expect_used)] // Regex literal, cannot fail
static PLACEHOLDER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{(\w+)\}").expect("valid regex"));

#[derive(Debug)]
pub enum TemplateAdminError {
    InvalidPlaceholder(String),
    Repository(TemplateRepositoryError),
}

impl From<TemplateRepositoryError> for TemplateAdminError {
    fn from(e: TemplateRepositoryError) -> Self {
        Self::Repository(e)
    }
}

pub type TemplateAdminResult<T> = Result<T, TemplateAdminError>;

#[derive(Clone)]
pub struct TemplateAdminService {
    repo: Arc<dyn TemplateRepositoryPort>,
}

impl TemplateAdminService {
    pub fn new(repo: Arc<dyn TemplateRepositoryPort>) -> Self {
        Self { repo }
    }

    fn validate_placeholders(text: &str) -> TemplateAdminResult<()> {
        for cap in PLACEHOLDER_RE.captures_iter(text) {
            let name = &cap[1];
            if !ALLOWED_PLACEHOLDERS.contains(&name) {
                tracing::error!("Invalid placeholder in template: {{{name}}}");
                return Err(TemplateAdminError::InvalidPlaceholder(name.to_string()));
            }
        }
        Ok(())
    }

    fn sanitize_body(body_html: &str) -> TemplateAdminResult<String> {
        Self::validate_placeholders(body_html)?;
        Ok(ammonia::clean(body_html))
    }

    pub async fn get_all_templates(&self) -> TemplateAdminResult<Vec<EmailTemplate>> {
        self.repo.fetch_all().await.map_err(Into::into)
    }

    pub async fn get_template(&self, name: &str) -> TemplateAdminResult<EmailTemplate> {
        self.repo.fetch_one(name).await.map_err(Into::into)
    }

    pub async fn create_template(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> TemplateAdminResult<EmailTemplate> {
        Self::validate_placeholders(subject)?;
        let sanitized_body = Self::sanitize_body(body_html)?;
        self.repo
            .create(name, subject, &sanitized_body)
            .await
            .map_err(Into::into)
    }

    pub async fn update_template(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> TemplateAdminResult<EmailTemplate> {
        Self::validate_placeholders(subject)?;
        let sanitized_body = Self::sanitize_body(body_html)?;
        self.repo
            .update(name, subject, &sanitized_body)
            .await
            .map_err(Into::into)
    }

    pub async fn delete_template(&self, name: &str) -> TemplateAdminResult<()> {
        self.repo.delete(name).await.map_err(Into::into)
    }
}
