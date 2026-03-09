use sqlx::PgPool;
use uuid::Uuid;

use crate::application::ports::auth_provider_repo_port::{
    AuthProviderMapping, AuthProviderRepoError, AuthProviderRepositoryPort,
};

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct UserAuthProviderDAO {
    pub user_id: Uuid,
    pub provider_name: String,
    pub provider_user_id: String,
    pub linked_at: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<serde_json::Value>,
}

impl From<UserAuthProviderDAO> for AuthProviderMapping {
    fn from(dao: UserAuthProviderDAO) -> Self {
        Self {
            user_id: dao.user_id,
            provider_name: dao.provider_name,
            provider_user_id: dao.provider_user_id,
            linked_at: dao.linked_at,
        }
    }
}

impl From<sqlx::Error> for AuthProviderRepoError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AuthProviderRepoError::NotFound,
            sqlx::Error::Database(ref db_err) if db_err.constraint().is_some() => {
                AuthProviderRepoError::AlreadyExists
            }
            other => AuthProviderRepoError::Unavailable(other.to_string()),
        }
    }
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
    ) -> Result<Option<UserAuthProviderDAO>, sqlx::Error> {
        sqlx::query_as::<_, UserAuthProviderDAO>(
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
    ) -> Result<Vec<UserAuthProviderDAO>, sqlx::Error> {
        sqlx::query_as::<_, UserAuthProviderDAO>(
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
    ) -> Result<UserAuthProviderDAO, sqlx::Error> {
        sqlx::query_as::<_, UserAuthProviderDAO>(
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

#[async_trait::async_trait]
impl AuthProviderRepositoryPort for UserAuthProviderRepo {
    async fn find_by_provider(
        &self,
        provider_name: &str,
        provider_user_id: &str,
    ) -> Result<Option<AuthProviderMapping>, AuthProviderRepoError> {
        Ok(self
            .find_by_provider(provider_name, provider_user_id)
            .await?
            .map(Into::into))
    }

    async fn find_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<AuthProviderMapping>, AuthProviderRepoError> {
        Ok(self
            .find_by_user_id(user_id)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    async fn create(
        &self,
        user_id: &Uuid,
        provider_name: &str,
        provider_user_id: &str,
    ) -> Result<AuthProviderMapping, AuthProviderRepoError> {
        Ok(self.create(user_id, provider_name, provider_user_id, None).await?.into())
    }

    async fn delete(
        &self,
        user_id: &Uuid,
        provider_name: &str,
    ) -> Result<bool, AuthProviderRepoError> {
        Ok(self.delete(user_id, provider_name).await?)
    }

    async fn count_by_user_id(
        &self,
        user_id: &Uuid,
    ) -> Result<usize, AuthProviderRepoError> {
        Ok(self.find_by_user_id(user_id).await?.len())
    }
}
