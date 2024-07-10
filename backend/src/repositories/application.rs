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
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_until: chrono::NaiveDate,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stripe_payment_id: Option<String>,
    pub application_text: Option<String>,
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
            INSERT INTO Application (user_id, role_name, valid_until, stripe_payment_id, application_text, timestamp)
            VALUES ($1, $2, $3, $4, $5, now())
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

    pub async fn fetch_one(
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

    pub async fn update(
        &self,
        item: Application,
        user_id: Uuid,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> Result<Application, sqlx::Error> {
        let application_updated = sqlx::query_as!(
            Application,
            r#"
            UPDATE Application
            SET 
                stripe_payment_id = $1,
                application_text = $2,
                timestamp = now()
            WHERE user_id = $3 AND role_name = $4 AND valid_until = $5
            RETURNING *
            "#,
            item.stripe_payment_id,
            item.application_text,
            user_id,
            role_name,
            valid_until
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(application_updated)
    }

    pub async fn delete(
        &self,
        user_id: Uuid,
        role_name: String,
        valid_until: chrono::NaiveDate,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM Application WHERE user_id = $1 AND role_name = $2 AND valid_until = $3",
            user_id,
            role_name,
            valid_until
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
