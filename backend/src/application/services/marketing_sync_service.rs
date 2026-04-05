use std::sync::Arc;

use crate::application::ports::{
    marketing_list_port::{ContactPushMode, MarketingContact, MarketingListPort, UpsertOutcome},
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
        // `all_tags` is the full set of role names (flagged or not). We treat
        // every role name as a tag we own in Mailchimp's namespace — even
        // roles that *were* flagged but are now unflagged. This way, flipping
        // a role's `sync_to_mailchimp_tag` from true to false causes the next
        // sync to mark the tag `inactive` on every member, removing it from
        // Mailchimp. If `all_tags` were filtered, the disabled tag would just
        // go un-mentioned and linger on members forever.
        let all_tags: Vec<String> = all_roles.iter().map(|r| r.name.0.clone()).collect();
        // `taggable` is the currently-flagged subset. Only these count as
        // "active" for any member — disabled roles are always inactive.
        let taggable: std::collections::HashSet<String> = all_roles
            .into_iter()
            .filter(|r| r.sync_to_mailchimp_tag)
            .map(|r| r.name.0)
            .collect();

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

    /// Event-driven push of a single member, awaited. Always uses
    /// `WithSubscription` — this path is specifically for resubscribe flows
    /// where the caller wants to know if Mailchimp fell back to `pending`.
    /// `None` means no adapter is configured or the call failed.
    pub async fn push_contact_awaited(&self, person: Person) -> Option<UpsertOutcome> {
        self.dispatch(person, ContactPushMode::WithSubscription)
            .await
    }

    /// Fire-and-forget push that includes subscription status. Use on paths
    /// where `email_notifications` changed or the contact is brand new
    /// (create). The caller doesn't block on Mailchimp.
    pub fn push_contact_async(&self, person: Person) {
        self.spawn_push(person, ContactPushMode::WithSubscription);
    }

    /// Fire-and-forget push that updates identity fields only (name,
    /// language, merge fields) without touching Mailchimp's subscription
    /// status. Use on profile edits that don't change `email_notifications`,
    /// so a contact sitting in `pending` or `unsubscribed` doesn't get
    /// force-transitioned and re-triggered on every unrelated save.
    pub fn push_identity_async(&self, person: Person) {
        self.spawn_push(person, ContactPushMode::IdentityOnly);
    }

    fn spawn_push(&self, person: Person, mode: ContactPushMode) {
        if self.marketing.is_none() {
            return;
        }
        let svc = self.clone();
        tokio::spawn(async move {
            svc.dispatch(person, mode).await;
        });
    }

    async fn dispatch(&self, person: Person, mode: ContactPushMode) -> Option<UpsertOutcome> {
        let marketing = self.marketing.as_ref()?;
        let contact = Self::contact_from_person(person);
        let email = contact.email.clone();
        match marketing.upsert_contact(contact, mode).await {
            Ok(outcome) => Some(outcome),
            Err(e) => {
                tracing::warn!("Mailchimp event-driven upsert failed for {email}: {e:?}");
                None
            }
        }
    }

    fn contact_from_person(person: Person) -> MarketingContact {
        MarketingContact {
            email: person.email.as_str().to_string(),
            first_name: person.first_name,
            last_name: person.last_name,
            language: person.language,
            subscribed: person.email_notifications,
            active_tags: Vec::new(),
        }
    }
}
