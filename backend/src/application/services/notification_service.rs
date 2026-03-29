use std::sync::Arc;

use crate::application::ports::email_port::EmailPort;
use crate::application::ports::template_renderer_port::TemplateRendererPort;
use crate::application::ports::template_repository_port::TemplateRepositoryPort;

#[derive(Clone)]
pub struct NotificationService {
    email_port: Option<Arc<dyn EmailPort>>,
    template_repo: Arc<dyn TemplateRepositoryPort>,
    renderer: Arc<dyn TemplateRendererPort>,
}

impl NotificationService {
    pub fn new(
        email_port: Option<Arc<dyn EmailPort>>,
        template_repo: Arc<dyn TemplateRepositoryPort>,
        renderer: Arc<dyn TemplateRendererPort>,
    ) -> Self {
        Self {
            email_port,
            template_repo,
            renderer,
        }
    }

    pub async fn send_notification(
        &self,
        template_name: Option<&str>,
        to: Option<&str>,
        recipient_name: &str,
        role_name: &str,
        locale: &str,
    ) {
        self.send_notification_with_vars(template_name, to, recipient_name, role_name, locale, &[])
            .await;
    }

    pub async fn send_notification_with_vars(
        &self,
        template_name: Option<&str>,
        to: Option<&str>,
        recipient_name: &str,
        role_name: &str,
        locale: &str,
        extra_vars: &[(&str, &str)],
    ) {
        let Some(template_name) = template_name else {
            return;
        };

        let Some(to) = to else {
            return;
        };

        let translation = match self
            .template_repo
            .fetch_translation(template_name, locale)
            .await
        {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Email template '{template_name}' configured but not found: {e:?}");
                return;
            }
        };

        let mut variables: Vec<(&str, &str)> =
            vec![("name", recipient_name), ("role_name", role_name)];
        variables.extend_from_slice(extra_vars);
        let subject = self.renderer.render(&translation.subject, &variables);
        let body = self.renderer.render(&translation.body_html, &variables);

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
}
