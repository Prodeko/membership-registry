use uuid::Uuid;

use crate::repositories::saved_filter::{NewSavedFilter, SavedFilter, SavedFilterRepo};

use super::errors::ServiceResult;

pub struct SavedFilterService {
    pub repo: SavedFilterRepo,
}

impl SavedFilterService {
    pub fn new(repo: SavedFilterRepo) -> Self {
        Self { repo }
    }

    pub async fn create(
        &self,
        saved_filter: NewSavedFilter,
        current_user_id: Uuid,
    ) -> ServiceResult<SavedFilter> {
        self.repo.create(saved_filter, current_user_id).await.map_err(|e| e.into())
    }

    pub async fn fetch_all_for_model(&self, current_user_id:  Uuid, model: Option<String>) -> ServiceResult<Vec<SavedFilter>> {
        self.repo
            .fetch_all(current_user_id, model)
            .await
            .map_err(|e| e.into())
    }

    pub async fn delete(
        &self,
        saved_filter_name: &str,
        current_user_id:  Uuid,
    ) -> ServiceResult<()> {
        self.repo
            .delete(saved_filter_name, current_user_id)
            .await
            .map_err(|e| e.into())
    }
}
