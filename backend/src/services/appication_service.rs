// src/application_service.rs

use crate::repositories::application::{Application, ApplicationRepo, NewApplication};
use uuid::Uuid;

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

    pub async fn get_application(
        &self,
        user_id: Uuid,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> Result<Application, String> {
        self.repo
            .fetch_one(user_id, role_name, valid_until)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn update_application(
        &self,
        updated_application: Application,
        user_id: Uuid,
        role_name: String,
        valid_until: chrono::NaiveDate
    ) -> Result<Application, String> {
        self.repo
            .update(updated_application, user_id, role_name, valid_until)
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn delete_application(&self, user_id: Uuid, role_name: String, valid_until: chrono::NaiveDate) -> Result<(), String> {
        self.repo
            .delete(user_id, role_name, valid_until)
            .await
            .map_err(|e| e.to_string())
    }
}
