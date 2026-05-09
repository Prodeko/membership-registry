use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::domain::{AttributeDefinition, AttributeValue, DriftEntry, EditableBy, SyncStatus};

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

/// Patch DTO for updating a definition. Each field's wire semantics:
/// - field omitted → leave unchanged
/// - `null` → clear (or no-op for non-nullable fields)
/// - value → set
#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "UpdateAttributeDefinition")]
pub struct UpdateAttributeDefinitionDTO {
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[ts(optional, type = "string | null")]
    pub description: Option<Option<String>>,
    #[serde(default, with = "::serde_with::rust::double_option")]
    #[ts(optional, type = "Array<string> | null")]
    pub allowed_values: Option<Option<Vec<String>>>,
    #[serde(default)]
    #[ts(optional)]
    pub sync_to_keycloak: Option<bool>,
    #[serde(default)]
    #[ts(optional)]
    pub editable_by: Option<EditableByDTO>,
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

/// Tagged-union wire form for a single drift observation.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename = "DriftEntry")]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DriftEntryDTO {
    RegistryOnly {
        user_id: Uuid,
        idp_subject: String,
        attribute: String,
        value: String,
    },
    RegistryUnlinked {
        user_id: Uuid,
        attribute: String,
        value: String,
    },
    KeycloakOnly {
        idp_subject: String,
        attribute: String,
        value: String,
    },
    ValueMismatch {
        user_id: Uuid,
        idp_subject: String,
        attribute: String,
        registry_value: String,
        keycloak_value: String,
    },
}

impl From<DriftEntry> for DriftEntryDTO {
    fn from(e: DriftEntry) -> Self {
        match e {
            DriftEntry::RegistryOnly {
                user_id,
                idp_subject,
                attribute,
                value,
            } => Self::RegistryOnly {
                user_id: user_id.0,
                idp_subject: idp_subject.0,
                attribute: attribute.into_inner(),
                value: value.into_inner(),
            },
            DriftEntry::RegistryUnlinked {
                user_id,
                attribute,
                value,
            } => Self::RegistryUnlinked {
                user_id: user_id.0,
                attribute: attribute.into_inner(),
                value: value.into_inner(),
            },
            DriftEntry::KeycloakOnly {
                idp_subject,
                attribute,
                value,
            } => Self::KeycloakOnly {
                idp_subject: idp_subject.0,
                attribute: attribute.into_inner(),
                value: value.into_inner(),
            },
            DriftEntry::ValueMismatch {
                user_id,
                idp_subject,
                attribute,
                registry_value,
                keycloak_value,
            } => Self::ValueMismatch {
                user_id: user_id.0,
                idp_subject: idp_subject.0,
                attribute: attribute.into_inner(),
                registry_value: registry_value.into_inner(),
                keycloak_value: keycloak_value.into_inner(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, rename = "AttributeSyncStatus")]
pub struct AttributeSyncStatusDTO {
    pub entries: Vec<DriftEntryDTO>,
}

impl From<SyncStatus> for AttributeSyncStatusDTO {
    fn from(s: SyncStatus) -> Self {
        Self {
            entries: s.entries.into_iter().map(Into::into).collect(),
        }
    }
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
