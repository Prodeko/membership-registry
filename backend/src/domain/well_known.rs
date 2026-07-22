//! Well-known role and attribute names the backend refers to by name.
//! Single home for such names — do not embed these as inline literals.

use crate::domain::{AttributeName, RoleName};

/// Registry role granting admin access.
pub const ADMIN_ROLE_NAME: &str = "admin";

/// [`ADMIN_ROLE_NAME`] as a domain type.
pub fn admin_role_name() -> RoleName {
    RoleName(ADMIN_ROLE_NAME.to_string())
}

/// Member attribute holding the address that receives admin notification
/// emails (e.g. the pending-application digest). Notifications go to the
/// attribute's value, and only for members who also hold [`ADMIN_ROLE_NAME`]
/// — the attribute alone does not subscribe a member.
pub const ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE: &str = "admin-notifications-email";

/// [`ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE`] as a domain type. Validity is
/// guaranteed by the unit test below, so no runtime error path is needed.
pub fn admin_notifications_email_attribute() -> AttributeName {
    AttributeName::new_unchecked(ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_notifications_email_attribute_is_a_valid_attribute_name() {
        assert_eq!(
            AttributeName::new(ADMIN_NOTIFICATIONS_EMAIL_ATTRIBUTE),
            Ok(admin_notifications_email_attribute())
        );
    }
}
