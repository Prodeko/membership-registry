use sqlx::PgPool;
use uuid::Uuid;

use crate::application::ports::repository_error::RepositoryError;
use crate::application::ports::saved_filter_repository_port::{
    NewSavedFilter, SavedFilter, SavedFilterRepositoryPort,
};

#[derive(Clone)]
pub struct SavedFilterRepo {
    pub pool: PgPool,
}

#[derive(Debug, sqlx::FromRow)]
struct SavedFilterDAO {
    pub name: String,
    pub filtered_model: String,
    pub owner_user_id: Uuid,
    pub visible_for_all: Option<bool>,
    pub search: Option<String>,
    pub sorting_col: Option<String>,
    pub sorting_desc: bool,
    pub custom_filters: Option<serde_json::Value>,
}

impl From<SavedFilterDAO> for SavedFilter {
    fn from(dao: SavedFilterDAO) -> Self {
        Self {
            name: dao.name,
            filtered_model: dao.filtered_model,
            owner_user_id: dao.owner_user_id,
            visible_for_all: dao.visible_for_all,
            search: dao.search,
            sorting_col: dao.sorting_col,
            sorting_desc: dao.sorting_desc,
            custom_filters: dao.custom_filters,
        }
    }
}

#[async_trait::async_trait]
impl SavedFilterRepositoryPort for SavedFilterRepo {
    async fn create(
        &self,
        saved_filter: NewSavedFilter,
        owner_user_id: Uuid,
    ) -> Result<SavedFilter, RepositoryError> {
        let created = sqlx::query_as!(
            SavedFilterDAO,
            r#"
                INSERT INTO
                    SavedFilter (name, filtered_model, owner_user_id, visible_for_all, search, sorting_col, sorting_desc, custom_filters)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING *
            "#,
            &saved_filter.name,
            &saved_filter.filtered_model,
            owner_user_id,
            saved_filter.visible_for_all,
            saved_filter.search,
            saved_filter.sorting_col,
            saved_filter.sorting_desc,
            saved_filter.custom_filters
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(SavedFilter::from(created))
    }

    async fn fetch_all(
        &self,
        current_user_id: Uuid,
        filtered_model: Option<String>,
    ) -> Result<Vec<SavedFilter>, RepositoryError> {
        let saved_filters = sqlx::query_as!(
            SavedFilterDAO,
            r#"
            SELECT *
            FROM SavedFilter
            WHERE
                (owner_user_id = $1 OR visible_for_all = true) AND
                ($2::text IS NULL OR filtered_model = $2)
            "#,
            current_user_id,
            filtered_model
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(saved_filters.into_iter().map(SavedFilter::from).collect())
    }

    async fn delete(
        &self,
        saved_filter_name: &str,
        current_user_id: Uuid,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            "DELETE FROM SavedFilter WHERE name = $1 AND owner_user_id = $2",
            saved_filter_name,
            current_user_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
