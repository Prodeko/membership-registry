use serde_json::Value;
use uuid::Uuid;

use crate::repositories::audit_log::{AuditLogEntryWithActor, AuditLogQueryParams, AuditLogRepo, NewAuditLogEntry};

use super::errors::ServiceResult;

#[derive(Clone)]
pub struct AuditLogService {
    pub repo: AuditLogRepo,
}

impl AuditLogService {
    pub fn new(repo: AuditLogRepo) -> Self {
        Self { repo }
    }

    /// Fire-and-forget audit log. Errors are logged but never propagated.
    pub async fn log(
        &self,
        actor_user_id: Option<Uuid>,
        action: &str,
        entity_type: &str,
        entity_id: &str,
        details: Option<Value>,
    ) {
        let entry = NewAuditLogEntry {
            actor_user_id,
            action: action.to_string(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            details,
        };

        if let Err(e) = self.repo.create(entry).await {
            tracing::error!("Failed to write audit log entry: {:?}", e);
        }
    }

    pub async fn get_logs(
        &self,
        params: AuditLogQueryParams,
    ) -> ServiceResult<Vec<AuditLogEntryWithActor>> {
        self.repo
            .fetch_paginated(params)
            .await
            .map_err(|e| e.into())
    }
}
