use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct UserAuthProvider {
    pub user_id: Uuid,
    pub provider_name: String,
    pub provider_user_id: String,
    pub linked_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct UserAuthProviderRepo {
    pub pool: PgPool,
}

impl UserAuthProviderRepo {
    pub async fn find_by_provider(
        &self,
        provider_name: &str,
        provider_user_id: &str,
    ) -> Result<Option<UserAuthProvider>, sqlx::Error> {
        sqlx::query_as::<_, UserAuthProvider>(
            "SELECT user_id, provider_name, provider_user_id, linked_at, metadata
             FROM UserAuthProvider
             WHERE provider_name = $1 AND provider_user_id = $2",
        )
        .bind(provider_name)
        .bind(provider_user_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<UserAuthProvider>, sqlx::Error> {
        sqlx::query_as::<_, UserAuthProvider>(
            "SELECT user_id, provider_name, provider_user_id, linked_at, metadata
             FROM UserAuthProvider
             WHERE user_id = $1
             ORDER BY linked_at ASC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn create(
        &self,
        user_id: &Uuid,
        provider_name: &str,
        provider_user_id: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<UserAuthProvider, sqlx::Error> {
        sqlx::query_as::<_, UserAuthProvider>(
            "INSERT INTO UserAuthProvider (user_id, provider_name, provider_user_id, metadata)
             VALUES ($1, $2, $3, $4)
             RETURNING user_id, provider_name, provider_user_id, linked_at, metadata",
        )
        .bind(user_id)
        .bind(provider_name)
        .bind(provider_user_id)
        .bind(metadata)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn delete(
        &self,
        user_id: &Uuid,
        provider_name: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM UserAuthProvider
             WHERE user_id = $1 AND provider_name = $2",
        )
        .bind(user_id)
        .bind(provider_name)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn exists(
        &self,
        user_id: &Uuid,
        provider_name: &str,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM UserAuthProvider WHERE user_id = $1 AND provider_name = $2)",
        )
        .bind(user_id)
        .bind(provider_name)
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }
}
