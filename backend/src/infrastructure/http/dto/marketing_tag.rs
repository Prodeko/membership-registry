use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::MarketingTag;

/// Admin-side view of a single marketing tag. Serialized unchanged to the
/// admin panel for CRUD editing.
#[derive(Debug, Serialize, Deserialize, Clone, TS)]
#[ts(export, rename = "MarketingTag")]
pub struct MarketingTagDTO {
    pub label: String,
    pub name_en: String,
    pub name_fi: String,
    pub desc_en: String,
    pub desc_fi: String,
    pub display_order: i32,
    pub auto_apply: bool,
}

impl From<MarketingTag> for MarketingTagDTO {
    fn from(t: MarketingTag) -> Self {
        Self {
            label: t.label,
            name_en: t.name_en,
            name_fi: t.name_fi,
            desc_en: t.desc_en,
            desc_fi: t.desc_fi,
            display_order: t.display_order,
            auto_apply: t.auto_apply,
        }
    }
}

impl From<MarketingTagDTO> for MarketingTag {
    fn from(t: MarketingTagDTO) -> Self {
        Self {
            label: t.label,
            name_en: t.name_en,
            name_fi: t.name_fi,
            desc_en: t.desc_en,
            desc_fi: t.desc_fi,
            display_order: t.display_order,
            auto_apply: t.auto_apply,
        }
    }
}

/// Create body. Takes the `label` (Mailchimp tag key) from the admin,
/// since it becomes the primary key and can't be changed later.
#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "CreateMarketingTag")]
pub struct CreateMarketingTagDTO {
    pub label: String,
    pub name_en: String,
    pub name_fi: String,
    pub desc_en: String,
    pub desc_fi: String,
    pub display_order: i32,
    pub auto_apply: bool,
}

impl From<CreateMarketingTagDTO> for MarketingTag {
    fn from(t: CreateMarketingTagDTO) -> Self {
        Self {
            label: t.label,
            name_en: t.name_en,
            name_fi: t.name_fi,
            desc_en: t.desc_en,
            desc_fi: t.desc_fi,
            display_order: t.display_order,
            auto_apply: t.auto_apply,
        }
    }
}

/// Update body. `label` comes from the URL path, not the body.
#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "UpdateMarketingTag")]
pub struct UpdateMarketingTagDTO {
    pub name_en: String,
    pub name_fi: String,
    pub desc_en: String,
    pub desc_fi: String,
    pub display_order: i32,
    pub auto_apply: bool,
}
