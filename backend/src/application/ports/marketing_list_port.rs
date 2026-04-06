#[derive(Debug)]
pub enum MarketingListError {
    RequestFailed(String),
    ApiError { status: u16, body: String },
}

/// Current Mailchimp subscription state of a contact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionState {
    Subscribed,
    Pending,
    Unsubscribed,
    NotAContact,
}

/// Whether a known marketing tag is currently active on a contact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagPreference {
    pub name: String,
    pub active: bool,
}

/// A contact's marketing subscription state plus the on/off status of each
/// tag in the known catalog. Tags outside the catalog are ignored; tags in
/// the catalog but missing from the contact are reported as inactive.
#[derive(Debug, Clone)]
pub struct MarketingPreferences {
    pub state: SubscriptionState,
    pub tags: Vec<TagPreference>,
}

/// Identity fields sent along with every write to Mailchimp so that the
/// contact's name and language stay fresh.
#[derive(Debug, Clone)]
pub struct ContactIdentity {
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub language: String,
}

#[async_trait::async_trait]
pub trait MarketingListPort: Send + Sync {
    /// Fetch a contact's current subscription state and tag assignments.
    /// Returns `NotAContact` if the email has never been added to the list.
    async fn fetch_preferences(
        &self,
        email: &str,
        known_tags: &[String],
    ) -> Result<MarketingPreferences, MarketingListError>;

    /// Subscribe a contact. Always PUTs `status: pending` — that sidesteps
    /// the Mailchimp compliance block entirely and triggers Mailchimp's
    /// opt-in email, which the user confirms from their inbox. Identity
    /// fields ride along so Mailchimp gets a fresh name/language.
    ///
    /// There is deliberately no unsubscribe counterpart: unsubscribing is
    /// done by the user through the email footer, not through the app.
    async fn subscribe(&self, identity: &ContactIdentity) -> Result<(), MarketingListError>;

    /// Update tag active/inactive states. Identity fields ride along in the
    /// implicit contact upsert so Mailchimp stays fresh without a separate
    /// path. The caller must ensure the contact exists on the list first.
    async fn set_tags(
        &self,
        identity: &ContactIdentity,
        tag_updates: &[TagPreference],
    ) -> Result<(), MarketingListError>;
}
