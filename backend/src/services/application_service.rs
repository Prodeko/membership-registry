// src/application_service.rs

use crate::repositories::application::{
    self, Application, ApplicationRepo, ApplicationTargetableRole, NewApplication,
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
        self.repo
            .create(new_application)
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

    pub async fn update_application_status(
        &self,
        application_id: Uuid,
        status: String,
    ) -> Result<(), String> {
        if !VALID_STATUSES.contains(&status.as_str()) {
            return Err(format!("Invalid status: {}", status));
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
}
