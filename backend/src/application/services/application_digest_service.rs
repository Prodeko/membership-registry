use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use crate::application::ports::application_repository_port::{
    ApplicationQueryPort, ApplicationWithMember,
};
use crate::application::ports::attribute_repository_port::AttributeRepositoryPort;
use crate::application::ports::email_port::EmailPort;
use crate::application::ports::role_repository_port::RoleRepositoryPort;
use crate::domain::well_known::{
    admin_notifications_email_attribute, ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE, ADMIN_ROLE_NAME,
};
use crate::domain::ApplicationStatus;

/// Sends admins a daily digest email listing membership applications that
/// await admin action: pending ones awaiting a decision and unpaid ones the
/// admin may reject to clear out. Recipients are members who hold the
/// `admin` role and have the `admin-notifications-email` attribute set; the
/// digest goes to the attribute's value. Scheduled by
/// `scheduler::start_application_digest_job` — every failure is logged and
/// swallowed so the job never aborts.
#[derive(Clone)]
pub struct ApplicationDigestService {
    application_queries: Arc<dyn ApplicationQueryPort>,
    attribute_repo: Arc<dyn AttributeRepositoryPort>,
    role_repo: Arc<dyn RoleRepositoryPort>,
    email_port: Option<Arc<dyn EmailPort>>,
    frontend_url: String,
}

impl ApplicationDigestService {
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

    pub async fn send_pending_digest(&self) {
        let Some(mut applications) = self.fetch_by_status(ApplicationStatus::Pending).await else {
            return;
        };
        let Some(unpaid) = self.fetch_by_status(ApplicationStatus::Unpaid).await else {
            return;
        };
        applications.extend(unpaid);
        if applications.is_empty() {
            tracing::debug!("Application digest: no applications awaiting action, skipping");
            return;
        }

        let Some(addresses) = self.resolve_recipients(applications.len()).await else {
            return;
        };

        let digest = build_digest(&applications, &self.frontend_url);

        let Some(port) = &self.email_port else {
            tracing::warn!(
                "Application digest: email is not configured, digest not delivered. \
                 [MOCK EMAIL] To: {addresses:?}, Subject: {}",
                digest.subject
            );
            return;
        };
        let total = addresses.len();
        let mut sent = 0usize;
        for to in addresses {
            match port.send_email(&to, &digest.subject, &digest.body).await {
                Ok(()) => sent += 1,
                Err(e) => {
                    tracing::error!("Application digest: failed to send to {to}: {e:?}");
                }
            }
        }
        if sent == 0 {
            tracing::error!(
                recipients = total,
                "Application digest: delivery failed for every recipient"
            );
        } else {
            tracing::info!(sent, failed = total - sent, "Application digest delivered");
        }
    }

    async fn fetch_by_status(
        &self,
        status: ApplicationStatus,
    ) -> Option<Vec<ApplicationWithMember>> {
        match self
            .application_queries
            .fetch_with_user_filtered(Some(status), None)
            .await
        {
            Ok(apps) => Some(apps),
            Err(e) => {
                tracing::error!(
                    "Application digest: failed to fetch {status:?} applications: {e:?}"
                );
                None
            }
        }
    }

    /// Resolves digest recipient addresses: members holding the
    /// notifications attribute, restricted to those who also hold the admin
    /// role so the attribute alone cannot subscribe anyone to applicant
    /// data. Returns `None` when there is no one to send to.
    async fn resolve_recipients(&self, application_count: usize) -> Option<BTreeSet<String>> {
        let holders = match self
            .attribute_repo
            .fetch_all_values_for(&admin_notifications_email_attribute())
            .await
        {
            Ok(values) => values,
            Err(e) => {
                tracing::error!("Application digest: failed to fetch recipients: {e:?}");
                return None;
            }
        };
        if holders.is_empty() {
            tracing::info!(
                "Application digest: {application_count} application(s) awaiting action but no \
                 members have the '{ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE}' attribute set"
            );
            return None;
        }

        let admins = match self.role_repo.fetch_members_by_role(ADMIN_ROLE_NAME).await {
            Ok(members) => members,
            Err(e) => {
                tracing::error!("Application digest: failed to fetch admin members: {e:?}");
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
                "Application digest: '{ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE}' is set on members \
                 without the '{ADMIN_ROLE_NAME}' role; not sending to them"
            );
        }

        // BTreeSet: dedup addresses shared between members, deterministic order.
        let addresses: BTreeSet<String> = holders
            .into_iter()
            .filter(|(id, _)| admin_ids.contains(&id.0))
            .map(|(_, value)| value.into_inner())
            .collect();
        if addresses.is_empty() {
            tracing::info!(
                "Application digest: {application_count} application(s) awaiting action but no \
                 '{ADMIN_ROLE_NAME}' members have the '{ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE}' \
                 attribute set"
            );
            return None;
        }
        Some(addresses)
    }
}

struct DigestEmail {
    subject: String,
    body: String,
}

fn build_digest(applications: &[ApplicationWithMember], frontend_url: &str) -> DigestEmail {
    let subject = format!(
        "{} membership application{} awaiting action",
        applications.len(),
        if applications.len() == 1 { "" } else { "s" }
    );
    let rows: String = applications
        .iter()
        .map(|app| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                html_escape(app.full_name.as_deref().unwrap_or("-")),
                html_escape(app.email.as_deref().unwrap_or("-")),
                html_escape(&app.role_name),
                status_label(&app.status),
                // Submission date in the same timezone the digest fires in.
                app.created_at
                    .with_timezone(&chrono_tz::Europe::Helsinki)
                    .format("%Y-%m-%d"),
            )
        })
        .collect();
    let body = format!(
        "<p>The following membership applications are awaiting action:</p>\
         <table border=\"1\" cellpadding=\"6\" cellspacing=\"0\">\
         <tr><th>Name</th><th>Email</th><th>Role</th><th>Status</th><th>Submitted</th></tr>{rows}</table>\
         <p><a href=\"{frontend_url}/applications\">Open the applications view</a></p>"
    );
    DigestEmail { subject, body }
}

fn status_label(status: &ApplicationStatus) -> &'static str {
    match status {
        ApplicationStatus::Unpaid => "Unpaid",
        ApplicationStatus::Pending => "Pending",
        ApplicationStatus::Approved => "Approved",
        ApplicationStatus::Rejected => "Rejected",
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
