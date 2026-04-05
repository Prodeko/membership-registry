use std::sync::Arc;

use crate::application::ports::{
    marketing_list_port::{MarketingContact, MarketingListPort},
    member_repository_port::{MemberRepositoryPort, MembersWithRolesParams},
    role_repository_port::RoleRepositoryPort,
};
use crate::domain::Person;

use super::errors::{ServiceError, ServiceResult};

/// Pushes member data into Mailchimp (one-way) and pulls unsubscribes back
/// to keep the DB flag in sync. Designed to be called once per day by the
/// scheduler. A no-op when no `MarketingListPort` is configured, so dev and
/// e2e runs don't need Mailchimp credentials.
#[derive(Clone)]
pub struct MarketingSyncService {
    member_repo: Arc<dyn MemberRepositoryPort>,
    role_repo: Arc<dyn RoleRepositoryPort>,
    marketing: Option<Arc<dyn MarketingListPort>>,
}

impl MarketingSyncService {
    pub fn new(
        member_repo: Arc<dyn MemberRepositoryPort>,
        role_repo: Arc<dyn RoleRepositoryPort>,
        marketing: Option<Arc<dyn MarketingListPort>>,
    ) -> Self {
        Self {
            member_repo,
            role_repo,
            marketing,
        }
    }

    pub async fn run_sync(&self) -> ServiceResult<()> {
        let Some(marketing) = &self.marketing else {
            tracing::debug!("Marketing list sync skipped: no adapter configured");
            return Ok(());
        };

        // Order matters: pull unsubscribes FIRST, then push. Mailchimp will
        // accept an API-initiated re-subscribe of a contact that was itself
        // unsubscribed via API/UI (the hard compliance block only applies to
        // campaign-link unsubscribes), so if we pushed first we'd overwrite
        // any fresh unsubscribes before we saw them. Pulling first means the
        // DB flag is flipped to `false` before we build the push payload, so
        // the push then sends `status: unsubscribed` which matches reality.
        match marketing.fetch_unsubscribed_emails().await {
            Ok(emails) => {
                let mut flipped = 0u64;
                for email in &emails {
                    match self
                        .member_repo
                        .set_email_notifications_by_email(email, false)
                        .await
                    {
                        Ok(n) => flipped += n,
                        Err(e) => {
                            tracing::warn!("Failed to flip email_notifications for {email}: {e:?}");
                        }
                    }
                }
                tracing::info!(
                    unsubscribed = emails.len(),
                    flipped,
                    "Marketing list unsubscribe pull complete"
                );
            }
            Err(e) => {
                tracing::error!("Marketing list unsubscribe pull failed: {e:?}");
                // Still try to push — worst case the push re-subscribes
                // someone we couldn't see, and we'll pick it up next run.
            }
        }

        let members = self
            .member_repo
            .fetch_members_with_roles(MembersWithRolesParams::default())
            .await
            .map_err(ServiceError::from)?;

        let all_roles = self
            .role_repo
            .fetch_all()
            .await
            .map_err(ServiceError::from)?;
        // Only roles explicitly marked for Mailchimp become tags. Roles with
        // `sync_to_mailchimp_tag = false` are invisible to Mailchimp entirely.
        let taggable: std::collections::HashSet<String> = all_roles
            .into_iter()
            .filter(|r| r.sync_to_mailchimp_tag)
            .map(|r| r.name.0)
            .collect();
        let all_tags: Vec<String> = taggable.iter().cloned().collect();

        let contacts: Vec<MarketingContact> = members
            .into_iter()
            .map(|m| MarketingContact {
                email: m.person.email.as_str().to_string(),
                first_name: m.person.first_name,
                last_name: m.person.last_name,
                language: m.person.language,
                subscribed: m.person.email_notifications,
                active_tags: m
                    .role_names
                    .into_iter()
                    .filter(|r| taggable.contains(r))
                    .collect(),
            })
            .collect();

        let contact_count = contacts.len();
        match marketing.sync_contacts(contacts, all_tags).await {
            Ok(stats) => tracing::info!(
                total = contact_count,
                upserted = stats.upserted,
                failed = stats.failed,
                "Marketing list push complete"
            ),
            Err(e) => {
                tracing::error!("Marketing list push failed: {e:?}");
            }
        }

        Ok(())
    }

    /// Fire-and-forget event-driven push of a single member. Called from
    /// `MemberService` whenever a member is created or updated so identity,
    /// language and subscription state land in Mailchimp within seconds
    /// instead of waiting for the daily scheduler. Tag state is not touched
    /// here — that remains a scheduler-only concern. No-op when the marketing
    /// port is unconfigured (dev/e2e).
    pub fn push_contact_async(&self, person: Person) {
        let Some(marketing) = self.marketing.clone() else {
            return;
        };

        let contact = MarketingContact {
            email: person.email.as_str().to_string(),
            first_name: person.first_name,
            last_name: person.last_name,
            language: person.language,
            subscribed: person.email_notifications,
            active_tags: Vec::new(),
        };
        let email = contact.email.clone();
        tokio::spawn(async move {
            if let Err(e) = marketing.upsert_contact(contact).await {
                tracing::warn!("Mailchimp event-driven upsert failed for {email}: {e:?}");
            }
        });
    }
}
