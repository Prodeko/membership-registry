#[derive(Debug)]
pub enum MarketingListError {
    RequestFailed(String),
    ApiError { status: u16, body: String },
}

/// Result of a single-contact upsert. The adapter returns `PendingConfirmation`
/// when it could not directly set `status: subscribed` because Mailchimp had
/// the contact in a compliance-blocked state and the adapter fell back to
/// `status: pending`. Callers may surface this to the user so they know to
/// confirm via the Mailchimp opt-in email.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertOutcome {
    Accepted,
    PendingConfirmation,
}

/// Selects whether an upsert should touch the Mailchimp subscription status
/// field. `IdentityOnly` is used by routine profile edits (name, language)
/// so that contacts in pending/unsubscribed state don't have their status
/// repeatedly overwritten — which would spam opt-in emails at contacts
/// who are already mid-confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactPushMode {
    /// Include `status` in the PUT payload. The adapter may fall back to
    /// `status: pending` if Mailchimp blocks a direct subscribe for
    /// compliance reasons.
    WithSubscription,
    /// Omit `status` from the PUT payload entirely. Existing Mailchimp
    /// subscription state is preserved. For new contacts, `status_if_new`
    /// still provides the initial state from `MarketingContact.subscribed`.
    IdentityOnly,
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

    /// Event-driven single-contact upsert. Does NOT reconcile tags (tag
    /// state is a scheduler-only concern to keep this path fast and free
    /// of role lookups).
    ///
    /// When `mode` is `WithSubscription`, the adapter also sets the
    /// Mailchimp subscription status from `contact.subscribed`. If that is
    /// blocked by Mailchimp's compliance state on re-subscribe, the adapter
    /// falls back to `status: pending` and returns `PendingConfirmation`.
    ///
    /// When `mode` is `IdentityOnly`, the adapter omits the `status` field
    /// entirely so that existing Mailchimp state (subscribed, unsubscribed,
    /// pending, cleaned) is preserved. Always returns `Accepted` on success;
    /// no compliance block is possible because no state transition is
    /// requested. Use this for profile edits that don't touch
    /// `email_notifications`.
    async fn upsert_contact(
        &self,
        contact: MarketingContact,
        mode: ContactPushMode,
    ) -> Result<UpsertOutcome, MarketingListError>;

    /// Emails currently in `status=unsubscribed` on the remote list.
    async fn fetch_unsubscribed_emails(&self) -> Result<Vec<String>, MarketingListError>;
}

#[derive(Debug, Default)]
pub struct SyncStats {
    pub upserted: usize,
    pub failed: usize,
}
