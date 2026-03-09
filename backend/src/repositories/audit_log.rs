use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::application::ports::audit_log_repository_port::{
    AuditLogEntryWithActor, AuditLogQueryParams, AuditLogRepositoryPort, NewAuditLogEntry,
};
use crate::application::ports::repository_error::RepositoryError;

#[derive(Clone)]
pub struct AuditLogRepo {
    pub pool: PgPool,
}

#[derive(Debug, sqlx::FromRow)]
struct AuditLogEntryWithActorDAO {
    pub id: Uuid,
    pub actor_user_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub action: String,
    pub entity_type: String,
    pub entity_id: String,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl From<AuditLogEntryWithActorDAO> for AuditLogEntryWithActor {
    fn from(dao: AuditLogEntryWithActorDAO) -> Self {
        Self {
            id: dao.id,
            actor_user_id: dao.actor_user_id,
            actor_name: dao.actor_name,
            action: dao.action,
            entity_type: dao.entity_type,
            entity_id: dao.entity_id,
            details: dao.details,
            created_at: dao.created_at,
        }
    }
}

#[async_trait::async_trait]
impl AuditLogRepositoryPort for AuditLogRepo {
    async fn create(&self, entry: NewAuditLogEntry) -> Result<(), RepositoryError> {
        sqlx::query!(
            r#"
            INSERT INTO audit_log (actor_user_id, action, entity_type, entity_id, details)
            VALUES ($1, $2, $3, $4, $5)
            "#,
            entry.actor_user_id,
            entry.action,
            entry.entity_type,
            entry.entity_id,
            entry.details,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn fetch_paginated(
        &self,
        params: AuditLogQueryParams,
    ) -> Result<Vec<AuditLogEntryWithActor>, RepositoryError> {
        let page_size = params.page_size.unwrap_or(50) as i64;
        let offset = params.offset.unwrap_or(0) as i64;

        let rows = sqlx::query_as!(
            AuditLogEntryWithActorDAO,
            r#"
            SELECT
                al.id,
                al.actor_user_id,
                m.full_name as actor_name,
                al.action,
                al.entity_type,
                al.entity_id,
                al.details,
                al.created_at
            FROM audit_log al
            LEFT JOIN member m ON al.actor_user_id = m.user_id
            WHERE
                ($1::text IS NULL OR al.action = $1)
                AND ($2::text IS NULL OR al.entity_type = $2)
                AND ($3::text IS NULL OR al.entity_id = $3)
                AND ($4::uuid IS NULL OR al.actor_user_id = $4)
                AND ($5::text IS NULL OR
                    al.action ILIKE '%' || $5 || '%'
                    OR al.entity_type ILIKE '%' || $5 || '%'
                    OR al.entity_id ILIKE '%' || $5 || '%'
                    OR m.full_name ILIKE '%' || $5 || '%'
                )
            ORDER BY al.created_at DESC
            LIMIT $6 OFFSET $7
            "#,
            params.action,
            params.entity_type,
            params.entity_id,
            params.actor_user_id,
            params.search,
            page_size,
            offset,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(AuditLogEntryWithActor::from).collect())
    }
}
