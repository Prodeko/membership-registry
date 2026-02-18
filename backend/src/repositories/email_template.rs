use serde::Serialize;
use sqlx::PgPool;
use ts_rs::TS;

#[derive(Debug, sqlx::FromRow, Serialize, TS)]
#[ts(export)]
pub struct EmailTemplate {
    pub name: String,
    pub subject: String,
    pub body_html: String,
}

#[derive(Clone)]
pub struct EmailTemplateRepo {
    pub pool: PgPool,
}

impl EmailTemplateRepo {
    pub async fn fetch_all(&self) -> Result<Vec<EmailTemplate>, sqlx::Error> {
        let templates = sqlx::query_as!(EmailTemplate, "SELECT * FROM EmailTemplate")
            .fetch_all(&self.pool)
            .await?;
        Ok(templates)
    }

    pub async fn fetch_one(&self, name: &str) -> Result<EmailTemplate, sqlx::Error> {
        let template = sqlx::query_as!(
            EmailTemplate,
            "SELECT * FROM EmailTemplate WHERE name = $1",
            name
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(template)
    }

    pub async fn create(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> Result<EmailTemplate, sqlx::Error> {
        let template = sqlx::query_as!(
            EmailTemplate,
            r#"
            INSERT INTO EmailTemplate (name, subject, body_html)
            VALUES ($1, $2, $3)
            RETURNING *
            "#,
            name,
            subject,
            body_html
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(template)
    }

    pub async fn update(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> Result<EmailTemplate, sqlx::Error> {
        let template = sqlx::query_as!(
            EmailTemplate,
            r#"
            UPDATE EmailTemplate SET subject = $2, body_html = $3
            WHERE name = $1
            RETURNING *
            "#,
            name,
            subject,
            body_html
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(template)
    }

    pub async fn delete(&self, name: &str) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM EmailTemplate WHERE name = $1", name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
