use chrono::NaiveDate;
use uuid::Uuid;

use super::repository_error::RepositoryError;
use crate::domain::{RoleGroup, RoleGroupId, RoleGroupMembership, RoleName};

#[async_trait::async_trait]
pub trait RoleGroupRepositoryPort: Send + Sync {
    async fn create(
        &self,
        name: &str,
        description: Option<&str>,
        keycloak_group_id: Option<&str>,
    ) -> Result<RoleGroup, RepositoryError>;

    async fn update(
        &self,
        id: &Uuid,
        name: &str,
        description: Option<&str>,
    ) -> Result<RoleGroup, RepositoryError>;

    async fn delete(&self, id: &Uuid) -> Result<(), RepositoryError>;

    async fn fetch_all(&self) -> Result<Vec<RoleGroup>, RepositoryError>;

    async fn fetch_by_id(&self, id: &Uuid) -> Result<RoleGroup, RepositoryError>;

    async fn set_roles(&self, id: &Uuid, role_names: &[RoleName]) -> Result<(), RepositoryError>;

    async fn create_member(
        &self,
        group_id: &Uuid,
        user_id: &Uuid,
        valid_from: NaiveDate,
        valid_until: Option<NaiveDate>,
    ) -> Result<(), RepositoryError>;

    async fn delete_member(
        &self,
        group_id: &Uuid,
        user_id: &Uuid,
        valid_from: NaiveDate,
    ) -> Result<(), RepositoryError>;

    async fn fetch_members_by_group(
        &self,
        group_id: &Uuid,
    ) -> Result<Vec<RoleGroupMembership>, RepositoryError>;

    async fn fetch_groups_by_member(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<RoleGroupMembership>, RepositoryError>;

    async fn fetch_expired_unsynced(&self) -> Result<Vec<RoleGroupMembership>, RepositoryError>;

    async fn mark_keycloak_synced(
        &self,
        group_id: &Uuid,
        user_id: &Uuid,
        valid_from: NaiveDate,
    ) -> Result<(), RepositoryError>;

    async fn set_keycloak_group_id(
        &self,
        id: &Uuid,
        keycloak_group_id: &str,
    ) -> Result<(), RepositoryError>;
}
