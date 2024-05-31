// role_member_repo.rs

use sqlx::{types::Uuid, PgPool};
use chrono::NaiveDate;

#[derive(Debug, Clone)]
pub struct RoleMember {
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
}

#[derive(Clone)]
pub struct RoleMemberRepo {
    pub pool: PgPool,
}

impl RoleMemberRepo {
    pub async fn create(&self, user_id: Uuid, role_name: &str, valid_from: NaiveDate, valid_until: Option<NaiveDate>) -> Result<(), sqlx::Error> {
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

    pub async fn fetch_by_user_and_role(&self, user_id: Uuid, role_name: &str) -> Result<Vec<RoleMember>, sqlx::Error> {
        let records = sqlx::query_as!(
            RoleMember,
            r#"
            SELECT user_id, role_name, valid_from, valid_until
            FROM RoleMember
            WHERE user_id = $1 AND role_name = $2
            "#,
            user_id,
            role_name
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    pub async fn update_valid_until(&self, user_id: Uuid, role_name: &str, valid_from: NaiveDate, new_valid_until: NaiveDate) -> Result<(), sqlx::Error> {
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

    pub async fn delete(&self, user_id: Uuid, role_name: &str, valid_from: NaiveDate) -> Result<(), sqlx::Error> {
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
}
