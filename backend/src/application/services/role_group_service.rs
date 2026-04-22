use std::sync::Arc;

use chrono::NaiveDate;
use uuid::Uuid;

use crate::application::ports::{
    auth_provider_repo_port::AuthProviderRepositoryPort,
    role_group_repository_port::RoleGroupRepositoryPort,
    rolesync_port::{IdpGroupId, IdpSubject, RoleSyncPort},
};
use crate::domain::{RoleGroup, RoleGroupMembership, RoleName};

use super::{
    audit_log_service::AuditLogService,
    errors::{ServiceError, ServiceResult},
};

#[derive(Clone)]
pub struct RoleGroupService {
    pub repo: Arc<dyn RoleGroupRepositoryPort>,
    pub role_sync: Arc<dyn RoleSyncPort>,
    pub auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    pub audit_log: AuditLogService,
}

impl RoleGroupService {
    pub fn new(
        repo: Arc<dyn RoleGroupRepositoryPort>,
        role_sync: Arc<dyn RoleSyncPort>,
        auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
        audit_log: AuditLogService,
    ) -> Self {
        Self {
            repo,
            role_sync,
            auth_provider_repo,
            audit_log,
        }
    }

    pub async fn create_group(
        &self,
        name: &str,
        description: Option<&str>,
        role_names: Vec<RoleName>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<RoleGroup> {
        // Create the Keycloak group first (strict — fail fast if KC unreachable)
        let kc_group_id = self
            .role_sync
            .create_group(name)
            .await
            .map_err(|_| ServiceError::IdpError)?;

        // Persist the group with the KC id
        let group = self
            .repo
            .create(name, description, Some(&kc_group_id.0))
            .await
            .map_err(ServiceError::from)?;

        // Set the role composition
        if !role_names.is_empty() {
            self.repo
                .set_roles(&group.id.0, &role_names)
                .await
                .map_err(ServiceError::from)?;

            self.role_sync
                .set_group_roles(&kc_group_id, &role_names)
                .await
                .map_err(|_| ServiceError::IdpError)?;
        }

        let role_name_strs: Vec<&str> = role_names.iter().map(|r| r.0.as_str()).collect();
        self.audit_log
            .log(
                actor_user_id,
                "role_group.create",
                "role_group",
                &group.id.0.to_string(),
                Some(serde_json::json!({
                    "id": group.id.0,
                    "name": name,
                    "role_names": role_name_strs,
                })),
            )
            .await;

        self.get_group(&group.id.0).await
    }

    pub async fn update_group(
        &self,
        id: &Uuid,
        name: &str,
        description: Option<&str>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<RoleGroup> {
        let group = self
            .repo
            .update(id, name, description)
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "role_group.update",
                "role_group",
                &id.to_string(),
                Some(serde_json::json!({
                    "id": id,
                    "name": name,
                    "description": description,
                })),
            )
            .await;

        Ok(group)
    }

    pub async fn delete_group(&self, id: &Uuid, actor_user_id: Option<Uuid>) -> ServiceResult<()> {
        let group = self
            .repo
            .fetch_by_id(id)
            .await
            .map_err(ServiceError::from)?;

        // Delete from Keycloak first (best-effort — don't fail if KC is unavailable)
        if let Some(ref kc_id) = group.keycloak_group_id {
            if let Err(e) = self
                .role_sync
                .delete_group(&IdpGroupId(kc_id.clone()))
                .await
            {
                tracing::error!(group_id = %id, "Failed to delete group from IdP: {e:?}");
            }
        }

        self.repo.delete(id).await.map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "role_group.delete",
                "role_group",
                &id.to_string(),
                Some(serde_json::json!({ "id": id, "name": group.name })),
            )
            .await;

        Ok(())
    }

    pub async fn get_all_groups(&self) -> ServiceResult<Vec<RoleGroup>> {
        self.repo.fetch_all().await.map_err(ServiceError::from)
    }

    pub async fn get_group(&self, id: &Uuid) -> ServiceResult<RoleGroup> {
        self.repo.fetch_by_id(id).await.map_err(ServiceError::from)
    }

