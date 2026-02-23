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
    ) {
        let Some(template_name) = template_name else {
            return;
        };

        let Some(to) = to else {
            return;
        };

        let template = match self.template_repo.fetch_one(template_name).await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!(
                    "Email template '{template_name}' configured but not found: {e:?}"
                );
                return;
            }
        };

        let variables = &[("name", recipient_name), ("role_name", role_name)];
        let subject = self.renderer.render(&template.subject, variables);
        let body = self.renderer.render(&template.body_html, variables);

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
