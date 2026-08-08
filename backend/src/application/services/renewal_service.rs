use std::sync::Arc;

use chrono::{Datelike, NaiveDate};
use uuid::Uuid;

use crate::application::ports::{
    auth_provider_repo_port::AuthProviderRepositoryPort,
    repository_error::RepositoryError,
    role_renewal_repository_port::RoleRenewalRepositoryPort,
    role_repository_port::RoleRepositoryPort,
    rolesync_port::{IdpSubject, RoleSyncPort},
};
use crate::domain::{RenewalStatus, RoleName, RoleRenewal};

use super::{
    audit_log_service::AuditLogService,
    errors::{ServiceError, ServiceResult},
    notification_service::NotificationService,
};

#[derive(Clone)]
pub struct RenewalService {
    renewal_repo: Arc<dyn RoleRenewalRepositoryPort>,
    role_repo: Arc<dyn RoleRepositoryPort>,
    role_sync: Arc<dyn RoleSyncPort>,
    auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    notification_service: NotificationService,
    audit_log: AuditLogService,
}

impl RenewalService {
    pub fn new(
        renewal_repo: Arc<dyn RoleRenewalRepositoryPort>,
        role_repo: Arc<dyn RoleRepositoryPort>,
        role_sync: Arc<dyn RoleSyncPort>,
        auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
        notification_service: NotificationService,
        audit_log: AuditLogService,
    ) -> Self {
        Self {
            renewal_repo,
            role_repo,
            role_sync,
            auth_provider_repo,
            notification_service,
            audit_log,
        }
    }

    /// Main entry point called by the scheduler. Handles the full renewal cycle:
    /// 1. Create renewal records for expiring memberships
    /// 2. Send notification emails at configured milestones
    /// 3. Mark overdue pending renewals as expired
    pub async fn process_pending_renewals(&self) -> ServiceResult<()> {
        self.create_renewal_records().await?;
        self.send_pending_notifications().await?;
        self.expire_overdue_renewals().await?;
        Ok(())
    }

    /// Create RoleRenewal records for memberships approaching expiry.
    async fn create_renewal_records(&self) -> ServiceResult<()> {
        let roles = self
            .role_repo
            .fetch_all()
            .await
            .map_err(ServiceError::from)?;

        // Look ahead as far as the earliest configured reminder needs
        let lookahead = roles
            .iter()
            .filter(|r| r.renewable)
            .flat_map(|r| r.renewal_notification_days.iter().copied())
            .max()
            .unwrap_or(0);

        let expiring = self
            .renewal_repo
            .find_expiring_renewable(lookahead)
            .await
            .map_err(ServiceError::from)?;

        for item in expiring {
            let period_months = match item.renewal_period_months {
                Some(m) if m > 0 => m,
                Some(m) => {
                    tracing::warn!(
                        role = %item.role_name,
                        renewal_period_months = m,
                        "Renewable role has invalid renewal_period_months, skipping"
                    );
                    continue;
                }
                None => {
                    tracing::warn!(
                        role = %item.role_name,
                        "Renewable role has no renewal_period_months configured, skipping"
                    );
                    continue;
                }
            };

            let new_valid_from = item.valid_until + chrono::Duration::days(1);
            let new_valid_until = add_months(new_valid_from, period_months);

            let renewal = RoleRenewal {
                renewal_id: Uuid::new_v4(),
                user_id: item.user_id,
                role_name: RoleName(item.role_name.clone()),
                old_valid_from: item.valid_from,
                old_valid_until: item.valid_until,
                new_valid_from,
                new_valid_until,
                status: RenewalStatus::Pending,
                stripe_payment_id: None,
                notified_days: Vec::new(),
            };

            match self.renewal_repo.create(&renewal).await {
                Ok(_) => {
                    tracing::info!(
                        user_id = %item.user_id,
                        role = %item.role_name,
                        "Created renewal record"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        user_id = %item.user_id,
                        role = %item.role_name,
                        "Failed to create renewal record: {e:?}"
                    );
                }
            }
        }

        Ok(())
    }

    /// Send notification emails for pending renewals at configured milestones.
    async fn send_pending_notifications(&self) -> ServiceResult<()> {
        let roles = self
            .role_repo
            .fetch_all()
            .await
            .map_err(ServiceError::from)?;

        for role in &roles {
            if !role.renewable {
                continue;
            }

            let template_name = match &role.renewal_email_template {
                Some(t) => t.clone(),
                None => continue,
            };

            let payment_link = match &role.renewal_payment_link {
                Some(l) => l.clone(),
                None => continue,
            };

            for &days in &role.renewal_notification_days {
                self.send_notifications_for_milestone(
                    &role.name.0,
                    &template_name,
                    &payment_link,
                    days,
                )
                .await;
            }
        }

        Ok(())
    }