    pub async fn set_group_roles(
        &self,
        id: &Uuid,
        role_names: Vec<RoleName>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<RoleGroup> {
        let group = self
            .repo
            .fetch_by_id(id)
            .await
            .map_err(ServiceError::from)?;

        let old_names: std::collections::HashSet<String> =
            group.role_names.iter().map(|r| r.0.clone()).collect();
        let new_names: std::collections::HashSet<String> =
            role_names.iter().map(|r| r.0.clone()).collect();
        let added: Vec<&str> = role_names
            .iter()
            .filter(|r| !old_names.contains(&r.0))
            .map(|r| r.0.as_str())
            .collect();
        let removed: Vec<&str> = group
            .role_names
            .iter()
            .filter(|r| !new_names.contains(&r.0))
            .map(|r| r.0.as_str())
            .collect();

        self.repo
            .set_roles(id, &role_names)
            .await
            .map_err(ServiceError::from)?;

        if let Some(ref kc_id) = group.keycloak_group_id {
            self.role_sync
                .set_group_roles(&IdpGroupId(kc_id.clone()), &role_names)
                .await
                .map_err(|_| ServiceError::IdpError)?;
        }

        self.audit_log
            .log(
                actor_user_id,
                "role_group.roles_updated",
                "role_group",
                &id.to_string(),
                Some(serde_json::json!({
                    "id": id,
                    "added": added,
                    "removed": removed,
                })),
            )
            .await;

        self.get_group(id).await
    }

    pub async fn assign_group(
        &self,
        group_id: &Uuid,
        user_id: Uuid,
        valid_from: NaiveDate,
        valid_until: Option<NaiveDate>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        let group = self
            .repo
            .fetch_by_id(group_id)
            .await
            .map_err(ServiceError::from)?;

        let providers = self
            .auth_provider_repo
            .find_by_user_id(&user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find auth providers for user: {e:?}");
                ServiceError::DatabaseError(format!("{e:?}"))
            })?;

        if let Some(ref kc_id) = group.keycloak_group_id {
            for provider in &providers {
                self.role_sync
                    .add_user_to_group(
                        &IdpSubject(provider.provider_user_id.clone()),
                        &IdpGroupId(kc_id.clone()),
                    )
                    .await
                    .map_err(|_| ServiceError::IdpError)?;
            }
        }

        self.repo
            .create_member(group_id, &user_id, valid_from, valid_until)
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "role_group_member.assign",
                "role_group_member",
                &format!("{}:{}", group_id, user_id),
                Some(serde_json::json!({
                    "group_id": group_id,
                    "user_id": user_id,
                    "valid_from": valid_from.to_string(),
                    "valid_until": valid_until.map(|d| d.to_string()),
                })),
            )
            .await;

        Ok(())
    }

    pub async fn remove_group_assignment(
        &self,
        group_id: &Uuid,
        user_id: Uuid,
        valid_from: NaiveDate,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        let group = self
            .repo
            .fetch_by_id(group_id)
            .await
            .map_err(ServiceError::from)?;

        let providers = self
            .auth_provider_repo
            .find_by_user_id(&user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to find auth providers for user: {e:?}");
                ServiceError::DatabaseError(format!("{e:?}"))
            })?;

        if let Some(ref kc_id) = group.keycloak_group_id {
            for provider in &providers {
                if let Err(e) = self
                    .role_sync
                    .remove_user_from_group(
                        &IdpSubject(provider.provider_user_id.clone()),
                        &IdpGroupId(kc_id.clone()),
                    )
                    .await
                {
                    tracing::error!(
                        group_id = %group_id,
                        user_id = %user_id,
                        "Failed to remove user from KC group: {e:?}"
                    );
                }
            }
        }

        self.repo
            .delete_member(group_id, &user_id, valid_from)
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "role_group_member.delete",
                "role_group_member",
                &format!("{}:{}", group_id, user_id),
                Some(serde_json::json!({
                    "group_id": group_id,
                    "user_id": user_id,
                    "valid_from": valid_from.to_string(),
                })),
            )
            .await;

        Ok(())
    }

    pub async fn get_group_members(
        &self,
        group_id: &Uuid,
    ) -> ServiceResult<Vec<RoleGroupMembership>> {
        self.repo
            .fetch_members_by_group(group_id)
            .await
            .map_err(ServiceError::from)
    }

    pub async fn get_member_groups(
        &self,
        user_id: &Uuid,
    ) -> ServiceResult<Vec<RoleGroupMembership>> {
        self.repo
            .fetch_groups_by_member(user_id)
            .await
            .map_err(ServiceError::from)
    }

    pub async fn cleanup_expired_group_memberships(&self) -> ServiceResult<u32> {
        let expired = self
            .repo
            .fetch_expired_unsynced()
            .await
            .map_err(ServiceError::from)?;

        let mut synced: u32 = 0;

        for membership in &expired {
            let group = match self.repo.fetch_by_id(&membership.group_id).await {
                Ok(g) => g,
                Err(e) => {
                    tracing::error!(
                        group_id = %membership.group_id,
                        "Failed to fetch group for cleanup: {e:?}"
                    );
                    continue;
                }
            };

            let providers = match self
                .auth_provider_repo
                .find_by_user_id(&membership.user_id)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(
                        user_id = %membership.user_id,
                        "Failed to fetch auth providers for group cleanup: {e:?}"
                    );
                    continue;
                }
            };

            if let Some(ref kc_id) = group.keycloak_group_id {
                for provider in &providers {
                    if let Err(e) = self
                        .role_sync
                        .remove_user_from_group(
                            &IdpSubject(provider.provider_user_id.clone()),
                            &IdpGroupId(kc_id.clone()),
                        )
                        .await
                    {
                        tracing::error!(
                            group_id = %membership.group_id,
                            user_id = %membership.user_id,
                            "Failed to remove expired user from KC group: {e:?}"
                        );
                    }
                }
            }

            if let Err(e) = self
                .repo
                .mark_keycloak_synced(
                    &membership.group_id,
                    &membership.user_id,
                    membership.valid_from,
                )
                .await
            {
                tracing::error!(
                    group_id = %membership.group_id,
                    user_id = %membership.user_id,
                    "Failed to mark group membership as KC synced: {e:?}"
                );
                continue;
            }

            self.audit_log
                .log(
                    None,
                    "role_group_member.expired",
                    "role_group_member",
                    &format!("{}:{}", membership.group_id, membership.user_id),
                    Some(serde_json::json!({
                        "group_id": membership.group_id,
                        "user_id": membership.user_id,
                        "valid_from": membership.valid_from.to_string(),
                        "valid_until": membership.valid_until.map(|d| d.to_string()),
                    })),
                )
                .await;

            synced += 1;
        }

        Ok(synced)
    }
}
