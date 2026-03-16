use std::sync::Arc;

use crate::application::ports::{
    application_repository_port::ApplicationWithMember,
    audit_log_repository_port::AuditLogEntryWithActor,
    data_export_port::{DataExportPort, Exportable, ExportedData, TabularData},
    member_repository_port::MemberWithRoles,
    role_repository_port::RoleStats,
};

use super::errors::{ServiceError, ServiceResult};

pub struct ExportService {
    adapter: Arc<dyn DataExportPort>,
}

impl ExportService {
    pub fn new(adapter: Arc<dyn DataExportPort>) -> Self {
        Self { adapter }
    }

    pub fn export<T: Exportable>(&self, items: &[T]) -> ServiceResult<ExportedData> {
        let tabular = TabularData::from_exportable(items);
        let bytes = self
            .adapter
            .serialize(&tabular)
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
            "has_accepted_policies",
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
            self.person.home_municipality.clone(),
            self.person.has_accepted_policies.to_string(),
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
