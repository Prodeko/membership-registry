use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use super::repository_error::RepositoryError;

// --- Read models ---

pub struct AuditLogEntry {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub details: Option<Value>,
    pub created_at: DateTime<Utc>,
}

pub struct AuditLogEntryWithActor {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub details: Option<Value>,
    pub created_at: DateTime<Utc>,
}

pub struct NewAuditLogEntry {
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub details: Option<Value>,
}

pub struct AuditLogQueryParams {
    pub page_size: Option<i32>,
    pub offset: Option<i32>,
    pub action: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub actor_user_id: Option<Uuid>,
    pub search: Option<String>,
}

#[async_trait::async_trait]
pub trait AuditLogRepositoryPort: Send + Sync {
    async fn create(&self, entry: NewAuditLogEntry) -> Result<(), RepositoryError>;
    async fn fetch_paginated(
        &self,
        params: AuditLogQueryParams,
    ) -> Result<Vec<AuditLogEntryWithActor>, RepositoryError>;
}
