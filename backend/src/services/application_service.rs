// src/application_service.rs

use crate::repositories::{
    application::{
        Application, ApplicationRepo, ApplicationTargetableRole, ApplicationWithMember,
        NewApplication,
    },
    role::RoleRepo,
};
use chrono::Local;
use serde::Serialize;
use serde_with::serde_as;
use uuid::Uuid;

use super::{
    audit_log_service::AuditLogService,
    errors::{ServiceError as E, ServiceResult},
    role_service::RoleService,
};

const VALID_STATUSES: [&str; 4] = ["unpaid", "pending", "approved", "rejected"];

pub struct ApplicationService {
    pub repo: ApplicationRepo,
    pub role_service: RoleService,
    pub audit_log: AuditLogService,
}

impl ApplicationService {
    pub fn new(repo: ApplicationRepo, role_service: RoleService, audit_log: AuditLogService) -> Self {
        Self { repo, role_service, audit_log }
    }

    pub async fn create_application(
        &self,
        new_application: NewApplication,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<Application> {
        let cloned_application = new_application.clone();
        let old_application = self
            .repo
            .fetch_existing(
                cloned_application.user_id,
                cloned_application.role_name,
                cloned_application.valid_until,
            )
            .await;

        if let Ok(application) = old_application {
            return Err(E::AlreadyExists);
        }

        let mut status = "pending".to_string();

        let targetable_role = self
            .repo
            .fetch_targetable_role(
                new_application.role_name.clone(),
                new_application.valid_until,
            )
            .await;

        if let Ok(role) = targetable_role {
            if !role.active {
                return Err(E::NotActive);
            }
            tracing::debug!(
                "Targetable role found: {:?}, payment link: {:?}",
                role.role_name, role.payment_link
            );
            if role.payment_link.is_some() && new_application.stripe_payment_id.is_none() {
                status = "unpaid".to_string();
            }
        }

        let application = self
            .repo
            .create(
                new_application.user_id,
                new_application.role_name,
                new_application.valid_until,
                new_application.application_text,
                new_application.optional_roles,
                status.clone(),
            )
            .await
            .map_err(|e| -> E { e.into() })?;

        self.audit_log.log(
            actor_user_id,
            "application.create",
            "application",
            &application.application_id.to_string(),
            Some(serde_json::json!({
                "role_name": application.role_name,
                "status": status,
            })),
        ).await;

        Ok(application)
    }

    pub async fn get_all_applications(&self) -> ServiceResult<Vec<Application>> {
        self.repo.fetch_all().await.map_err(|e| e.into())
    }

    pub async fn get_application(&self, application_id: Uuid) -> ServiceResult<Application> {
        self.repo
            .fetch_one(application_id)
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_application_with_member(
        &self,
        application_id: Uuid,
    ) -> ServiceResult<ApplicationWithMember> {
        self.repo
            .fetch_with_member_one(application_id)
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_applications_with_member_filtered(
        &self,
        status: Option<String>,
        search: Option<String>,
    ) -> ServiceResult<Vec<ApplicationWithMember>> {
        self.repo
            .fetch_with_user_filtered(status, search)
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_applications_for_user(&self, user_id: Uuid) -> ServiceResult<Vec<Application>> {
        self.repo
            .fetch_applications_for_user(user_id)
            .await
            .map_err(|e| e.into())
    }

    pub async fn update_application_status(
        &self,
        application_id: Uuid,
        status: String,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        if !VALID_STATUSES.contains(&status.as_str()) {
            return Err(E::InvalidStatus);
        }

        let application = self.get_application(application_id).await?;
        let old_status = application.status.clone();

        if let Some(ref app_status) = application.status {
            if app_status != "pending" && app_status != "unpaid" {
                return Err(E::ApplicationAlreadyProcessed);
            }
        }

        self.repo.update_status(application_id, status.clone()).await?;

        let today = Local::now().naive_local().date();

        self.role_service
            .add_role_member(
                application.user_id,
                application.role_name.as_str(),
                today,
                Some(application.valid_until),
                actor_user_id,
            )
            .await?;

        self.audit_log.log(
            actor_user_id,
            "application.update_status",
            "application",
            &application_id.to_string(),
            Some(serde_json::json!({
                "old_status": old_status,
                "new_status": status,
            })),
        ).await;

        Ok(())
    }

    pub async fn delete_application(&self, application_id: Uuid, actor_user_id: Option<Uuid>) -> ServiceResult<()> {
        self.repo.delete(application_id).await.map_err(|e| -> E { e.into() })?;

        self.audit_log.log(
            actor_user_id,
            "application.delete",
            "application",
            &application_id.to_string(),
            None,
        ).await;

        Ok(())
    }

    pub async fn create_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
        payment_link: Option<String>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.repo
            .create_targetable_role(role_name.clone(), valid_until, active, payment_link)
            .await
            .map_err(|e| -> E { e.into() })?;

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
        self.repo
            .fetch_all_targetable_roles()
            .await
            .map_err(|e| e.into())
    }

    pub async fn update_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.repo
            .update_targetable_role(role_name.clone(), valid_until, active)
            .await
            .map_err(|e| -> E { e.into() })?;

        self.audit_log.log(
            actor_user_id,
            "targetable_role.update",
            "targetable_role",
            &format!("{}:{}", role_name, valid_until),
            Some(serde_json::json!({ "role_name": role_name, "active": active })),
        ).await;

        Ok(())
    }

    pub async fn get_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> ServiceResult<ApplicationTargetableRole> {
        self.repo
            .fetch_targetable_role(role_name, valid_until)
            .await
            .map_err(|e| e.into())
    }

    pub async fn delete_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.repo
            .delete_targetable_role(role_name.clone(), valid_until)
            .await
            .map_err(|e| -> E { e.into() })?;

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
        self.repo
            .update_payment_id(application_id, stripe_payment_id.clone())
            .await
            .map_err(|e| -> E { e.into() })?;

        self.audit_log.log(
            None,
            "application.payment_received",
            "application",
            &application_id.to_string(),
            Some(serde_json::json!({ "stripe_payment_id": stripe_payment_id })),
        ).await;

        Ok(())
    }
}
