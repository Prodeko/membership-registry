use chrono::{Duration, NaiveDate};
use uuid::Uuid;

/// Upper bound for renewal window, grace period, and notification offsets.
/// Keeps admin-supplied day counts far away from date-arithmetic overflow.
pub const MAX_RENEWAL_DAYS: i32 = 3650;

/// The configured Stripe payment link with the renewal's ID appended as
/// `client_reference_id`, respecting any query string already on the link.
pub fn renewal_payment_url(payment_link: &str, renewal_id: Uuid) -> String {
    let separator = if payment_link.contains('?') { '&' } else { '?' };
    format!("{payment_link}{separator}client_reference_id={renewal_id}")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleName(pub String);

impl From<RoleName> for String {
    fn from(name: RoleName) -> Self {
        name.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    pub name: RoleName,
    pub color: Option<String>,
    pub description: Option<String>,
    pub renewable: bool,
    pub renewal_payment_link: Option<String>,
    pub renewal_period_months: Option<i32>,
    pub renewal_email_template: Option<String>,
    pub renewal_notification_days: Vec<i32>,
    pub renewal_window_days: i32,
    pub grace_period_days: i32,
}

impl Role {
    /// Whether a membership expiring at `valid_until` can be renewed on `today`:
    /// the role is renewable and `today` is inside the pre-expiry window or the
    /// post-expiry grace period.
    pub fn renewal_is_open(&self, valid_until: NaiveDate, today: NaiveDate) -> bool {
        self.renewable
            && today >= valid_until - Duration::days(self.renewal_window_days as i64)
            && today <= valid_until + Duration::days(self.grace_period_days as i64)
    }

    pub fn validate_renewal_config(&self) -> Result<(), String> {
        if !(1..=MAX_RENEWAL_DAYS).contains(&self.renewal_window_days) {
            return Err(format!(
                "renewal_window_days must be between 1 and {MAX_RENEWAL_DAYS}"
            ));
        }
        if !(0..=MAX_RENEWAL_DAYS).contains(&self.grace_period_days) {
            return Err(format!(
                "grace_period_days must be between 0 and {MAX_RENEWAL_DAYS}"
            ));
        }
        if self.renewal_notification_days.iter().any(|&d| d < 1) {
            return Err("renewal_notification_days must all be at least 1".to_string());
        }
        if let Some(&max_offset) = self.renewal_notification_days.iter().max() {
            if max_offset > self.renewal_window_days {
                return Err(format!(
                    "reminder offsets must not exceed the renewal window \
                     ({max_offset} > {} days): members would be emailed a payment \
                     link before renewal opens",
                    self.renewal_window_days
                ));
            }
        }
        if self.renewable {
            match &self.renewal_payment_link {
                None => {
                    return Err("a renewable role requires a renewal payment link".to_string());
                }
                Some(link)
                    if !link.starts_with("https://")
                        || link.len() <= "https://".len()
                        || link.contains(char::is_whitespace)
                        || link.contains('#') =>
                {
                    return Err("renewal payment link must be an absolute https:// URL".to_string());
                }
                Some(_) => {}
            }
            if !self.renewal_period_months.is_some_and(|m| m > 0) {
                return Err(
                    "a renewable role requires a positive renewal period (months)".to_string(),
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    fn role(window: i32, grace: i32) -> Role {
        Role {
            name: RoleName("member".to_string()),
            color: None,
            description: None,
            renewable: true,
            renewal_payment_link: Some("https://pay".to_string()),
            renewal_period_months: Some(12),
            renewal_email_template: None,
            renewal_notification_days: vec![30, 7, 1],
            renewal_window_days: window,
            grace_period_days: grace,
        }
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn renewal_opens_window_days_before_expiry() {
        let r = role(30, 0);
        let expiry = d(2026, 12, 31);
        assert!(!r.renewal_is_open(expiry, d(2026, 11, 30)));
        assert!(r.renewal_is_open(expiry, d(2026, 12, 1)));
        assert!(r.renewal_is_open(expiry, d(2026, 12, 31)));
    }

    #[test]
    fn renewal_closes_after_grace_period() {
        let r = role(30, 14);
        let expiry = d(2026, 12, 31);
        assert!(r.renewal_is_open(expiry, d(2027, 1, 14)));
        assert!(!r.renewal_is_open(expiry, d(2027, 1, 15)));
    }

    #[test]
    fn zero_grace_closes_renewal_at_expiry() {
        let r = role(30, 0);
        let expiry = d(2026, 12, 31);
        assert!(!r.renewal_is_open(expiry, d(2027, 1, 1)));
    }

    #[test]
    fn non_renewable_role_is_never_open() {
        let mut r = role(30, 14);
        r.renewable = false;
        assert!(!r.renewal_is_open(d(2026, 12, 31), d(2026, 12, 15)));
    }

    #[test]
    fn validate_rejects_bad_config() {
        assert!(role(0, 0).validate_renewal_config().is_err());
        assert!(role(30, -1).validate_renewal_config().is_err());
        let mut r = role(30, 0);
        r.renewal_notification_days = vec![30, 0];
        assert!(r.validate_renewal_config().is_err());
        assert!(role(90, 14).validate_renewal_config().is_ok());
    }

    #[test]
    fn validate_bounds_day_fields() {
        assert!(role(MAX_RENEWAL_DAYS + 1, 0)
            .validate_renewal_config()
            .is_err());
        assert!(role(30, MAX_RENEWAL_DAYS + 1)
            .validate_renewal_config()
            .is_err());
        assert!(role(MAX_RENEWAL_DAYS, 0).validate_renewal_config().is_ok());
    }

    #[test]
    fn validate_rejects_offsets_beyond_window() {
        let mut r = role(14, 0);
        r.renewal_notification_days = vec![30, 7, 1];
        assert!(r.validate_renewal_config().is_err());
        r.renewal_notification_days = vec![14, 7, 1];
        assert!(r.validate_renewal_config().is_ok());
    }

    #[test]
    fn validate_renewable_requires_link_and_period() {
        let mut r = role(30, 0);
        r.renewal_payment_link = None;
        assert!(r.validate_renewal_config().is_err());

        let mut r = role(30, 0);
        r.renewal_period_months = None;
        assert!(r.validate_renewal_config().is_err());
        r.renewal_period_months = Some(0);
        assert!(r.validate_renewal_config().is_err());

        // Non-renewable roles may leave the payment config empty
        let mut r = role(30, 0);
        r.renewable = false;
        r.renewal_payment_link = None;
        r.renewal_period_months = None;
        assert!(r.validate_renewal_config().is_ok());
    }

    #[test]
    fn validate_rejects_malformed_payment_links() {
        for bad in [
            "http://buy.stripe.com/x",
            "https://",
            "https://pay me",
            "https://buy.stripe.com/x#frag",
        ] {
            let mut r = role(30, 0);
            r.renewal_payment_link = Some(bad.to_string());
            assert!(r.validate_renewal_config().is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn payment_url_appends_client_reference_id() {
        let id = Uuid::parse_str("9707582e-c149-45a7-bae1-4b0f4de4b06f").unwrap();
        assert_eq!(
            renewal_payment_url("https://buy.stripe.com/x", id),
            format!("https://buy.stripe.com/x?client_reference_id={id}")
        );
        assert_eq!(
            renewal_payment_url("https://buy.stripe.com/x?locale=fi", id),
            format!("https://buy.stripe.com/x?locale=fi&client_reference_id={id}")
        );
    }
}
