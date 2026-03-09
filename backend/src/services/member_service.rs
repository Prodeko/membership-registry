use std::sync::Arc;

use crate::application::ports::{
    auth_provider_repo_port::AuthProviderRepositoryPort,
    user_admin_port::UserAdminPort,
};
use crate::repositories::member::{Member, MemberRepo, MemberWithRoles, MembersWithRolesParams, NewMember};
use uuid::Uuid;

use super::{audit_log_service::AuditLogService, errors::{ServiceError, ServiceResult}};

#[derive(Clone)]
pub struct MemberService {
    pub repo: MemberRepo,
    pub user_admin: Arc<dyn UserAdminPort>,
    pub auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    pub audit_log: AuditLogService,
}

impl MemberService {
    pub fn new(
        repo: MemberRepo,
        user_admin: Arc<dyn UserAdminPort>,
        auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
        audit_log: AuditLogService,
    ) -> Self {
        Self { repo, user_admin, auth_provider_repo, audit_log }
    }

    pub async fn create_member(
        &self,
        member_to_add: NewMember,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<Member> {
        let member = self.repo
            .create(member_to_add)
            .await
            .map_err(|e| -> ServiceError { e.into() })?;

        self.audit_log.log(
            actor_user_id,
            "member.create",
            "member",
            &member.user_id.to_string(),
            Some(serde_json::json!({
                "email": member.email,
                "first_name": member.first_name,
                "last_name": member.last_name,
            })),
        ).await;

        Ok(member)
    }

    pub async fn get_all_members(&self) -> ServiceResult<Vec<Member>> {
        self.repo.fetch_all().await.map_err(|e| e.into())
    }

    pub async fn get_members_with_ids(
        &self,
        user_ids: Option<Vec<Uuid>>,
    ) -> ServiceResult<Vec<Member>> {
         self
            .repo
            .fetch_with_ids(user_ids)
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_member(&self, id: Uuid) -> ServiceResult<Member> {
        self.repo.fetch_one(id).await.map_err(|e| e.into())
    }

    pub async fn get_member_with_user(
        &self,
        id: Uuid,
    ) -> ServiceResult<Member> {
        let auth_providers = self
            .auth_provider_repo
            .find_by_user_id(&id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find auth providers: {e:?}");
                ServiceError::DatabaseError
            })?;

        if auth_providers.is_empty() {
            return Err(ServiceError::NotFound);
        }

        let primary_provider = &auth_providers[0];

        let user = self
            .user_admin
            .get_user(&primary_provider.provider_user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get IdP user: {e:?}");
                ServiceError::IdpError
            })?;

        let member = self.repo.fetch_one(id).await?;

        let email = user.email.unwrap_or(member.email);

        Ok(Member {
            user_id: member.user_id,
            first_name: member.first_name,
            last_name: member.last_name,
            full_name: member.full_name,
            home_municipality: member.home_municipality,
            has_accepted_policies: member.has_accepted_policies,
            email,
        })
    }

    pub async fn update_member(
        &self,
        updated_member: Member,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<Member> {
        let user_id = updated_member.user_id;
        let member = self.repo
            .update(updated_member, user_id, None)
            .await
            .map_err(|e| -> ServiceError { e.into() })?;

        self.audit_log.log(
            actor_user_id,
            "member.update",
            "member",
            &member.user_id.to_string(),
            Some(serde_json::json!({
                "email": member.email,
                "first_name": member.first_name,
                "last_name": member.last_name,
            })),
        ).await;

        Ok(member)
    }

    pub async fn delete_member(&self, id: Uuid, actor_user_id: Option<Uuid>) -> ServiceResult<()> {
        self.repo.delete(id).await.map_err(|e| -> ServiceError { e.into() })?;

        self.audit_log.log(
            actor_user_id,
            "member.delete",
            "member",
            &id.to_string(),
            None,
        ).await;

        Ok(())
    }

    pub async fn delete_many(&self, ids: Vec<Uuid>, actor_user_id: Option<Uuid>) -> ServiceResult<()> {
        self.repo.delete_many(ids.clone()).await.map_err(|e| -> ServiceError { e.into() })?;

        self.audit_log.log(
            actor_user_id,
            "member.delete_many",
            "member",
            "batch",
            Some(serde_json::json!({ "deleted_member_ids": ids })),
        ).await;

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn get_members_with_roles(
        &self,
        page_size: Option<u64>,
        offset: Option<u64>,
        roles: Option<Vec<String>>,
        search: Option<String>,
        order_by: Option<String>,
        order_desc: Option<bool>,
        valid_from: Option<chrono::NaiveDate>,
        valid_until: Option<chrono::NaiveDate>,
    ) -> ServiceResult<Vec<MemberWithRoles>> {
        self
            .repo
            .fetch_members_with_roles(
                MembersWithRolesParams {
                    valid_from,
                    valid_until,
                    roles,
                    page_size,
                    offset,
                    search,
                    order_by,
                    order_desc,
                },
            )
            .await.map_err(|e| e.into())
    }

    pub async fn log_export(&self, actor_user_id: Option<Uuid>) {
        self.audit_log.log(
            actor_user_id,
            "member.export_csv",
            "member",
            "export",
            None,
        ).await;
    }
}
