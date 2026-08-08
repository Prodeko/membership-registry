use chrono::NaiveDate;
use uuid::Uuid;

use super::repository_error::RepositoryError;
use crate::domain::{Person, RenewalPrompt, Role, RoleName};

#[derive(Debug, Clone)]
pub struct RoleMembership {
    pub user_id: Uuid,
    pub role_name: RoleName,
    pub valid_from: NaiveDate,
    pub valid_until: Option<NaiveDate>,
    pub renewable: bool,
    pub renewal_due: bool,
    pub renewal_deadline: Option<NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct RoleStats {
    pub name: RoleName,
    pub color: Option<String>,
    pub description: Option<String>,
    pub member_count: Option<i64>,
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

#[async_trait::async_trait]
pub trait RoleRepositoryPort: Send + Sync {
    async fn create(&self, role: &Role) -> Result<Role, RepositoryError>;

    async fn update(&self, role: &Role) -> Result<Role, RepositoryError>;

    async fn fetch_all(&self) -> Result<Vec<Role>, RepositoryError>;

    async fn fetch_by_name(&self, role_name: &str) -> Result<Role, RepositoryError>;

    async fn delete(&self, role_name: &str) -> Result<(), RepositoryError>;

    async fn create_role_member(
        &self,
        user_id: &Uuid,
        role_name: &str,
        valid_from: NaiveDate,
        valid_until: Option<NaiveDate>,
    ) -> Result<(), RepositoryError>;

    /// Insert a role membership, or overwrite `valid_until` when a row already
    /// exists for `(user_id, role_name, valid_from)`.
    async fn upsert_role_member(
        &self,
        user_id: &Uuid,
        role_name: &str,
        valid_from: NaiveDate,
        valid_until: Option<NaiveDate>,
    ) -> Result<(), RepositoryError>;

    async fn create_role_members_batch(
        &self,
        user_ids: &[Uuid],
        role_names: &[String],
        valid_from: NaiveDate,
        valid_until: Option<NaiveDate>,
    ) -> Result<(), RepositoryError>;

    async fn update_valid_until(
        &self,
        user_id: &Uuid,
        role_name: &str,
        valid_from: NaiveDate,
        new_valid_until: NaiveDate,
    ) -> Result<(), RepositoryError>;

    async fn delete_role_member(
        &self,
        user_id: &Uuid,
        role_name: &str,
        valid_from: NaiveDate,
    ) -> Result<(), RepositoryError>;

    async fn fetch_roles_by_member(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<RoleMembership>, RepositoryError>;

    async fn fetch_members_by_role(&self, role_name: &str) -> Result<Vec<Person>, RepositoryError>;

    async fn fetch_roles_with_stats(
        &self,
        params: RolesWithStatsParams,
    ) -> Result<Vec<RoleStats>, RepositoryError>;

    async fn fetch_expired_unsynced(&self) -> Result<Vec<RoleMembership>, RepositoryError>;

    async fn mark_keycloak_synced(
        &self,
        user_id: &Uuid,
        role_name: &str,
        valid_from: NaiveDate,
    ) -> Result<(), RepositoryError>;

    /// Fetch the configured renewal banner prompts for a role (any number of locales).
    async fn fetch_renewal_prompts(
        &self,
        role_name: &str,
    ) -> Result<Vec<RenewalPrompt>, RepositoryError>;

    /// Replace a role's renewal banner prompts with exactly `prompts` (delete + insert, atomic).
    async fn replace_renewal_prompts(
        &self,
        role_name: &str,
        prompts: &[RenewalPrompt],
    ) -> Result<(), RepositoryError>;
}
