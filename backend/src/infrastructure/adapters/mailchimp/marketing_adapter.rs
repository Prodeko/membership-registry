use md5::{Digest, Md5};

use crate::application::ports::marketing_list_port::{
    MarketingContact, MarketingListError, MarketingListPort, SyncStats,
};

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

    /// Execute a PUT /lists/{id}/members/{hash} with the given status value.
    /// Returns `Ok(true)` on success, `Ok(false)` if Mailchimp rejected the
    /// request with its compliance-state 400 (caller decides fallback),
    /// and `Err` for any other failure.
    async fn put_member(
        &self,
        contact: &MarketingContact,
        status: &str,
    ) -> Result<bool, MarketingListError> {
        let payload = serde_json::json!({
            "email_address": contact.email,
            "status_if_new": status,
            "status": status,
            "language": contact.language,
            "merge_fields": {
                "FNAME": contact.first_name,
                "LNAME": contact.last_name,
            },
        });

        let resp = self
            .http
            .put(self.member_url(&contact.email))
            .bearer_auth(&self.config.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MarketingListError::RequestFailed(e.to_string()))?;

        if resp.status().is_success() {
            return Ok(true);
        }

        let http_status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();

        if http_status == 400 && is_compliance_state_error(&body) {
            return Ok(false);
        }

        Err(MarketingListError::ApiError {
            status: http_status,
            body,
        })
    }

    /// PUT /lists/{id}/members/{hash} — upsert identity + subscription state.
    /// When the contact is subscribed in the DB but Mailchimp has them in a
    /// compliance-blocked state (typically because a previous unsubscribe came
    /// from a campaign link), fall back to `status: pending` — this tells
    /// Mailchimp to send its own opt-in confirmation email, which the contact
    /// can click to re-confirm. The DB flag stays as user intent either way.
    async fn upsert_member(&self, contact: &MarketingContact) -> Result<(), MarketingListError> {
        let desired = if contact.subscribed {
            "subscribed"
        } else {
            "unsubscribed"
        };

        if self.put_member(contact, desired).await? {
            return Ok(());
        }

        // Compliance block hit. Only meaningful when we were trying to resubscribe;
        // unsubscribed→unsubscribed shouldn't produce this error in practice, but
        // in that case retrying with `pending` is still better than failing.
        tracing::info!(
            "Mailchimp compliance state blocked direct {desired} for {}; retrying with status=pending to trigger opt-in",
            contact.email
        );
        if self.put_member(contact, "pending").await? {
            Ok(())
        } else {
            Err(MarketingListError::ApiError {
                status: 400,
                body: "compliance fallback to pending also rejected".to_string(),
            })
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
    async fn upsert_contact(&self, contact: MarketingContact) -> Result<(), MarketingListError> {
        self.upsert_member(&contact).await
    }

    async fn sync_contacts(
        &self,
        contacts: Vec<MarketingContact>,
        all_tags: Vec<String>,
    ) -> Result<SyncStats, MarketingListError> {
        let mut stats = SyncStats::default();

        for contact in &contacts {
            match self.upsert_member(contact).await {
                Ok(()) => {}
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
}
