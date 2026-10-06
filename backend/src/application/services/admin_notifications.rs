//! Recipient lookup and delivery shared by the admin emails about
//! membership applications (`ApplicationAlertService` and
//! `ApplicationDigestService`). `label` prefixes every log line so the two
//! stay distinguishable. Every failure is logged and swallowed.

use std::collections::{BTreeSet, HashSet};

use crate::application::ports::attribute_repository_port::AttributeRepositoryPort;
use crate::application::ports::email_port::EmailPort;
use crate::application::ports::role_repository_port::RoleRepositoryPort;
use crate::domain::attribute::AttributeValue;
use crate::domain::well_known::{
    admin_notifications_email_attribute, ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE, ADMIN_ROLE_NAME,
};

/// Resolves recipient addresses: members holding the notifications
/// attribute, restricted to those who also hold the admin role so the
/// attribute alone cannot subscribe anyone to applicant data. Returns
/// `None` when there is no one to send to.
pub(super) async fn resolve_recipients(
    attribute_repo: &dyn AttributeRepositoryPort,
    role_repo: &dyn RoleRepositoryPort,
    label: &str,
) -> Option<BTreeSet<String>> {
    let holders = match attribute_repo
        .fetch_all_values_for(&admin_notifications_email_attribute())
        .await
    {
        Ok(values) => values,
        Err(e) => {
            tracing::error!("{label}: failed to fetch recipients: {e:?}");
            return None;
        }
    };
    if holders.is_empty() {
        tracing::info!(
            "{label}: no members have the '{ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE}' attribute set"
        );
        return None;
    }

    let admins = match role_repo.fetch_members_by_role(ADMIN_ROLE_NAME).await {
        Ok(members) => members,
        Err(e) => {
            tracing::error!("{label}: failed to fetch admin members: {e:?}");
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
            "{label}: '{ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE}' is set on members without the \
             '{ADMIN_ROLE_NAME}' role; not sending to them"
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
            "{label}: no '{ADMIN_ROLE_NAME}' members have the \
             '{ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE}' attribute set"
        );
        return None;
    }
    Some(addresses)
}

/// Sends the email to every address, continuing past individual failures.
/// Returns how many were sent, or `None` when email is not configured.
pub(super) async fn send_to_each(
    email_port: Option<&dyn EmailPort>,
    addresses: BTreeSet<String>,
    subject: &str,
    body: &str,
    label: &str,
) -> Option<usize> {
    let Some(port) = email_port else {
        tracing::warn!(
            "{label}: email is not configured, not delivered. \
             [MOCK EMAIL] To: {addresses:?}, Subject: {subject}"
        );
        return None;
    };
    let mut sent = 0usize;
    for to in addresses {
        match port.send_email(&to, subject, body).await {
            Ok(()) => sent += 1,
            Err(e) => {
                tracing::error!("{label}: failed to send to {to}: {e:?}");
            }
        }
    }
    Some(sent)
}

pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
