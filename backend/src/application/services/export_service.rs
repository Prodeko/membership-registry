use std::sync::Arc;

use crate::application::ports::{
    application_repository_port::ApplicationWithMember,
    audit_log_repository_port::AuditLogEntryWithActor,
    data_export_port::{DataExportPort, Exportable, ExportedData, TabularData},
    member_repository_port::MemberWithRoles,
    role_repository_port::RoleStats,
};

use std::collections::HashMap;

use uuid::Uuid;

use super::errors::{ServiceError, ServiceResult};

pub struct ExportService {
    adapter: Arc<dyn DataExportPort>,
}

impl ExportService {
    pub fn new(adapter: Arc<dyn DataExportPort>) -> Self {
        Self { adapter }
    }

    pub fn export<T: Exportable>(&self, items: &[T]) -> ServiceResult<ExportedData> {
        self.export_table(&TabularData::from_exportable(items))
    }

    pub fn export_table(&self, tabular: &TabularData) -> ServiceResult<ExportedData> {
        let bytes = self
            .adapter
            .serialize(tabular)
            .map_err(ServiceError::from)?;

        Ok(ExportedData {
            bytes,
            content_type: self.adapter.content_type().to_string(),
            file_extension: self.adapter.file_extension().to_string(),
        })
    }
}

impl Exportable for MemberWithRoles {
    fn headers() -> Vec<&'static str> {
        vec![
            "user_id",
            "first_name",
            "last_name",
            "full_name",
            "home_municipality",
            "email_notifications",
            "email",
            "role_names",
        ]
    }

    fn to_row(&self) -> Vec<String> {
        vec![
            self.person.id.0.to_string(),
            self.person.first_name.clone(),
            self.person.last_name.clone(),
            self.person.full_name.clone().unwrap_or_default(),
            self.person.home_municipality.clone().unwrap_or_default(),
            self.person.email_notifications.to_string(),
            self.person.email.as_str().to_string(),
            self.role_names.join(", "),
        ]
    }
}

impl Exportable for ApplicationWithMember {
    fn headers() -> Vec<&'static str> {
        vec![
            "application_id",
            "user_id",
            "full_name",
            "email",
            "role_name",
            "valid_until",
            "created_at",
            "status",
            "stripe_payment_id",
            "optional_roles",
            "application_text",
        ]
    }

    fn to_row(&self) -> Vec<String> {
        vec![
            self.application_id.0.to_string(),
            self.user_id.to_string(),
            self.full_name.clone().unwrap_or_default(),
            self.email.clone().unwrap_or_default(),
            self.role_name.clone(),
            self.valid_until.to_string(),
            self.created_at.to_rfc3339(),
            format!("{:?}", self.status),
            self.stripe_payment_id.clone().unwrap_or_default(),
            self.optional_roles
                .as_ref()
                .map(|r| r.join(", "))
                .unwrap_or_default(),
            self.application_text.clone().unwrap_or_default(),
        ]
    }
}

impl Exportable for AuditLogEntryWithActor {
    fn headers() -> Vec<&'static str> {
        vec![
            "id",
            "actor_user_id",
            "actor_name",
            "action",
            "entity_type",
            "entity_id",
            "details",
            "created_at",
        ]
    }

    fn to_row(&self) -> Vec<String> {
        vec![
            self.id.to_string(),
            self.actor_user_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            self.actor_name.clone().unwrap_or_default(),
            self.action.clone(),
            self.entity_type.clone(),
            self.entity_id.clone(),
            self.details
                .as_ref()
                .map(|d| d.to_string())
                .unwrap_or_default(),
            self.created_at.to_rfc3339(),
        ]
    }
}

impl Exportable for RoleStats {
    fn headers() -> Vec<&'static str> {
        vec![
            "name",
            "color",
            "description",
            "member_count",
            "active_member_count",
        ]
    }

    fn to_row(&self) -> Vec<String> {
        vec![
            self.name.0.clone(),
            self.color.clone().unwrap_or_default(),
            self.description.clone().unwrap_or_default(),
            self.member_count.map(|c| c.to_string()).unwrap_or_default(),
            self.active_member_count
                .map(|c| c.to_string())
                .unwrap_or_default(),
        ]
    }
}

/// One column of the member export. The fixed ones are named like the
/// member import's columns, so an export with language and attributes can
/// be imported back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemberExportColumn {
    UserId,
    FirstName,
    LastName,
    FullName,
    Email,
    HomeMunicipality,
    Language,
    EmailNotifications,
    RoleNames,
    Groups,
    /// An attribute, by name; the column is named after it.
    Attribute(String),
}

/// Request key for an attribute column: `attribute:<name>`.
pub const ATTRIBUTE_COLUMN_PREFIX: &str = "attribute:";

impl MemberExportColumn {
    /// The columns the member export had before columns were selectable.
    pub fn defaults() -> Vec<Self> {
        use MemberExportColumn::*;
        vec![
            UserId,
            FirstName,
            LastName,
            FullName,
            HomeMunicipality,
            EmailNotifications,
            Email,
            RoleNames,
        ]
    }

