use serde_json::Value;
use uuid::Uuid;

use super::repository_error::RepositoryError;

// --- Read models ---

pub struct SavedFilter {
    pub name: String,
    pub filtered_model: String,
    pub owner_user_id: Uuid,
    pub visible_for_all: Option<bool>,
    pub search: Option<String>,
    pub sorting_col: Option<String>,
    pub sorting_desc: bool,
    pub custom_filters: Option<Value>,
}

pub struct NewSavedFilter {
    pub name: String,
    pub filtered_model: String,
    pub visible_for_all: bool,
    pub search: Option<String>,
    pub sorting_col: Option<String>,
    pub sorting_desc: bool,
    pub custom_filters: Option<Value>,
}

#[async_trait::async_trait]
pub trait SavedFilterRepositoryPort: Send + Sync {
    async fn create(
        &self,
        filter: NewSavedFilter,
        owner_user_id: Uuid,
    ) -> Result<SavedFilter, RepositoryError>;
    async fn fetch_all(
        &self,
        current_user_id: Uuid,
        filtered_model: Option<String>,
    ) -> Result<Vec<SavedFilter>, RepositoryError>;
    async fn delete(&self, name: &str, owner_user_id: Uuid) -> Result<(), RepositoryError>;
}
