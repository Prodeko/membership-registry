use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use super::member::Member;

#[derive(Clone)]
pub struct RoleRepo {
    pub pool: PgPool,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoleMember {
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
}

impl RoleRepo {
    pub async fn create(&self, role_name: &str) -> Result<Role, sqlx::Error> {
        let created =
            sqlx::query_as::<_, Role>("INSERT INTO Role (name) VALUES ($1) RETURNING name")
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

    pub async fn fetch_by_name(&self, role_name: &str) -> Result<Role, sqlx::Error> {
        let role = sqlx::query_as::<_, Role>("SELECT name FROM Role WHERE name = $1")
            .bind(role_name)
            .fetch_one(&self.pool)
            .await?;
        Ok(role)
    }

    pub async fn delete(&self, role_name: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM Role WHERE name = $1")
            .bind(role_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn create_role_member(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: NaiveDate,
        valid_until: Option<NaiveDate>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
          INSERT INTO RoleMember (user_id, role_name, valid_from, valid_until)
          VALUES ($1, $2, $3, $4)
          "#,
            user_id,
            role_name,
            valid_from,
            valid_until
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_valid_until(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: NaiveDate,
        new_valid_until: NaiveDate,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
          UPDATE RoleMember
          SET valid_until = $4
          WHERE user_id = $1 AND role_name = $2 AND valid_from = $3
          "#,
            user_id,
            role_name,
            valid_from,
            new_valid_until
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_role_member(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: NaiveDate,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"
          DELETE FROM RoleMember
          WHERE user_id = $1 AND role_name = $2 AND valid_from = $3
          "#,
            user_id,
            role_name,
            valid_from
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn fetch_roles_by_member(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<RoleMember>, sqlx::Error> {
        let records = sqlx::query_as!(
            RoleMember,
            r#"
          SELECT user_id, role_name, valid_from, valid_until
          FROM RoleMember
          WHERE user_id = $1
          ORDER BY valid_from DESC
          "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    pub async fn fetch_members_by_role(&self, role_name: &str) -> Result<Vec<Member>, sqlx::Error> {
        let records = sqlx::query_as!(
            Member,
            r#"
          SELECT Member.user_id, first_name, last_name, full_name, home_municipality, has_accepted_policies
          FROM RoleMember JOIN Member ON RoleMember.user_id = Member.user_id
          WHERE role_name = $1
          "#,
            role_name
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }
}
