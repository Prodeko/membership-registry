use serde::{Deserialize, Serialize};
use sqlx::{types::chrono, PgPool};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct NewApplication {
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_until: chrono::NaiveDate,
    pub stripe_payment_id: Option<String>,
    pub application_text: Option<String>,
}


#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Application {
    pub application_id: Uuid,
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_until: chrono::NaiveDate,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stripe_payment_id: Option<String>,
    pub application_text: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct ApplicationTargetableRole {
    pub role_name: String,
    pub valid_until: chrono::NaiveDate,
    pub active: bool,
}

#[derive(Clone)]
pub struct ApplicationRepo {
    pub pool: PgPool,
}

impl ApplicationRepo {
    pub async fn create(
        &self,
        application_to_add: NewApplication,
    ) -> Result<Application, sqlx::Error> {
        let application_created = sqlx::query_as!(
            Application,
            r#"
            INSERT INTO Application (user_id, role_name, valid_until, stripe_payment_id, application_text, timestamp, status)
            VALUES ($1, $2, $3, $4, $5, now(), 'pending')
            RETURNING *
            "#,
          application_to_add.user_id,
          application_to_add.role_name,
          application_to_add.valid_until,
          application_to_add.stripe_payment_id,
          application_to_add.application_text
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(application_created)
    }

    pub async fn fetch_all(&self) -> Result<Vec<Application>, sqlx::Error> {
        let applications = sqlx::query_as!(Application, "SELECT * FROM Application")
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

    pub async fn delete(&self, application_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM Application WHERE application_id = $1",
            application_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
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

    pub async fn create_targetable_role(
        &self,
        role_name: String,
        valid_until: chrono::NaiveDate,
        active: Option<bool>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO ApplicationTargetableRole (role_name, valid_until, active) VALUES ($1, $2, $3)",
            role_name,
            valid_until,
            active
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
}
