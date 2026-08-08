use chrono::NaiveDate;
use uuid::Uuid;

use super::RoleName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenewalStatus {
    Pending,
    Paid,
    Expired,
}

#[derive(Debug, Clone)]
pub struct RoleRenewal {
    pub renewal_id: Uuid,
    pub user_id: Uuid,
    pub role_name: RoleName,
    pub old_valid_from: NaiveDate,
    pub old_valid_until: NaiveDate,
    pub new_valid_from: NaiveDate,
    pub new_valid_until: NaiveDate,
    pub status: RenewalStatus,
    pub stripe_payment_id: Option<String>,
    /// Day offsets (days before expiry) at which a reminder has been sent.
    pub notified_days: Vec<i32>,
}

/// Reminder milestones a renewal has crossed but not yet been notified for:
/// every configured offset of at least `days_left` days that is absent from
/// `notified`. One email should cover all of them, so a renewal created with
/// several milestones already in the past gets one reminder, not a burst.
pub fn milestones_due(offsets: &[i32], days_left: i32, notified: &[i32]) -> Vec<i32> {
    offsets
        .iter()
        .copied()
        .filter(|&d| d >= days_left && !notified.contains(&d))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::milestones_due;

    #[test]
    fn all_crossed_milestones_are_due_at_once() {
        assert_eq!(milestones_due(&[90, 60, 30, 7, 1], 25, &[]), vec![90, 60, 30]);
    }

    #[test]
    fn notified_milestones_are_not_resent() {
        assert!(milestones_due(&[90, 60, 30, 7, 1], 25, &[90, 60, 30]).is_empty());
        assert_eq!(milestones_due(&[90, 60, 30, 7, 1], 7, &[90, 60, 30]), vec![7]);
    }

    #[test]
    fn uncrossed_milestones_are_not_due() {
        assert!(milestones_due(&[7, 1], 25, &[]).is_empty());
    }
}
