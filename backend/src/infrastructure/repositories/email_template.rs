use sqlx::PgPool;

use crate::application::ports::repository_error::RepositoryError;
use crate::application::ports::template_repository_port::TemplateRepositoryPort;
use crate::domain::{EmailTemplate, EmailTemplateTranslation};

#[derive(Debug, sqlx::FromRow)]
struct EmailTemplateDAO {
    pub name: String,
}

impl From<EmailTemplateDAO> for EmailTemplate {
    fn from(dao: EmailTemplateDAO) -> Self {
        Self { name: dao.name }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct EmailTemplateTranslationDAO {
    pub template_name: String,
    pub locale: String,
    pub subject: String,
    pub body_html: String,
}

impl From<EmailTemplateTranslationDAO> for EmailTemplateTranslation {
    fn from(dao: EmailTemplateTranslationDAO) -> Self {
        Self {
            template_name: dao.template_name,
            locale: dao.locale,
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
        let templates = sqlx::query_as!(EmailTemplateDAO, "SELECT name FROM EmailTemplate")
            .fetch_all(&self.pool)
            .await?;
        Ok(templates.into_iter().map(Into::into).collect())
    }

    async fn create(&self, name: &str) -> Result<EmailTemplate, RepositoryError> {
        let template = sqlx::query_as!(
            EmailTemplateDAO,
            r#"
            INSERT INTO EmailTemplate (name)
            VALUES ($1)
            RETURNING name
            "#,
            name
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

    async fn fetch_translation(
        &self,
        name: &str,
        locale: &str,
    ) -> Result<EmailTemplateTranslation, RepositoryError> {
        // Try requested locale first, fall back to 'fi'
        let translation = sqlx::query_as!(
            EmailTemplateTranslationDAO,
            r#"
            SELECT template_name, locale, subject, body_html
            FROM EmailTemplateTranslation
            WHERE template_name = $1 AND locale = $2
            "#,
            name,
            locale
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(t) = translation {
            return Ok(t.into());
        }

        // Fallback to Finnish
        let fallback = sqlx::query_as!(
            EmailTemplateTranslationDAO,
            r#"
            SELECT template_name, locale, subject, body_html
            FROM EmailTemplateTranslation
            WHERE template_name = $1 AND locale = 'fi'
            "#,
            name
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(fallback.into())
    }

    async fn fetch_translations(
        &self,
        name: &str,
    ) -> Result<Vec<EmailTemplateTranslation>, RepositoryError> {
        let translations = sqlx::query_as!(
            EmailTemplateTranslationDAO,
            r#"
            SELECT template_name, locale, subject, body_html
            FROM EmailTemplateTranslation
            WHERE template_name = $1
            ORDER BY locale
            "#,
            name
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(translations.into_iter().map(Into::into).collect())
    }

    async fn upsert_translation(
        &self,
        template_name: &str,
        locale: &str,
        subject: &str,
        body_html: &str,
    ) -> Result<EmailTemplateTranslation, RepositoryError> {
        let translation = sqlx::query_as!(
            EmailTemplateTranslationDAO,
            r#"
            INSERT INTO EmailTemplateTranslation (template_name, locale, subject, body_html)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (template_name, locale) DO UPDATE
            SET subject = EXCLUDED.subject, body_html = EXCLUDED.body_html
            RETURNING template_name, locale, subject, body_html
            "#,
            template_name,
            locale,
            subject,
            body_html
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(translation.into())
    }

    async fn delete_translation(
        &self,
        template_name: &str,
        locale: &str,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            "DELETE FROM EmailTemplateTranslation WHERE template_name = $1 AND locale = $2",
            template_name,
            locale
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
