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
    /// Crate-internal — domain invariants only hold if construction goes
    /// through `new` for everything outside the trusted boundary.
    pub(crate) fn new_unchecked(s: String) -> Self {
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
    /// Whitespace-only, or leading/trailing whitespace. KC stores values
    /// verbatim and echoes them back, so accepting `"IV "` would silently
    /// mismatch any `allowed_values` entry of `"IV"`. We reject rather than
    /// trim — silent normalization breaks parse-don't-validate.
    Whitespace,
}

/// Keycloak's practical attribute value limit. Postgres has no length
/// constraint (only `value <> ''`), so this is the only length enforcement
/// point.
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
        if s.trim().is_empty() || s.trim().len() != s.len() {
            return Err(InvalidAttributeValue::Whitespace);
        }
        Ok(Self(s))
    }

    /// Construct from a trusted or external source without validation.
    /// Crate-internal — domain invariants only hold if construction goes
    /// through `new` for caller-supplied values. Used for DB rows and for
    /// KC-observed values during drift detection (where the registry's
    /// stricter rules can't be retroactively imposed).
    pub(crate) fn new_unchecked(s: String) -> Self {
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
    default_value: Option<AttributeValue>,
    sync_to_keycloak: bool,
    editable_by: EditableBy,
    /// Members must hold a value for this attribute: application forms can't
    /// be submitted without it and members can't clear it themselves.
    required: bool,
    /// Members may hold several values at once (a multichoice attribute).
    /// Single-valued attributes hold exactly one.
    multiple: bool,
    /// Members may give one free-text value outside `allowed_values` (an
    /// "other" choice). Only meaningful when `allowed_values` is set.
    allow_other: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AttributeValidationError {
    /// A member value must have at least one element; clearing is a
    /// separate operation.
    Empty,
    /// More than one value for an attribute that isn't `multiple`.
    TooManyValues,
    DuplicateValue(AttributeValue),
    NotInAllowedValues(AttributeValue),
    /// More than one value outside `allowed_values` on an `allow_other`
    /// attribute; only a single "other" entry is permitted.
    TooManyOtherValues,
}

#[derive(Debug, PartialEq, Eq)]
pub enum InvalidAttributeDefinition {
    EmptyAllowedValues,
    DefaultNotInAllowedValues,
}

impl AttributeDefinition {
    /// Constructs a definition. Rejects `Some(empty)` allowed_values, and a
    /// `default_value` that violates `allowed_values` when both are present.
    /// `None` allowed_values means "any value", `Some(non_empty)` means
    /// "must be one of".
    pub fn new(
        name: AttributeName,
        description: Option<String>,
        allowed_values: Option<Vec<AttributeValue>>,
        default_value: Option<AttributeValue>,
        sync_to_keycloak: bool,
        editable_by: EditableBy,
        required: bool,
    ) -> Result<Self, InvalidAttributeDefinition> {
        if matches!(&allowed_values, Some(v) if v.is_empty()) {
            return Err(InvalidAttributeDefinition::EmptyAllowedValues);
        }
        if let (Some(allowed), Some(default)) = (&allowed_values, &default_value) {
            if !allowed.iter().any(|v| v == default) {
                return Err(InvalidAttributeDefinition::DefaultNotInAllowedValues);
            }
        }
        Ok(Self {
            name,
            description,
            allowed_values,
            default_value,
            sync_to_keycloak,
            editable_by,
            required,
            multiple: false,
            allow_other: false,
        })
    }

    /// Constructs from a trusted source (DB row) without validation. Empty
    /// `allowed_values` are normalized to `None`. Crate-internal.
    pub(crate) fn new_unchecked(
        name: AttributeName,
        description: Option<String>,
        allowed_values: Option<Vec<AttributeValue>>,
        default_value: Option<AttributeValue>,
        sync_to_keycloak: bool,
        editable_by: EditableBy,
        required: bool,
    ) -> Self {
        let allowed_values = allowed_values.filter(|v| !v.is_empty());
        Self {
            name,
            description,
            allowed_values,
            default_value,
            sync_to_keycloak,
            editable_by,
            required,
            multiple: false,
            allow_other: false,
        }
    }

    /// Marks the definition multichoice. Kept off the constructors so that
    /// the many single-valued call sites don't grow another positional flag.
    pub fn with_multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Lets members give one value outside `allowed_values`. Like
    /// `with_multiple`, kept off the constructors.
    pub fn with_allow_other(mut self, allow_other: bool) -> Self {
        self.allow_other = allow_other;
        self
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

    pub fn default_value(&self) -> Option<&AttributeValue> {
        self.default_value.as_ref()
    }

    pub fn sync_to_keycloak(&self) -> bool {
        self.sync_to_keycloak
    }

    pub fn editable_by(&self) -> EditableBy {
        self.editable_by
    }

    pub fn required(&self) -> bool {
        self.required
    }

    pub fn multiple(&self) -> bool {
        self.multiple
    }

    pub fn allow_other(&self) -> bool {
        self.allow_other
    }

    /// Validates a complete set of values for one member: non-empty, at most
    /// one unless `multiple`, no duplicates, and each within `allowed_values`
    /// except for a single "other" value when `allow_other` is set.
    pub fn validate(&self, values: &[AttributeValue]) -> Result<(), AttributeValidationError> {
        if values.is_empty() {
            return Err(AttributeValidationError::Empty);
        }
        if !self.multiple && values.len() > 1 {
            return Err(AttributeValidationError::TooManyValues);
        }
        let mut others = 0;
        for (i, value) in values.iter().enumerate() {
            if values[..i].contains(value) {
                return Err(AttributeValidationError::DuplicateValue(value.clone()));
            }
            if let Some(allowed) = &self.allowed_values {
                if !allowed.contains(value) {
                    if !self.allow_other {
                        return Err(AttributeValidationError::NotInAllowedValues(value.clone()));
                    }
                    others += 1;
                }
            }
        }
        if others > 1 {
            return Err(AttributeValidationError::TooManyOtherValues);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberAttribute {
    pub user_id: PersonId,
    pub name: AttributeName,
    /// Never empty; exactly one element unless the definition is `multiple`.
    pub values: Vec<AttributeValue>,
}

/// Order-insensitive equality of two value lists. Keycloak keeps values in
/// insertion order, but the registry doesn't treat order as meaningful, so
/// drift detection must not flag a reordering as a mismatch.
pub fn same_values(a: &[AttributeValue], b: &[AttributeValue]) -> bool {
    let mut a: Vec<&str> = a.iter().map(AttributeValue::as_str).collect();
    let mut b: Vec<&str> = b.iter().map(AttributeValue::as_str).collect();
    a.sort_unstable();
    b.sort_unstable();
    a == b
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
        values: Vec<AttributeValue>,
    },
    /// Registry has a value but the user has no linked identity provider, so
    /// no KC subject exists to compare against. These cannot be auto-synced
    /// without first linking the user to KC.
    RegistryUnlinked {
        user_id: PersonId,
        attribute: AttributeName,
        values: Vec<AttributeValue>,
    },
    /// Keycloak has a value the registry doesn't track for any linked user.
    KeycloakOnly {
        idp_subject: IdpSubject,
        attribute: AttributeName,
        values: Vec<AttributeValue>,
    },
    /// Both sides have values but they disagree (compared as sets).
    ValueMismatch {
        user_id: PersonId,
        idp_subject: IdpSubject,
        attribute: AttributeName,
        registry_values: Vec<AttributeValue>,
        keycloak_values: Vec<AttributeValue>,
    },
    /// Keycloak holds more than one value for an attribute the registry
    /// treats as single-valued. The registry can't safely overwrite without
    /// clobbering data; admins must clean up the extras in Keycloak.
    KeycloakMultivalued {
        idp_subject: IdpSubject,
        attribute: AttributeName,
        values: Vec<AttributeValue>,
    },
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
    fn attribute_value_rejects_whitespace_only() {
        for s in [" ", "   ", "\u{00A0}", " \u{00A0} "] {
            assert_eq!(
                AttributeValue::new(s),
                Err(InvalidAttributeValue::Whitespace),
                "expected {s:?} to be rejected as whitespace-only"
            );
        }
    }

    #[test]
    fn attribute_value_rejects_leading_or_trailing_whitespace() {
        for s in [" IV", "IV ", " IV "] {
            assert_eq!(
                AttributeValue::new(s),
                Err(InvalidAttributeValue::Whitespace),
                "expected {s:?} to be rejected for edge whitespace"
            );
        }
        // Internal whitespace is preserved verbatim — only edges are rejected.
        assert!(AttributeValue::new("käytäntö ässät").is_ok());
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
            None,
            false,
            EditableBy::Admin,
            false,
        )
        .unwrap();
        assert!(def.validate(&[av("anything")]).is_ok());
    }

    #[test]
    fn definition_validate_enforces_enum() {
        let def = AttributeDefinition::new(
            AttributeName::new("xq-year").unwrap(),
            None,
            Some(vec![av("I"), av("II"), av("IV")]),
            None,
            true,
            EditableBy::Admin,
            false,
        )
        .unwrap();
        assert!(def.validate(&[av("IV")]).is_ok());
        assert_eq!(
            def.validate(&[av("V")]),
            Err(AttributeValidationError::NotInAllowedValues(av("V")))
        );
    }

    #[test]
    fn validate_value_lists() {
        use AttributeValidationError::*;
        let languages = |multiple: bool, allow_other: bool| {
            AttributeDefinition::new(
                AttributeName::new("languages").unwrap(),
                None,
                Some(vec![av("fi"), av("sv"), av("en")]),
                None,
                false,
                EditableBy::Both,
                false,
            )
            .unwrap()
            .with_multiple(multiple)
            .with_allow_other(allow_other)
        };
        // (multiple, allow_other, values, expected)
        let cases: &[(bool, bool, &[&str], Result<(), AttributeValidationError>)] = &[
            (true, false, &[], Err(Empty)),
            (false, false, &["fi", "en"], Err(TooManyValues)),
            (true, false, &["fi", "en"], Ok(())),
            (
                true,
                false,
                &["fi", "de"],
                Err(NotInAllowedValues(av("de"))),
            ),
            (true, false, &["fi", "fi"], Err(DuplicateValue(av("fi")))),
            (false, true, &["de"], Ok(())),
            (true, true, &["fi", "en", "de"], Ok(())),
            (true, true, &["fi", "de", "fr"], Err(TooManyOtherValues)),
            (true, true, &["de", "de"], Err(DuplicateValue(av("de")))),
            (false, true, &["fi", "de"], Err(TooManyValues)),
        ];
        for (multiple, allow_other, values, expected) in cases {
            let values: Vec<_> = values.iter().map(|v| av(v)).collect();
            assert_eq!(
                &languages(*multiple, *allow_other).validate(&values),
                expected,
                "multiple={multiple} allow_other={allow_other} {values:?}"
            );
        }
    }

    #[test]
    fn same_values_ignores_order() {
        assert!(same_values(&[av("fi"), av("en")], &[av("en"), av("fi")]));
        assert!(!same_values(&[av("fi")], &[av("fi"), av("en")]));
    }

    #[test]
    fn definition_rejects_empty_allowed_values() {
        let r = AttributeDefinition::new(
            AttributeName::new("note").unwrap(),
            None,
            Some(vec![]),
            None,
            false,
            EditableBy::Admin,
            false,
        );
        assert_eq!(r, Err(InvalidAttributeDefinition::EmptyAllowedValues));
    }

    #[test]
    fn definition_accepts_default_within_allowed_values() {
        let def = AttributeDefinition::new(
            AttributeName::new("membership-type").unwrap(),
            None,
            Some(vec![av("true"), av("external")]),
            Some(av("external")),
            true,
            EditableBy::Admin,
            false,
        )
        .unwrap();
        assert_eq!(
            def.default_value().map(AttributeValue::as_str),
            Some("external")
        );
    }

    #[test]
    fn definition_rejects_default_outside_allowed_values() {
        let r = AttributeDefinition::new(
            AttributeName::new("membership-type").unwrap(),
            None,
            Some(vec![av("true"), av("external")]),
            Some(av("other")),
            true,
            EditableBy::Admin,
            false,
        );
        assert_eq!(
            r,
            Err(InvalidAttributeDefinition::DefaultNotInAllowedValues)
        );
    }

    #[test]
    fn definition_accepts_default_with_no_allowed_values() {
        let def = AttributeDefinition::new(
            AttributeName::new("note").unwrap(),
            None,
            None,
            Some(av("anything goes")),
            false,
            EditableBy::Admin,
            false,
        )
        .unwrap();
        assert_eq!(
            def.default_value().map(AttributeValue::as_str),
            Some("anything goes")
        );
    }
}
