use std::sync::Arc;

use crate::application::ports::application_repository_port::{
    ApplicationQueryPort, ApplicationWithMember,
};
use crate::application::ports::attribute_repository_port::AttributeRepositoryPort;
use crate::application::ports::email_port::EmailPort;
use crate::application::ports::role_repository_port::RoleRepositoryPort;
use crate::domain::ApplicationStatus;

use super::admin_notifications::{html_escape, resolve_recipients, send_to_each};

const LABEL: &str = "Application digest";

/// Sends admins a daily digest email listing membership applications that
/// have awaited admin action for at least `stale_after_days`: pending ones
/// awaiting a decision and unpaid ones the admin may reject to clear out.
/// Fresh applications are left out because `ApplicationAlertService` already
/// emailed admins about each one as it arrived; the digest is the reminder
/// for the ones nobody acted on. Recipients are members who hold the
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
    stale_after_days: u32,
}

impl ApplicationDigestService {
    pub fn new(
        application_queries: Arc<dyn ApplicationQueryPort>,
        attribute_repo: Arc<dyn AttributeRepositoryPort>,
        role_repo: Arc<dyn RoleRepositoryPort>,
        email_port: Option<Arc<dyn EmailPort>>,
        frontend_url: String,
        stale_after_days: u32,
    ) -> Self {
        Self {
            application_queries,
            attribute_repo,
            role_repo,
            email_port,
            frontend_url,
            stale_after_days,
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
        let cutoff = chrono::Utc::now() - chrono::Duration::days(i64::from(self.stale_after_days));
        applications.retain(|app| app.created_at <= cutoff);
        if applications.is_empty() {
            tracing::debug!(
                stale_after_days = self.stale_after_days,
                "Application digest: no stale applications awaiting action, skipping"
            );
            return;
        }

        let Some(addresses) =
            resolve_recipients(&*self.attribute_repo, &*self.role_repo, LABEL).await
        else {
            return;
        };

        let digest = build_digest(&applications, &self.frontend_url);

        let total = addresses.len();
        let Some(sent) = send_to_each(
            self.email_port.as_deref(),
            addresses,
            &digest.subject,
            &digest.body,
            LABEL,
        )
        .await
        else {
            return;
        };
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
}

struct DigestEmail {
    subject: String,
    body: String,
}

fn build_digest(applications: &[ApplicationWithMember], frontend_url: &str) -> DigestEmail {
    let subject = format!(
        "{} membership application{} still awaiting action",
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
        "<p>The following membership applications have been awaiting action for a while:</p>\
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
