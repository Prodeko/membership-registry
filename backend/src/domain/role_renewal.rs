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
    pub notified_30d: bool,
    pub notified_7d: bool,
    pub notified_1d: bool,
}
