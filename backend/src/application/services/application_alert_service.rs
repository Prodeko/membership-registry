use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use uuid::Uuid;

use crate::application::ports::application_repository_port::{
    ApplicationQueryPort, ApplicationWithMember,
};
use crate::application::ports::attribute_repository_port::AttributeRepositoryPort;
use crate::application::ports::email_port::EmailPort;
use crate::application::ports::role_repository_port::RoleRepositoryPort;
use crate::domain::attribute::AttributeValue;
use crate::domain::well_known::{
    admin_notifications_email_attribute, ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE, ADMIN_ROLE_NAME,
};

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
        let Some(addresses) = self.resolve_recipients().await else {
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

        let Some(port) = &self.email_port else {
            tracing::warn!(
                "Application alert: email is not configured, alert not delivered. \
                 [MOCK EMAIL] To: {addresses:?}, Subject: {}",
                alert.subject
            );
            return;
        };
        let total = addresses.len();
        let mut sent = 0usize;
        for to in addresses {
            match port.send_email(&to, &alert.subject, &alert.body).await {
                Ok(()) => sent += 1,
                Err(e) => {
                    tracing::error!("Application alert: failed to send to {to}: {e:?}");
                }
            }
        }
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

    /// Resolves alert recipient addresses: members holding the
    /// notifications attribute, restricted to those who also hold the admin
    /// role so the attribute alone cannot subscribe anyone to applicant
    /// data. Returns `None` when there is no one to send to.
    async fn resolve_recipients(&self) -> Option<BTreeSet<String>> {
        let holders = match self
            .attribute_repo
            .fetch_all_values_for(&admin_notifications_email_attribute())
            .await
        {
            Ok(values) => values,
            Err(e) => {
                tracing::error!("Application alert: failed to fetch recipients: {e:?}");
                return None;
            }
        };
        if holders.is_empty() {
            tracing::info!(
                "Application alert: no members have the '{ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE}' \
                 attribute set"
            );
            return None;
        }

        let admins = match self.role_repo.fetch_members_by_role(ADMIN_ROLE_NAME).await {
            Ok(members) => members,
            Err(e) => {
                tracing::error!("Application alert: failed to fetch admin members: {e:?}");
                return None;
            }
        };
        let admin_ids: HashSet<uuid::Uuid> = admins.into_iter().map(|p| p.id.0).collect();

        let skipped = holders
            .iter()
            .filter(|(id, _)| !admin_ids.contains(&id.0))
            .count();
        if skipped > 0 {
            tracing::warn!(
                skipped,
                "Application alert: '{ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE}' is set on members \
                 without the '{ADMIN_ROLE_NAME}' role; not sending to them"
            );
        }

        // BTreeSet: dedup addresses shared between members, deterministic order.
        let addresses: BTreeSet<String> = holders
            .into_iter()
            .filter(|(id, _)| admin_ids.contains(&id.0))
            .flat_map(|(_, values)| values.into_iter().map(AttributeValue::into_inner))
            .collect();
        if addresses.is_empty() {
            tracing::info!(
                "Application alert: no '{ADMIN_ROLE_NAME}' members have the \
                 '{ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE}' attribute set"
            );
            return None;
        }
        Some(addresses)
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

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
