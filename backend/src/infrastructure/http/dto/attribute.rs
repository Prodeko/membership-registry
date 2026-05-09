use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::domain::{AttributeDefinition, AttributeValue, EditableBy};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export, rename = "EditableBy")]
#[serde(rename_all = "lowercase")]
pub enum EditableByDTO {
    Admin,
    User,
    Both,
}

impl From<EditableBy> for EditableByDTO {
    fn from(v: EditableBy) -> Self {
        match v {
            EditableBy::Admin => Self::Admin,
            EditableBy::User => Self::User,
            EditableBy::Both => Self::Both,
        }
    }
}

impl From<EditableByDTO> for EditableBy {
    fn from(v: EditableByDTO) -> Self {
        match v {
            EditableByDTO::Admin => Self::Admin,
            EditableByDTO::User => Self::User,
            EditableByDTO::Both => Self::Both,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename = "AttributeDefinition")]
pub struct AttributeDefinitionDTO {
    pub name: String,
    pub description: Option<String>,
    pub allowed_values: Option<Vec<String>>,
    pub sync_to_keycloak: bool,
    pub editable_by: EditableByDTO,
}

impl From<AttributeDefinition> for AttributeDefinitionDTO {
    fn from(d: AttributeDefinition) -> Self {
        let allowed = d
            .allowed_values()
            .map(|vs| vs.iter().map(|v| v.as_str().to_string()).collect());
        Self {
            sync_to_keycloak: d.sync_to_keycloak(),
            editable_by: d.editable_by().into(),
            description: d.description().map(str::to_string),
            allowed_values: allowed,
            name: d.name().clone().into_inner(),
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "CreateAttributeDefinition")]
pub struct CreateAttributeDefinitionDTO {
    pub name: String,
    pub description: Option<String>,
    pub allowed_values: Option<Vec<String>>,
    pub sync_to_keycloak: bool,
    pub editable_by: EditableByDTO,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "UpdateAttributeDefinition")]
pub struct UpdateAttributeDefinitionDTO {
    pub description: Option<String>,
    pub allowed_values: Option<Vec<String>>,
    pub sync_to_keycloak: bool,
    pub editable_by: EditableByDTO,
}

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "MemberAttribute")]
pub struct MemberAttributeDTO {
    pub name: String,
    pub value: Option<String>,
    pub editable: bool,
    pub allowed_values: Option<Vec<String>>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "SetMemberAttribute")]
pub struct SetMemberAttributeDTO {
    pub value: String,
}

/// Helper used by the routing layer to compute the per-row `editable` flag for
/// a given actor.
pub fn editable_for_admin(d: &AttributeDefinition) -> bool {
    !matches!(d.editable_by(), EditableBy::User)
}

pub fn editable_for_self(d: &AttributeDefinition) -> bool {
    !matches!(d.editable_by(), EditableBy::Admin)
}

/// Parse a wire-level `Vec<String>` of allowed_values into validated
/// `Vec<AttributeValue>`. Empty input vec returns Ok(empty) — the caller is
/// responsible for converting empty to None if that's the intended semantics.
pub fn parse_allowed_values(
    raw: Option<Vec<String>>,
) -> Result<Option<Vec<AttributeValue>>, crate::domain::InvalidAttributeValue> {
    raw.map(|vs| {
        vs.into_iter()
            .map(AttributeValue::new)
            .collect::<Result<Vec<_>, _>>()
    })
    .transpose()
}
