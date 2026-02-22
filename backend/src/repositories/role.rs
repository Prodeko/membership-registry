use super::member::Member;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Clone)]
pub struct RoleRepo {
    pub pool: PgPool,
}

#[derive(Debug, sqlx::FromRow, Serialize, Deserialize, Default, TS)]
#[ts(export)]
pub struct Role {
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct RoleMember {
    pub user_id: Uuid,
    pub role_name: String,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct RoleStats {
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    #[ts(type = "number | null")]
    pub member_count: Option<i64>,
    #[ts(type = "number | null")]
    pub active_member_count: Option<i64>,
}


#[derive(Default)]
pub struct RolesWithStatsParams {
    pub page_size: Option<u64>,
    pub offset: Option<u64>,
    pub search: Option<String>,
    pub order_by: Option<String>,
    pub order_desc: Option<bool>,
}

impl RolesWithStatsParams {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RoleRepo {
    pub async fn create(&self, role: Role) -> Result<Role, sqlx::Error> {
        let created =
            sqlx::query_as::<_, Role>("INSERT INTO Role (name, color) VALUES ($1, $2) RETURNING *")
                .bind(role.name)
                .bind(role.color)
                .fetch_one(&self.pool)
                .await?;
        Ok(created)
    }

    pub async fn fetch_all(&self) -> Result<Vec<Role>, sqlx::Error> {
        let roles = sqlx::query_as::<_, Role>("SELECT * FROM Role")
            .fetch_all(&self.pool)
            .await?;
        Ok(roles)
    }

    pub async fn fetch_by_name(&self, role_name: &str) -> Result<Role, sqlx::Error> {
        let role = sqlx::query_as::<_, Role>("SELECT * FROM Role WHERE name = $1")
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
        user_id: &Uuid,
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
        user_id: &Uuid,
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
        user_id: &Uuid,
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
        user_id: &Uuid,
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
          SELECT Member.user_id, email, first_name, last_name, full_name, home_municipality, has_accepted_policies
          FROM RoleMember JOIN Member ON RoleMember.user_id = Member.user_id
          WHERE role_name = $1
          "#,
            role_name
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }

    pub async fn fetch_roles_with_stats(
        &self,
        RolesWithStatsParams {
            page_size,
            offset,
            search,
            order_by,
            order_desc,
        }: RolesWithStatsParams,
    ) -> Result<Vec<RoleStats>, sqlx::Error> {
        let records = sqlx::query_as!(
            RoleStats,
            r#"-- sql
            SELECT
                Role.*,
                COUNT(DISTINCT RoleMember.user_id) as member_count,
                COUNT(
                    CASE WHEN (
                        valid_until IS NULL OR (
                            valid_until >= CURRENT_DATE AND
                            valid_from <= CURRENT_DATE
                        )
                    ) THEN 1
                    END
                ) as active_member_count
            FROM Role LEFT JOIN RoleMember ON Role.name = RoleMember.role_name
            WHERE 
                $3::varchar IS NULL OR Role.name ILIKE '%' || $3 || '%'
            GROUP BY Role.name, Role.color
            ORDER BY
                CASE WHEN $5 = 'ASC' THEN
                    CASE LOWER($4)
                        WHEN 'name' THEN Role.name
                        WHEN 'color' THEN Role.color
                        WHEN 'description' THEN Role.description
                        WHEN 'member_count' THEN COUNT(DISTINCT RoleMember.user_id)::text
                        WHEN 'active_member_cout' THEN 
                            COUNT(
                                CASE WHEN (
                                    valid_until IS NULL OR (
                                        valid_until >= CURRENT_DATE AND
                                        valid_from <= CURRENT_DATE
                                    )
                                ) THEN 1
                                END
                            )::text
                        ELSE Role.name
                    END
                END ASC,
                CASE WHEN $5 = 'DESC' THEN
                    CASE LOWER($4)
                        WHEN 'name' THEN Role.name
                        WHEN 'color' THEN Role.color
                        WHEN 'description' THEN Role.description
                        WHEN 'member_count' THEN COUNT(RoleMember.user_id)::text
                        WHEN 'active_member_cout' THEN 
                            COUNT(
                                CASE WHEN (
                                    valid_until IS NULL OR (
                                        valid_until >= CURRENT_DATE AND
                                        valid_from <= CURRENT_DATE
                                    )
                                ) THEN 1
                                END
                            )::text
                        ELSE Role.name
                    END
                END DESC
            LIMIT $1 OFFSET $2::Integer * $1::Integer
          "#,
            page_size.map(|x| x as i32).unwrap_or(i32::MAX) as i32,
            offset.unwrap_or(0) as i64,
            search,
            order_by,
            if order_desc.unwrap_or(false) { "DESC" } else { "ASC" },
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(records)
    }
}
