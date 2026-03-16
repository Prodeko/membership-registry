use std::sync::Arc;

use crate::application::ports::application_repository_port::{
    ApplicationCommandPort, ApplicationQueryPort, ApplicationTargetableRole, ApplicationWithMember,
    TargetableRolePort,
};
use crate::domain::application::{
    Application, ApplicationAction, ApplicationStatus, ApplicationTransition, NewApplication,
    TransitionError,
};
use chrono::NaiveDate;
use uuid::Uuid;

use crate::application::services::notification_service::NotificationService;

use super::{
    audit_log_service::AuditLogService,
    errors::{ServiceError as E, ServiceResult},
    role_service::RoleService,
};

pub struct CreateApplicationParams {
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_until: NaiveDate,
    pub stripe_payment_id: Option<String>,
    pub optional_roles: Option<Vec<String>>,
    pub application_text: Option<String>,
}

pub struct ApplicationService {
    pub application_commands: Arc<dyn ApplicationCommandPort>,
    pub application_queries: Arc<dyn ApplicationQueryPort>,
    pub targetable_roles: Arc<dyn TargetableRolePort>,
    pub role_service: RoleService,
    pub audit_log: AuditLogService,
    pub notification_service: NotificationService,
}

impl ApplicationService {
    pub fn new(
        application_commands: Arc<dyn ApplicationCommandPort>,
        application_queries: Arc<dyn ApplicationQueryPort>,
        targetable_roles: Arc<dyn TargetableRolePort>,
        role_service: RoleService,
        audit_log: AuditLogService,
        notification_service: NotificationService,
    ) -> Self {
        Self {
            application_commands,
            application_queries,
            targetable_roles,
            role_service,
            audit_log,
            notification_service,
        }
    }

    pub async fn create_application(
        &self,
        params: CreateApplicationParams,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<Application> {
        let existing = self
            .application_queries
            .fetch_existing(params.user_id, params.role_name.clone(), params.valid_until)
            .await;

        if existing.is_ok() {
            return Err(E::AlreadyExists);
        }

        let targetable_role = self
            .targetable_roles
            .fetch_targetable_role(params.role_name.clone(), params.valid_until)
            .await
            .map_err(E::from)?;

        if !targetable_role.active {
            return Err(E::NotActive);
        }

        let requires_payment =
            targetable_role.payment_link.is_some() && params.stripe_payment_id.is_none();

        let (new_application, _creation) = NewApplication::create(
            params.user_id,
            params.role_name,
            params.valid_until,
            params.stripe_payment_id,
            params.optional_roles,
            params.application_text,
            requires_payment,
        );

        let application = self
            .application_commands
            .create(&new_application)
            .await
            .map_err(E::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "application.create",
                "application",
                &application.application_id.0.to_string(),
                Some(serde_json::json!({
                    "role_name": &application.role_name,
                })),
            )
            .await;

        Ok(application)
    }

    pub async fn get_all_applications(&self) -> ServiceResult<Vec<Application>> {
        self.application_queries.fetch_all().await.map_err(E::from)
    }

    pub async fn get_application(&self, application_id: Uuid) -> ServiceResult<Application> {
        self.application_queries
            .fetch_one(application_id)
            .await
            .map_err(E::from)
    }

    pub async fn get_application_with_member(
        &self,
        application_id: Uuid,
    ) -> ServiceResult<ApplicationWithMember> {
        self.application_queries
            .fetch_with_member_one(application_id)
            .await
            .map_err(E::from)
    }

    pub async fn get_applications_with_member_filtered(
        &self,
        status: Option<ApplicationStatus>,
        search: Option<String>,
    ) -> ServiceResult<Vec<ApplicationWithMember>> {
        self.application_queries
            .fetch_with_user_filtered(status, search)
            .await
            .map_err(E::from)
    }

    pub async fn get_applications_for_user(
        &self,
        user_id: Uuid,
    ) -> ServiceResult<Vec<Application>> {
        self.application_queries
            .fetch_applications_for_user(user_id)
            .await
            .map_err(E::from)
    }

    pub async fn update_application_status(
        &self,
        application_id: Uuid,
        action: ApplicationAction,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        let application = self.get_application(application_id).await?;

        let (new_status, transition) = application.apply(action).map_err(|e| match e {
            TransitionError::AlreadyTerminal => E::ApplicationAlreadyProcessed,
            TransitionError::InvalidAction { .. } => E::InvalidStatus,
        })?;

        self.application_commands
            .update_status(application_id, &new_status)
            .await
            .map_err(E::from)?;

        match transition {
            ApplicationTransition::PaymentReceived => {
                self.audit_log
                    .log(
                        actor_user_id,
                        "application.payment_received",
                        "application",
                        &application_id.to_string(),
                        None,
                    )
                    .await;
            }
            ApplicationTransition::Approved => {
                self.role_service
                    .add_role_member(
                        application.user_id,
                        &application.role_name,
                        chrono::Utc::now().date_naive(),
                        Some(application.valid_until),
                        actor_user_id,
                    )
                    .await?;

                self.send_status_notification(&application, &new_status)
                    .await;

                self.audit_log
                    .log(
                        actor_user_id,
                        "application.approved",
                        "application",
                        &application_id.to_string(),
                        Some(serde_json::json!({
                            "role_name": &application.role_name,
                        })),
                    )
                    .await;
            }
            ApplicationTransition::Rejected => {
                self.send_status_notification(&application, &new_status)
                    .await;

                self.audit_log
                    .log(
                        actor_user_id,
                        "application.rejected",
                        "application",
                        &application_id.to_string(),
                        None,
                    )
                    .await;
            }
        }

        Ok(())
    }

    async fn send_status_notification(
        &self,
        application: &Application,
        status: &ApplicationStatus,
    ) {
        let template_name = self
            .targetable_roles
            .fetch_targetable_role(application.role_name.clone(), application.valid_until)
            .await
            .ok()
            .and_then(|tr| match status {
                ApplicationStatus::Approved => tr.approved_email_template,
                ApplicationStatus::Rejected => tr.rejected_email_template,
                _ => None,
            });

        let member = self
            .application_queries
            .fetch_with_member_one(application.application_id.0)
            .await
            .ok();

        let locale = member
            .as_ref()
            .and_then(|m| m.language.as_deref())
            .unwrap_or("fi");

        self.notification_service
            .send_notification(
                template_name.as_deref(),
                member.as_ref().and_then(|m| m.email.as_deref()),
                member
                    .as_ref()
                    .and_then(|m| m.full_name.as_deref())
                    .unwrap_or_default(),
                &application.role_name,
                locale,
            )
            .await;
    }

    pub async fn withdraw_application(
        &self,
        application_id: Uuid,
        user_id: Uuid,
    ) -> ServiceResult<()> {
        let application = self.get_application(application_id).await?;

        if application.user_id != user_id {
            return Err(E::Forbidden);
        }

        application.can_withdraw().map_err(|e| match e {
            TransitionError::AlreadyTerminal => E::ApplicationAlreadyProcessed,
            TransitionError::InvalidAction { .. } => E::InvalidStatus,
        })?;

        self.application_commands
            .delete(application_id)
            .await
            .map_err(E::from)?;

        self.audit_log
            .log(
                Some(user_id),
                "application.withdraw",
                "application",
                &application_id.to_string(),
                Some(serde_json::json!({
                    "role_name": &application.role_name,
                })),
            )
            .await;

        Ok(())
    }

    pub async fn delete_application(
        &self,
        application_id: Uuid,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.application_commands
            .delete(application_id)
            .await
            .map_err(E::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "application.delete",
                "application",
                &application_id.to_string(),
                None,
            )
            .await;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
        payment_link: Option<String>,
        approved_email_template: Option<String>,
        rejected_email_template: Option<String>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.targetable_roles
            .create_targetable_role(
                role_name.clone(),
                valid_until,
                active,
                payment_link,
                approved_email_template,
                rejected_email_template,
            )
            .await
            .map_err(E::from)?;

        self.audit_log.log(
            actor_user_id,
            "targetable_role.create",
            "targetable_role",
            &format!("{}:{}", role_name, valid_until),
            Some(serde_json::json!({ "role_name": role_name, "valid_until": valid_until.to_string() })),
        ).await;

        Ok(())
    }

    pub async fn fetch_all_targetable_roles(
        &self,
    ) -> ServiceResult<Vec<ApplicationTargetableRole>> {
        self.targetable_roles
            .fetch_all_targetable_roles()
            .await
            .map_err(E::from)
    }

    pub async fn update_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.targetable_roles
            .update_targetable_role(role_name.clone(), valid_until, active)
            .await
            .map_err(E::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "targetable_role.update",
                "targetable_role",
                &format!("{}:{}", role_name, valid_until),
                Some(serde_json::json!({ "role_name": role_name, "active": active })),
            )
            .await;

        Ok(())
    }

