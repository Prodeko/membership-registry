use std::sync::Arc;

use uuid::Uuid;

use crate::application::ports::saved_filter_repository_port::{
    NewSavedFilter, SavedFilter, SavedFilterRepositoryPort,
};

use super::errors::ServiceResult;

pub struct SavedFilterService {
    repo: Arc<dyn SavedFilterRepositoryPort>,
}

impl SavedFilterService {
    pub fn new(repo: Arc<dyn SavedFilterRepositoryPort>) -> Self {
        Self { repo }
    }

    pub async fn create(
        &self,
        saved_filter: NewSavedFilter,
        current_user_id: Uuid,
    ) -> ServiceResult<SavedFilter> {
        self.repo
            .create(saved_filter, current_user_id)
            .await
            .map_err(|e| e.into())
    }

    pub async fn fetch_all_for_model(
        &self,
        current_user_id: Uuid,
        model: Option<String>,
    ) -> ServiceResult<Vec<SavedFilter>> {
        self.repo
            .fetch_all(current_user_id, model)
            .await
            .map_err(|e| e.into())
    }

    pub async fn delete(
        &self,
        saved_filter_name: &str,
        current_user_id: Uuid,
    ) -> ServiceResult<()> {
        self.repo
            .delete(saved_filter_name, current_user_id)
            .await
            .map_err(|e| e.into())
    }
}
