use md5::{Digest, Md5};

use crate::application::ports::marketing_list_port::{
    ContactPushMode, MarketingContact, MarketingListError, MarketingListPort, SyncStats,
    UpsertOutcome,
};

/// Internal outcome of a single `PUT /lists/{id}/members/{hash}` call.
/// `ComplianceBlocked` is a 400 with a body matching Mailchimp's
/// "Member In Compliance State" error; only meaningful when the caller
/// was trying to set `status: subscribed`.
enum PutOutcome {
    Accepted,
    ComplianceBlocked,
}

use super::config::MailchimpConfig;

pub struct MailchimpMarketingAdapter {
    config: MailchimpConfig,
    http: reqwest::Client,
}

impl MailchimpMarketingAdapter {
    pub fn new(config: MailchimpConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Mailchimp identifies members by MD5(lowercased email).
    fn subscriber_hash(email: &str) -> String {
        let mut hasher = Md5::new();
        hasher.update(email.to_lowercase().as_bytes());
        let digest = hasher.finalize();
        let mut out = String::with_capacity(32);
        for byte in digest {
            out.push_str(&format!("{byte:02x}"));
        }
        out
    }

    fn member_url(&self, email: &str) -> String {
        format!(
            "{}/lists/{}/members/{}",
            self.config.base_url(),
            self.config.list_id,
            Self::subscriber_hash(email)
        )
    }

    fn tags_url(&self, email: &str) -> String {
        format!("{}/tags", self.member_url(email))
    }

    /// Build the PUT /lists/{id}/members/{hash} payload. When `override_status`
    /// is `Some`, it's placed in the `status` field, requesting a transition.
    /// When `None`, the `status` field is omitted entirely so Mailchimp
    /// leaves the contact's current state untouched. `status_if_new` always
    /// matches the DB's intent (`contact.subscribed`) so brand-new contacts
    /// get created in the right initial state regardless of mode.
    fn build_put_payload(
        contact: &MarketingContact,
        override_status: Option<&str>,
    ) -> serde_json::Value {
        let initial = if contact.subscribed {
            "subscribed"
        } else {
            "unsubscribed"
        };
        let mut payload = serde_json::json!({
            "email_address": contact.email,
            "status_if_new": initial,
            "language": contact.language,
            "merge_fields": {
                "FNAME": contact.first_name,
                "LNAME": contact.last_name,
            },
        });
        if let Some(s) = override_status {
            payload["status"] = serde_json::Value::String(s.to_string());
        }
        payload
    }

    /// Execute a PUT /lists/{id}/members/{hash}. Returns `Accepted` on 2xx,
    /// `ComplianceBlocked` if Mailchimp rejected the status transition with
    /// its compliance-state 400 (only possible when `override_status` was
    /// `Some`), and `Err` for any other failure.
    async fn put_member(
        &self,
        contact: &MarketingContact,
        override_status: Option<&str>,
    ) -> Result<PutOutcome, MarketingListError> {
        let payload = Self::build_put_payload(contact, override_status);

        let resp = self
            .http
            .put(self.member_url(&contact.email))
            .bearer_auth(&self.config.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MarketingListError::RequestFailed(e.to_string()))?;

        if resp.status().is_success() {
            return Ok(PutOutcome::Accepted);
        }

        let http_status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();

        // Compliance state only triggers when we were requesting a status
        // change. Identity-only calls shouldn't produce it.
        if http_status == 400 && override_status.is_some() && is_compliance_state_error(&body) {
            return Ok(PutOutcome::ComplianceBlocked);
        }

        Err(MarketingListError::ApiError {
            status: http_status,
            body,
        })
    }

    /// PUT /lists/{id}/members/{hash} — upsert identity and (optionally)
    /// subscription state. When `mode` is `WithSubscription` and Mailchimp
    /// blocks the transition for compliance, fall back to `status: pending`
    /// which tells Mailchimp to send its own opt-in confirmation email.
    /// The DB flag stays as user intent either way.
    async fn upsert_member(
        &self,
        contact: &MarketingContact,
        mode: ContactPushMode,
    ) -> Result<UpsertOutcome, MarketingListError> {
        let override_status = match mode {
            ContactPushMode::IdentityOnly => None,
            ContactPushMode::WithSubscription => Some(if contact.subscribed {
                "subscribed"
            } else {
                "unsubscribed"
            }),
        };

        match self.put_member(contact, override_status).await? {
            PutOutcome::Accepted => return Ok(UpsertOutcome::Accepted),
            PutOutcome::ComplianceBlocked => {}
        }

        // Reaching here implies `override_status` was Some — identity-only
        // calls don't produce ComplianceBlocked. Retry with `pending`.
        tracing::info!(
            "Mailchimp compliance state blocked direct subscribe for {}; retrying with status=pending to trigger opt-in",
            contact.email
        );
        match self.put_member(contact, Some("pending")).await? {
            PutOutcome::Accepted => Ok(UpsertOutcome::PendingConfirmation),
            PutOutcome::ComplianceBlocked => Err(MarketingListError::ApiError {
                status: 400,
                body: "compliance fallback to pending also rejected".to_string(),
            }),
        }
    }

    /// POST /lists/{id}/members/{hash}/tags — set each known tag explicitly
    /// to `active` or `inactive` so removed roles are cleared, not left dangling.
    /// `is_syncing: true` suppresses activity-feed noise.
    async fn reconcile_tags(
        &self,
        contact: &MarketingContact,
        all_tags: &[String],
    ) -> Result<(), MarketingListError> {
        if all_tags.is_empty() {
            return Ok(());
        }

        let tag_objects: Vec<serde_json::Value> = all_tags
            .iter()
            .map(|tag| {
                let active = contact.active_tags.iter().any(|t| t == tag);
                serde_json::json!({
                    "name": tag,
                    "status": if active { "active" } else { "inactive" },
                })
            })
            .collect();

        let payload = serde_json::json!({
            "tags": tag_objects,
            "is_syncing": true,
        });

        let resp = self
            .http
            .post(self.tags_url(&contact.email))
            .bearer_auth(&self.config.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MarketingListError::RequestFailed(e.to_string()))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            Err(MarketingListError::ApiError { status, body })
        }
    }
}

