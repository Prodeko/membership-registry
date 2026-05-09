use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::domain::{AttributeDefinition, AttributeValue, DriftEntry, EditableBy};

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

impl From<Vec<DriftEntry>> for AttributeSyncStatusDTO {
    fn from(entries: Vec<DriftEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AttributeName, IdpSubject, PersonId};

    fn av(s: &str) -> AttributeValue {
        AttributeValue::new(s).unwrap()
    }

    #[test]
    fn parse_allowed_values_ok_path() {
        let raw = Some(vec!["one".to_string(), "two".to_string()]);
        let parsed = parse_allowed_values(raw).unwrap().unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].as_str(), "one");
    }

    #[test]
    fn parse_allowed_values_rejects_empty_element() {
        let raw = Some(vec!["ok".to_string(), "".to_string()]);
        let parsed = parse_allowed_values(raw);
        assert!(parsed.is_err());
    }

    #[test]
    fn parse_allowed_values_rejects_control_char() {
        let raw = Some(vec!["ok\nbad".to_string()]);
        assert!(parse_allowed_values(raw).is_err());
    }

    #[test]
    fn parse_allowed_values_passes_through_none() {
        assert!(parse_allowed_values(None).unwrap().is_none());
    }

    #[test]
    fn editable_by_dto_round_trip() {
        for v in [EditableBy::Admin, EditableBy::User, EditableBy::Both] {
            let dto: EditableByDTO = v.into();
            let back: EditableBy = dto.into();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn editable_by_dto_serializes_lowercase() {
        let json = serde_json::to_string(&EditableByDTO::Admin).unwrap();
        assert_eq!(json, "\"admin\"");
        let json = serde_json::to_string(&EditableByDTO::User).unwrap();
        assert_eq!(json, "\"user\"");
        let json = serde_json::to_string(&EditableByDTO::Both).unwrap();
        assert_eq!(json, "\"both\"");
    }

    #[test]
    fn drift_entry_dto_uses_tagged_union() {
        let entry = DriftEntry::RegistryUnlinked {
            user_id: PersonId(uuid::Uuid::nil()),
            attribute: AttributeName::new("xq-year").unwrap(),
            value: av("IV"),
        };
        let dto: DriftEntryDTO = entry.into();
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["type"], "registry_unlinked");
        assert_eq!(json["attribute"], "xq-year");
        assert_eq!(json["value"], "IV");
    }

    #[test]
    fn drift_entry_dto_value_mismatch_carries_both_values() {
        let entry = DriftEntry::ValueMismatch {
            user_id: PersonId(uuid::Uuid::nil()),
            idp_subject: IdpSubject("kc-1".to_string()),
            attribute: AttributeName::new("xq-year").unwrap(),
            registry_value: av("IV"),
            keycloak_value: av("II"),
        };
        let dto: DriftEntryDTO = entry.into();
        let json = serde_json::to_value(&dto).unwrap();
        assert_eq!(json["type"], "value_mismatch");
        assert_eq!(json["registry_value"], "IV");
        assert_eq!(json["keycloak_value"], "II");
    }

    #[test]
    fn sync_status_dto_serializes_with_entries_field() {
        let entries: Vec<DriftEntry> = vec![DriftEntry::KeycloakOnly {
            idp_subject: IdpSubject("kc-1".to_string()),
            attribute: AttributeName::new("xq-year").unwrap(),
            value: av("IV"),
        }];
        let dto: AttributeSyncStatusDTO = entries.into();
        let json = serde_json::to_value(&dto).unwrap();
        let entries = json["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["type"], "keycloak_only");
    }

    #[test]
    fn update_attribute_definition_dto_omitted_field_means_leave() {
        // serde double_option: missing field deserializes to None.
        let json = "{}";
        let dto: UpdateAttributeDefinitionDTO = serde_json::from_str(json).unwrap();
        assert!(dto.description.is_none());
        assert!(dto.allowed_values.is_none());
        assert!(dto.sync_to_keycloak.is_none());
    }

    #[test]
    fn update_attribute_definition_dto_null_field_means_clear() {
        let json = r#"{"description": null}"#;
        let dto: UpdateAttributeDefinitionDTO = serde_json::from_str(json).unwrap();
        // double_option: null deserializes to Some(None) → clear.
        assert_eq!(dto.description, Some(None));
    }

    #[test]
    fn update_attribute_definition_dto_value_means_set() {
        let json = r#"{"description": "the year"}"#;
        let dto: UpdateAttributeDefinitionDTO = serde_json::from_str(json).unwrap();
        assert_eq!(dto.description, Some(Some("the year".to_string())));
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
/// `Vec<AttributeValue>`.
///
/// `Some(vec![])` is preserved as `Ok(Some(vec![]))`; the outer `Option` is
/// passed through verbatim. Callers that treat empty as "no constraint" must
/// collapse `Some(vec![])` to `None` themselves — `AttributeDefinition::new`
/// rejects it as invalid.
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
