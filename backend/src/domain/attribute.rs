use crate::domain::{IdpSubject, PersonId};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AttributeName(String);

#[derive(Debug, PartialEq, Eq)]
pub enum InvalidAttributeName {
    Empty,
    InvalidChars,
    EdgeHyphen,
}

impl AttributeName {
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidAttributeName> {
        let s = s.into();
        if s.is_empty() {
            return Err(InvalidAttributeName::Empty);
        }
        if !s
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(InvalidAttributeName::InvalidChars);
        }
        if s.starts_with('-') || s.ends_with('-') {
            return Err(InvalidAttributeName::EdgeHyphen);
        }
        Ok(Self(s))
    }

    /// Construct from a trusted source (e.g. database) without validation.
    pub fn new_unchecked(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeValue(String);

#[derive(Debug, PartialEq, Eq)]
pub enum InvalidAttributeValue {
    Empty,
    TooLong,
    ControlChar,
}

/// Keycloak's practical attribute value limit; Postgres has no constraint
/// so this is the only enforcement point.
pub const ATTRIBUTE_VALUE_MAX_LEN: usize = 4096;

impl AttributeValue {
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidAttributeValue> {
        let s = s.into();
        if s.is_empty() {
            return Err(InvalidAttributeValue::Empty);
        }
        if s.len() > ATTRIBUTE_VALUE_MAX_LEN {
            return Err(InvalidAttributeValue::TooLong);
        }
        if s.chars().any(|c| c.is_control()) {
            return Err(InvalidAttributeValue::ControlChar);
        }
        Ok(Self(s))
    }

    pub fn new_unchecked(s: String) -> Self {
        Self(s)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditableBy {
    Admin,
    User,
    Both,
}

impl EditableBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::User => "user",
            Self::Both => "both",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeDefinition {
    name: AttributeName,
    description: Option<String>,
    allowed_values: Option<Vec<AttributeValue>>,
    sync_to_keycloak: bool,
    editable_by: EditableBy,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AttributeValidationError {
    NotInAllowedValues,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InvalidAttributeDefinition {
    EmptyAllowedValues,
}

impl AttributeDefinition {
    /// Constructs a definition, rejecting `Some(empty)` allowed_values.
    /// `None` means "any value", `Some(non_empty)` means "must be one of".
    pub fn new(
        name: AttributeName,
        description: Option<String>,
        allowed_values: Option<Vec<AttributeValue>>,
        sync_to_keycloak: bool,
        editable_by: EditableBy,
    ) -> Result<Self, InvalidAttributeDefinition> {
        if matches!(&allowed_values, Some(v) if v.is_empty()) {
            return Err(InvalidAttributeDefinition::EmptyAllowedValues);
        }
        Ok(Self {
            name,
            description,
            allowed_values,
            sync_to_keycloak,
            editable_by,
        })
    }

    /// Constructs from a trusted source (DB row) without validation. Empty
    /// `allowed_values` are normalized to `None`.
    pub fn new_unchecked(
        name: AttributeName,
        description: Option<String>,
        allowed_values: Option<Vec<AttributeValue>>,
        sync_to_keycloak: bool,
        editable_by: EditableBy,
    ) -> Self {
        let allowed_values = allowed_values.and_then(|v| if v.is_empty() { None } else { Some(v) });
        Self {
            name,
            description,
            allowed_values,
            sync_to_keycloak,
            editable_by,
        }
    }

    pub fn name(&self) -> &AttributeName {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn allowed_values(&self) -> Option<&[AttributeValue]> {
        self.allowed_values.as_deref()
    }

    pub fn sync_to_keycloak(&self) -> bool {
        self.sync_to_keycloak
    }

    pub fn editable_by(&self) -> EditableBy {
        self.editable_by
    }

    pub fn validate(&self, value: &AttributeValue) -> Result<(), AttributeValidationError> {
        match &self.allowed_values {
            None => Ok(()),
            Some(allowed) if allowed.iter().any(|v| v == value) => Ok(()),
            Some(_) => Err(AttributeValidationError::NotInAllowedValues),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberAttribute {
    pub user_id: PersonId,
    pub name: AttributeName,
    pub value: AttributeValue,
}

/// One observed disagreement between the registry and Keycloak for a single
/// `(subject, attribute)` pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriftEntry {
    /// Registry has a value the corresponding Keycloak user is missing.
    RegistryOnly {
        user_id: PersonId,
        idp_subject: IdpSubject,
        attribute: AttributeName,
        value: AttributeValue,
    },
    /// Registry has a value but the user has no linked identity provider, so
    /// no KC subject exists to compare against. These cannot be auto-synced
    /// without first linking the user to KC.
    RegistryUnlinked {
        user_id: PersonId,
        attribute: AttributeName,
        value: AttributeValue,
    },
    /// Keycloak has a value the registry doesn't track for any linked user.
    KeycloakOnly {
        idp_subject: IdpSubject,
        attribute: AttributeName,
        value: AttributeValue,
    },
    /// Both sides have a value but they disagree.
    ValueMismatch {
        user_id: PersonId,
        idp_subject: IdpSubject,
        attribute: AttributeName,
        registry_value: AttributeValue,
        keycloak_value: AttributeValue,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncStatus {
    pub entries: Vec<DriftEntry>,
}

impl SyncStatus {
    pub fn is_in_sync(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attribute_name_accepts_kebab_case() {
        assert!(AttributeName::new("xq-year").is_ok());
        assert!(AttributeName::new("membership-type").is_ok());
        assert!(AttributeName::new("a").is_ok());
    }

    #[test]
    fn attribute_name_rejects_invalid() {
        for s in ["", " ", "Xq-Year", "xq year", "xq_year", "-x", "x-"] {
            assert!(
                AttributeName::new(s).is_err(),
                "expected {s:?} to be rejected"
            );
        }
    }

    #[test]
    fn attribute_value_rejects_empty() {
        assert_eq!(AttributeValue::new(""), Err(InvalidAttributeValue::Empty));
        assert!(AttributeValue::new("IV").is_ok());
    }

    #[test]
    fn attribute_value_rejects_too_long() {
        let s = "a".repeat(ATTRIBUTE_VALUE_MAX_LEN + 1);
        assert_eq!(AttributeValue::new(s), Err(InvalidAttributeValue::TooLong));
        let s = "a".repeat(ATTRIBUTE_VALUE_MAX_LEN);
        assert!(AttributeValue::new(s).is_ok());
    }

    #[test]
    fn attribute_value_rejects_control_chars() {
        for c in ['\0', '\n', '\r', '\t', '\x07'] {
            let s = format!("ok{c}value");
            assert_eq!(
                AttributeValue::new(&s),
                Err(InvalidAttributeValue::ControlChar),
                "expected {s:?} to be rejected"
            );
        }
        assert!(AttributeValue::new("käytäntö ässät").is_ok());
    }

    fn av(s: &str) -> AttributeValue {
        AttributeValue::new(s).unwrap()
    }

    #[test]
    fn definition_validate_accepts_when_no_allowed_values() {
        let def = AttributeDefinition::new(
            AttributeName::new("note").unwrap(),
            None,
            None,
            false,
            EditableBy::Admin,
        )
        .unwrap();
        assert!(def.validate(&av("anything")).is_ok());
    }

    #[test]
    fn definition_validate_enforces_enum() {
        let def = AttributeDefinition::new(
            AttributeName::new("xq-year").unwrap(),
            None,
            Some(vec![av("I"), av("II"), av("IV")]),
            true,
            EditableBy::Admin,
        )
        .unwrap();
        assert!(def.validate(&av("IV")).is_ok());
        assert_eq!(
            def.validate(&av("V")),
            Err(AttributeValidationError::NotInAllowedValues)
        );
    }

    #[test]
    fn definition_rejects_empty_allowed_values() {
        let r = AttributeDefinition::new(
            AttributeName::new("note").unwrap(),
            None,
            Some(vec![]),
            false,
            EditableBy::Admin,
        );
        assert_eq!(r, Err(InvalidAttributeDefinition::EmptyAllowedValues));
    }
}