    async fn send_notifications_for_milestone(
        &self,
        role_name: &str,
        template_name: &str,
        payment_link: &str,
        days: i32,
    ) {
        let pending = match self
            .renewal_repo
            .find_pending_needing_notification(role_name, days)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(
                    role = role_name,
                    days = days,
                    "Failed to fetch pending notifications: {e:?}"
                );
                return;
            }
        };

        for item in &pending {
            let full_payment_link =
                format!("{}?client_reference_id={}", payment_link, item.renewal_id);

            let valid_until_str = item.old_valid_until.to_string();

            self.notification_service
                .send_notification_with_vars(
                    Some(template_name),
                    Some(&item.email),
                    &format!("{} {}", item.first_name, item.last_name),
                    role_name,
                    &item.language,
                    &[
                        ("payment_link", &full_payment_link),
                        ("valid_until", &valid_until_str),
                    ],
                )
                .await;

            if let Err(e) = self.renewal_repo.mark_notified(item.renewal_id, days).await {
                tracing::error!(
                    renewal_id = %item.renewal_id,
                    days = days,
                    "Failed to mark notification as sent: {e:?}"
                );
            }

            tracing::info!(
                user_id = %item.user_id,
                role = role_name,
                days_before = days,
                "Sent renewal notification"
            );
        }
    }

    /// Mark overdue pending renewals as expired.
    async fn expire_overdue_renewals(&self) -> ServiceResult<()> {
        let overdue = self
            .renewal_repo
            .find_overdue_pending()
            .await
            .map_err(ServiceError::from)?;

        for renewal in overdue {
            if let Err(e) = self.renewal_repo.mark_expired(renewal.renewal_id).await {
                tracing::error!(
                    renewal_id = %renewal.renewal_id,
                    "Failed to mark renewal as expired: {e:?}"
                );
                continue;
            }

            self.audit_log
                .log(
                    None,
                    "role_renewal.expired",
                    "role_renewal",
                    &renewal.renewal_id.to_string(),
                    Some(serde_json::json!({
                        "user_id": renewal.user_id,
                        "role_name": renewal.role_name.0,
                        "old_valid_until": renewal.old_valid_until.to_string(),
                    })),
                )
                .await;
        }

        Ok(())
    }

    /// Process a renewal payment from Stripe webhook.
    /// Returns true if the UUID matched a renewal, false if it was not found
    /// (allowing the caller to try application payments instead).
    pub async fn process_renewal_payment(
        &self,
        renewal_id: Uuid,
        payment_intent_id: String,
    ) -> ServiceResult<bool> {
        let renewal = match self.renewal_repo.find_by_id(renewal_id).await {
            Ok(r) => r,
            Err(RepositoryError::NotFound) => return Ok(false),
            Err(e) => return Err(ServiceError::from(e)),
        };

        if renewal.status != RenewalStatus::Pending {
            // Already paid or expired — idempotent
            return Ok(true);
        }

        // Mark as paid
        self.renewal_repo
            .mark_paid(renewal_id, &payment_intent_id)
            .await
            .map_err(ServiceError::from)?;

        // Create new role membership
        self.role_repo
            .create_role_member(
                &renewal.user_id,
                &renewal.role_name.0,
                renewal.new_valid_from,
                Some(renewal.new_valid_until),
            )
            .await
            .map_err(ServiceError::from)?;

        // Sync to Keycloak
        let providers = self
            .auth_provider_repo
            .find_by_user_id(&renewal.user_id)
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("{e:?}")))?;

        for provider in &providers {
            if let Err(e) = self
                .role_sync
                .assign_role(
                    &IdpSubject(provider.provider_user_id.clone()),
                    &renewal.role_name,
                )
                .await
            {
                tracing::error!(
                    user_id = %renewal.user_id,
                    role = %renewal.role_name.0,
                    "Failed to sync renewed role to IdP: {e:?}"
                );
            }
        }

        self.audit_log
            .log(
                None,
                "role_renewal.paid",
                "role_renewal",
                &renewal_id.to_string(),
                Some(serde_json::json!({
                    "user_id": renewal.user_id,
                    "role_name": renewal.role_name.0,
                    "new_valid_from": renewal.new_valid_from.to_string(),
                    "new_valid_until": renewal.new_valid_until.to_string(),
                    "stripe_payment_id": payment_intent_id,
                })),
            )
            .await;

        tracing::info!(
            user_id = %renewal.user_id,
            role = %renewal.role_name.0,
            new_valid_from = %renewal.new_valid_from,
            new_valid_until = %renewal.new_valid_until,
            "Renewal payment processed, new role membership created"
        );

        Ok(true)
    }

    /// Get-or-create a pending renewal for the member's latest dated membership
    /// of `role_name` and return the Stripe payment URL for it. Fails unless
    /// the role's renewal window (or grace period) is currently open.
    pub async fn start_member_renewal(
        &self,
        user_id: Uuid,
        role_name: &str,
    ) -> ServiceResult<String> {
        let role = self
            .role_repo
            .fetch_by_name(role_name)
            .await
            .map_err(ServiceError::from)?;

        let (payment_link, period_months) =
            match (&role.renewal_payment_link, role.renewal_period_months) {
                (Some(link), Some(m)) if role.renewable && m > 0 => (link.clone(), m),
                _ => {
                    return Err(ServiceError::Constraint(
                        "Role is not configured for renewal".to_string(),
                    ))
                }
            };

        let memberships = self
            .role_repo
            .fetch_roles_by_member(&user_id)
            .await
            .map_err(ServiceError::from)?;
        let membership = memberships
            .iter()
            .filter(|m| m.role_name.0 == role_name && m.valid_until.is_some())
            .max_by_key(|m| m.valid_until)
            .ok_or(ServiceError::NotFound)?;
        let valid_until = match membership.valid_until {
            Some(d) => d,
            None => return Err(ServiceError::NotFound),
        };

        let today = chrono::Utc::now().date_naive();
        if !role.renewal_is_open(valid_until, today) {
            return Err(ServiceError::Constraint(
                "Renewal window is not open".to_string(),
            ));
        }

        if let Some(existing) = self
            .renewal_repo
            .find_pending(user_id, role_name, membership.valid_from)
            .await
            .map_err(ServiceError::from)?
        {
            return Ok(format!(
                "{payment_link}?client_reference_id={}",
                existing.renewal_id
            ));
        }

        let new_valid_from = valid_until + chrono::Duration::days(1);
        let renewal = RoleRenewal {
            renewal_id: Uuid::new_v4(),
            user_id,
            role_name: RoleName(role_name.to_string()),
            old_valid_from: membership.valid_from,
            old_valid_until: valid_until,
            new_valid_from,
            new_valid_until: add_months(new_valid_from, period_months),
            status: RenewalStatus::Pending,
            stripe_payment_id: None,
            notified_days: Vec::new(),
        };

        let created = match self.renewal_repo.create(&renewal).await {
            Ok(r) => r,
            // Unique pending index: a concurrent request won the race
            Err(RepositoryError::Constraint(_)) => self
                .renewal_repo
                .find_pending(user_id, role_name, membership.valid_from)
                .await
                .map_err(ServiceError::from)?
                .ok_or_else(|| {
                    ServiceError::DatabaseError("pending renewal disappeared".to_string())
                })?,
            Err(e) => return Err(ServiceError::from(e)),
        };

        self.audit_log
            .log(
                Some(user_id),
                "role_renewal.started",
                "role_renewal",
                &created.renewal_id.to_string(),
                Some(serde_json::json!({
                    "user_id": user_id,
                    "role_name": role_name,
                    "old_valid_until": valid_until.to_string(),
                })),
            )
            .await;

        Ok(format!(
            "{payment_link}?client_reference_id={}",
            created.renewal_id
        ))
    }
}

