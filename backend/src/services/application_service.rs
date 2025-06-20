// src/application_service.rs

use crate::repositories::application::{
    Application, ApplicationRepo, ApplicationTargetableRole, ApplicationWithMember, NewApplication,
};
use serde::Serialize;
use serde_with::serde_as;
use uuid::Uuid;

use super::errors::{ServiceError as E, ServiceResult};

const VALID_STATUSES: [&str; 3] = ["pending", "approved", "rejected"];

pub struct ApplicationService {
    pub repo: ApplicationRepo,
}

impl ApplicationService {
    pub fn new(repo: ApplicationRepo) -> Self {
        Self { repo }
    }

    pub async fn create_application(
        &self,
        new_application: NewApplication,
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

        let result = self
            .repo
            .create(
                new_application.user_id,
                new_application.role_name,
                new_application.valid_until,
                new_application.application_text,
                new_application.optional_roles,
            )
            .await;

        result.map_err(|e| e.into())
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

    pub async fn update_application_status(
        &self,
        application_id: Uuid,
        status: String,
    ) -> ServiceResult<()> {
        if !VALID_STATUSES.contains(&status.as_str()) {
            return Err(E::InvalidStatus);
        }

        let application = self.get_application(application_id).await?;

        if let Some(app_status) = application.status {
            if app_status != "pending" {
                return Err(E::ApplicationAlreadyProcessed);
            }
        }

        self.repo
            .update_status(application_id, status)
            .await
            .map_err(|e| e.into())
    }

    pub async fn delete_application(&self, application_id: Uuid) -> ServiceResult<()> {
        self.repo.delete(application_id).await.map_err(|e| e.into())
    }

    pub async fn create_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
        payment_link: Option<String>,
    ) -> ServiceResult<()> {
        self.repo
            .create_targetable_role(role_name, valid_until, active, payment_link)
            .await
            .map_err(|e| e.into())
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
    ) -> ServiceResult<()> {
        self.repo
            .update_targetable_role(role_name, valid_until, active)
            .await
            .map_err(|e| e.into())
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
    ) -> ServiceResult<()> {
        self.repo
            .delete_targetable_role(role_name, valid_until)
            .await
            .map_err(|e| e.into())
    }

    pub async fn update_payment_id(
        &self,
        application_id: Uuid,
        stripe_payment_id: String,
    ) -> ServiceResult<()> {
        self.repo
            .update_payment_id(application_id, stripe_payment_id)
            .await
            .map_err(|e| e.into())
    }

}
