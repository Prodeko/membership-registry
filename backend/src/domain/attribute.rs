use crate::domain::PersonId;

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
    pub name: AttributeName,
    pub description: Option<String>,
    pub allowed_values: Option<Vec<String>>,
    pub sync_to_keycloak: bool,
    pub editable_by: EditableBy,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AttributeValidationError {
    NotInAllowedValues,
}

impl AttributeDefinition {
    pub fn validate(&self, value: &AttributeValue) -> Result<(), AttributeValidationError> {
        match &self.allowed_values {
            None => Ok(()),
            Some(allowed) if allowed.iter().any(|v| v == value.as_str()) => Ok(()),
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

    #[test]
    fn definition_validate_accepts_when_no_allowed_values() {
        let def = AttributeDefinition {
            name: AttributeName::new("note").unwrap(),
            description: None,
            allowed_values: None,
            sync_to_keycloak: false,
            editable_by: EditableBy::Admin,
        };
        let v = AttributeValue::new("anything").unwrap();
        assert!(def.validate(&v).is_ok());
    }

    #[test]
    fn definition_validate_enforces_enum() {
        let def = AttributeDefinition {
            name: AttributeName::new("xq-year").unwrap(),
            description: None,
            allowed_values: Some(vec!["I".into(), "II".into(), "IV".into()]),
            sync_to_keycloak: true,
            editable_by: EditableBy::Admin,
        };
        assert!(def.validate(&AttributeValue::new("IV").unwrap()).is_ok());
        assert_eq!(
            def.validate(&AttributeValue::new("V").unwrap()),
            Err(AttributeValidationError::NotInAllowedValues)
        );
    }
}
