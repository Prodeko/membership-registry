use std::sync::Arc;

use uuid::Uuid;

use crate::application::ports::{
    auth_provider_repo_port::AuthProviderRepositoryPort,
    repository_error::RepositoryError,
    role_renewal_repository_port::RoleRenewalRepositoryPort,
    role_repository_port::RoleRepositoryPort,
    rolesync_port::{IdpSubject, RoleSyncPort},
};
use crate::domain::{
    add_months, milestones_due, renewal_payment_url, RenewalStatus, Role, RoleName, RoleRenewal,
};

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
        let expiring = self
            .renewal_repo
            .find_expiring_renewable()
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

            self.send_notifications_for_role(role, &template_name, &payment_link)
                .await;
        }

        Ok(())
    }

    /// Send at most one reminder email per pending renewal, covering every
    /// configured milestone the renewal has crossed but not yet been notified
    /// for. Marking all crossed milestones at once prevents a burst of emails
    /// when a renewal is created with several milestones already in the past.
    async fn send_notifications_for_role(
        &self,
        role: &Role,
        template_name: &str,
        payment_link: &str,
    ) {
        let role_name = &role.name.0;
        let Some(&max_offset) = role.renewal_notification_days.iter().max() else {
            return;
        };

        let pending = match self
            .renewal_repo
            .find_pending_needing_notification(role_name, max_offset)
            .await
        {
            Ok(p) => p,
            Err(e) => {
                tracing::error!(
                    role = role_name,
                    "Failed to fetch pending notifications: {e:?}"
                );
                return;
            }
        };

        for item in &pending {
            let due = milestones_due(
                &role.renewal_notification_days,
                item.days_left,
                &item.notified_days,
            );
            if due.is_empty() {
                continue;
            }

            let full_payment_link = renewal_payment_url(payment_link, item.renewal_id);
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

            for &days in &due {
                if let Err(e) = self.renewal_repo.mark_notified(item.renewal_id, days).await {
                    tracing::error!(
                        renewal_id = %item.renewal_id,
                        days = days,
                        "Failed to mark notification as sent: {e:?}"
                    );
                }
            }

            tracing::info!(
                user_id = %item.user_id,
                role = role_name,
                days_before = item.days_left,
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
            return Ok(renewal_payment_url(&payment_link, existing.renewal_id));
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
            // The unique pending index fired: a concurrent request won the
            // race, so reuse the renewal it created. Any other constraint
            // violation is a real error and must propagate.
            Err(RepositoryError::Constraint(msg))
                if msg.contains("idx_role_renewal_pending_unique") =>
            {
                self.renewal_repo
                    .find_pending(user_id, role_name, membership.valid_from)
                    .await
                    .map_err(ServiceError::from)?
                    .ok_or_else(|| {
                        ServiceError::DatabaseError("pending renewal disappeared".to_string())
                    })?
            }
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

        Ok(renewal_payment_url(&payment_link, created.renewal_id))
    }
}