/// Detect Mailchimp's "Member In Compliance State" error. Mailchimp returns a
/// 400 whose JSON body contains a `title` field identifying the specific error
/// class. We match loosely on substring to avoid being fragile to wording tweaks.
fn is_compliance_state_error(body: &str) -> bool {
    let lower = body.to_lowercase();
    lower.contains("compliance state") || lower.contains("in compliance")
}

#[async_trait::async_trait]
impl MarketingListPort for MailchimpMarketingAdapter {
    async fn upsert_contact(
        &self,
        contact: MarketingContact,
        mode: ContactPushMode,
    ) -> Result<UpsertOutcome, MarketingListError> {
        self.upsert_member(&contact, mode).await
    }

    async fn sync_contacts(
        &self,
        contacts: Vec<MarketingContact>,
        all_tags: Vec<String>,
    ) -> Result<SyncStats, MarketingListError> {
        let mut stats = SyncStats::default();

        for contact in &contacts {
            // The bulk path pushes subscription as the source of truth for
            // daily reconciliation, but deliberately SKIPS the
            // pending-fallback that the event-driven path uses. If Mailchimp
            // compliance-blocks a direct subscribe here, we leave the
            // contact in whatever state they're in. Otherwise the scheduler
            // would re-PUT `status: pending` every day for any contact
            // stuck mid-confirmation, which Mailchimp can interpret as a
            // reason to re-send the opt-in email — turning the daily sync
            // into a spam loop. The event-driven resubscribe path (UI
            // click) is the only place that's allowed to trigger a fresh
            // opt-in email.
            let override_status = Some(if contact.subscribed {
                "subscribed"
            } else {
                "unsubscribed"
            });
            match self.put_member(contact, override_status).await {
                Ok(PutOutcome::Accepted) => {}
                Ok(PutOutcome::ComplianceBlocked) => {
                    tracing::debug!(
                        "Mailchimp compliance-blocked scheduled subscribe for {}; leaving contact state untouched",
                        contact.email
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Mailchimp upsert failed for {}: {:?}; skipping tag sync for this contact",
                        contact.email,
                        e
                    );
                    stats.failed += 1;
                    continue;
                }
            }

            if let Err(e) = self.reconcile_tags(contact, &all_tags).await {
                tracing::warn!(
                    "Mailchimp tag reconciliation failed for {}: {:?}",
                    contact.email,
                    e
                );
                // Tag failure alone shouldn't mark the whole contact failed —
                // identity/subscription state already made it through.
            }

            stats.upserted += 1;
        }

        Ok(stats)
    }

