use sqlx::PgPool;
use uuid::Uuid;

use crate::application::ports::attribute_repository_port::{
    AttributeRepositoryPort, CreateAttributeDefinition, UpdateAttributeDefinition,
};
use crate::application::ports::repository_error::RepositoryError;
use crate::domain::{
    AttributeDefinition, AttributeName, AttributeValue, EditableBy, MemberAttribute, PersonId,
};

// --- DAO types ---

#[derive(Debug, Clone, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "attribute_editable_by", rename_all = "lowercase")]
enum EditableByDAO {
    Admin,
    User,
    Both,
}

impl From<EditableByDAO> for EditableBy {
    fn from(v: EditableByDAO) -> Self {
        match v {
            EditableByDAO::Admin => Self::Admin,
            EditableByDAO::User => Self::User,
            EditableByDAO::Both => Self::Both,
        }
    }
}

impl From<EditableBy> for EditableByDAO {
    fn from(v: EditableBy) -> Self {
        match v {
            EditableBy::Admin => Self::Admin,
            EditableBy::User => Self::User,
            EditableBy::Both => Self::Both,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AttributeDefinitionDAO {
    name: String,
    description: Option<String>,
    allowed_values: Option<Vec<String>>,
    sync_to_keycloak: bool,
    editable_by: EditableByDAO,
}

impl From<AttributeDefinitionDAO> for AttributeDefinition {
    fn from(row: AttributeDefinitionDAO) -> Self {
        Self {
            name: AttributeName::new_unchecked(row.name),
            description: row.description,
            allowed_values: row.allowed_values,
            sync_to_keycloak: row.sync_to_keycloak,
            editable_by: row.editable_by.into(),
        }
    }
}

// --- Repository ---

#[derive(Clone)]
pub struct AttributeRepo {
    pub pool: PgPool,
}

#[async_trait::async_trait]
impl AttributeRepositoryPort for AttributeRepo {
    async fn create_definition(
        &self,
        input: CreateAttributeDefinition,
    ) -> Result<AttributeDefinition, RepositoryError> {
        let editable_by = EditableByDAO::from(input.editable_by);
        let row = sqlx::query_as!(
            AttributeDefinitionDAO,
            r#"INSERT INTO AttributeDefinition (name, description, allowed_values, sync_to_keycloak, editable_by)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING name, description, allowed_values,
                         sync_to_keycloak,
                         editable_by AS "editable_by: EditableByDAO""#,
            input.name.as_str(),
            input.description.as_deref(),
            input.allowed_values.as_deref(),
            input.sync_to_keycloak,
            editable_by as EditableByDAO,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn update_definition(
        &self,
        name: &AttributeName,
        input: UpdateAttributeDefinition,
    ) -> Result<AttributeDefinition, RepositoryError> {
        let editable_by = EditableByDAO::from(input.editable_by);
        let row = sqlx::query_as!(
            AttributeDefinitionDAO,
            r#"UPDATE AttributeDefinition
               SET description = $2, allowed_values = $3, sync_to_keycloak = $4, editable_by = $5
               WHERE name = $1
               RETURNING name, description, allowed_values,
                         sync_to_keycloak,
                         editable_by AS "editable_by: EditableByDAO""#,
            name.as_str(),
            input.description.as_deref(),
            input.allowed_values.as_deref(),
            input.sync_to_keycloak,
            editable_by as EditableByDAO,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn delete_definition(&self, name: &AttributeName) -> Result<(), RepositoryError> {
        sqlx::query!(
            "DELETE FROM AttributeDefinition WHERE name = $1",
            name.as_str()
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn fetch_definition(
        &self,
        name: &AttributeName,
    ) -> Result<Option<AttributeDefinition>, RepositoryError> {
        let row = sqlx::query_as!(
            AttributeDefinitionDAO,
            r#"SELECT name, description, allowed_values,
                      sync_to_keycloak,
                      editable_by AS "editable_by: EditableByDAO"
               FROM AttributeDefinition WHERE name = $1"#,
            name.as_str(),
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(Into::into))
    }

    async fn fetch_all_definitions(&self) -> Result<Vec<AttributeDefinition>, RepositoryError> {
        let rows = sqlx::query_as!(
            AttributeDefinitionDAO,
            r#"SELECT name, description, allowed_values,
                      sync_to_keycloak,
                      editable_by AS "editable_by: EditableByDAO"
               FROM AttributeDefinition ORDER BY name"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn upsert_member_value(
        &self,
        user_id: &Uuid,
        name: &AttributeName,
        value: &AttributeValue,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"INSERT INTO MemberAttribute (user_id, attribute_name, value)
               VALUES ($1, $2, $3)
               ON CONFLICT (user_id, attribute_name) DO UPDATE
                 SET value = EXCLUDED.value, updated_at = NOW()"#,
            user_id,
            name.as_str(),
            value.as_str(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_member_value(
        &self,
        user_id: &Uuid,
        name: &AttributeName,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            "DELETE FROM MemberAttribute WHERE user_id = $1 AND attribute_name = $2",
            user_id,
            name.as_str(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn fetch_member_values(
        &self,
        user_id: &Uuid,
    ) -> Result<Vec<MemberAttribute>, RepositoryError> {
        let rows = sqlx::query!(
            r#"SELECT user_id, attribute_name, value
               FROM MemberAttribute
               WHERE user_id = $1
               ORDER BY attribute_name"#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| MemberAttribute {
                user_id: PersonId(r.user_id),
                name: AttributeName::new_unchecked(r.attribute_name),
                value: AttributeValue::new_unchecked(r.value),
            })
            .collect())
    }

    async fn fetch_all_values_for(
        &self,
        name: &AttributeName,
    ) -> Result<Vec<(Uuid, AttributeValue)>, RepositoryError> {
        let rows = sqlx::query!(
            "SELECT user_id, value FROM MemberAttribute WHERE attribute_name = $1",
            name.as_str(),
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.user_id, AttributeValue::new_unchecked(r.value)))
            .collect())
    }
}
