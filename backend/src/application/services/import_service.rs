use std::collections::HashSet;
use std::sync::Arc;

use crate::application::ports::tabular_parse_port::{TabularParseError, TabularParsePort};
use crate::application::ports::user_admin_port::UserAdminPort;
use crate::application::services::attribute_service::AttributeService;
use crate::application::services::errors::ServiceResult;
use crate::application::services::member_service::MemberService;
use crate::application::services::role_service::RoleService;
use crate::domain::{AttributeDefinition, AttributeName, AttributeValue, EditableBy, Email};

const KNOWN_MEMBER_COLUMNS: &[&str] = &[
    "email",
    "first_name",
    "last_name",
    "home_municipality",
    "language",
    "email_notifications",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowAction {
    Create,
    Update,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberPreviewRow {
    pub line: usize,
    pub email: String,
    pub result: Result<RowAction, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberImportPreview {
    /// A file-level problem (unparseable, missing/unknown columns) that stops
    /// the whole import. When set, `rows` is empty.
    pub fatal_error: Option<String>,
    pub rows: Vec<MemberPreviewRow>,
}

impl MemberImportPreview {
    pub fn create_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.result == Ok(RowAction::Create))
            .count()
    }
    pub fn update_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.result == Ok(RowAction::Update))
            .count()
    }
    pub fn error_count(&self) -> usize {
        self.rows.iter().filter(|r| r.result.is_err()).count()
    }
}

/// Resolved column indices for a members CSV.
pub(crate) struct MemberColumns {
    pub email: usize,
    pub first_name: Option<usize>,
    pub last_name: Option<usize>,
    pub home_municipality: Option<usize>,
    pub language: Option<usize>,
    pub email_notifications: Option<usize>,
    pub attributes: Vec<(usize, AttributeName)>,
}

pub(crate) fn resolve_member_columns(
    headers: &[String],
    defs: &[AttributeDefinition],
) -> Result<MemberColumns, String> {
    let idx_of = |name: &str| headers.iter().position(|h| h.eq_ignore_ascii_case(name));
    let email = idx_of("email").ok_or_else(|| "missing required column: email".to_string())?;

    let mut attributes = Vec::new();
    let mut unknown = Vec::new();
    for (i, h) in headers.iter().enumerate() {
        if KNOWN_MEMBER_COLUMNS.contains(&h.to_ascii_lowercase().as_str()) {
            continue;
        }
        match AttributeName::new(h.clone()) {
            Ok(name) if defs.iter().any(|d| d.name() == &name) => attributes.push((i, name)),
            _ => unknown.push(h.clone()),
        }
    }
    if !unknown.is_empty() {
        return Err(format!("unknown column(s): {}", unknown.join(", ")));
    }

    Ok(MemberColumns {
        email,
        first_name: idx_of("first_name"),
        last_name: idx_of("last_name"),
        home_municipality: idx_of("home_municipality"),
        language: idx_of("language"),
        email_notifications: idx_of("email_notifications"),
        attributes,
    })
}

pub(crate) fn cell<'a>(idx: Option<usize>, rec: &'a [String]) -> Option<&'a str> {
    idx.map(|i| rec[i].as_str())
}

pub(crate) fn parse_bool(s: &str) -> Result<bool, ()> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(()),
    }
}

pub struct ImportService {
    parser: Arc<dyn TabularParsePort>,
    member_service: MemberService,
    attribute_service: Arc<AttributeService>,
    role_service: RoleService,
    user_admin: Arc<dyn UserAdminPort>,
}

impl ImportService {
    pub fn new(
        parser: Arc<dyn TabularParsePort>,
        member_service: MemberService,
        attribute_service: Arc<AttributeService>,
        role_service: RoleService,
        user_admin: Arc<dyn UserAdminPort>,
    ) -> Self {
        Self {
            parser,
            member_service,
            attribute_service,
            role_service,
            user_admin,
        }
    }

    pub async fn preview_members(&self, bytes: &[u8]) -> ServiceResult<MemberImportPreview> {
        let parsed = match self.parser.parse(bytes) {
            Ok(p) => p,
            Err(TabularParseError::Malformed(m)) => {
                return Ok(MemberImportPreview {
                    fatal_error: Some(format!("Could not parse CSV: {m}")),
                    rows: vec![],
                });
            }
        };

        let defs = self.attribute_service.list_definitions().await?;
        let cols = match resolve_member_columns(&parsed.headers, &defs) {
            Ok(c) => c,
            Err(e) => {
                return Ok(MemberImportPreview {
                    fatal_error: Some(e),
                    rows: vec![],
                });
            }
        };

        let mut rows = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for (i, rec) in parsed.records.iter().enumerate() {
            let email = rec[cols.email].trim().to_string();
            let result = self.classify_member_row(&cols, &defs, rec, &mut seen).await;
            rows.push(MemberPreviewRow {
                line: i + 1,
                email,
                result,
            });
        }
        Ok(MemberImportPreview {
            fatal_error: None,
            rows,
        })
    }

    async fn classify_member_row(
        &self,
        cols: &MemberColumns,
        defs: &[AttributeDefinition],
        rec: &[String],
        seen: &mut HashSet<String>,
    ) -> Result<RowAction, String> {
        let raw_email = rec[cols.email].trim();
        Email::new(raw_email.to_string()).map_err(|e| format!("invalid email: {e}"))?;
        if !seen.insert(raw_email.to_ascii_lowercase()) {
            return Err("duplicate email within file".into());
        }

        // Attribute cells: empty = no-op, "null" = clear, else validated.
        for (idx, name) in &cols.attributes {
            let value = &rec[*idx];
            if value.is_empty() || value == "null" {
                continue;
            }
            let def = defs
                .iter()
                .find(|d| d.name() == name)
                .ok_or_else(|| format!("unknown attribute '{}'", name.as_str()))?;
            if def.editable_by() == EditableBy::User {
                return Err(format!(
                    "attribute '{}' is user-editable and cannot be set via import",
                    name.as_str()
                ));
            }
            let parsed = AttributeValue::new(value.clone())
                .map_err(|e| format!("invalid value for '{}': {e:?}", name.as_str()))?;
            def.validate(&parsed)
                .map_err(|_| format!("value '{value}' not allowed for '{}'", name.as_str()))?;
        }

        if let Some(v) = cell(cols.email_notifications, rec) {
            if !v.trim().is_empty() && parse_bool(v).is_err() {
                return Err("email_notifications must be true/false".into());
            }
        }

        let existing = self
            .member_service
            .find_by_email(raw_email)
            .await
            .map_err(|e| format!("lookup failed: {e:?}"))?;

        if existing.is_some() {
            Ok(RowAction::Update)
        } else {
            if cell(cols.first_name, rec)
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                return Err("first_name required for new member".into());
            }
            if cell(cols.last_name, rec)
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            {
                return Err("last_name required for new member".into());
            }
            Ok(RowAction::Create)
        }
    }
}
