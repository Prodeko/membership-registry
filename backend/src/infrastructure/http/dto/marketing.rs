use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::ports::marketing_list_port::{SubscriptionState, TagPreference};
use crate::application::services::marketing_service::{UserMarketingPreferences, UserMarketingTag};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, TS)]
#[ts(export, rename = "SubscriptionState")]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStateDTO {
    Subscribed,
    Pending,
    Unsubscribed,
    NotAContact,
}

impl From<SubscriptionState> for SubscriptionStateDTO {
    fn from(s: SubscriptionState) -> Self {
        match s {
            SubscriptionState::Subscribed => Self::Subscribed,
            SubscriptionState::Pending => Self::Pending,
            SubscriptionState::Unsubscribed => Self::Unsubscribed,
            SubscriptionState::NotAContact => Self::NotAContact,
        }
    }
}

/// Input shape for `set_tags`: the user toggles individual tags by label
/// with a new active state. The admin-managed catalog metadata (names,
/// descriptions, order) is not sent back from the client.
#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export, rename = "TagPreferenceUpdate")]
pub struct TagPreferenceUpdateDTO {
    pub label: String,
    pub active: bool,
}

impl From<TagPreferenceUpdateDTO> for TagPreference {
    fn from(t: TagPreferenceUpdateDTO) -> Self {
        Self {
            name: t.label,
            active: t.active,
        }
    }
}

/// User-facing view of a single marketing tag: catalog metadata joined
/// with the user's current active state.
#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export, rename = "UserMarketingTag")]
pub struct UserMarketingTagDTO {
    pub label: String,
    pub name_en: String,
    pub name_fi: String,
    pub desc_en: String,
    pub desc_fi: String,
    pub display_order: i32,
    pub auto_apply: bool,
    pub active: bool,
}

impl From<UserMarketingTag> for UserMarketingTagDTO {
    fn from(t: UserMarketingTag) -> Self {
        Self {
            label: t.label,
            name_en: t.name_en,
            name_fi: t.name_fi,
            desc_en: t.desc_en,
            desc_fi: t.desc_fi,
            display_order: t.display_order,
            auto_apply: t.auto_apply,
            active: t.active,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export, rename = "MarketingPreferences")]
pub struct MarketingPreferencesDTO {
    pub state: SubscriptionStateDTO,
    pub tags: Vec<UserMarketingTagDTO>,
}

impl From<UserMarketingPreferences> for MarketingPreferencesDTO {
    fn from(p: UserMarketingPreferences) -> Self {
        Self {
            state: p.state.into(),
            tags: p.tags.into_iter().map(UserMarketingTagDTO::from).collect(),
        }
    }
}

/// Either resubscribe (which re-enables `auto_apply` tags) or update the
/// active/inactive state of individual tags. There is no unsubscribe
/// variant: users opt out entirely via the Mailchimp email footer, not the
/// app.
#[derive(Debug, Deserialize, Clone, TS)]
#[ts(export, rename = "MarketingPreferencesUpdate")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MarketingPreferencesUpdateDTO {
    Subscribe,
    SetTags { tags: Vec<TagPreferenceUpdateDTO> },
}
