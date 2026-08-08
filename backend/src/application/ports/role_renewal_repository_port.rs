use chrono::NaiveDate;
use uuid::Uuid;

use crate::domain::RoleRenewal;

use super::repository_error::RepositoryError;

/// A role membership that is approaching expiry and is eligible for renewal.
pub struct RenewableExpiring {
    pub user_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub language: String,
    pub role_name: String,
    pub valid_from: NaiveDate,
    pub valid_until: NaiveDate,
    pub renewal_period_months: Option<i32>,
}

/// A pending renewal that is due for notification at a given milestone.
pub struct PendingNotification {
    pub renewal_id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub language: String,
    pub role_name: String,
    pub old_valid_until: NaiveDate,
}

#[async_trait::async_trait]
pub trait RoleRenewalRepositoryPort: Send + Sync {
    /// Find all renewable role memberships expiring within `days_ahead` days
    /// that do NOT already have a pending renewal.
    async fn find_expiring_renewable(
        &self,
        days_ahead: i32,
    ) -> Result<Vec<RenewableExpiring>, RepositoryError>;

    /// Find pending renewals that need notification for a specific day offset.
    /// Returns renewals where old_valid_until is between today and today + days,
    /// that offset is not yet recorded as notified, and the member has email
    /// notifications enabled.
    async fn find_pending_needing_notification(
        &self,
        role_name: &str,
        days: i32,
    ) -> Result<Vec<PendingNotification>, RepositoryError>;

    /// Create a new renewal record. Returns the created renewal.
    async fn create(&self, renewal: &RoleRenewal) -> Result<RoleRenewal, RepositoryError>;

    /// Find a renewal by ID.
    async fn find_by_id(&self, renewal_id: Uuid) -> Result<RoleRenewal, RepositoryError>;

    /// Find a pending renewal for a specific role membership.
    async fn find_pending(
        &self,
        user_id: Uuid,
        role_name: &str,
        old_valid_from: NaiveDate,
    ) -> Result<Option<RoleRenewal>, RepositoryError>;

    /// Mark a renewal as paid with the given Stripe payment ID.
    async fn mark_paid(
        &self,
        renewal_id: Uuid,
        stripe_payment_id: &str,
    ) -> Result<(), RepositoryError>;

    /// Mark a renewal as expired.
    async fn mark_expired(&self, renewal_id: Uuid) -> Result<(), RepositoryError>;

    /// Record that the reminder for the given day offset has been sent.
    /// Appends the offset if it is not already recorded.
    async fn mark_notified(&self, renewal_id: Uuid, days: i32) -> Result<(), RepositoryError>;

    /// Find all pending renewals whose old_valid_until plus the role's grace
    /// period has passed.
    async fn find_overdue_pending(&self) -> Result<Vec<RoleRenewal>, RepositoryError>;
}
