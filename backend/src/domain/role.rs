use chrono::{Duration, NaiveDate};

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
    /// inside the pre-expiry window or the post-expiry grace period.
    pub fn renewal_is_open(&self, valid_until: NaiveDate, today: NaiveDate) -> bool {
        self.renewable
            && today >= valid_until - Duration::days(self.renewal_window_days as i64)
            && today <= valid_until + Duration::days(self.grace_period_days as i64)
    }

    pub fn validate_renewal_config(&self) -> Result<(), String> {
        if self.renewal_window_days < 1 {
            return Err("renewal_window_days must be at least 1".to_string());
        }
        if self.grace_period_days < 0 {
            return Err("grace_period_days must not be negative".to_string());
        }
        if self.renewal_notification_days.iter().any(|&d| d < 1) {
            return Err("renewal_notification_days must all be at least 1".to_string());
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
}
