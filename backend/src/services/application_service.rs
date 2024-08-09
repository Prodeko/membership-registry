// src/application_service.rs

use std::clone;

use crate::repositories::application::{
    self, Application, ApplicationRepo, ApplicationTargetableRole, ApplicationWithMember, NewApplication
};
use uuid::Uuid;

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
    ) -> Result<Application, String> {
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
            return Err(format!(
                "Application already exists with id: {}",
                application.application_id
            ));
        }

        self.repo
            .create(
                new_application.user_id,
                new_application.role_name,
                new_application.valid_until,
                new_application.application_text,
            )
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_all_applications(&self) -> Result<Vec<Application>, String> {
        self.repo.fetch_all().await.map_err(|e| e.to_string())
    }

    pub async fn get_application(&self, application_id: Uuid) -> Result<Application, String> {
        self.repo
            .fetch_one(application_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_applications_with_member_filtered(
        &self,
        status: Option<String>,
        search: Option<String>,
    ) -> Result<Vec<ApplicationWithMember>, String> {
        self.repo
            .fetch_with_user_filtered(status, search)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn update_application_status(
        &self,
        application_id: Uuid,
        status: String,
    ) -> Result<(), String> {
        if !VALID_STATUSES.contains(&status.as_str()) {
            return Err(format!("Invalid status: {}", status));
        }

        let application = self.get_application(application_id).await?;

        if let Some(app_status) = application.status {
            if app_status != "pending" {
                return Err(format!(
                    "Application needs to be in pending status. Current status is: {}",
                    app_status
                ));
            }
        }

        self.repo
            .update_status(application_id, status)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn delete_application(&self, application_id: Uuid) -> Result<(), String> {
        self.repo
            .delete(application_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn create_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
    ) -> Result<(), String> {
        self.repo
            .create_targetable_role(role_name, valid_until, active)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn fetch_all_targetable_roles(
        &self,
    ) -> Result<Vec<ApplicationTargetableRole>, String> {
        self.repo
            .fetch_all_targetable_roles()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn update_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
    ) -> Result<(), String> {
        self.repo
            .update_targetable_role(role_name, valid_until, active)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn update_payment_id(
        &self,
        application_id: Uuid,
        stripe_payment_id: String,
    ) -> Result<(), String> {
        self.repo
            .update_payment_id(application_id, stripe_payment_id)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn get_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> Result<ApplicationTargetableRole, String> {
        self.repo
            .fetch_targetable_role(role_name, valid_until)
            .await
            .map_err(|e| e.to_string())
    }
}