    pub async fn get_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> ServiceResult<ApplicationTargetableRole> {
        self.targetable_roles
            .fetch_targetable_role(role_name, valid_until)
            .await
            .map_err(E::from)
    }

    pub async fn delete_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.targetable_roles
            .delete_targetable_role(role_name.clone(), valid_until)
            .await
            .map_err(E::from)?;

        self.audit_log.log(
            actor_user_id,
            "targetable_role.delete",
            "targetable_role",
            &format!("{}:{}", role_name, valid_until),
            Some(serde_json::json!({ "role_name": role_name, "valid_until": valid_until.to_string() })),
        ).await;

        Ok(())
    }

    pub async fn update_payment_id(
        &self,
        application_id: Uuid,
        stripe_payment_id: String,
    ) -> ServiceResult<()> {
        let application = self.get_application(application_id).await?;

        // Idempotent: if payment was already recorded, return Ok so Stripe
        // does not keep retrying the webhook.
        if application.stripe_payment_id.as_deref() == Some(stripe_payment_id.as_str()) {
            return Ok(());
        }

        let (new_status, _transition) = application
            .apply(ApplicationAction::PaymentReceived)
            .map_err(|e| match e {
                TransitionError::AlreadyTerminal => E::ApplicationAlreadyProcessed,
                TransitionError::InvalidAction { .. } => E::InvalidStatus,
            })?;

        self.application_commands
            .update_payment_id(application_id, stripe_payment_id.clone(), &new_status)
            .await
            .map_err(E::from)?;

        self.audit_log
            .log(
                None,
                "application.payment_received",
                "application",
                &application_id.to_string(),
                Some(serde_json::json!({ "stripe_payment_id": stripe_payment_id })),
            )
            .await;

        Ok(())
    }
}
