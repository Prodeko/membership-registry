use std::collections::HashSet;
use std::sync::Arc;

use chrono::NaiveDate;

use crate::application::ports::attribute_repository_port::CreateAttributeDefinition;
use crate::application::ports::tabular_parse_port::{TabularParseError, TabularParsePort};
use crate::application::ports::user_admin_port::UserAdminPort;
use crate::application::services::attribute_service::{
    AttributeService, UpdateAttributeDefinitionPatch,
};
use crate::application::services::errors::ServiceResult;
use crate::application::services::member_service::MemberService;
use crate::application::services::role_service::RoleService;
use crate::domain::{
    AttributeDefinition, AttributeName, AttributeValue, EditableBy, Email, NewPerson, Patch,
    Person, PersonId, UpdatePersonData,
};
use uuid::Uuid;

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
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberPreviewRow {
    pub line: usize,
    pub email: String,
    pub result: Result<RowAction, String>,
    /// Human-readable `field: old → new` entries; non-empty only for updates.
    pub changes: Vec<String>,
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
    pub fn unchanged_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.result == Ok(RowAction::Unchanged))
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

pub(crate) fn cell(idx: Option<usize>, rec: &[String]) -> Option<&str> {
    idx.map(|i| rec[i].as_str())
}

/// Language of Keycloak-sent emails for this row: only `en` is honored,
/// everything else (including absent) falls back to `fi`.
fn invite_locale(language_cell: Option<&str>) -> &'static str {
    match language_cell.map(str::trim) {
        Some(l) if l.eq_ignore_ascii_case("en") => "en",
        _ => "fi",
    }
}

