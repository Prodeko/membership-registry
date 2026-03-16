use std::sync::Arc;

use crate::application::ports::{
    auth_provider_repo_port::AuthProviderRepositoryPort,
    role_repository_port::{RoleMembership, RoleRepositoryPort, RoleStats, RolesWithStatsParams},
    rolesync_port::{IdpSubject, RoleSyncPort},
};
use crate::domain::{Person, Role, RoleName};
use uuid::Uuid;

use super::{
    audit_log_service::AuditLogService,
    errors::{ServiceError, ServiceResult},
    member_service::MemberService,
};

#[derive(Clone)]
pub struct RoleService {
    pub role_repo: Arc<dyn RoleRepositoryPort>,
    pub member_service: MemberService,
    pub role_sync: Arc<dyn RoleSyncPort>,
    pub auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    pub audit_log: AuditLogService,
}

impl RoleService {
    pub fn new(
        role_repo: Arc<dyn RoleRepositoryPort>,
        member_service: MemberService,
        role_sync: Arc<dyn RoleSyncPort>,
        auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
        audit_log: AuditLogService,
    ) -> Self {
        Self {
            role_repo,
            member_service,
            role_sync,
            auth_provider_repo,
            audit_log,
        }
    }

    pub async fn create_role(
        &self,
        new_role: &Role,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<Role> {
        self.role_sync
            .create_role(&new_role.name)
            .await
            .map_err(|_| ServiceError::IdpError)?;

        let role = self
            .role_repo
            .create(new_role)
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "role.create",
                "role",
                &role.name.0,
                Some(serde_json::json!({ "role_name": &role.name.0 })),
            )
            .await;

        Ok(role)
    }

    pub async fn get_all_roles(&self) -> ServiceResult<Vec<Role>> {
        self.role_repo.fetch_all().await.map_err(ServiceError::from)
    }

    pub async fn delete_role(
        &self,
        role_name: &str,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.role_repo
            .delete(role_name)
            .await
            .map_err(ServiceError::from)?;

        if let Err(e) = self
            .role_sync
            .delete_role(&RoleName(role_name.to_string()))
            .await
        {
            tracing::error!("Failed to delete role from IdP: {e:?}");
        }

        self.audit_log
            .log(actor_user_id, "role.delete", "role", role_name, None)
            .await;

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

        self.role_repo
            .create_role_member(&user_id, role_name, valid_from, valid_until)
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
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
            )
            .await;

        Ok(())
    }

    pub async fn get_role(&self, role_name: &str) -> ServiceResult<Role> {
        self.role_repo
            .fetch_by_name(role_name)
            .await
            .map_err(ServiceError::from)
    }

    pub async fn get_member_roles(&self, user_id: Uuid) -> ServiceResult<Vec<RoleMembership>> {
        self.role_repo
            .fetch_roles_by_member(&user_id)
            .await
            .map_err(ServiceError::from)
    }

    pub async fn get_role_members(&self, role_name: &str) -> ServiceResult<Vec<Person>> {
        self.role_repo
            .fetch_members_by_role(role_name)
            .await
            .map_err(ServiceError::from)
    }

    pub async fn add_many_role_members(
        &self,
        user_ids: Vec<Uuid>,
        role_names: Vec<String>,
        valid_from: chrono::NaiveDate,
        valid_until: Option<chrono::NaiveDate>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        // Sync roles to IdP for each user (external calls, cannot be batched)
        for user_id in &user_ids {
            let providers = self
                .auth_provider_repo
                .find_by_user_id(user_id)
                .await
                .map_err(|e| {
                    tracing::error!("Failed to find auth providers for user: {e:?}");
                    ServiceError::DatabaseError
                })?;

            for role_name in &role_names {
                for provider in &providers {
                    self.role_sync
                        .assign_role(
                            &IdpSubject(provider.provider_user_id.clone()),
                            &RoleName(role_name.to_string()),
                        )
                        .await
                        .map_err(|_| ServiceError::IdpError)?;
                }
            }
        }

        // Batch insert all role memberships in a single query
        self.role_repo
            .create_role_members_batch(&user_ids, &role_names, valid_from, valid_until)
            .await
            .map_err(ServiceError::from)?;

        // Audit log each assignment
        for role_name in &role_names {
            for user_id in &user_ids {
                self.audit_log
                    .log(
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
                    )
                    .await;
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
        self.role_repo
            .update_valid_until(&user_id, role_name, valid_from, new_valid_until)
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "role_member.update",
                "role_member",
                &format!("{}:{}", user_id, role_name),
                Some(serde_json::json!({
                    "user_id": user_id,
                    "role_name": role_name,
                    "new_valid_until": new_valid_until.to_string(),
                })),
            )
            .await;

        Ok(())
    }

    pub async fn delete_role_membership(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: chrono::NaiveDate,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.role_repo
            .delete_role_member(&user_id, role_name, valid_from)
            .await
            .map_err(ServiceError::from)?;

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
                tracing::error!(
                    "Failed to remove IdP role for provider {}: {e:?}",
                    provider.provider_user_id
                );
            }
        }

        self.audit_log
            .log(
                actor_user_id,
                "role_member.delete",
                "role_member",
                &format!("{}:{}", user_id, role_name),
                Some(serde_json::json!({
                    "user_id": user_id,
                    "role_name": role_name,
                })),
            )
            .await;

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
        self.role_repo
            .fetch_roles_with_stats(RolesWithStatsParams {
                page_size,
                offset,
                search,
                order_by,
                order_desc,
            })
            .await
            .map_err(ServiceError::from)
    }
}