    async fn fetch_unsubscribed_emails(&self) -> Result<Vec<String>, MarketingListError> {
        let mut result = Vec::new();
        let mut offset: usize = 0;
        let count: usize = 1000;

        loop {
            let url = format!(
                "{}/lists/{}/members?status=unsubscribed&fields=members.email_address&count={}&offset={}",
                self.config.base_url(),
                self.config.list_id,
                count,
                offset,
            );

            let resp = self
                .http
                .get(&url)
                .bearer_auth(&self.config.api_key)
                .send()
                .await
                .map_err(|e| MarketingListError::RequestFailed(e.to_string()))?;

            if !resp.status().is_success() {
                let status = resp.status().as_u16();
                let body = resp.text().await.unwrap_or_default();
                return Err(MarketingListError::ApiError { status, body });
            }

            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| MarketingListError::RequestFailed(e.to_string()))?;

            let members = body
                .get("members")
                .and_then(|m| m.as_array())
                .cloned()
                .unwrap_or_default();

            let page_len = members.len();
            for m in members {
                if let Some(email) = m.get("email_address").and_then(|e| e.as_str()) {
                    result.push(email.to_string());
                }
            }

            if page_len < count {
                break;
            }
            offset += count;
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_contact(subscribed: bool) -> MarketingContact {
        MarketingContact {
            email: "foo@example.com".to_string(),
            first_name: "Foo".to_string(),
            last_name: "Bar".to_string(),
            language: "fi".to_string(),
            subscribed,
            active_tags: vec![],
        }
    }

    #[test]
    fn subscriber_hash_is_md5_of_lowercased_email() {
        // Reference value from the Mailchimp docs example: "urist.mcvankab@freddiesjokes.co"
        // md5("urist.mcvankab@freddiesjokes.co") = "62eeb292278cc15f5817cb78f7790b08"
        // We verify the lowercasing behavior with a mixed-case input.
        let lower = MailchimpMarketingAdapter::subscriber_hash("foo@bar.com");
        let upper = MailchimpMarketingAdapter::subscriber_hash("FOO@BAR.com");
        assert_eq!(lower, upper);
        assert_eq!(lower.len(), 32);
        assert!(lower.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn payload_with_override_status_includes_status_field() {
        let contact = test_contact(true);
        let payload = MailchimpMarketingAdapter::build_put_payload(&contact, Some("subscribed"));
        assert_eq!(payload["status"], "subscribed");
        assert_eq!(payload["status_if_new"], "subscribed");
        assert_eq!(payload["email_address"], "foo@example.com");
        assert_eq!(payload["language"], "fi");
        assert_eq!(payload["merge_fields"]["FNAME"], "Foo");
        assert_eq!(payload["merge_fields"]["LNAME"], "Bar");
    }

    #[test]
    fn payload_identity_only_omits_status_field() {
        // The critical assertion for the identity-only path: Mailchimp must
        // not see a `status` field at all, so an existing contact in
        // `pending` or `unsubscribed` state keeps that state rather than
        // being force-transitioned (and re-triggering opt-in email flows).
        let contact = test_contact(true);
        let payload = MailchimpMarketingAdapter::build_put_payload(&contact, None);
        assert!(
            payload.get("status").is_none(),
            "identity-only payload must omit `status`, got: {payload}",
        );
        // status_if_new still present — needed by Mailchimp only when creating
        // a new contact, ignored when the contact already exists.
        assert_eq!(payload["status_if_new"], "subscribed");
        assert_eq!(payload["merge_fields"]["FNAME"], "Foo");
    }

    #[test]
    fn payload_status_if_new_reflects_db_intent_for_unsubscribed_contact() {
        // If the contact doesn't yet exist in Mailchimp and the DB says
        // they're unsubscribed, `status_if_new` must carry that intent —
        // otherwise a first-time sync would create them as subscribed.
        let contact = test_contact(false);
        let payload = MailchimpMarketingAdapter::build_put_payload(&contact, None);
        assert_eq!(payload["status_if_new"], "unsubscribed");
        assert!(payload.get("status").is_none());
    }
}
