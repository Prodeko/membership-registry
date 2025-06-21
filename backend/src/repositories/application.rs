use serde::{Deserialize, Serialize};
use sqlx::{types::chrono, PgPool};
use uuid::Uuid;

#[derive(Deserialize, Debug, Clone)]
pub struct NewApplication {
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_until: chrono::NaiveDate,
    pub stripe_payment_id: Option<String>,
    pub application_text: Option<String>,
    pub optional_roles: Option<Vec<String>>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Application {
    pub application_id: Uuid,
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_until: chrono::NaiveDate,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stripe_payment_id: Option<String>,
    pub optional_roles: Option<Vec<String>>,
    pub application_text: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct ApplicationWithMember {
    pub application_id: Uuid,
    pub user_id: Uuid,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub role_name: String,
    pub valid_until: chrono::NaiveDate,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stripe_payment_id: Option<String>,
    pub optional_roles: Option<Vec<String>>,
    pub application_text: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct ApplicationTargetableRole {
    pub role_name: String,
    pub valid_until: chrono::NaiveDate,
    pub active: bool,
    pub optional_roles: Option<Vec<String>>,
    pub payment_link: Option<String>,
}

#[derive(Clone)]
pub struct ApplicationRepo {
    pub pool: PgPool,
}

impl ApplicationRepo {
    pub async fn create(
        &self,
        user_id: Uuid,
        role_name: String,
        valid_until: chrono::NaiveDate,
        application_text: Option<String>,
        optional_roles: Option<Vec<String>>,
        status: String
    ) -> Result<Application, sqlx::Error> {
        let application_created = sqlx::query_as!(
            Application,
            r#"
            INSERT INTO Application (user_id, role_name, valid_until, application_text, optional_roles, timestamp, status)
            VALUES ($1, $2, $3, $4, $5, now(), $6)
            RETURNING *
            "#,
          user_id,
          role_name,
          valid_until,
          application_text,
          optional_roles.as_deref(),
          status
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(application_created)
    }

    pub async fn fetch_all(&self) -> Result<Vec<Application>, sqlx::Error> {
        let applications = sqlx::query_as!(
            Application,
            "SELECT * FROM Application ORDER BY timestamp DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(applications)
    }

    pub async fn fetch_one(&self, application_id: Uuid) -> Result<Application, sqlx::Error> {
        let application = sqlx::query_as!(
            Application,
            "SELECT * FROM Application WHERE application_id = $1",
            application_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(application)
    }

    pub async fn fetch_with_user_filtered(
        &self,
        status: Option<String>,
        search: Option<String>,
    ) -> Result<Vec<ApplicationWithMember>, sqlx::Error> {
        let applications = sqlx::query_as!(
            ApplicationWithMember,
            r#"
            SELECT a.*, m.full_name, m.email
            FROM Application a
            JOIN Member m ON a.user_id = m.user_id
            WHERE ($1 = '' OR a.status = $1)
            AND ($2 = '' OR m.full_name ILIKE '%' || $2 || '%' OR m.email ILIKE '%' || $2 || '%')
            ORDER BY a.timestamp DESC
            "#,
            status.unwrap_or("".to_string()),
            search.unwrap_or("".to_string())
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(applications)
    }

    pub async fn delete(&self, application_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM Application WHERE application_id = $1",
            application_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn fetch_existing(
        &self,
        user_id: Uuid,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> Result<Application, sqlx::Error> {
        let application = sqlx::query_as!(
            Application,
            "SELECT * FROM Application WHERE user_id = $1 AND role_name = $2 AND valid_until = $3",
            user_id,
            role_name,
            valid_until
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(application)
    }

    pub async fn update_status(
        &self,
        application_id: Uuid,
        status: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE Application SET status = $2 WHERE application_id = $1",
            application_id,
            status
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn fetch_all_targetable_roles(
        &self,
    ) -> Result<Vec<ApplicationTargetableRole>, sqlx::Error> {
        let targetable_roles = sqlx::query_as!(
            ApplicationTargetableRole,
            "SELECT * FROM ApplicationTargetableRole"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(targetable_roles)
    }

    pub async fn fetch_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> Result<ApplicationTargetableRole, sqlx::Error> {
        let targetable_role = sqlx::query_as!(
            ApplicationTargetableRole,
            "SELECT * FROM ApplicationTargetableRole WHERE role_name = $1 AND valid_until = $2",
            role_name,
            valid_until
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(targetable_role)
    }

    pub async fn create_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
        payment_link: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO ApplicationTargetableRole (role_name, valid_until, active, payment_link) VALUES ($1, $2, $3, $4)",
            role_name,
            valid_until,
            active,
            payment_link
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE ApplicationTargetableRole SET active = $3 WHERE role_name = $1 AND valid_until = $2",
            role_name,
            valid_until,
            active
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM ApplicationTargetableRole WHERE role_name = $1 AND valid_until = $2",
            role_name,
            valid_until
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_payment_id(
        &self,
        application_id: Uuid,
        stripe_payment_id: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE Application SET stripe_payment_id = $2 WHERE application_id = $1",
            application_id,
            stripe_payment_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
