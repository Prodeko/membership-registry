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
    /// Role names the member currently holds. These become Mailchimp tags.
    pub active_tags: Vec<String>,
}

#[async_trait::async_trait]
pub trait MarketingListPort: Send + Sync {
    /// Upsert every contact and reconcile its tag state. The adapter uses
    /// `all_tags` to mark tags the member no longer has as inactive, so a
    /// member whose role was removed loses the corresponding Mailchimp tag.
    async fn sync_contacts(
        &self,
        contacts: Vec<MarketingContact>,
        all_tags: Vec<String>,
    ) -> Result<SyncStats, MarketingListError>;

    /// Emails currently in `status=unsubscribed` on the remote list.
    async fn fetch_unsubscribed_emails(&self) -> Result<Vec<String>, MarketingListError>;
}

#[derive(Debug, Default)]
pub struct SyncStats {
    pub upserted: usize,
    pub failed: usize,
}
