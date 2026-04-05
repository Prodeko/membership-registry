use sqlx::PgPool;

use crate::application::ports::marketing_tag_repository_port::MarketingTagRepositoryPort;
use crate::application::ports::repository_error::RepositoryError;
use crate::domain::MarketingTag;

#[derive(Debug, sqlx::FromRow)]
struct MarketingTagDAO {
    pub label: String,
    pub name_en: String,
    pub name_fi: String,
    pub desc_en: String,
    pub desc_fi: String,
    pub display_order: i32,
    pub auto_apply: bool,
}

impl From<MarketingTagDAO> for MarketingTag {
    fn from(dao: MarketingTagDAO) -> Self {
        Self {
            label: dao.label,
            name_en: dao.name_en,
            name_fi: dao.name_fi,
            desc_en: dao.desc_en,
            desc_fi: dao.desc_fi,
            display_order: dao.display_order,
            auto_apply: dao.auto_apply,
        }
    }
}

#[derive(Clone)]
pub struct MarketingTagRepo {
    pub pool: PgPool,
}

#[async_trait::async_trait]
impl MarketingTagRepositoryPort for MarketingTagRepo {
    async fn fetch_all(&self) -> Result<Vec<MarketingTag>, RepositoryError> {
        let rows = sqlx::query_as!(
            MarketingTagDAO,
            r#"
            SELECT label, name_en, name_fi, desc_en, desc_fi, display_order, auto_apply
            FROM MarketingTag
            ORDER BY display_order ASC, label ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn create(&self, tag: &MarketingTag) -> Result<MarketingTag, RepositoryError> {
        let row = sqlx::query_as!(
            MarketingTagDAO,
            r#"
            INSERT INTO MarketingTag (label, name_en, name_fi, desc_en, desc_fi, display_order, auto_apply)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING label, name_en, name_fi, desc_en, desc_fi, display_order, auto_apply
            "#,
            tag.label,
            tag.name_en,
            tag.name_fi,
            tag.desc_en,
            tag.desc_fi,
            tag.display_order,
            tag.auto_apply,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn update(&self, tag: &MarketingTag) -> Result<MarketingTag, RepositoryError> {
        let row = sqlx::query_as!(
            MarketingTagDAO,
            r#"
            UPDATE MarketingTag
            SET name_en = $2,
                name_fi = $3,
                desc_en = $4,
                desc_fi = $5,
                display_order = $6,
                auto_apply = $7
            WHERE label = $1
            RETURNING label, name_en, name_fi, desc_en, desc_fi, display_order, auto_apply
            "#,
            tag.label,
            tag.name_en,
            tag.name_fi,
            tag.desc_en,
            tag.desc_fi,
            tag.display_order,
            tag.auto_apply,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn delete(&self, label: &str) -> Result<(), RepositoryError> {
        let result = sqlx::query!("DELETE FROM MarketingTag WHERE label = $1", label)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(RepositoryError::NotFound);
        }
        Ok(())
    }
}
