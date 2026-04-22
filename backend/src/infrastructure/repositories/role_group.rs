use chrono::NaiveDate;
use sqlx::PgPool;
use uuid::Uuid;

use crate::application::ports::repository_error::RepositoryError;
use crate::application::ports::role_group_repository_port::RoleGroupRepositoryPort;
use crate::domain::{RoleGroup, RoleGroupId, RoleGroupMembership, RoleName};

// ---------------------------------------------------------------------------
// DAOs
// ---------------------------------------------------------------------------

#[derive(Debug, sqlx::FromRow)]
struct RoleGroupDAO {
    id: Uuid,
    name: String,
    description: Option<String>,
    keycloak_group_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct RoleGroupRoleDAO {
    role_name: String,
}

#[derive(Debug, sqlx::FromRow)]
struct RoleGroupMemberDAO {
    group_id: Uuid,
    group_name: String,
    user_id: Uuid,
    valid_from: NaiveDate,
    valid_until: Option<NaiveDate>,
}

// ---------------------------------------------------------------------------
// Repository
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct RoleGroupRepo {
    pub pool: PgPool,
}

impl RoleGroupRepo {
    async fn fetch_roles_for_group(&self, group_id: &Uuid) -> Result<Vec<RoleName>, sqlx::Error> {
        let rows = sqlx::query_as!(
            RoleGroupRoleDAO,
            r#"SELECT role_name FROM RoleGroupRole WHERE group_id = $1 ORDER BY role_name"#,
            group_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| RoleName(r.role_name)).collect())
    }

    async fn to_role_group(&self, dao: RoleGroupDAO) -> Result<RoleGroup, sqlx::Error> {
        let role_names = self.fetch_roles_for_group(&dao.id).await?;
        Ok(RoleGroup {
            id: RoleGroupId(dao.id),
            name: dao.name,
            description: dao.description,
            keycloak_group_id: dao.keycloak_group_id,
            role_names,
        })
    }
}

