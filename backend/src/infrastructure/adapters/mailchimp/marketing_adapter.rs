use md5::{Digest, Md5};

use crate::application::ports::marketing_list_port::{
    ContactIdentity, ListContact, MarketingListError, MarketingListPort, MarketingPreferences,
    SubscriptionState, TagPreference,
};

use super::config::MailchimpConfig;

/// Contacts fetched per page when listing the audience (Mailchimp's max).
const LIST_PAGE_SIZE: usize = 1000;

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

    /// Upsert the contact with `status` for existing contacts; new contacts
    /// are always created as `subscribed`.
    async fn put_member(
        &self,
        identity: &ContactIdentity,
        status: &str,
    ) -> Result<(), MarketingListError> {
        let payload = serde_json::json!({
            "email_address": identity.email,
            "status": status,
            "status_if_new": "subscribed",
            "language": identity.language,
            "merge_fields": {
                "FNAME": identity.first_name,
                "LNAME": identity.last_name,
            },
        });

        let resp = self
            .http
            .put(self.member_url(&identity.email))
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

/// Map a Mailchimp member `status` to a `SubscriptionState`. Unknown or
/// missing statuses count as unsubscribed, so they are never re-added.
fn parse_state(body: &serde_json::Value) -> SubscriptionState {
    match body.get("status").and_then(|s| s.as_str()) {
        Some("subscribed") => SubscriptionState::Subscribed,
        Some("pending") => SubscriptionState::Pending,
        Some("archived") => SubscriptionState::Archived,
        _ => SubscriptionState::Unsubscribed,
    }
}

/// Parse one page of `GET /lists/{id}/members` into contacts.
fn parse_contacts_page(body: &serde_json::Value) -> Vec<ListContact> {
    body.get("members")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let email = m.get("email_address")?.as_str()?.to_string();
                    Some(ListContact {
                        email,
                        state: parse_state(m),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Parse the `{status, tags}` JSON payload returned by Mailchimp into a
/// `MarketingPreferences`. Extracted as a pure function so it's directly
/// unit-testable without HTTP mocking.
fn parse_preferences(body: &serde_json::Value, known_tags: &[String]) -> MarketingPreferences {
    let state = parse_state(body);

    let contact_tag_names: std::collections::HashSet<String> = body
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let tags = known_tags
        .iter()
        .map(|name| TagPreference {
            name: name.clone(),
            active: contact_tag_names.contains(name),
        })
        .collect();

    MarketingPreferences { state, tags }
}

fn not_a_contact_preferences(known_tags: &[String]) -> MarketingPreferences {
    MarketingPreferences {
        state: SubscriptionState::NotAContact,
        tags: known_tags
            .iter()
            .map(|name| TagPreference {
                name: name.clone(),
                active: false,
            })
            .collect(),
    }
}

#[async_trait::async_trait]
impl MarketingListPort for MailchimpMarketingAdapter {
    async fn list_contacts(&self) -> Result<Vec<ListContact>, MarketingListError> {
        let url = format!(
            "{}/lists/{}/members",
            self.config.base_url(),
            self.config.list_id
        );
        let mut contacts = Vec::new();
        loop {
            let resp = self
                .http
                .get(&url)
                .bearer_auth(&self.config.api_key)
                .query(&[
                    ("fields", "members.email_address,members.status,total_items"),
                    ("count", &LIST_PAGE_SIZE.to_string()),
                    ("offset", &contacts.len().to_string()),
                ])
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
            let page = parse_contacts_page(&body);
            let total = body
                .get("total_items")
                .and_then(|t| t.as_u64())
                .unwrap_or(0) as usize;
            let page_len = page.len();
            contacts.extend(page);
            if page_len == 0 || contacts.len() >= total {
                return Ok(contacts);
            }
        }
    }

    async fn fetch_preferences(
        &self,
        email: &str,
        known_tags: &[String],
    ) -> Result<MarketingPreferences, MarketingListError> {
        let url = format!("{}?fields=status,tags", self.member_url(email));

        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.config.api_key)
            .send()
            .await
            .map_err(|e| MarketingListError::RequestFailed(e.to_string()))?;

        if resp.status().as_u16() == 404 {
            return Ok(not_a_contact_preferences(known_tags));
        }

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(MarketingListError::ApiError { status, body });
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MarketingListError::RequestFailed(e.to_string()))?;

        Ok(parse_preferences(&body, known_tags))
    }

    async fn subscribe(&self, identity: &ContactIdentity) -> Result<(), MarketingListError> {
        // New contacts are created as `subscribed` directly — at signup time
        // the user has accepted our policies, which is the consent we need,
        // so there's no double-opt-in step.
        //
        // For *existing* contacts (e.g. the "resubscribe" button pressed by
        // someone who had previously unsubscribed) we still use `pending`:
        // Mailchimp's compliance block refuses to flip an unsubscribed
        // contact back to `subscribed` via API, so `pending` is the only
        // way through and it triggers Mailchimp's confirmation email.
        self.put_member(identity, "pending").await
    }

    async fn restore(&self, identity: &ContactIdentity) -> Result<(), MarketingListError> {
        // Archived contacts count as existing, so `status_if_new` would not
        // apply; set `subscribed` explicitly. Not blocked by compliance
        // because contacts we archive were never unsubscribed.
        self.put_member(identity, "subscribed").await
    }

    async fn archive(&self, email: &str) -> Result<(), MarketingListError> {
        // DELETE on a list member archives it (permanent deletion is a
        // separate `delete-permanent` action we never use).
        let resp = self
            .http
            .delete(self.member_url(email))
            .bearer_auth(&self.config.api_key)
            .send()
            .await
            .map_err(|e| MarketingListError::RequestFailed(e.to_string()))?;

        if resp.status().is_success() || resp.status().as_u16() == 404 {
            Ok(())
        } else {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            Err(MarketingListError::ApiError { status, body })
        }
    }

    async fn set_tags(
        &self,
        identity: &ContactIdentity,
        tag_updates: &[TagPreference],
    ) -> Result<(), MarketingListError> {
        if tag_updates.is_empty() {
            return Ok(());
        }

        let tag_objects: Vec<serde_json::Value> = tag_updates
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "status": if t.active { "active" } else { "inactive" },
                })
            })
            .collect();

        let payload = serde_json::json!({
            "tags": tag_objects,
            "is_syncing": false,
        });

        let resp = self
            .http
            .post(self.tags_url(&identity.email))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscriber_hash_is_md5_of_lowercased_email() {
        let lower = MailchimpMarketingAdapter::subscriber_hash("foo@bar.com");
        let upper = MailchimpMarketingAdapter::subscriber_hash("FOO@BAR.com");
        assert_eq!(lower, upper);
        assert_eq!(lower.len(), 32);
        assert!(lower.chars().all(|c| c.is_ascii_hexdigit()));
    }

    fn known_tags() -> Vec<String> {
        vec![
            "weekly_newsletter".to_string(),
            "event_advertisements".to_string(),
        ]
    }

    #[test]
    fn parse_preferences_subscribed_with_some_tags() {
        let body = serde_json::json!({
            "status": "subscribed",
            "tags": [{"id": 1, "name": "weekly_newsletter"}],
        });
        let prefs = parse_preferences(&body, &known_tags());
        assert_eq!(prefs.state, SubscriptionState::Subscribed);
        assert_eq!(prefs.tags.len(), 2);
        assert!(prefs
            .tags
            .iter()
            .any(|t| t.name == "weekly_newsletter" && t.active));
        assert!(prefs
            .tags
            .iter()
            .any(|t| t.name == "event_advertisements" && !t.active));
    }

    #[test]
    fn parse_preferences_pending() {
        let body = serde_json::json!({ "status": "pending", "tags": [] });
        let prefs = parse_preferences(&body, &known_tags());
        assert_eq!(prefs.state, SubscriptionState::Pending);
        assert!(prefs.tags.iter().all(|t| !t.active));
    }

    #[test]
    fn parse_preferences_unsubscribed() {
        let body = serde_json::json!({ "status": "unsubscribed", "tags": [] });
        let prefs = parse_preferences(&body, &known_tags());
        assert_eq!(prefs.state, SubscriptionState::Unsubscribed);
    }

    #[test]
    fn parse_preferences_archived() {
        let body = serde_json::json!({ "status": "archived", "tags": [] });
        let prefs = parse_preferences(&body, &known_tags());
        assert_eq!(prefs.state, SubscriptionState::Archived);
    }

    #[test]
    fn parse_preferences_ignores_unknown_tags() {
        let body = serde_json::json!({
            "status": "subscribed",
            "tags": [
                {"id": 1, "name": "legacy_role_tag"},
                {"id": 2, "name": "weekly_newsletter"},
            ],
        });
        let prefs = parse_preferences(&body, &known_tags());
        assert_eq!(prefs.tags.len(), 2);
        assert!(prefs.tags.iter().all(|t| t.name != "legacy_role_tag"));
    }

    #[test]
    fn parse_contacts_page_reads_email_and_status() {
        let body = serde_json::json!({
            "members": [
                {"email_address": "a@x.fi", "status": "subscribed"},
                {"email_address": "b@x.fi", "status": "archived"},
                {"email_address": "c@x.fi", "status": "cleaned"},
                {"status": "pending"},
            ],
            "total_items": 4,
        });
        assert_eq!(
            parse_contacts_page(&body),
            vec![
                ListContact {
                    email: "a@x.fi".to_string(),
                    state: SubscriptionState::Subscribed,
                },
                ListContact {
                    email: "b@x.fi".to_string(),
                    state: SubscriptionState::Archived,
                },
                ListContact {
                    email: "c@x.fi".to_string(),
                    state: SubscriptionState::Unsubscribed,
                },
            ]
        );
    }

    #[test]
    fn not_a_contact_has_all_tags_inactive() {
        let prefs = not_a_contact_preferences(&known_tags());
        assert_eq!(prefs.state, SubscriptionState::NotAContact);
        assert_eq!(prefs.tags.len(), 2);
        assert!(prefs.tags.iter().all(|t| !t.active));
    }
}
