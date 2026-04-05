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

/// Subscription transition the caller wants to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionAction {
    Subscribe,
    Unsubscribe,
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

    /// Subscribe or unsubscribe. Subscribe always PUTs `status: pending`
    /// (sidesteps compliance-block entirely) and triggers Mailchimp's opt-in
    /// email immediately. Unsubscribe PUTs `status: unsubscribed`. Both
    /// carry current identity fields so Mailchimp gets a fresh name/language.
    async fn set_subscription(
        &self,
        identity: &ContactIdentity,
        action: SubscriptionAction,
    ) -> Result<(), MarketingListError>;

    /// Update tag active/inactive states. Identity fields ride along in the
    /// implicit contact upsert so Mailchimp stays fresh without a separate
    /// path. The caller must ensure the contact exists on the list first.
    async fn set_tags(
        &self,
        identity: &ContactIdentity,
        tag_updates: &[TagPreference],
    ) -> Result<(), MarketingListError>;
}
