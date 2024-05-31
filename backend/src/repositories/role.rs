use serde::Serialize;
use sqlx::PgPool;

#[derive(Clone)]
pub struct RoleRepo {
    pub pool: PgPool,
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Role {
    pub name: String,
}

impl RoleRepo {
    pub async fn create_role(&self, role_name: &str) -> Result<Role, sqlx::Error> {
        let created = sqlx::query_as::<_, Role>("INSERT INTO Role (name) VALUES ($1) RETURNING name")
            .bind(role_name)
            .fetch_one(&self.pool)
            .await?;
        Ok(created)
    }

    pub async fn fetch_all(&self) -> Result<Vec<Role>, sqlx::Error> {
        let roles = sqlx::query_as::<_, Role>("SELECT name FROM Role")
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .collect();
        Ok(roles)
    }

    pub async fn delete_role(&self, role_name: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM Role WHERE name = $1")
            .bind(role_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
