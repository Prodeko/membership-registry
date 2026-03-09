use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;
use uuid::Uuid;

use crate::application::ports::audit_log_repository_port::{
    AuditLogEntryWithActor, AuditLogQueryParams,
};

#[derive(Debug, Serialize, TS)]
#[ts(export, rename = "AuditLogEntryWithActor")]
pub struct AuditLogEntryWithActorDTO {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    #[ts(type = "Record<string, unknown> | null")]
    pub details: Option<Value>,
    pub created_at: DateTime<Utc>,
}

impl From<AuditLogEntryWithActor> for AuditLogEntryWithActorDTO {
    fn from(e: AuditLogEntryWithActor) -> Self {
        Self {
            id: e.id,
            actor_user_id: e.actor_user_id,
            actor_name: e.actor_name,
            action: e.action,
            entity_type: e.entity_type,
            entity_id: e.entity_id,
            details: e.details,
            created_at: e.created_at,
        }
    }
}

#[derive(Debug, Deserialize, TS)]
#[ts(export, rename = "AuditLogQueryParams")]
pub struct AuditLogQueryParamsDTO {
    pub page_size: Option<i32>,
    pub offset: Option<i32>,
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub actor_user_id: Option<Uuid>,
    pub search: Option<String>,
}

impl From<AuditLogQueryParamsDTO> for AuditLogQueryParams {
    fn from(p: AuditLogQueryParamsDTO) -> Self {
        Self {
            page_size: p.page_size,
            offset: p.offset,
            action: p.action,
            entity_type: p.entity_type,
            entity_id: p.entity_id,
            actor_user_id: p.actor_user_id,
            search: p.search,
        }
    }
}
