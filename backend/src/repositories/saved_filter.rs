use super::member::Member;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::{types::Json, PgPool};
use uuid::Uuid;

#[derive(Clone)]
pub struct SavedFilterRepo {
    pub pool: PgPool,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct SavedFilter {
    pub name: String,
    pub model: String,
    pub owner_user_id: Uuid,
    pub visible_to_all: bool,
    pub search: String,
    pub sorting_col: String,
    pub sorting_desc: bool,
    pub custom_filters: serde_json::Value,
}

#[derive(Deserialize)]
pub struct NewSavedFilter {
    pub name: String,
    pub model: String,
    pub visible_to_all: Option<bool>,
    pub search: Option<String>,
    pub sorting_col: Option<String>,
    pub sorting_desc: bool,
    pub custom_filters: Option<serde_json::Value>,
}

impl SavedFilterRepo {
    pub async fn create(
        &self,
        saved_filter: NewSavedFilter,
        owner_user_id: Uuid,
    ) -> Result<SavedFilter, sqlx::Error> {
        let created =
            sqlx::query_as::<_, SavedFilter>(r#"
                INSERT INTO 
                    SavedFilter (name, model, owner_user_id, visible_to_all, search, sorting_col, sorting_desc, custom_filters)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#)
            .bind(&saved_filter.name)
            .bind(&saved_filter.model)
            .bind(owner_user_id)
            .bind(saved_filter.visible_to_all)
            .bind(saved_filter.search)
            .bind(saved_filter.sorting_col)
            .bind(saved_filter.sorting_desc)
            .bind(&saved_filter.custom_filters)
            .fetch_one(&self.pool)
            .await?;

        Ok(created)
    }

    pub async fn fetch_all(
        &self,
        current_user_id: Uuid,
        model: Option<String>,
    ) -> Result<Vec<SavedFilter>, sqlx::Error> {
        let saved_filters = sqlx::query_as::<_, SavedFilter>(r#"
            SELECT * 
            FROM SavedFilter 
            WHERE 
                (owner_user_id = $1 OR visible_to_all = true) AND
                ($2::text IS NULL OR model = $2)
            "#,
        )
        .bind(current_user_id)
        .bind(model)
        .fetch_all(&self.pool)
        .await?;
        Ok(saved_filters)
    }

    pub async fn delete(
        &self,
        saved_filter_name: &str,
        current_user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM SavedFilter WHERE name = $1 AND owner_user_id = $2")
            .bind(saved_filter_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