pub(crate) fn parse_bool(s: &str) -> Result<bool, ()> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => Err(()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowOutcome {
    Created,
    Updated,
    Unchanged,
    Skipped(String),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberResultRow {
    pub line: usize,
    pub email: String,
    pub outcome: RowOutcome,
    pub warning: Option<String>,
    /// Human-readable `field: old → new` entries; non-empty only for updates.
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberImportReport {
    pub fatal_error: Option<String>,
    pub rows: Vec<MemberResultRow>,
}

impl MemberImportReport {
    pub fn count(&self, want: &RowOutcome) -> usize {
        self.rows.iter().filter(|r| &r.outcome == want).count()
    }
}

fn build_new_person(cols: &MemberColumns, rec: &[String], id: Uuid) -> Result<NewPerson, String> {
    let email = Email::new(rec[cols.email].trim().to_string())
        .map_err(|e| format!("invalid email: {e}"))?;
    let present = |idx: Option<usize>| cell(idx, rec).map(str::trim).filter(|s| !s.is_empty());
    Ok(NewPerson {
        id: PersonId(id),
        email,
        first_name: present(cols.first_name).unwrap_or("").to_string(),
        last_name: present(cols.last_name).unwrap_or("").to_string(),
        home_municipality: present(cols.home_municipality).map(String::from),
        email_notifications: present(cols.email_notifications)
            .map(|s| parse_bool(s).unwrap_or(true))
            .unwrap_or(true),
        language: present(cols.language).unwrap_or("fi").to_string(),
    })
}

fn build_update_data(cols: &MemberColumns, rec: &[String], current: &Person) -> UpdatePersonData {
    let present = |idx: Option<usize>| cell(idx, rec).map(str::trim).filter(|s| !s.is_empty());
    UpdatePersonData {
        first_name: present(cols.first_name)
            .map(String::from)
            .unwrap_or_else(|| current.first_name.clone()),
        last_name: present(cols.last_name)
            .map(String::from)
            .unwrap_or_else(|| current.last_name.clone()),
        home_municipality: present(cols.home_municipality)
            .map(String::from)
            .or_else(|| current.home_municipality.clone()),
        email_notifications: present(cols.email_notifications)
            .and_then(|s| parse_bool(s).ok())
            .unwrap_or(current.email_notifications),
        language: present(cols.language)
            .map(String::from)
            .unwrap_or_else(|| current.language.clone()),
        email: None, // email is the match key; import never changes it
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
            let (result, changes) =
                match self.classify_member_row(&cols, &defs, rec, &mut seen).await {
                    Ok((action, changes)) => (Ok(action), changes),
                    Err(e) => (Err(e), vec![]),
                };
            rows.push(MemberPreviewRow {
                line: i + 1,
                email,
                result,
                changes,
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
    ) -> Result<(RowAction, Vec<String>), String> {
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

        if let Some(current) = existing {
            let changes = self.member_changes(cols, rec, &current).await?;
            if changes.is_empty() {
                Ok((RowAction::Unchanged, changes))
            } else {
                Ok((RowAction::Update, changes))
            }
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
            Ok((RowAction::Create, vec![]))
        }
    }

    /// `field: old → new` entries for everything an update row would modify.
    /// Empty cells never count as changes (they keep the current value).
    async fn member_changes(
        &self,
        cols: &MemberColumns,
        rec: &[String],
        current: &Person,
    ) -> Result<Vec<String>, String> {
        const EMPTY: &str = "(empty)";
        let present = |idx: Option<usize>| cell(idx, rec).map(str::trim).filter(|s| !s.is_empty());
        let mut changes = Vec::new();

        let mut field = |name: &str, incoming: Option<&str>, current: Option<&str>| {
            if let Some(new) = incoming {
                if Some(new) != current {
                    changes.push(format!("{name}: {} → {new}", current.unwrap_or(EMPTY)));
                }
            }
        };
        field(
            "first_name",
            present(cols.first_name),
            Some(&current.first_name),
        );
        field(
            "last_name",
            present(cols.last_name),
            Some(&current.last_name),
        );
        field(
            "home_municipality",
            present(cols.home_municipality),
            current.home_municipality.as_deref(),
        );
        field("language", present(cols.language), Some(&current.language));

        if let Some(new) = present(cols.email_notifications).and_then(|s| parse_bool(s).ok()) {
            if new != current.email_notifications {
                changes.push(format!(
                    "email_notifications: {} → {new}",
                    current.email_notifications
                ));
            }
        }

        if !cols.attributes.is_empty() {
            let current_values = self
                .attribute_service
                .fetch_for_member(current.id.clone())
                .await
                .map_err(|e| format!("attribute lookup failed: {e:?}"))?;
            for (idx, name) in &cols.attributes {
                let value = rec[*idx].as_str();
                if value.is_empty() {
                    continue;
                }
                let old = current_values
                    .iter()
                    .find(|a| &a.name == name)
                    .map(|a| a.value.as_str());
                if value == "null" {
                    if let Some(old) = old {
                        changes.push(format!("{}: {old} → (cleared)", name.as_str()));
                    }
                } else if old != Some(value) {
                    changes.push(format!(
                        "{}: {} → {value}",
                        name.as_str(),
                        old.unwrap_or(EMPTY)
                    ));
                }
            }
        }
        Ok(changes)
    }

    pub async fn apply_members(
        &self,
        bytes: &[u8],
        send_invites: bool,
        actor: Option<Uuid>,
    ) -> ServiceResult<MemberImportReport> {
        let parsed = match self.parser.parse(bytes) {
            Ok(p) => p,
            Err(TabularParseError::Malformed(m)) => {
                return Ok(MemberImportReport {
                    fatal_error: Some(format!("Could not parse CSV: {m}")),
                    rows: vec![],
                });
            }
        };
        let defs = self.attribute_service.list_definitions().await?;
        let cols = match resolve_member_columns(&parsed.headers, &defs) {
            Ok(c) => c,
            Err(e) => {
                return Ok(MemberImportReport {
                    fatal_error: Some(e),
                    rows: vec![],
                });
            }
        };

        let mut rows = Vec::new();
        let mut seen = HashSet::new();
        for (i, rec) in parsed.records.iter().enumerate() {
            let email = rec[cols.email].trim().to_string();
            let (outcome, warning, changes) =
                match self.classify_member_row(&cols, &defs, rec, &mut seen).await {
                    Err(reason) => (RowOutcome::Skipped(reason), None, vec![]),
                    Ok((RowAction::Create, _)) => {
                        let (outcome, warning) =
                            self.apply_create(&cols, rec, send_invites, actor).await;
                        (outcome, warning, vec![])
                    }
                    Ok((RowAction::Update, changes)) => {
                        let (outcome, warning) = self.apply_update(&cols, rec, actor).await;
                        (outcome, warning, changes)
                    }
                    Ok((RowAction::Unchanged, _)) => (RowOutcome::Unchanged, None, vec![]),
                };
            rows.push(MemberResultRow {
                line: i + 1,
                email,
                outcome,
                warning,
                changes,
            });
        }
        Ok(MemberImportReport {
            fatal_error: None,
            rows,
        })
    }

    async fn apply_create(
        &self,
        cols: &MemberColumns,
        rec: &[String],
        send_invites: bool,
        actor: Option<Uuid>,
    ) -> (RowOutcome, Option<String>) {
        let raw_email = rec[cols.email].trim();
        let first = cell(cols.first_name, rec).unwrap_or("").trim();
        let last = cell(cols.last_name, rec).unwrap_or("").trim();
        let locale = invite_locale(cell(cols.language, rec));

        let mut warning = None;
        let subject = match self.user_admin.find_by_email(raw_email).await {
            Ok(Some(s)) => {
                // The account predates this import; align its locale with the
                // CSV before any invite email decides its language from it.
                if let Err(e) = self.user_admin.update_user_locale(&s, locale).await {
                    warning = Some(format!("locale sync failed: {e:?}"));
                }
                s
            }
            Ok(None) => match self
                .user_admin
                .create_user(raw_email, first, last, locale)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    return (
                        RowOutcome::Failed(format!("keycloak create failed: {e:?}")),
                        None,
                    );
                }
            },
            Err(e) => {
                return (
                    RowOutcome::Failed(format!("keycloak lookup failed: {e:?}")),
                    None,
                );
            }
        };
        let id = match Uuid::parse_str(&subject) {
            Ok(u) => u,
            Err(_) => {
                return (
                    RowOutcome::Failed("keycloak subject is not a uuid".into()),
                    None,
                );
            }
        };
        let new_person = match build_new_person(cols, rec, id) {
            Ok(p) => p,
            Err(e) => return (RowOutcome::Failed(e), None),
        };
        if let Err(e) = self
            .member_service
            .provision_member(new_person, &subject, actor)
            .await
        {
            return (RowOutcome::Failed(format!("provision failed: {e:?}")), None);
        }
        if let Err(e) = self.apply_attributes(PersonId(id), cols, rec, actor).await {
            return (RowOutcome::Failed(e), None);
        }

        if send_invites {
            let actions = vec!["UPDATE_PASSWORD".to_string(), "VERIFY_EMAIL".to_string()];
            if let Err(e) = self
                .user_admin
                .send_required_actions_email(&subject, &actions)
                .await
            {
                let msg = format!("invite email failed: {e:?}");
                warning = Some(match warning {
                    Some(w) => format!("{w}; {msg}"),
                    None => msg,
                });
            }
        }
        (RowOutcome::Created, warning)
    }

    async fn apply_update(
        &self,
        cols: &MemberColumns,
        rec: &[String],
        actor: Option<Uuid>,
    ) -> (RowOutcome, Option<String>) {
        let raw_email = rec[cols.email].trim();
        let current = match self.member_service.find_by_email(raw_email).await {
            Ok(Some(p)) => p,
            Ok(None) => {
                return (
                    RowOutcome::Failed("member disappeared before update".into()),
                    None,
                );
            }
            Err(e) => return (RowOutcome::Failed(format!("lookup failed: {e:?}")), None),
        };
        let data = build_update_data(cols, rec, &current);
        if let Err(e) = self
            .member_service
            .update_member(current.id.0, data, actor)
            .await
        {
            return (RowOutcome::Failed(format!("update failed: {e:?}")), None);
        }
        if let Err(e) = self
            .apply_attributes(current.id.clone(), cols, rec, actor)
            .await
        {
            return (RowOutcome::Failed(e), None);
        }
        (RowOutcome::Updated, None)
    }

    async fn apply_attributes(
        &self,
        user_id: PersonId,
        cols: &MemberColumns,
        rec: &[String],
        actor: Option<Uuid>,
    ) -> Result<(), String> {
        for (idx, name) in &cols.attributes {
            let value = &rec[*idx];
            if value.is_empty() {
                continue;
            }
            if value == "null" {
                self.attribute_service
                    .clear_as_admin(user_id.clone(), name, actor)
                    .await
                    .map_err(|e| format!("clear '{}' failed: {e:?}", name.as_str()))?;
            } else {
                let parsed = AttributeValue::new(value.clone())
                    .map_err(|e| format!("invalid value for '{}': {e:?}", name.as_str()))?;
                self.attribute_service
                    .set_as_admin(user_id.clone(), name, parsed, actor)
                    .await
                    .map_err(|e| format!("set '{}' failed: {e:?}", name.as_str()))?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolePreviewRow {
    pub line: usize,
    pub email: String,
    pub role_name: String,
    pub result: Result<RowAction, String>,
    /// Human-readable `field: old → new` entries; non-empty only for updates.
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleImportPreview {
    pub fatal_error: Option<String>,
    pub rows: Vec<RolePreviewRow>,
}

impl RoleImportPreview {
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
    pub fn unchanged_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.result == Ok(RowAction::Unchanged))
            .count()
    }
    pub fn error_count(&self) -> usize {
        self.rows.iter().filter(|r| r.result.is_err()).count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleResultRow {
    pub line: usize,
    pub email: String,
    pub role_name: String,
    pub outcome: RowOutcome,
    /// Human-readable `field: old → new` entries; non-empty only for updates.
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleImportReport {
    pub fatal_error: Option<String>,
    pub rows: Vec<RoleResultRow>,
}

impl RoleImportReport {
    pub fn count(&self, want: &RowOutcome) -> usize {
        self.rows.iter().filter(|r| &r.outcome == want).count()
    }
}

struct RoleColumns {
    email: usize,
    role_name: usize,
    valid_from: usize,
    valid_until: Option<usize>,
}

fn resolve_role_columns(headers: &[String]) -> Result<RoleColumns, String> {
    let idx = |n: &str| headers.iter().position(|h| h.eq_ignore_ascii_case(n));
    Ok(RoleColumns {
        email: idx("email").ok_or_else(|| "missing required column: email".to_string())?,
        role_name: idx("role_name")
            .ok_or_else(|| "missing required column: role_name".to_string())?,
        valid_from: idx("valid_from")
            .ok_or_else(|| "missing required column: valid_from".to_string())?,
        valid_until: idx("valid_until"),
    })
}

struct RoleRowData {
    user_id: Uuid,
    role_name: String,
    valid_from: NaiveDate,
    valid_until: Option<NaiveDate>,
}

impl ImportService {
    pub async fn preview_roles(&self, bytes: &[u8]) -> ServiceResult<RoleImportPreview> {
        let parsed = match self.parser.parse(bytes) {
            Ok(p) => p,
            Err(TabularParseError::Malformed(m)) => {
                return Ok(RoleImportPreview {
                    fatal_error: Some(format!("Could not parse CSV: {m}")),
                    rows: vec![],
                });
            }
        };
        let cols = match resolve_role_columns(&parsed.headers) {
            Ok(c) => c,
            Err(e) => {
                return Ok(RoleImportPreview {
                    fatal_error: Some(e),
                    rows: vec![],
                });
            }
        };
        let role_names = self.role_name_set().await?;

        let mut rows = Vec::new();
        for (i, rec) in parsed.records.iter().enumerate() {
            let email = rec[cols.email].trim().to_string();
            let role_name = rec[cols.role_name].trim().to_string();
            let (result, changes) = match self.parse_role_row(&cols, rec, &role_names).await {
                Ok(data) => match self.classify_role_row(&data).await {
                    Ok((action, changes)) => (Ok(action), changes),
                    Err(e) => (Err(e), vec![]),
                },
                Err(e) => (Err(e), vec![]),
            };
            rows.push(RolePreviewRow {
                line: i + 1,
                email,
                role_name,
                result,
                changes,
            });
        }
        Ok(RoleImportPreview {
            fatal_error: None,
            rows,
        })
    }

    pub async fn apply_roles(
        &self,
        bytes: &[u8],
        actor: Option<Uuid>,
    ) -> ServiceResult<RoleImportReport> {
        let parsed = match self.parser.parse(bytes) {
            Ok(p) => p,
            Err(TabularParseError::Malformed(m)) => {
                return Ok(RoleImportReport {
                    fatal_error: Some(format!("Could not parse CSV: {m}")),
                    rows: vec![],
                });
            }
        };
        let cols = match resolve_role_columns(&parsed.headers) {
            Ok(c) => c,
            Err(e) => {
                return Ok(RoleImportReport {
                    fatal_error: Some(e),
                    rows: vec![],
                });
            }
        };
        let role_names = self.role_name_set().await?;

        let mut rows = Vec::new();
        for (i, rec) in parsed.records.iter().enumerate() {
            let email = rec[cols.email].trim().to_string();
            let role_name = rec[cols.role_name].trim().to_string();
            let (outcome, changes) = match self.parse_role_row(&cols, rec, &role_names).await {
                Err(e) => (RowOutcome::Skipped(e), vec![]),
                Ok(data) => match self.classify_role_row(&data).await {
                    Err(e) => (RowOutcome::Skipped(e), vec![]),
                    Ok((RowAction::Unchanged, _)) => (RowOutcome::Unchanged, vec![]),
                    Ok((action, changes)) => match self
                        .role_service
                        .upsert_role_member(
                            data.user_id,
                            &data.role_name,
                            data.valid_from,
                            data.valid_until,
                            actor,
                        )
                        .await
                    {
                        Ok(()) => match action {
                            RowAction::Update => (RowOutcome::Updated, changes),
                            _ => (RowOutcome::Created, vec![]),
                        },
                        Err(e) => (RowOutcome::Failed(format!("assign failed: {e:?}")), vec![]),
                    },
                },
            };
            rows.push(RoleResultRow {
                line: i + 1,
                email,
                role_name,
                outcome,
                changes,
            });
        }
        Ok(RoleImportReport {
            fatal_error: None,
            rows,
        })
    }

    async fn role_name_set(&self) -> ServiceResult<HashSet<String>> {
        let roles = self.role_service.get_all_roles().await?;
        Ok(roles.into_iter().map(|r| r.name.0).collect())
    }

    async fn parse_role_row(
        &self,
        cols: &RoleColumns,
        rec: &[String],
        role_names: &HashSet<String>,
    ) -> Result<RoleRowData, String> {
        let email = rec[cols.email].trim();
        let member = self
            .member_service
            .find_by_email(email)
            .await
            .map_err(|e| format!("lookup failed: {e:?}"))?
            .ok_or_else(|| "no member with this email".to_string())?;

        let role_name = rec[cols.role_name].trim().to_string();
        if !role_names.contains(&role_name) {
            return Err(format!("unknown role '{role_name}'"));
        }

        let valid_from = NaiveDate::parse_from_str(rec[cols.valid_from].trim(), "%Y-%m-%d")
            .map_err(|_| "valid_from must be YYYY-MM-DD".to_string())?;
        let valid_until = match cell(cols.valid_until, rec)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(s) => Some(
                NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .map_err(|_| "valid_until must be YYYY-MM-DD".to_string())?,
            ),
            None => None,
        };
        if let Some(vu) = valid_until {
            if vu < valid_from {
                return Err("valid_until is before valid_from".into());
            }
        }
        Ok(RoleRowData {
            user_id: member.id.0,
            role_name,
            valid_from,
            valid_until,
        })
    }

    async fn classify_role_row(
        &self,
        data: &RoleRowData,
    ) -> Result<(RowAction, Vec<String>), String> {
        let existing = self
            .role_service
            .get_member_roles(data.user_id)
            .await
            .map_err(|e| format!("lookup failed: {e:?}"))?;
        let Some(current) = existing
            .iter()
            .find(|m| m.role_name.0 == data.role_name && m.valid_from == data.valid_from)
        else {
            return Ok((RowAction::Create, vec![]));
        };
        if current.valid_until == data.valid_until {
            return Ok((RowAction::Unchanged, vec![]));
        }
        let fmt = |d: Option<NaiveDate>| {
            d.map(|d| d.to_string())
                .unwrap_or_else(|| "(empty)".to_string())
        };
        Ok((
            RowAction::Update,
            vec![format!(
                "valid_until: {} → {}",
                fmt(current.valid_until),
                fmt(data.valid_until)
            )],
        ))
    }
}

// ---------------------------------------------------------------------------
// Attribute definition import
// ---------------------------------------------------------------------------

const KNOWN_ATTRIBUTE_COLUMNS: &[&str] = &[
    "name",
    "description",
    "allowed_values",
    "default_value",
    "editable_by",
    "sync_to_keycloak",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributePreviewRow {
    pub line: usize,
    pub name: String,
    pub result: Result<RowAction, String>,
    /// Human-readable `field: old → new` entries; non-empty only for updates.
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeImportPreview {
    pub fatal_error: Option<String>,
    pub rows: Vec<AttributePreviewRow>,
}

impl AttributeImportPreview {
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
    pub fn unchanged_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.result == Ok(RowAction::Unchanged))
            .count()
    }
    pub fn error_count(&self) -> usize {
        self.rows.iter().filter(|r| r.result.is_err()).count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeResultRow {
    pub line: usize,
    pub name: String,
    pub outcome: RowOutcome,
    /// Human-readable `field: old → new` entries; non-empty only for updates.
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeImportReport {
    pub fatal_error: Option<String>,
    pub rows: Vec<AttributeResultRow>,
}

impl AttributeImportReport {
    pub fn count(&self, want: &RowOutcome) -> usize {
        self.rows.iter().filter(|r| &r.outcome == want).count()
    }
}

struct AttributeColumns {
    name: usize,
    description: Option<usize>,
    allowed_values: Option<usize>,
    default_value: Option<usize>,
    editable_by: Option<usize>,
    sync_to_keycloak: Option<usize>,
}

fn resolve_attribute_columns(headers: &[String]) -> Result<AttributeColumns, String> {
    let idx = |n: &str| headers.iter().position(|h| h.eq_ignore_ascii_case(n));
    let unknown: Vec<String> = headers
        .iter()
        .filter(|h| !KNOWN_ATTRIBUTE_COLUMNS.contains(&h.to_ascii_lowercase().as_str()))
        .cloned()
        .collect();
    if !unknown.is_empty() {
        return Err(format!("unknown column(s): {}", unknown.join(", ")));
    }
    Ok(AttributeColumns {
        name: idx("name").ok_or_else(|| "missing required column: name".to_string())?,
        description: idx("description"),
        allowed_values: idx("allowed_values"),
        default_value: idx("default_value"),
        editable_by: idx("editable_by"),
        sync_to_keycloak: idx("sync_to_keycloak"),
    })
}

/// Parsed row intent. Field cells follow the shared import convention:
/// empty = keep current (or unset on create), `null` = clear.
struct AttributeRowData {
    name: AttributeName,
    description: Patch<String>,
    allowed_values: Patch<Vec<AttributeValue>>,
    default_value: Patch<AttributeValue>,
    editable_by: Option<EditableBy>,
    sync_to_keycloak: Option<bool>,
}

fn parse_attribute_row(
    cols: &AttributeColumns,
    rec: &[String],
    seen: &mut HashSet<String>,
) -> Result<AttributeRowData, String> {
    let raw_name = rec[cols.name].trim();
    let name =
        AttributeName::new(raw_name.to_string()).map_err(|e| format!("invalid name: {e:?}"))?;
    if !seen.insert(raw_name.to_ascii_lowercase()) {
        return Err("duplicate name within file".into());
    }
    let raw = |idx: Option<usize>| cell(idx, rec).map(str::trim).filter(|s| !s.is_empty());

    let description = match raw(cols.description) {
        None => Patch::Leave,
        Some("null") => Patch::Clear,
        Some(v) => Patch::Set(v.to_string()),
    };
    let allowed_values = match raw(cols.allowed_values) {
        None => Patch::Leave,
        Some("null") => Patch::Clear,
        Some(v) => Patch::Set(
            v.split('|')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| {
                    AttributeValue::new(s)
                        .map_err(|e| format!("invalid allowed value '{s}': {e:?}"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        ),
    };
    let default_value = match raw(cols.default_value) {
        None => Patch::Leave,
        Some("null") => Patch::Clear,
        Some(v) => {
            Patch::Set(AttributeValue::new(v).map_err(|e| format!("invalid default_value: {e:?}"))?)
        }
    };
    let editable_by = match raw(cols.editable_by) {
        None => None,
        Some(v) => Some(match v.to_ascii_lowercase().as_str() {
            "admin" => EditableBy::Admin,
            "user" => EditableBy::User,
            "both" => EditableBy::Both,
            _ => return Err("editable_by must be admin, user or both".into()),
        }),
    };
    let sync_to_keycloak = match raw(cols.sync_to_keycloak) {
        None => None,
        Some(v) => {
            Some(parse_bool(v).map_err(|_| "sync_to_keycloak must be true/false".to_string())?)
        }
    };
    Ok(AttributeRowData {
        name,
        description,
        allowed_values,
        default_value,
        editable_by,
        sync_to_keycloak,
    })
}

fn joined_values(vs: &[AttributeValue]) -> String {
    vs.iter()
        .map(AttributeValue::as_str)
        .collect::<Vec<_>>()
        .join("|")
}

fn attribute_changes(data: &AttributeRowData, existing: &AttributeDefinition) -> Vec<String> {
    const EMPTY: &str = "(empty)";
    let mut changes = Vec::new();

    match &data.description {
        Patch::Set(d) if existing.description() != Some(d.as_str()) => changes.push(format!(
            "description: {} → {d}",
            existing.description().unwrap_or(EMPTY)
        )),
        Patch::Clear if existing.description().is_some() => changes.push(format!(
            "description: {} → (cleared)",
            existing.description().unwrap_or(EMPTY)
        )),
        _ => {}
    }
    match &data.allowed_values {
        Patch::Set(vs) => {
            let old = existing.allowed_values().map(joined_values);
            let new = joined_values(vs);
            if old.as_deref() != Some(new.as_str()) {
                changes.push(format!(
                    "allowed_values: {} → {new}",
                    old.unwrap_or_else(|| EMPTY.to_string())
                ));
            }
        }
        Patch::Clear if existing.allowed_values().is_some() => changes.push(format!(
            "allowed_values: {} → (cleared)",
            existing
                .allowed_values()
                .map(joined_values)
                .unwrap_or_else(|| EMPTY.to_string())
        )),
        _ => {}
    }
    match &data.default_value {
        Patch::Set(v) if existing.default_value() != Some(v) => changes.push(format!(
            "default_value: {} → {}",
            existing
                .default_value()
                .map(AttributeValue::as_str)
                .unwrap_or(EMPTY),
            v.as_str()
        )),
        Patch::Clear if existing.default_value().is_some() => changes.push(format!(
            "default_value: {} → (cleared)",
            existing
                .default_value()
                .map(AttributeValue::as_str)
                .unwrap_or(EMPTY)
        )),
        _ => {}
    }
    if let Some(e) = data.editable_by {
        if e != existing.editable_by() {
            changes.push(format!(
                "editable_by: {} → {}",
                existing.editable_by().as_str(),
                e.as_str()
            ));
        }
    }
    if let Some(s) = data.sync_to_keycloak {
        if s != existing.sync_to_keycloak() {
            changes.push(format!(
                "sync_to_keycloak: {} → {s}",
                existing.sync_to_keycloak()
            ));
        }
    }
    changes
}

fn classify_attribute_row(
    data: &AttributeRowData,
    defs: &[AttributeDefinition],
) -> Result<(RowAction, Vec<String>), String> {
    match defs.iter().find(|d| d.name() == &data.name) {
        None => {
            if let (Patch::Set(allowed), Patch::Set(dv)) =
                (&data.allowed_values, &data.default_value)
            {
                if !allowed.contains(dv) {
                    return Err(format!(
                        "default_value '{}' not in allowed_values",
                        dv.as_str()
                    ));
                }
            }
            Ok((RowAction::Create, vec![]))
        }
        Some(existing) => {
            let resolved_allowed = data
                .allowed_values
                .clone()
                .apply(existing.allowed_values().map(<[AttributeValue]>::to_vec));
            let resolved_default = data
                .default_value
                .clone()
                .apply(existing.default_value().cloned());
            if let (Some(a), Some(d)) = (&resolved_allowed, &resolved_default) {
                if !a.contains(d) {
                    return Err(format!(
                        "default_value '{}' not in allowed_values",
                        d.as_str()
                    ));
                }
            }
            let changes = attribute_changes(data, existing);
            Ok(if changes.is_empty() {
                (RowAction::Unchanged, vec![])
            } else {
                (RowAction::Update, changes)
            })
        }
    }
}

impl ImportService {
    pub async fn preview_attributes(&self, bytes: &[u8]) -> ServiceResult<AttributeImportPreview> {
        let parsed = match self.parser.parse(bytes) {
            Ok(p) => p,
            Err(TabularParseError::Malformed(m)) => {
                return Ok(AttributeImportPreview {
                    fatal_error: Some(format!("Could not parse CSV: {m}")),
                    rows: vec![],
                });
            }
        };
        let cols = match resolve_attribute_columns(&parsed.headers) {
            Ok(c) => c,
            Err(e) => {
                return Ok(AttributeImportPreview {
                    fatal_error: Some(e),
                    rows: vec![],
                });
            }
        };
        let defs = self.attribute_service.list_definitions().await?;

        let mut rows = Vec::new();
        let mut seen = HashSet::new();
        for (i, rec) in parsed.records.iter().enumerate() {
            let name = rec[cols.name].trim().to_string();
            let (result, changes) = match parse_attribute_row(&cols, rec, &mut seen)
                .and_then(|data| classify_attribute_row(&data, &defs))
            {
                Ok((action, changes)) => (Ok(action), changes),
                Err(e) => (Err(e), vec![]),
            };
            rows.push(AttributePreviewRow {
                line: i + 1,
                name,
                result,
                changes,
            });
        }
        Ok(AttributeImportPreview {
            fatal_error: None,
            rows,
        })
    }

    pub async fn apply_attributes_import(
        &self,
        bytes: &[u8],
        actor: Option<Uuid>,
    ) -> ServiceResult<AttributeImportReport> {
        let parsed = match self.parser.parse(bytes) {
            Ok(p) => p,
            Err(TabularParseError::Malformed(m)) => {
                return Ok(AttributeImportReport {
                    fatal_error: Some(format!("Could not parse CSV: {m}")),
                    rows: vec![],
                });
            }
        };
        let cols = match resolve_attribute_columns(&parsed.headers) {
            Ok(c) => c,
            Err(e) => {
                return Ok(AttributeImportReport {
                    fatal_error: Some(e),
                    rows: vec![],
                });
            }
        };
        let defs = self.attribute_service.list_definitions().await?;

        let mut rows = Vec::new();
        let mut seen = HashSet::new();
        for (i, rec) in parsed.records.iter().enumerate() {
            let name = rec[cols.name].trim().to_string();
            let (outcome, changes) = match parse_attribute_row(&cols, rec, &mut seen) {
                Err(e) => (RowOutcome::Skipped(e), vec![]),
                Ok(data) => match classify_attribute_row(&data, &defs) {
                    Err(e) => (RowOutcome::Skipped(e), vec![]),
                    Ok((RowAction::Unchanged, _)) => (RowOutcome::Unchanged, vec![]),
                    Ok((RowAction::Create, _)) => {
                        let input = CreateAttributeDefinition {
                            name: data.name.clone(),
                            description: data.description.apply(None),
                            allowed_values: data.allowed_values.apply(None),
                            default_value: data.default_value.apply(None),
                            sync_to_keycloak: data.sync_to_keycloak.unwrap_or(false),
                            editable_by: data.editable_by.unwrap_or(EditableBy::Admin),
                        };
                        match self.attribute_service.create_definition(input, actor).await {
                            Ok(_) => (RowOutcome::Created, vec![]),
                            Err(e) => (RowOutcome::Failed(format!("create failed: {e:?}")), vec![]),
                        }
                    }
                    Ok((RowAction::Update, changes)) => {
                        let patch = UpdateAttributeDefinitionPatch {
                            description: data.description.clone(),
                            allowed_values: data.allowed_values.clone(),
                            default_value: data.default_value.clone(),
                            sync_to_keycloak: data.sync_to_keycloak,
                            editable_by: data.editable_by,
                        };
                        match self
                            .attribute_service
                            .update_definition(&data.name, patch, actor)
                            .await
                        {
                            Ok(_) => (RowOutcome::Updated, changes),
                            Err(e) => (RowOutcome::Failed(format!("update failed: {e:?}")), vec![]),
                        }
                    }
                },
            };
            rows.push(AttributeResultRow {
                line: i + 1,
                name,
                outcome,
                changes,
            });
        }
        Ok(AttributeImportReport {
            fatal_error: None,
            rows,
        })
    }
}
