use std::collections::BTreeSet;
use std::sync::Arc;

use crate::application::ports::application_repository_port::{
    ApplicationQueryPort, ApplicationWithMember,
};
use crate::application::ports::attribute_repository_port::AttributeRepositoryPort;
use crate::application::ports::email_port::EmailPort;
use crate::domain::well_known::ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE;
use crate::domain::{ApplicationStatus, AttributeName};

/// Sends admins a daily digest email listing membership applications that
/// await a decision. Recipients are members holding the
/// `admin-notifications-email` attribute; the digest goes to the attribute's
/// value. Invoked by the cron job wired in `main.rs` — every failure is
/// logged and swallowed so the job never aborts.
#[derive(Clone)]
pub struct ApplicationDigestService {
    application_queries: Arc<dyn ApplicationQueryPort>,
    attribute_repo: Arc<dyn AttributeRepositoryPort>,
    email_port: Option<Arc<dyn EmailPort>>,
    frontend_url: String,
}

impl ApplicationDigestService {
    pub fn new(
        application_queries: Arc<dyn ApplicationQueryPort>,
        attribute_repo: Arc<dyn AttributeRepositoryPort>,
        email_port: Option<Arc<dyn EmailPort>>,
        frontend_url: String,
    ) -> Self {
        Self {
            application_queries,
            attribute_repo,
            email_port,
            frontend_url,
        }
    }

    pub async fn send_pending_digest(&self) {
        let pending = match self
            .application_queries
            .fetch_with_user_filtered(Some(ApplicationStatus::Pending), None)
            .await
        {
            Ok(apps) => apps,
            Err(e) => {
                tracing::error!("Application digest: failed to fetch pending applications: {e:?}");
                return;
            }
        };
        if pending.is_empty() {
            tracing::debug!("Application digest: no pending applications, skipping");
            return;
        }

        let attr_name = match AttributeName::new(ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE) {
            Ok(name) => name,
            Err(e) => {
                tracing::error!(
                    "Application digest: well-known attribute name is invalid: {e:?}"
                );
                return;
            }
        };
        let recipients = match self.attribute_repo.fetch_all_values_for(&attr_name).await {
            Ok(values) => values,
            Err(e) => {
                tracing::error!("Application digest: failed to fetch recipients: {e:?}");
                return;
            }
        };
        // BTreeSet: dedup addresses shared between members, deterministic order.
        let addresses: BTreeSet<String> = recipients
            .into_iter()
            .map(|(_, value)| value.into_inner())
            .collect();
        if addresses.is_empty() {
            tracing::info!(
                "Application digest: {} pending application(s) but no members have the \
                 '{ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE}' attribute set",
                pending.len()
            );
            return;
        }

        let subject = format!(
            "{} pending membership application{}",
            pending.len(),
            if pending.len() == 1 { "" } else { "s" }
        );
        let body = build_digest_body(&pending, &self.frontend_url);

        let Some(port) = &self.email_port else {
            tracing::info!("[MOCK EMAIL] To: {addresses:?}, Subject: {subject}");
            return;
        };
        for to in addresses {
            if let Err(e) = port.send_email(&to, &subject, &body).await {
                tracing::error!("Application digest: failed to send to {to}: {e:?}");
            }
        }
    }
}

fn build_digest_body(applications: &[ApplicationWithMember], frontend_url: &str) -> String {
    let rows: String = applications
        .iter()
        .map(|app| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(app.full_name.as_deref().unwrap_or("-")),
                html_escape(app.email.as_deref().unwrap_or("-")),
                html_escape(&app.role_name),
                app.created_at.format("%Y-%m-%d"),
            )
        })
        .collect();
    format!(
        "<p>The following membership applications are awaiting a decision:</p>\
         <table border=\"1\" cellpadding=\"6\" cellspacing=\"0\">\
         <tr><th>Name</th><th>Email</th><th>Role</th><th>Submitted</th></tr>{rows}</table>\
         <p><a href=\"{frontend_url}/applications\">Open the applications view</a></p>"
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