/// Add months to a date, clamping to the last day of the target month.
/// Panics in debug mode if months is not positive.
fn add_months(date: NaiveDate, months: i32) -> NaiveDate {
    debug_assert!(
        months > 0,
        "add_months requires a positive month count, got {months}"
    );

    let total_months = date.month0() as i32 + months;
    let target_year = date.year() + total_months / 12;
    let target_month = (total_months % 12) as u32 + 1;

    // Try the same day, then clamp to last day of month
    NaiveDate::from_ymd_opt(target_year, target_month, date.day())
        .or_else(|| {
            // Last day of target month
            let next_month = if target_month == 12 {
                NaiveDate::from_ymd_opt(target_year + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(target_year, target_month + 1, 1)
            };
            next_month.map(|d| d - chrono::Duration::days(1))
        })
        .unwrap_or(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_months_basic() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        assert_eq!(
            add_months(date, 12),
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );
    }

    #[test]
    fn test_add_months_year_boundary() {
        let date = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        assert_eq!(
            add_months(date, 12),
            NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()
        );
    }

    #[test]
    fn test_add_months_leap_year() {
        let date = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        // 2025 is not a leap year, so Feb 29 clamps to Feb 28
        assert_eq!(
            add_months(date, 12),
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()
        );
    }

    #[test]
    #[should_panic(expected = "add_months requires a positive month count")]
    fn test_add_months_rejects_zero() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        add_months(date, 0);
    }

    #[test]
    #[should_panic(expected = "add_months requires a positive month count")]
    fn test_add_months_rejects_negative() {
        let date = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        add_months(date, -1);
    }
}
