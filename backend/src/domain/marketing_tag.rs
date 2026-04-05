/// A marketing tag that users can toggle on their profile. The `label` is
/// the Mailchimp tag identifier (e.g. `weekly_newsletter`) — Mailchimp
/// creates the tag on its side the first time we push it. The name and
/// description fields are localized copy shown in the profile UI.
///
/// `display_order` controls the order of tags in the user UI (ascending).
/// `auto_apply` means the tag is activated automatically for new users on
/// registration and when a user uses the "resubscribe" flow. It does NOT
/// backfill existing Mailchimp contacts — that is an admin responsibility
/// to perform in the Mailchimp UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketingTag {
    pub label: String,
    pub name_en: String,
    pub name_fi: String,
    pub desc_en: String,
    pub desc_fi: String,
    pub display_order: i32,
    pub auto_apply: bool,
}
