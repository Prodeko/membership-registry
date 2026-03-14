use serde_json::Value;
use uuid::Uuid;

use super::repository_error::RepositoryError;
use crate::domain::{NewPerson, Person, UpdatePersonData};

#[derive(Debug)]
pub struct MemberWithRoles {
    pub person: Person,
    pub role_names: Value,
}

#[derive(Default)]
pub struct MembersWithRolesParams {
    pub valid_from: Option<chrono::NaiveDate>,
    pub valid_until: Option<chrono::NaiveDate>,
    pub roles: Option<Vec<String>>,
    pub search: Option<String>,
    pub order_by: Option<String>,
    pub order_desc: Option<bool>,
    pub page_size: Option<u64>,
    pub offset: Option<u64>,
}

#[async_trait::async_trait]
pub trait MemberRepositoryPort: Send + Sync {
    async fn create(&self, new: NewPerson) -> Result<Person, RepositoryError>;

    async fn fetch_all(&self) -> Result<Vec<Person>, RepositoryError>;

    async fn fetch_with_ids(&self, ids: Option<Vec<Uuid>>) -> Result<Vec<Person>, RepositoryError>;

    async fn fetch_one(&self, id: Uuid) -> Result<Person, RepositoryError>;

    async fn update(
        &self,
        user_id: Uuid,
        data: &UpdatePersonData,
    ) -> Result<Person, RepositoryError>;

    async fn delete(&self, id: Uuid) -> Result<(), RepositoryError>;

    async fn delete_many(&self, ids: Vec<Uuid>) -> Result<(), RepositoryError>;

    async fn fetch_members_with_roles(
        &self,
        params: MembersWithRolesParams,
    ) -> Result<Vec<MemberWithRoles>, RepositoryError>;
}
