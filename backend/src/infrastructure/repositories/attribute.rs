use sqlx::PgPool;

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
    default_value: Option<String>,
    sync_to_keycloak: bool,
    editable_by: EditableByDAO,
    required: bool,
    multiple: bool,
}

impl From<AttributeDefinitionDAO> for AttributeDefinition {
    fn from(row: AttributeDefinitionDAO) -> Self {
        let allowed = row
            .allowed_values
            .map(|vs| vs.into_iter().map(AttributeValue::new_unchecked).collect());
        let default = row.default_value.map(AttributeValue::new_unchecked);
        AttributeDefinition::new_unchecked(
            AttributeName::new_unchecked(row.name),
            row.description,
            allowed,
            default,
            row.sync_to_keycloak,
            row.editable_by.into(),
            row.required,
        )
        .with_multiple(row.multiple)
    }
}

fn unchecked_values(values: Vec<String>) -> Vec<AttributeValue> {
    values
        .into_iter()
        .map(AttributeValue::new_unchecked)
        .collect()
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
        let allowed: Option<Vec<String>> = input
            .allowed_values
            .as_ref()
            .map(|vs| vs.iter().map(|v| v.as_str().to_string()).collect());
        let default = input.default_value.as_ref().map(|v| v.as_str().to_string());
        let row = sqlx::query_as!(
            AttributeDefinitionDAO,
            r#"INSERT INTO AttributeDefinition (name, description, allowed_values, default_value, sync_to_keycloak, editable_by, required, multiple)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING name, description, allowed_values, default_value,
                         sync_to_keycloak,
                         editable_by AS "editable_by: EditableByDAO", required, multiple"#,
            input.name.as_str(),
            input.description.as_deref(),
            allowed.as_deref(),
            default.as_deref(),
            input.sync_to_keycloak,
            editable_by as EditableByDAO,
            input.required,
            input.multiple,
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
        let allowed: Option<Vec<String>> = input
            .allowed_values
            .as_ref()
            .map(|vs| vs.iter().map(|v| v.as_str().to_string()).collect());
        let default = input.default_value.as_ref().map(|v| v.as_str().to_string());
        let row = sqlx::query_as!(
            AttributeDefinitionDAO,
            r#"UPDATE AttributeDefinition
               SET description = $2, allowed_values = $3, default_value = $4,
                   sync_to_keycloak = $5, editable_by = $6, required = $7,
                   multiple = $8
               WHERE name = $1
               RETURNING name, description, allowed_values, default_value,
                         sync_to_keycloak,
                         editable_by AS "editable_by: EditableByDAO", required, multiple"#,
            name.as_str(),
            input.description.as_deref(),
            allowed.as_deref(),
            default.as_deref(),
            input.sync_to_keycloak,
            editable_by as EditableByDAO,
            input.required,
            input.multiple,
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
            r#"SELECT name, description, allowed_values, default_value,
                      sync_to_keycloak,
                      editable_by AS "editable_by: EditableByDAO", required, multiple
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
            r#"SELECT name, description, allowed_values, default_value,
                      sync_to_keycloak,
                      editable_by AS "editable_by: EditableByDAO", required, multiple
               FROM AttributeDefinition ORDER BY name"#
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn upsert_member_value(
        &self,
        user_id: &PersonId,
        name: &AttributeName,
        values: &[AttributeValue],
    ) -> Result<(), RepositoryError> {
        let values: Vec<String> = values.iter().map(|v| v.as_str().to_string()).collect();
        sqlx::query!(
            r#"INSERT INTO MemberAttribute (user_id, attribute_name, value_list)
               VALUES ($1, $2, $3)
               ON CONFLICT (user_id, attribute_name) DO UPDATE
                 SET value_list = EXCLUDED.value_list, updated_at = NOW()"#,
            user_id.0,
            name.as_str(),
            &values,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_member_value(
        &self,
        user_id: &PersonId,
        name: &AttributeName,
    ) -> Result<(), RepositoryError> {
        sqlx::query!(
            "DELETE FROM MemberAttribute WHERE user_id = $1 AND attribute_name = $2",
            user_id.0,
            name.as_str(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn fetch_member_values(
        &self,
        user_id: &PersonId,
    ) -> Result<Vec<MemberAttribute>, RepositoryError> {
        let rows = sqlx::query!(
            r#"SELECT user_id, attribute_name, value_list
               FROM MemberAttribute
               WHERE user_id = $1
               ORDER BY attribute_name"#,
            user_id.0,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| MemberAttribute {
                user_id: PersonId(r.user_id),
                name: AttributeName::new_unchecked(r.attribute_name),
                values: unchecked_values(r.value_list),
            })
            .collect())
    }

    async fn fetch_all_values_for(
        &self,
        name: &AttributeName,
    ) -> Result<Vec<(PersonId, Vec<AttributeValue>)>, RepositoryError> {
        let rows = sqlx::query!(
            "SELECT user_id, value_list FROM MemberAttribute WHERE attribute_name = $1",
            name.as_str(),
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| (PersonId(r.user_id), unchecked_values(r.value_list)))
            .collect())
    }
}
