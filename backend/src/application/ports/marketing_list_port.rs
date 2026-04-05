#[derive(Debug)]
pub enum MarketingListError {
    RequestFailed(String),
    ApiError { status: u16, body: String },
}

/// Flattened view of a member for marketing-list sync.
/// Lives in the port layer so the adapter never touches domain types.
#[derive(Debug, Clone)]
pub struct MarketingContact {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub language: String,
    /// Derived from `Person.email_notifications`.
    pub subscribed: bool,
    /// Role names the member currently holds, already filtered to those
    /// whose `sync_to_mailchimp_tag` flag is true. These become Mailchimp
    /// tags. Only used by `sync_contacts`; ignored by `upsert_contact`.
    pub active_tags: Vec<String>,
}

#[async_trait::async_trait]
pub trait MarketingListPort: Send + Sync {
    /// Bulk reconcile. Upserts every contact and reconciles its tag state.
    /// The adapter uses `all_tags` to mark tags the member no longer has as
    /// inactive, so a member whose role was removed loses the Mailchimp tag.
    /// Called by the daily scheduler.
    async fn sync_contacts(
        &self,
        contacts: Vec<MarketingContact>,
        all_tags: Vec<String>,
    ) -> Result<SyncStats, MarketingListError>;

    /// Event-driven single-contact upsert. Updates identity + subscription
    /// state only; does NOT reconcile tags (tag state is a scheduler-only
    /// concern to keep this path fast and free of role lookups). If the
    /// contact is blocked by Mailchimp's compliance state on re-subscribe,
    /// the adapter falls back to `status: pending` to trigger Mailchimp's
    /// own opt-in confirmation flow.
    async fn upsert_contact(&self, contact: MarketingContact) -> Result<(), MarketingListError>;

    /// Emails currently in `status=unsubscribed` on the remote list.
    async fn fetch_unsubscribed_emails(&self) -> Result<Vec<String>, MarketingListError>;
}

#[derive(Debug, Default)]
pub struct SyncStats {
    pub upserted: usize,
    pub failed: usize,
}
