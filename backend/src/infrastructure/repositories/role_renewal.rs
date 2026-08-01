use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::application::ports::repository_error::RepositoryError;
use crate::application::ports::role_renewal_repository_port::{
    PendingNotification, RenewableExpiring, RoleRenewalRepositoryPort,
};
use crate::domain::{RenewalStatus, RoleName, RoleRenewal};

// --- DAO types ---

#[derive(Debug, sqlx::FromRow)]
struct RoleRenewalDAO {
    renewal_id: Uuid,
    user_id: Uuid,
    role_name: String,
    old_valid_from: NaiveDate,
    old_valid_until: NaiveDate,
    new_valid_from: NaiveDate,
    new_valid_until: NaiveDate,
    status: String,
    stripe_payment_id: Option<String>,
    notified_30d: bool,
    notified_7d: bool,
    notified_1d: bool,
}

impl From<RoleRenewalDAO> for RoleRenewal {
    fn from(dao: RoleRenewalDAO) -> Self {
        let status = match dao.status.as_str() {
            "paid" => RenewalStatus::Paid,
            "expired" => RenewalStatus::Expired,
            _ => RenewalStatus::Pending,
        };
        Self {
            renewal_id: dao.renewal_id,
            user_id: dao.user_id,
            role_name: RoleName(dao.role_name),
            old_valid_from: dao.old_valid_from,
            old_valid_until: dao.old_valid_until,
            new_valid_from: dao.new_valid_from,
            new_valid_until: dao.new_valid_until,
            status,
            stripe_payment_id: dao.stripe_payment_id,
            notified_30d: dao.notified_30d,
            notified_7d: dao.notified_7d,
            notified_1d: dao.notified_1d,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RenewableExpiringDAO {
    user_id: Uuid,
    email: String,
    first_name: String,
    last_name: String,
    language: String,
    role_name: String,
    valid_from: NaiveDate,
    valid_until: NaiveDate,
    renewal_period_months: Option<i32>,
}

impl From<RenewableExpiringDAO> for RenewableExpiring {
    fn from(dao: RenewableExpiringDAO) -> Self {
        Self {
            user_id: dao.user_id,
            email: dao.email,
            first_name: dao.first_name,
            last_name: dao.last_name,
            language: dao.language,
            role_name: dao.role_name,
            valid_from: dao.valid_from,
            valid_until: dao.valid_until,
            renewal_period_months: dao.renewal_period_months,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct PendingNotificationDAO {
    renewal_id: Uuid,
    user_id: Uuid,
    email: String,
    first_name: String,
    last_name: String,
    language: String,
    role_name: String,
    old_valid_until: NaiveDate,
}

impl From<PendingNotificationDAO> for PendingNotification {
    fn from(dao: PendingNotificationDAO) -> Self {
        Self {
            renewal_id: dao.renewal_id,
            user_id: dao.user_id,
            email: dao.email,
            first_name: dao.first_name,
            last_name: dao.last_name,
            language: dao.language,
            role_name: dao.role_name,
            old_valid_until: dao.old_valid_until,
        }
    }
}

// --- Repository ---

#[derive(Clone)]
pub struct RoleRenewalRepo {
    pub pool: PgPool,
}

#[async_trait::async_trait]
impl RoleRenewalRepositoryPort for RoleRenewalRepo {
    async fn find_expiring_renewable(
        &self,
        days_ahead: i32,
    ) -> Result<Vec<RenewableExpiring>, RepositoryError> {
        let rows = sqlx::query_as::<_, RenewableExpiringDAO>(
            r#"
            SELECT
                rm.user_id,
                m.email,
                m.first_name,
                m.last_name,
                m.language,
                rm.role_name,
                rm.valid_from,
                rm.valid_until,
                r.renewal_period_months
            FROM RoleMember rm
            JOIN Role r ON r.name = rm.role_name
            JOIN Member m ON m.user_id = rm.user_id
            WHERE r.renewable = TRUE
              AND rm.valid_until IS NOT NULL
              AND rm.valid_until >= CURRENT_DATE
              AND rm.valid_until <= CURRENT_DATE + ($1 || ' days')::interval
              AND rm.keycloak_removed_at IS NULL
              AND NOT EXISTS (
                  SELECT 1 FROM RoleRenewal rr
                  WHERE rr.user_id = rm.user_id
                    AND rr.role_name = rm.role_name
                    AND rr.old_valid_from = rm.valid_from
                    AND rr.status = 'pending'
              )
            "#,
        )
        .bind(days_ahead)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_pending_needing_notification(
        &self,
        role_name: &str,
        days: i32,
    ) -> Result<Vec<PendingNotification>, RepositoryError> {
        let notified_column = match days {
            30 => "rr.notified_30d",
            7 => "rr.notified_7d",
            1 => "rr.notified_1d",
            _ => {
                return Err(RepositoryError::Unexpected(format!(
                    "Unsupported notification milestone: {days} days"
                )))
            }
        };

        let query = format!(
            r#"
            SELECT
                rr.renewal_id,
                rr.user_id,
                m.email,
                m.first_name,
                m.last_name,
                m.language,
                rr.role_name,
                rr.old_valid_until
            FROM RoleRenewal rr
            JOIN Member m ON m.user_id = rr.user_id
            WHERE rr.role_name = $1
              AND rr.status = 'pending'
              AND rr.old_valid_until >= CURRENT_DATE
              AND rr.old_valid_until <= CURRENT_DATE + ($2 || ' days')::interval
              AND NOT {notified_column}
              AND m.email_notifications = TRUE
            "#
        );

        let rows = sqlx::query_as::<_, PendingNotificationDAO>(sqlx::AssertSqlSafe(query))
            .bind(role_name)
            .bind(days)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create(&self, renewal: &RoleRenewal) -> Result<RoleRenewal, RepositoryError> {
        let status_str = match renewal.status {
            RenewalStatus::Pending => "pending",
            RenewalStatus::Paid => "paid",
            RenewalStatus::Expired => "expired",
        };

        let row = sqlx::query_as::<_, RoleRenewalDAO>(
            r#"
            INSERT INTO RoleRenewal (
                renewal_id, user_id, role_name, old_valid_from, old_valid_until,
                new_valid_from, new_valid_until, status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8::renewal_status)
            RETURNING renewal_id, user_id, role_name, old_valid_from, old_valid_until,
                      new_valid_from, new_valid_until, status::text, stripe_payment_id,
                      notified_30d, notified_7d, notified_1d
            "#,
        )
        .bind(renewal.renewal_id)
        .bind(renewal.user_id)
        .bind(&renewal.role_name.0)
        .bind(renewal.old_valid_from)
        .bind(renewal.old_valid_until)
        .bind(renewal.new_valid_from)
        .bind(renewal.new_valid_until)
        .bind(status_str)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn find_by_id(&self, renewal_id: Uuid) -> Result<RoleRenewal, RepositoryError> {
        let row = sqlx::query_as::<_, RoleRenewalDAO>(
            r#"
            SELECT renewal_id, user_id, role_name, old_valid_from, old_valid_until,
                   new_valid_from, new_valid_until, status::text, stripe_payment_id,
                   notified_30d, notified_7d, notified_1d
            FROM RoleRenewal
            WHERE renewal_id = $1
            "#,
        )
        .bind(renewal_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn find_pending(
        &self,
        user_id: Uuid,
        role_name: &str,
        old_valid_from: NaiveDate,
    ) -> Result<Option<RoleRenewal>, RepositoryError> {
        let row = sqlx::query_as::<_, RoleRenewalDAO>(
            r#"
            SELECT renewal_id, user_id, role_name, old_valid_from, old_valid_until,
                   new_valid_from, new_valid_until, status::text, stripe_payment_id,
                   notified_30d, notified_7d, notified_1d
            FROM RoleRenewal
            WHERE user_id = $1 AND role_name = $2 AND old_valid_from = $3 AND status = 'pending'
            "#,
        )
        .bind(user_id)
        .bind(role_name)
        .bind(old_valid_from)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    async fn mark_paid(
        &self,
        renewal_id: Uuid,
        stripe_payment_id: &str,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            UPDATE RoleRenewal
            SET status = 'paid'::renewal_status, stripe_payment_id = $2
            WHERE renewal_id = $1
            "#,
        )
        .bind(renewal_id)
        .bind(stripe_payment_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_expired(&self, renewal_id: Uuid) -> Result<(), RepositoryError> {
        sqlx::query(
            r#"
            UPDATE RoleRenewal
            SET status = 'expired'::renewal_status
            WHERE renewal_id = $1
            "#,
        )
        .bind(renewal_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn mark_notified(&self, renewal_id: Uuid, days: i32) -> Result<(), RepositoryError> {
        match days {
            30 => {
                sqlx::query("UPDATE RoleRenewal SET notified_30d = TRUE WHERE renewal_id = $1")
                    .bind(renewal_id)
                    .execute(&self.pool)
                    .await?;
            }
            7 => {
                sqlx::query("UPDATE RoleRenewal SET notified_7d = TRUE WHERE renewal_id = $1")
                    .bind(renewal_id)
                    .execute(&self.pool)
                    .await?;
            }
            1 => {
                sqlx::query("UPDATE RoleRenewal SET notified_1d = TRUE WHERE renewal_id = $1")
                    .bind(renewal_id)
                    .execute(&self.pool)
                    .await?;
            }
            _ => {
                return Err(RepositoryError::Unexpected(format!(
                    "Unsupported notification milestone: {days} days"
                )));
            }
        }
        Ok(())
    }

    async fn find_overdue_pending(&self) -> Result<Vec<RoleRenewal>, RepositoryError> {
        let rows = sqlx::query_as::<_, RoleRenewalDAO>(
            r#"
            SELECT renewal_id, user_id, role_name, old_valid_from, old_valid_until,
                   new_valid_from, new_valid_until, status::text, stripe_payment_id,
                   notified_30d, notified_7d, notified_1d
            FROM RoleRenewal
            WHERE status = 'pending' AND old_valid_until < CURRENT_DATE
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
