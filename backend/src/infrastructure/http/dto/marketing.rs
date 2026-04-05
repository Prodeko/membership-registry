use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::application::ports::marketing_list_port::{
    MarketingPreferences, SubscriptionState, TagPreference,
};

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

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export, rename = "TagPreference")]
pub struct TagPreferenceDTO {
    pub name: String,
    pub active: bool,
}

impl From<TagPreference> for TagPreferenceDTO {
    fn from(t: TagPreference) -> Self {
        Self {
            name: t.name,
            active: t.active,
        }
    }
}

impl From<TagPreferenceDTO> for TagPreference {
    fn from(t: TagPreferenceDTO) -> Self {
        Self {
            name: t.name,
            active: t.active,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export, rename = "MarketingPreferences")]
pub struct MarketingPreferencesDTO {
    pub state: SubscriptionStateDTO,
    pub tags: Vec<TagPreferenceDTO>,
}

impl From<MarketingPreferences> for MarketingPreferencesDTO {
    fn from(p: MarketingPreferences) -> Self {
        Self {
            state: p.state.into(),
            tags: p.tags.into_iter().map(TagPreferenceDTO::from).collect(),
        }
    }
}

/// Either resubscribe (which also re-enables every tag) or update the
/// active/inactive state of individual tags. There is no unsubscribe
/// variant: users opt out entirely via the Mailchimp email footer, not the
/// app.
#[derive(Debug, Deserialize, Clone, TS)]
#[ts(export, rename = "MarketingPreferencesUpdate")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MarketingPreferencesUpdateDTO {
    Subscribe,
    SetTags { tags: Vec<TagPreferenceDTO> },
}
