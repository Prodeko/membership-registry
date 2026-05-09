use uuid::Uuid;

use super::repository_error::RepositoryError;
use crate::domain::{
    AttributeDefinition, AttributeName, AttributeValue, EditableBy, MemberAttribute,
};

#[derive(Debug, Clone)]
pub struct CreateAttributeDefinition {
    pub name: AttributeName,
    pub description: Option<String>,
    pub allowed_values: Option<Vec<AttributeValue>>,
    pub sync_to_keycloak: bool,
    pub editable_by: EditableBy,
}

#[derive(Debug, Clone)]
pub struct UpdateAttributeDefinition {
    pub description: Option<String>,
    pub allowed_values: Option<Vec<AttributeValue>>,
    pub sync_to_keycloak: bool,
    pub editable_by: EditableBy,
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

    async fn upsert_member_value(
        &self,
        user_id: &Uuid,
        name: &AttributeName,
        value: &AttributeValue,
    ) -> Result<(), RepositoryError>;

    async fn delete_member_value(
        &self,
        user_id: &Uuid,
        name: &AttributeName,
    ) -> Result<(), RepositoryError>;

    async fn fetch_member_values(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<MemberAttribute>, RepositoryError>;

    /// All (user_id, value) pairs for a given attribute. Used by drift detection.
    async fn fetch_all_values_for(
        &self,
        name: &AttributeName,
    ) -> Result<Vec<(Uuid, AttributeValue)>, RepositoryError>;
}
