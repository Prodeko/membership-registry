use std::sync::Arc;

use uuid::Uuid;

use crate::application::ports::application_repository_port::{
    ApplicationQueryPort, ApplicationWithMember,
};
use crate::application::ports::attribute_repository_port::AttributeRepositoryPort;
use crate::application::ports::email_port::EmailPort;
use crate::application::ports::role_repository_port::RoleRepositoryPort;

use super::admin_notifications::{html_escape, resolve_recipients, send_to_each};

const LABEL: &str = "Application alert";

/// Emails admins once per membership application, at the moment it starts
/// awaiting an admin decision: on submission for roles without payment, or
/// when payment is received for paid roles. Recipients are members who hold
/// the `admin` role and have the `admin-notifications-email` attribute set;
/// the email goes to the attribute's value. Called by `ApplicationService` —
/// every failure is logged and swallowed so an alert can never fail the
/// application flow that triggered it.
#[derive(Clone)]
pub struct ApplicationAlertService {
    application_queries: Arc<dyn ApplicationQueryPort>,
    attribute_repo: Arc<dyn AttributeRepositoryPort>,
    role_repo: Arc<dyn RoleRepositoryPort>,
    email_port: Option<Arc<dyn EmailPort>>,
    frontend_url: String,
}

impl ApplicationAlertService {
    pub fn new(
        application_queries: Arc<dyn ApplicationQueryPort>,
        attribute_repo: Arc<dyn AttributeRepositoryPort>,
        role_repo: Arc<dyn RoleRepositoryPort>,
        email_port: Option<Arc<dyn EmailPort>>,
        frontend_url: String,
    ) -> Self {
        Self {
            application_queries,
            attribute_repo,
            role_repo,
            email_port,
            frontend_url,
        }
    }

    pub async fn notify_new_application(&self, application_id: Uuid) {
        // Recipients first: when nobody is subscribed there is no reason to
        // load the applicant.
        let Some(addresses) =
            resolve_recipients(&*self.attribute_repo, &*self.role_repo, LABEL).await
        else {
            return;
        };

        let application = match self
            .application_queries
            .fetch_with_member_one(application_id)
            .await
        {
            Ok(app) => app,
            Err(e) => {
                tracing::error!(
                    %application_id,
                    "Application alert: failed to fetch application: {e:?}"
                );
                return;
            }
        };

        let alert = build_alert(&application, &self.frontend_url);

        let total = addresses.len();
        let Some(sent) = send_to_each(
            self.email_port.as_deref(),
            addresses,
            &alert.subject,
            &alert.body,
            LABEL,
        )
        .await
        else {
            return;
        };
        if sent == 0 {
            tracing::error!(
                %application_id,
                recipients = total,
                "Application alert: delivery failed for every recipient"
            );
        } else {
            tracing::info!(
                %application_id,
                sent,
                failed = total - sent,
                "Application alert delivered"
            );
        }
    }
}

struct AlertEmail {
    subject: String,
    body: String,
}

fn build_alert(app: &ApplicationWithMember, frontend_url: &str) -> AlertEmail {
    let subject = format!("New membership application: {}", app.role_name);
    let mut rows = format!(
        "<tr><th align=\"left\">Name</th><td>{}</td></tr>\
         <tr><th align=\"left\">Email</th><td>{}</td></tr>\
         <tr><th align=\"left\">Role</th><td>{}</td></tr>\
         <tr><th align=\"left\">Valid until</th><td>{}</td></tr>",
        html_escape(app.full_name.as_deref().unwrap_or("-")),
        html_escape(app.email.as_deref().unwrap_or("-")),
        html_escape(&app.role_name),
        app.valid_until.format("%Y-%m-%d"),
    );
    if let Some(text) = app.application_text.as_deref().filter(|t| !t.is_empty()) {
        rows.push_str(&format!(
            "<tr><th align=\"left\">Application text</th><td>{}</td></tr>",
            html_escape(text)
        ));
    }
    let body = format!(
        "<p>A new membership application is awaiting a decision:</p>\
         <table border=\"1\" cellpadding=\"6\" cellspacing=\"0\">{rows}</table>\
         <p><a href=\"{frontend_url}/applications\">Open the applications view</a></p>"
    );
    AlertEmail { subject, body }
}
