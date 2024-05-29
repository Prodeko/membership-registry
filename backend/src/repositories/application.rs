use serde::{Deserialize, Serialize};
use sqlx::{types::chrono, PgPool};


#[derive(Deserialize, Debug)]
pub struct NewApplication {
    pub user_id: String,
    pub role_name: String,
    pub stripe_payment_id: Option<String>,
    pub application_text: Option<String>,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Application {
    pub user_id: String,
    pub role_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub stripe_payment_id: Option<String>,
    pub application_text: Option<String>,
}

#[derive(Clone)]
pub struct ApplicationRepo {
    pub pool: PgPool,
}

impl ApplicationRepo {
    pub async fn create(&self, application_to_add: NewApplication) -> Result<Application, sqlx::Error> {
        let application_created = sqlx::query_as::<_, Application>(
            r#"
            INSERT INTO Application (user_id, role_name, stripe_payment_id, application_text, timestamp)
            VALUES (CAST($1 AS UUID), $2, $3, $4, now())
            RETURNING *, CAST(user_id AS TEXT) as user_id
            "#,
        )
        .bind(application_to_add.user_id)
        .bind(application_to_add.role_name)
        .bind(application_to_add.stripe_payment_id)
        .bind(application_to_add.application_text)
        .fetch_one(&self.pool)
        .await?;
        Ok(application_created)
    }

    pub async fn fetch_all(&self) -> Result<Vec<Application>, sqlx::Error> {
        let applications = sqlx::query_as::<_, Application>(
            "SELECT *, CAST(user_id AS TEXT) as user_id FROM Application"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(applications)
    }

    pub async fn fetch_one(&self, user_id: String, role_name: String) -> Result<Application, sqlx::Error> {
        let application = sqlx::query_as::<_, Application>(
            "SELECT * FROM Application WHERE user_id = $1 AND role_name = $2"
        )
        .bind(user_id)
        .bind(role_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(application)
    }

    pub async fn update(&self, item: Application, user_id: String, role_name: String) -> Result<Application, sqlx::Error> {
        let application_updated = sqlx::query_as::<_, Application>(
            r#"
            UPDATE Application
            SET 
                stripe_payment_id = $1,
                application_text = $2,
                timestamp = now()
            WHERE user_id = $3 AND role_name = $4
            RETURNING *, CAST(user_id AS TEXT) as user_id
            "#,
        )
        .bind(item.stripe_payment_id)
        .bind(item.application_text)
        .bind(user_id)
        .bind(role_name)
        .fetch_one(&self.pool)
        .await?;
        Ok(application_updated)
    }

    pub async fn delete(&self, user_id: String, role_name: String) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM Application WHERE user_id = $1 AND role_name = $2")
            .bind(user_id)
            .bind(role_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
