use std::sync::{Arc, LazyLock};

use regex::Regex;

use crate::application::ports::email_port::EmailPort;
use crate::repositories::email_template::{EmailTemplate, EmailTemplateRepo};

use super::errors::{ServiceError, ServiceResult};

const ALLOWED_PLACEHOLDERS: &[&str] = &["name", "role_name"];
#[allow(clippy::expect_used)] // Regex literal, cannot fail
static PLACEHOLDER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{(\w+)\}").expect("valid regex"));

#[derive(Clone)]
pub struct NotificationService {
    email_port: Option<Arc<dyn EmailPort>>,
    email_template_repo: EmailTemplateRepo,
}

impl NotificationService {
    pub fn new(
        email_port: Option<Arc<dyn EmailPort>>,
        email_template_repo: EmailTemplateRepo,
    ) -> Self {
        Self {
            email_port,
            email_template_repo,
        }
    }

    fn validate_placeholders(text: &str) -> ServiceResult<()> {
        for cap in PLACEHOLDER_RE.captures_iter(text) {
            let name = &cap[1];
            if !ALLOWED_PLACEHOLDERS.contains(&name) {
                tracing::error!("Invalid placeholder in template: {{{name}}}");
                return Err(ServiceError::InvalidTemplate);
            }
        }
        Ok(())
    }

    fn sanitize_body(body_html: &str) -> ServiceResult<String> {
        Self::validate_placeholders(body_html)?;
        Ok(ammonia::clean(body_html))
    }

    fn render_template(template: &str, name: &str, role_name: &str) -> String {
        template
            .replace("{name}", name)
            .replace("{role_name}", role_name)
    }

    pub async fn send_notification(
        &self,
        template_name: Option<&str>,
        to: Option<&str>,
        recipient_name: &str,
        role_name: &str,
    ) {
        let Some(template_name) = template_name else {
            return;
        };

        let Some(to) = to else {
            return;
        };

        let template = match self.email_template_repo.fetch_one(template_name).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Email template '{template_name}' configured but not found: {e}");
                return;
            }
        };

        let subject = Self::render_template(&template.subject, recipient_name, role_name);
        let body = Self::render_template(&template.body_html, recipient_name, role_name);

        let Some(port) = &self.email_port else {
            tracing::info!("[MOCK EMAIL] To: {to}, Subject: {subject}");
            return;
        };

        let to = to.to_string();
        let port = Arc::clone(port);
        tokio::spawn(async move {
            if let Err(e) = port.send_email(&to, &subject, &body).await {
                tracing::error!("Failed to send email to {to}: {e:?}");
            }
        });
    }

    // -- Email template CRUD --

    pub async fn get_all_templates(&self) -> ServiceResult<Vec<EmailTemplate>> {
        self.email_template_repo
            .fetch_all()
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_template(&self, name: &str) -> ServiceResult<EmailTemplate> {
        self.email_template_repo
            .fetch_one(name)
            .await
            .map_err(|e| e.into())
    }

    pub async fn create_template(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> ServiceResult<EmailTemplate> {
        Self::validate_placeholders(subject)?;
        let sanitized_body = Self::sanitize_body(body_html)?;
        self.email_template_repo
            .create(name, subject, &sanitized_body)
            .await
            .map_err(|e| e.into())
    }

    pub async fn update_template(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> ServiceResult<EmailTemplate> {
        Self::validate_placeholders(subject)?;
        let sanitized_body = Self::sanitize_body(body_html)?;
        self.email_template_repo
            .update(name, subject, &sanitized_body)
            .await
            .map_err(|e| e.into())
    }

    pub async fn delete_template(&self, name: &str) -> ServiceResult<()> {
        self.email_template_repo
            .delete(name)
            .await
            .map_err(|e| e.into())
    }
}
