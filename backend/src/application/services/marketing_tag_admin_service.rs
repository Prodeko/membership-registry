use std::sync::Arc;

use serde_json;
use uuid::Uuid;

use crate::application::ports::marketing_tag_repository_port::MarketingTagRepositoryPort;
use crate::domain::MarketingTag;

use super::audit_log_service::AuditLogService;
use super::errors::{ServiceError, ServiceResult};

/// Admin-facing CRUD for the marketing tag catalog. Changes here only
/// affect which tags users can toggle on their profile page; no
/// side effects on Mailchimp. Admins are expected to manage Mailchimp-side
/// state (historical backfills, tags outside the user-editable catalog)
/// directly in the Mailchimp UI.
#[derive(Clone)]
pub struct MarketingTagAdminService {
    repo: Arc<dyn MarketingTagRepositoryPort>,
    audit_log: AuditLogService,
}

impl MarketingTagAdminService {
    pub fn new(repo: Arc<dyn MarketingTagRepositoryPort>, audit_log: AuditLogService) -> Self {
        Self { repo, audit_log }
    }

    pub async fn list_tags(&self) -> ServiceResult<Vec<MarketingTag>> {
        self.repo.fetch_all().await.map_err(ServiceError::from)
    }

    pub async fn create_tag(
        &self,
        tag: MarketingTag,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<MarketingTag> {
        let created = self.repo.create(&tag).await.map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "marketing_tag.create",
                "marketing_tag",
                &created.label,
                Some(serde_json::json!({
                    "auto_apply": created.auto_apply,
                    "display_order": created.display_order,
                })),
            )
            .await;

        Ok(created)
    }

    pub async fn update_tag(
        &self,
        tag: MarketingTag,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<MarketingTag> {
        let updated = self.repo.update(&tag).await.map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "marketing_tag.update",
                "marketing_tag",
                &updated.label,
                Some(serde_json::json!({
                    "auto_apply": updated.auto_apply,
                    "display_order": updated.display_order,
                })),
            )
            .await;

        Ok(updated)
    }

    pub async fn delete_tag(&self, label: &str, actor_user_id: Option<Uuid>) -> ServiceResult<()> {
        self.repo.delete(label).await.map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "marketing_tag.delete",
                "marketing_tag",
                label,
                None,
            )
            .await;

        Ok(())
    }
}