#[async_trait::async_trait]
impl RoleGroupRepositoryPort for RoleGroupRepo {
    async fn create(
        &self,
        name: &str,
        description: Option<&str>,
        keycloak_group_id: Option<&str>,
    ) -> Result<RoleGroup, RepositoryError> {
        let row = sqlx::query_as!(
            RoleGroupDAO,
            r#"INSERT INTO RoleGroup (name, description, keycloak_group_id)
               VALUES ($1, $2, $3)
               RETURNING id, name, description, keycloak_group_id"#,
            name,
            description,
            keycloak_group_id,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(RepositoryError::from)?;

        self.to_role_group(row).await.map_err(RepositoryError::from)
    }

    async fn update(
        &self,
        id: &Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<RoleGroup, RepositoryError> {
        let row = sqlx::query_as!(
            RoleGroupDAO,
            r#"UPDATE RoleGroup SET name = $2, description = $3
               WHERE id = $1
               RETURNING id, name, description, keycloak_group_id"#,
            id,
            name,
            description,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(RepositoryError::from)?;

        self.to_role_group(row).await.map_err(RepositoryError::from)
    }

    async fn delete(&self, id: &Uuid) -> Result<(), RepositoryError> {
        sqlx::query!(r#"DELETE FROM RoleGroup WHERE id = $1"#, id)
            .execute(&self.pool)
            .await
            .map_err(RepositoryError::from)?;
        Ok(())
    }

    async fn fetch_all(&self) -> Result<Vec<RoleGroup>, RepositoryError> {
        let rows = sqlx::query_as!(
            RoleGroupDAO,
            r#"SELECT id, name, description, keycloak_group_id FROM RoleGroup ORDER BY name"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::from)?;

        let mut groups = Vec::with_capacity(rows.len());
        for row in rows {
            let group = self
                .to_role_group(row)
                .await
                .map_err(RepositoryError::from)?;
            groups.push(group);
        }
        Ok(groups)
    }

    async fn fetch_by_id(&self, id: &Uuid) -> Result<RoleGroup, RepositoryError> {
        let row = sqlx::query_as!(
            RoleGroupDAO,
            r#"SELECT id, name, description, keycloak_group_id FROM RoleGroup WHERE id = $1"#,
            id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(RepositoryError::from)?;

        self.to_role_group(row).await.map_err(RepositoryError::from)
    }

    async fn set_roles(&self, id: &Uuid, role_names: &[RoleName]) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(RepositoryError::from)?;

        sqlx::query!(r#"DELETE FROM RoleGroupRole WHERE group_id = $1"#, id)
            .execute(&mut *tx)
            .await
            .map_err(RepositoryError::from)?;

        for role_name in role_names {
            sqlx::query!(
                r#"INSERT INTO RoleGroupRole (group_id, role_name) VALUES ($1, $2)
                   ON CONFLICT DO NOTHING"#,
                id,
                role_name.0,
            )
            .execute(&mut *tx)
            .await
            .map_err(RepositoryError::from)?;
        }

        tx.commit().await.map_err(RepositoryError::from)?;
        Ok(())
    }

    async fn create_member(
        &self,
        group_id: &Uuid,
        user_id: &Uuid,
        valid_from: NaiveDate,
        valid_until: Option<NaiveDate>,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"INSERT INTO RoleGroupMember (group_id, user_id, valid_from, valid_until)
               VALUES ($1, $2, $3, $4)"#,
            group_id,
            user_id,
            valid_from,
            valid_until,
        )
        .execute(&self.pool)
        .await
        .map_err(RepositoryError::from)?;
        Ok(())
    }

    async fn delete_member(
        &self,
        group_id: &Uuid,
        user_id: &Uuid,
        valid_from: NaiveDate,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"DELETE FROM RoleGroupMember WHERE group_id = $1 AND user_id = $2 AND valid_from = $3"#,
            group_id,
            user_id,
            valid_from,
        )
        .execute(&self.pool)
        .await
        .map_err(RepositoryError::from)?;
        Ok(())
    }

    async fn fetch_members_by_group(
        &self,
        group_id: &Uuid,
    ) -> Result<Vec<RoleGroupMembership>, RepositoryError> {
        let rows = sqlx::query_as!(
            RoleGroupMemberDAO,
            r#"SELECT rgm.group_id, rg.name as group_name, rgm.user_id, rgm.valid_from, rgm.valid_until
               FROM RoleGroupMember rgm
               JOIN RoleGroup rg ON rg.id = rgm.group_id
               WHERE rgm.group_id = $1
               ORDER BY rgm.valid_from DESC"#,
            group_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::from)?;

        Ok(rows
            .into_iter()
            .map(|r| RoleGroupMembership {
                group_id: r.group_id,
                group_name: r.group_name,
                user_id: r.user_id,
                valid_from: r.valid_from,
                valid_until: r.valid_until,
            })
            .collect())
    }

    async fn fetch_groups_by_member(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<RoleGroupMembership>, RepositoryError> {
        let rows = sqlx::query_as!(
            RoleGroupMemberDAO,
            r#"SELECT rgm.group_id, rg.name as group_name, rgm.user_id, rgm.valid_from, rgm.valid_until
               FROM RoleGroupMember rgm
               JOIN RoleGroup rg ON rg.id = rgm.group_id
               WHERE rgm.user_id = $1
               ORDER BY rgm.valid_from DESC"#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::from)?;

        Ok(rows
            .into_iter()
            .map(|r| RoleGroupMembership {
                group_id: r.group_id,
                group_name: r.group_name,
                user_id: r.user_id,
                valid_from: r.valid_from,
                valid_until: r.valid_until,
            })
            .collect())
    }

    async fn fetch_expired_unsynced(&self) -> Result<Vec<RoleGroupMembership>, RepositoryError> {
        let rows = sqlx::query_as!(
            RoleGroupMemberDAO,
            r#"SELECT rgm.group_id, rg.name as group_name, rgm.user_id, rgm.valid_from, rgm.valid_until
               FROM RoleGroupMember rgm
               JOIN RoleGroup rg ON rg.id = rgm.group_id
               WHERE rgm.valid_until < CURRENT_DATE
                 AND rgm.keycloak_removed_at IS NULL"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(RepositoryError::from)?;

        Ok(rows
            .into_iter()
            .map(|r| RoleGroupMembership {
                group_id: r.group_id,
                group_name: r.group_name,
                user_id: r.user_id,
                valid_from: r.valid_from,
                valid_until: r.valid_until,
            })
            .collect())
    }

    async fn mark_keycloak_synced(
        &self,
        group_id: &Uuid,
        user_id: &Uuid,
        valid_from: NaiveDate,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"UPDATE RoleGroupMember
               SET keycloak_removed_at = NOW()
               WHERE group_id = $1 AND user_id = $2 AND valid_from = $3"#,
            group_id,
            user_id,
            valid_from,
        )
        .execute(&self.pool)
        .await
        .map_err(RepositoryError::from)?;
        Ok(())
    }

    async fn set_keycloak_group_id(
        &self,
        id: &Uuid,
        keycloak_group_id: &str,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"UPDATE RoleGroup SET keycloak_group_id = $2 WHERE id = $1"#,
            id,
            keycloak_group_id,
        )
        .execute(&self.pool)
        .await
        .map_err(RepositoryError::from)?;
        Ok(())
    }
}
