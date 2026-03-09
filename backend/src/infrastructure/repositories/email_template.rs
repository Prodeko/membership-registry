use sqlx::PgPool;

use crate::application::ports::repository_error::RepositoryError;
use crate::application::ports::template_repository_port::TemplateRepositoryPort;
use crate::domain::EmailTemplate;

#[derive(Debug, sqlx::FromRow)]
struct EmailTemplateDAO {
    pub name: String,
    pub subject: String,
    pub body_html: String,
}

impl From<EmailTemplateDAO> for EmailTemplate {
    fn from(dao: EmailTemplateDAO) -> Self {
        Self {
            name: dao.name,
            subject: dao.subject,
            body_html: dao.body_html,
        }
    }
}

#[derive(Clone)]
pub struct EmailTemplateRepo {
    pub pool: PgPool,
}

#[async_trait::async_trait]
impl TemplateRepositoryPort for EmailTemplateRepo {
    async fn fetch_all(&self) -> Result<Vec<EmailTemplate>, RepositoryError> {
        let templates = sqlx::query_as!(EmailTemplateDAO, "SELECT * FROM EmailTemplate")
            .fetch_all(&self.pool)
            .await?;
        Ok(templates.into_iter().map(Into::into).collect())
    }

    async fn fetch_one(&self, name: &str) -> Result<EmailTemplate, RepositoryError> {
        let template = sqlx::query_as!(
            EmailTemplateDAO,
            "SELECT * FROM EmailTemplate WHERE name = $1",
            name
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(template.into())
    }

    async fn create(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> Result<EmailTemplate, RepositoryError> {
        let template = sqlx::query_as!(
            EmailTemplateDAO,
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
        Ok(template.into())
    }

    async fn update(
        &self,
        name: &str,
        subject: &str,
        body_html: &str,
    ) -> Result<EmailTemplate, RepositoryError> {
        let template = sqlx::query_as!(
            EmailTemplateDAO,
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
        Ok(template.into())
    }

    async fn delete(&self, name: &str) -> Result<(), RepositoryError> {
        sqlx::query!("DELETE FROM EmailTemplate WHERE name = $1", name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
