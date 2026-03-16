use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use super::member::MemberDAO;
use crate::application::ports::repository_error::RepositoryError;
use crate::application::ports::role_repository_port::{
    RoleMembership, RoleRepositoryPort, RoleStats as PortRoleStats, RolesWithStatsParams,
};
use crate::domain::{Person, Role, RoleName};

// --- DAO types (private, map DB shape) ---

#[derive(Debug, sqlx::FromRow)]
struct RoleDAO {
    name: String,
    color: Option<String>,
    description: Option<String>,
}

impl From<RoleDAO> for Role {
    fn from(row: RoleDAO) -> Self {
        Self {
            name: RoleName(row.name),
            color: row.color,
            description: row.description,
        }
    }
}

#[derive(Debug)]
struct RoleMemberDAO {
    user_id: Uuid,
    role_name: String,
    valid_from: NaiveDate,
    valid_until: Option<NaiveDate>,
}

impl From<RoleMemberDAO> for RoleMembership {
    fn from(row: RoleMemberDAO) -> Self {
        Self {
            user_id: row.user_id,
            role_name: RoleName(row.role_name),
            valid_from: row.valid_from,
            valid_until: row.valid_until,
        }
    }
}

#[derive(Debug)]
struct RoleStatsDAO {
    name: String,
    color: Option<String>,
    description: Option<String>,
    member_count: Option<i64>,
    active_member_count: Option<i64>,
}

impl From<RoleStatsDAO> for PortRoleStats {
    fn from(row: RoleStatsDAO) -> Self {
        Self {
            name: RoleName(row.name),
            color: row.color,
            description: row.description,
            member_count: row.member_count,
            active_member_count: row.active_member_count,
        }
    }
}

// --- Repository ---

#[derive(Clone)]
pub struct RoleRepo {
    pub pool: PgPool,
}

#[async_trait::async_trait]
impl RoleRepositoryPort for RoleRepo {
    async fn create(&self, role: &Role) -> Result<Role, RepositoryError> {
        let row = sqlx::query_as!(
            RoleDAO,
            "INSERT INTO Role (name, color) VALUES ($1, $2) RETURNING name, color, description",
            &role.name.0,
            role.color.as_deref(),
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn fetch_all(&self) -> Result<Vec<Role>, RepositoryError> {
        let rows = sqlx::query_as!(RoleDAO, "SELECT name, color, description FROM Role")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_by_name(&self, role_name: &str) -> Result<Role, RepositoryError> {
        let row = sqlx::query_as!(
            RoleDAO,
            "SELECT name, color, description FROM Role WHERE name = $1",
            role_name,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn delete(&self, role_name: &str) -> Result<(), RepositoryError> {
        sqlx::query("DELETE FROM Role WHERE name = $1")
            .bind(role_name)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn create_role_member(
        &self,
        user_id: &Uuid,
        role_name: &str,
        valid_from: NaiveDate,
        valid_until: Option<NaiveDate>,
    ) -> Result<(), RepositoryError> {
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

    async fn update_valid_until(
        &self,
        user_id: &Uuid,
        role_name: &str,
        valid_from: NaiveDate,
        new_valid_until: NaiveDate,
    ) -> Result<(), RepositoryError> {
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

    async fn delete_role_member(
        &self,
        user_id: &Uuid,
        role_name: &str,
        valid_from: NaiveDate,
    ) -> Result<(), RepositoryError> {
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

    async fn fetch_roles_by_member(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<RoleMembership>, RepositoryError> {
        let rows = sqlx::query_as!(
            RoleMemberDAO,
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
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_members_by_role(&self, role_name: &str) -> Result<Vec<Person>, RepositoryError> {
        let rows = sqlx::query_as!(
            MemberDAO,
            r#"
          SELECT Member.user_id, email, first_name, last_name, full_name, home_municipality, has_accepted_policies, email_notifications, language
          FROM RoleMember JOIN Member ON RoleMember.user_id = Member.user_id
          WHERE role_name = $1
          "#,
            role_name
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn fetch_roles_with_stats(
        &self,
        RolesWithStatsParams {
            page_size,
            offset,
            search,
            order_by,
            order_desc,
        }: RolesWithStatsParams,
    ) -> Result<Vec<PortRoleStats>, RepositoryError> {
        let rows = sqlx::query_as!(
            RoleStatsDAO,
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
            if order_desc.unwrap_or(false) {
                "DESC"
            } else {
                "ASC"
            },
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }
}
