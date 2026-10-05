use super::repository_error::RepositoryError;
use crate::domain::{
    AttributeDefinition, AttributeName, AttributeValue, EditableBy, MemberAttribute, PersonId,
};

#[derive(Debug, Clone)]
pub struct CreateAttributeDefinition {
    pub name: AttributeName,
    pub description: Option<String>,
    pub allowed_values: Option<Vec<AttributeValue>>,
    pub default_value: Option<AttributeValue>,
    pub sync_to_keycloak: bool,
    pub editable_by: EditableBy,
    pub required: bool,
    pub multiple: bool,
    pub allow_other: bool,
}

#[derive(Debug, Clone)]
pub struct UpdateAttributeDefinition {
    pub description: Option<String>,
    pub allowed_values: Option<Vec<AttributeValue>>,
    pub default_value: Option<AttributeValue>,
    pub sync_to_keycloak: bool,
    pub editable_by: EditableBy,
    pub required: bool,
    pub multiple: bool,
    pub allow_other: bool,
}

#[async_trait::async_trait]
pub trait AttributeRepositoryPort: Send + Sync {
    // --- Definition CRUD ---

    async fn create_definition(
        &self,
        input: CreateAttributeDefinition,
    ) -> Result<AttributeDefinition, RepositoryError>;

    async fn update_definition(
        &self,
        name: &AttributeName,
        input: UpdateAttributeDefinition,
    ) -> Result<AttributeDefinition, RepositoryError>;

    async fn delete_definition(&self, name: &AttributeName) -> Result<(), RepositoryError>;

    async fn fetch_definition(
        &self,
        name: &AttributeName,
    ) -> Result<Option<AttributeDefinition>, RepositoryError>;

    async fn fetch_all_definitions(&self) -> Result<Vec<AttributeDefinition>, RepositoryError>;

    // --- Member values ---

    /// Replace the member's whole value list for `name`. `values` is
    /// non-empty; clearing goes through `delete_member_value`.
    async fn upsert_member_value(
        &self,
        user_id: &PersonId,
        name: &AttributeName,
        values: &[AttributeValue],
    ) -> Result<(), RepositoryError>;

    async fn delete_member_value(
        &self,
        user_id: &PersonId,
        name: &AttributeName,
    ) -> Result<(), RepositoryError>;

    async fn fetch_member_values(
        &self,
        user_id: &PersonId,
    ) -> Result<Vec<MemberAttribute>, RepositoryError>;

    /// Every member's value list for a given attribute.
    async fn fetch_all_values_for(
        &self,
        name: &AttributeName,
    ) -> Result<Vec<(PersonId, Vec<AttributeValue>)>, RepositoryError>;
}
