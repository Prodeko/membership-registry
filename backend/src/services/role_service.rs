use std::sync::Arc;

use crate::application::ports::{
    auth_provider_repo_port::AuthProviderRepositoryPort,
    rolesync_port::{IdpSubject, RoleSyncPort},
};
use crate::domain::RoleName;
use crate::repositories::{
    member::Member,
    role::{Role, RoleMember, RoleRepo, RoleStats, RolesWithStatsParams},
};
use uuid::Uuid;

use super::{audit_log_service::AuditLogService, errors::{ServiceError, ServiceResult}, member_service::MemberService};

#[derive(Clone)]
pub struct RoleService {
    pub repo: RoleRepo,
    pub member_service: MemberService,
    pub role_sync: Arc<dyn RoleSyncPort>,
    pub auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    pub audit_log: AuditLogService,
}

impl RoleService {
    pub fn new(
        repo: RoleRepo,
        member_service: MemberService,
        role_sync: Arc<dyn RoleSyncPort>,
        auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
        audit_log: AuditLogService,
    ) -> Self {
        Self {
            repo,
            member_service,
            role_sync,
            auth_provider_repo,
            audit_log,
        }
    }

    pub async fn create_role(&self, new_role: Role, actor_user_id: Option<Uuid>) -> ServiceResult<Role> {
        self.role_sync
            .create_role(&RoleName(new_role.name.clone()))
            .await
            .map_err(|_| ServiceError::IdpError)?;

        let role = self.repo.create(new_role).await.map_err(|e| -> ServiceError { e.into() })?;

        self.audit_log.log(
            actor_user_id,
            "role.create",
            "role",
            &role.name,
            Some(serde_json::json!({ "role_name": role.name })),
        ).await;

        Ok(role)
    }

    pub async fn get_all_roles(&self) -> ServiceResult<Vec<Role>> {
        self.repo.fetch_all().await.map_err(|e| e.into())
    }

    pub async fn delete_role(&self, role_name: &str, actor_user_id: Option<Uuid>) -> ServiceResult<()> {
        self.repo.delete(role_name).await.map_err(|e| -> ServiceError { e.into() })?;

        if let Err(e) = self.role_sync.delete_role(&RoleName(role_name.to_string())).await {
            tracing::error!("Failed to delete role from IdP: {e:?}");
        }

        self.audit_log.log(
            actor_user_id,
            "role.delete",
            "role",
            role_name,
            None,
        ).await;

        Ok(())
    }

    pub async fn add_role_member(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: chrono::NaiveDate,
        valid_until: Option<chrono::NaiveDate>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        let providers = self
            .auth_provider_repo
            .find_by_user_id(&user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find auth providers for user: {e:?}");
                ServiceError::DatabaseError
            })?;

        for provider in providers {
            self.role_sync
                .assign_role(
                    &IdpSubject(provider.provider_user_id),
                    &RoleName(role_name.to_string()),
                )
                .await
                .map_err(|_| ServiceError::IdpError)?;
        }

        self.repo
            .create_role_member(&user_id, role_name, valid_from, valid_until)
            .await
            .map_err(|e| -> ServiceError { e.into() })?;

        self.audit_log.log(
            actor_user_id,
            "role_member.assign",
            "role_member",
            &format!("{}:{}", user_id, role_name),
            Some(serde_json::json!({
                "user_id": user_id,
                "role_name": role_name,
                "valid_from": valid_from.to_string(),
                "valid_until": valid_until.map(|d| d.to_string()),
            })),
        ).await;

        Ok(())
    }

    pub async fn get_role(&self, role_name: &str) -> ServiceResult<Role> {
        self.repo
            .fetch_by_name(role_name)
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_member_roles(&self, user_id: Uuid) -> ServiceResult<Vec<RoleMember>> {
        self.repo
            .fetch_roles_by_member(&user_id)
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_role_members(&self, role_name: &str) -> ServiceResult<Vec<Member>> {
        self.repo
            .fetch_members_by_role(role_name)
            .await
            .map_err(|e| e.into())
    }

    pub async fn add_many_role_members(
        &self,
        user_ids: Vec<Uuid>,
        role_names: Vec<String>,
        valid_from: chrono::NaiveDate,
        valid_until: Option<chrono::NaiveDate>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        for role_name in &role_names {
            for user_id in &user_ids {
                self.add_role_member(*user_id, role_name.as_str(), valid_from, valid_until, actor_user_id)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn update_role_member(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: chrono::NaiveDate,
        new_valid_until: chrono::NaiveDate,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.repo
            .update_valid_until(&user_id, role_name, valid_from, new_valid_until)
            .await
            .map_err(|e| -> ServiceError { e.into() })?;

        self.audit_log.log(
            actor_user_id,
            "role_member.update",
            "role_member",
            &format!("{}:{}", user_id, role_name),
            Some(serde_json::json!({
                "user_id": user_id,
                "role_name": role_name,
                "new_valid_until": new_valid_until.to_string(),
            })),
        ).await;

        Ok(())
    }

    pub async fn delete_role_membership(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: chrono::NaiveDate,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.repo
            .delete_role_member(&user_id, role_name, valid_from)
            .await
            .map_err(|e| -> ServiceError { e.into() })?;

        let providers = self
            .auth_provider_repo
            .find_by_user_id(&user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find auth providers for user: {e:?}");
                ServiceError::DatabaseError
            })?;

        for provider in providers {
            if let Err(e) = self
                .role_sync
                .remove_role(
                    &IdpSubject(provider.provider_user_id.clone()),
                    &RoleName(role_name.to_string()),
                )
                .await
            {
                tracing::error!("Failed to remove IdP role for provider {}: {e:?}", provider.provider_user_id);
            }
        }

        self.audit_log.log(
            actor_user_id,
            "role_member.delete",
            "role_member",
            &format!("{}:{}", user_id, role_name),
            Some(serde_json::json!({
                "user_id": user_id,
                "role_name": role_name,
            })),
        ).await;

        Ok(())
    }

    pub async fn get_role_stats(
        &self,
        page_size: Option<u64>,
        offset: Option<u64>,
        search: Option<String>,
        order_by: Option<String>,
        order_desc: Option<bool>,
    ) -> ServiceResult<Vec<RoleStats>> {
        self.repo
            .fetch_roles_with_stats(RolesWithStatsParams {
                page_size,
                offset,
                search,
                order_by,
                order_desc,
            })
            .await
            .map_err(|e| e.into())
    }
}