    /// Parses a request key: a fixed column name or `attribute:<name>`.
    pub fn parse(key: &str) -> Option<Self> {
        use MemberExportColumn::*;
        if let Some(name) = key.strip_prefix(ATTRIBUTE_COLUMN_PREFIX) {
            return (!name.is_empty()).then(|| Attribute(name.to_string()));
        }
        Some(match key {
            "user_id" => UserId,
            "first_name" => FirstName,
            "last_name" => LastName,
            "full_name" => FullName,
            "email" => Email,
            "home_municipality" => HomeMunicipality,
            "language" => Language,
            "email_notifications" => EmailNotifications,
            "role_names" => RoleNames,
            "groups" => Groups,
            _ => return None,
        })
    }

    fn header(&self) -> String {
        use MemberExportColumn::*;
        match self {
            UserId => "user_id",
            FirstName => "first_name",
            LastName => "last_name",
            FullName => "full_name",
            Email => "email",
            HomeMunicipality => "home_municipality",
            Language => "language",
            EmailNotifications => "email_notifications",
            RoleNames => "role_names",
            Groups => "groups",
            Attribute(name) => return name.clone(),
        }
        .to_string()
    }
}

/// Builds the member export table for `columns`. `attributes` maps an
/// attribute name to each member's values; a multiple-values attribute's
/// values are joined with `; `, as the import expects.
pub fn member_export_table(
    members: &[MemberWithRoles],
    columns: &[MemberExportColumn],
    attributes: &HashMap<String, HashMap<Uuid, Vec<String>>>,
) -> TabularData {
    use MemberExportColumn::*;
    let rows = members
        .iter()
        .map(|m| {
            let p = &m.person;
            columns
                .iter()
                .map(|c| match c {
                    UserId => p.id.0.to_string(),
                    FirstName => p.first_name.clone(),
                    LastName => p.last_name.clone(),
                    FullName => p.full_name.clone().unwrap_or_default(),
                    Email => p.email.as_str().to_string(),
                    HomeMunicipality => p.home_municipality.clone().unwrap_or_default(),
                    Language => p.language.clone(),
                    EmailNotifications => p.email_notifications.to_string(),
                    RoleNames => m.role_names.join(", "),
                    Groups => m.group_names.join(", "),
                    Attribute(name) => attributes
                        .get(name)
                        .and_then(|by_user| by_user.get(&p.id.0))
                        .map(|vs| vs.join("; "))
                        .unwrap_or_default(),
                })
                .collect()
        })
        .collect();
    TabularData {
        headers: columns.iter().map(MemberExportColumn::header).collect(),
        rows,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Email, Person, PersonId};

    fn member() -> MemberWithRoles {
        MemberWithRoles {
            person: Person {
                id: PersonId(Uuid::nil()),
                email: Email::new("maija@example.com".to_string()).unwrap(),
                first_name: "Maija".to_string(),
                last_name: "Meikäläinen".to_string(),
                full_name: Some("Maija Meikäläinen".to_string()),
                home_municipality: None,
                email_notifications: true,
                language: "fi".to_string(),
            },
            role_names: vec!["full-member".to_string(), "board".to_string()],
            group_names: vec!["Board 2026".to_string()],
        }
    }

    #[test]
    fn parse_accepts_fixed_and_attribute_columns_only() {
        assert_eq!(
            MemberExportColumn::parse("language"),
            Some(MemberExportColumn::Language)
        );
        assert_eq!(
            MemberExportColumn::parse("attribute:study-year"),
            Some(MemberExportColumn::Attribute("study-year".to_string()))
        );
        assert_eq!(MemberExportColumn::parse("attribute:"), None);
        assert_eq!(MemberExportColumn::parse("password"), None);
    }

    #[test]
    fn table_has_selected_columns_in_order_with_attribute_values() {
        use MemberExportColumn::*;
        let mut attributes = HashMap::new();
        attributes.insert(
            "languages".to_string(),
            HashMap::from([(Uuid::nil(), vec!["fi".to_string(), "en".to_string()])]),
        );
        let table = member_export_table(
            &[member()],
            &[
                Email,
                Language,
                Attribute("languages".to_string()),
                Attribute("study-year".to_string()),
                RoleNames,
                Groups,
            ],
            &attributes,
        );
        assert_eq!(
            table.headers,
            [
                "email",
                "language",
                "languages",
                "study-year",
                "role_names",
                "groups"
            ]
        );
        assert_eq!(
            table.rows,
            [[
                "maija@example.com",
                "fi",
                "fi; en",
                "",
                "full-member, board",
                "Board 2026"
            ]]
        );
    }

    #[test]
    fn default_columns_match_the_previous_export() {
        let table = member_export_table(
            &[member()],
            &MemberExportColumn::defaults(),
            &HashMap::new(),
        );
        assert_eq!(table.headers, <MemberWithRoles as Exportable>::headers());
        assert_eq!(table.rows, [member().to_row()]);
    }
}
