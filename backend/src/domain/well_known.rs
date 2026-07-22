//! Well-known role and attribute names the backend refers to by name.
//! Single home for such names — do not embed these as inline literals.

/// Registry role granting admin access.
pub const ADMIN_ROLE_NAME: &str = "admin";

/// Member attribute holding the address that receives admin notification
/// emails (e.g. the pending-application digest). Members who have this
/// attribute set receive notifications at the attribute's value.
pub const ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE: &str = "admin-notifications-email";
