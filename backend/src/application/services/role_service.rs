use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberKeycloakSyncStatus {
    pub in_sync: bool,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    /// Roles in Keycloak that have an expired registry record — removable when opted in.
    pub expired_in_keycloak: Vec<String>,
    /// Roles in Keycloak with no registry record — never touched by sync.
    pub unmanaged_in_keycloak: Vec<String>,
    pub missing_in_keycloak: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncMissingRolesSummary {
    pub added: u32,
    pub failed: u32,
    pub removed: u32,
    pub remove_failed: u32,
    pub users_processed: u32,
}

#[derive(Clone)]
pub struct RoleService {
    pub role_repo: Arc<dyn RoleRepositoryPort>,
    pub member_service: MemberService,
    pub role_sync: Arc<dyn RoleSyncPort>,
    pub auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    pub audit_log: AuditLogService,
    keycloak_sync_cache: Cache<String, HashMap<Uuid, MemberKeycloakSyncStatus>>,
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
            keycloak_sync_cache: Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(60))
                .build(),
        }
    }

    fn invalidate_sync_cache(&self) {
        self.keycloak_sync_cache.invalidate_all();
    }

    pub async fn create_role(
        &self,
        new_role: &Role,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<Role> {
        new_role
            .validate_renewal_config()
            .map_err(ServiceError::Constraint)?;

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

        self.invalidate_sync_cache();
        Ok(role)
    }

    pub async fn update_role(
        &self,
        role: &Role,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<Role> {
        role.validate_renewal_config()
            .map_err(ServiceError::Constraint)?;

        let updated = self
            .role_repo
            .update(role)
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "role.update",
                "role",
                &updated.name.0,
                Some(serde_json::json!({
                    "role_name": &updated.name.0,
                    "renewable": updated.renewable,
                })),
            )
            .await;

        Ok(updated)
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

        self.invalidate_sync_cache();
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
                ServiceError::DatabaseError(format!("{e:?}"))
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

        self.invalidate_sync_cache();
        Ok(())
    }

    /// Upsert a role membership: (re-)assign the role in Keycloak, then insert
    /// or overwrite the DB row keyed on (user_id, role_name, valid_from).
    pub async fn upsert_role_member(
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
            .map_err(|e| ServiceError::DatabaseError(format!("{e:?}")))?;
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
            .upsert_role_member(&user_id, role_name, valid_from, valid_until)
            .await
            .map_err(ServiceError::from)?;

        self.audit_log
            .log(
                actor_user_id,
                "role_member.upsert",
                "role_member",
                &format!("{user_id}:{role_name}"),
                Some(serde_json::json!({
                    "user_id": user_id,
                    "role_name": role_name,
                    "valid_from": valid_from.to_string(),
                    "valid_until": valid_until.map(|d| d.to_string()),
                })),
            )
            .await;

        self.invalidate_sync_cache();
        Ok(())
    }

    /// Assigns a role to a member, writing the DB row first and then attempting
    /// to sync to the IdP on a best-effort basis. Unlike `add_role_member`,
    /// this does not fail the operation if the IdP sync fails — the failure is
    /// logged and the admin UI's keycloak sync status surfaces the drift so it
    /// can be reconciled later.
    ///
    /// Use this on flows where it is more important to record the domain
    /// decision (e.g. application approval) than to guarantee immediate IdP
    /// consistency.
    pub async fn add_role_member_best_effort(
        &self,
        user_id: Uuid,
        role_name: &str,
        valid_from: chrono::NaiveDate,
        valid_until: Option<chrono::NaiveDate>,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        self.role_repo
            .create_role_member(&user_id, role_name, valid_from, valid_until)
            .await
            .map_err(ServiceError::from)?;

        match self.auth_provider_repo.find_by_user_id(&user_id).await {
            Ok(providers) => {
                for provider in providers {
                    if let Err(e) = self
                        .role_sync
                        .assign_role(
                            &IdpSubject(provider.provider_user_id.clone()),
                            &RoleName(role_name.to_string()),
                        )
                        .await
                    {
                        tracing::warn!(
                            user_id = %user_id,
                            provider = %provider.provider_user_id,
                            role = %role_name,
                            "Best-effort IdP role sync failed; role will need to be reconciled later: {e:?}"
                        );
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    user_id = %user_id,
                    role = %role_name,
                    "Failed to fetch auth providers for best-effort role sync: {e:?}"
                );
            }
        }

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

        self.invalidate_sync_cache();
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
                    ServiceError::DatabaseError(format!("{e:?}"))
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

        self.invalidate_sync_cache();
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

        self.invalidate_sync_cache();
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
                ServiceError::DatabaseError(format!("{e:?}"))
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

        self.invalidate_sync_cache();
        Ok(())
    }

    pub async fn cleanup_expired_roles(&self) -> ServiceResult<u32> {
        let expired = self
            .role_repo
            .fetch_expired_unsynced()
            .await
            .map_err(ServiceError::from)?;

        let total = expired.len();
        let mut synced: u32 = 0;

        for membership in &expired {
            let providers = match self
                .auth_provider_repo
                .find_by_user_id(&membership.user_id)
                .await
            {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(
                        user_id = %membership.user_id,
                        role = %membership.role_name.0,
                        "Failed to fetch auth providers for expired role cleanup: {e:?}"
                    );
                    continue;
                }
            };

            let mut any_failed = false;
            for provider in &providers {
                if let Err(e) = self
                    .role_sync
                    .remove_role(
                        &IdpSubject(provider.provider_user_id.clone()),
                        &membership.role_name,
                    )
                    .await
                {
                    tracing::error!(
                        user_id = %membership.user_id,
                        provider = %provider.provider_user_id,
                        role = %membership.role_name.0,
                        "Failed to remove expired role from IdP: {e:?}"
                    );
                    any_failed = true;
                }
            }

            if any_failed {
                continue;
            }

            if let Err(e) = self
                .role_repo
                .mark_keycloak_synced(
                    &membership.user_id,
                    &membership.role_name.0,
                    membership.valid_from,
                )
                .await
            {
                tracing::error!(
                    user_id = %membership.user_id,
                    role = %membership.role_name.0,
                    "Failed to mark expired role as synced: {e:?}"
                );
                continue;
            }

            self.audit_log
                .log(
                    None,
                    "role_member.expired",
                    "role_member",
                    &format!("{}:{}", membership.user_id, membership.role_name.0),
                    Some(serde_json::json!({
                        "user_id": membership.user_id,
                        "role_name": membership.role_name.0,
                        "valid_from": membership.valid_from.to_string(),
                        "valid_until": membership.valid_until.map(|d| d.to_string()),
                    })),
                )
                .await;

            synced += 1;
        }

        tracing::info!(
            total_expired = total,
            synced = synced,
            "Expired role cleanup complete"
        );

        if synced > 0 {
            self.invalidate_sync_cache();
        }

        Ok(synced)
    }

    /// Adds every registry role membership that is missing in Keycloak to the
    /// corresponding Keycloak user. Registry is the source of truth; extras in
    /// Keycloak (roles KC has but registry doesn't) are NOT touched here.
    ///
    /// Best-effort: a failure on one assign_role call is logged and counted,
    /// but does not abort the run. Returns totals so the UI can surface them.
    /// When `remove_expired` is true, also removes registry-expired roles from KC.
    pub async fn sync_missing_roles_to_keycloak(
        &self,
        actor_user_id: Option<Uuid>,
        remove_expired: bool,
    ) -> ServiceResult<SyncMissingRolesSummary> {
        // Bypass cache — we want a fresh diff at the moment of sync.
        let status = self.compute_keycloak_sync_status().await?;

        let users_processed = status.len() as u32;
        let mut added: u32 = 0;
        let mut failed: u32 = 0;
        let mut removed: u32 = 0;
        let mut remove_failed: u32 = 0;

        // Pre-fetch expired unsynced rows once so we can look up valid_from when marking.
        let expired_unsynced = if remove_expired {
            self.role_repo
                .fetch_expired_unsynced()
                .await
                .map_err(ServiceError::from)?
        } else {
            vec![]
        };

        for (user_id, drift) in &status {
            let needs_add = !drift.missing_in_keycloak.is_empty();
            let needs_remove = remove_expired && !drift.expired_in_keycloak.is_empty();

            if !needs_add && !needs_remove {
                continue;
            }

            let providers = match self.auth_provider_repo.find_by_user_id(user_id).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::error!(
                        user_id = %user_id,
                        "Failed to fetch auth providers for role sync: {e:?}"
                    );
                    failed += drift.missing_in_keycloak.len() as u32;
                    if remove_expired {
                        remove_failed += drift.expired_in_keycloak.len() as u32;
                    }
                    continue;
                }
            };

            if providers.is_empty() {
                // User has no Keycloak account — nothing to do in KC.
                tracing::debug!(
                    user_id = %user_id,
                    "No auth providers; skipping {} missing role(s)",
                    drift.missing_in_keycloak.len()
                );
                continue;
            }

            // --- ADD missing roles ---
            for role_name_str in &drift.missing_in_keycloak {
                let role = RoleName(role_name_str.clone());
                let mut any_success = false;

                for provider in &providers {
                    match self
                        .role_sync
                        .assign_role(&IdpSubject(provider.provider_user_id.clone()), &role)
                        .await
                    {
                        Ok(()) => any_success = true,
                        Err(e) => tracing::error!(
                            user_id = %user_id,
                            role = %role_name_str,
                            provider = %provider.provider_user_id,
                            "assign_role failed: {e:?}"
                        ),
                    }
                }

                if any_success {
                    added += 1;
                    self.audit_log
                        .log(
                            actor_user_id,
                            "role_member.kc_sync_add",
                            "role_member",
                            &format!("{}:{}", user_id, role_name_str),
                            Some(serde_json::json!({
                                "user_id": user_id,
                                "role_name": role_name_str,
                            })),
                        )
                        .await;
                } else {
                    failed += 1;
                }
            }

            // --- REMOVE expired roles (opt-in) ---
            if remove_expired {
                for role_name_str in &drift.expired_in_keycloak {
                    let role = RoleName(role_name_str.clone());
                    let mut any_failed = false;

                    for provider in &providers {
                        if let Err(e) = self
                            .role_sync
                            .remove_role(&IdpSubject(provider.provider_user_id.clone()), &role)
                            .await
                        {
                            tracing::error!(
                                user_id = %user_id,
                                role = %role_name_str,
                                provider = %provider.provider_user_id,
                                "remove_role failed: {e:?}"
                            );
                            any_failed = true;
                        }
                    }

                    if any_failed {
                        remove_failed += 1;
                        continue;
                    }

                    // Mark all matching expired rows as KC-synced so the scheduler won't re-process them.
                    for membership in expired_unsynced
                        .iter()
                        .filter(|m| m.user_id == *user_id && m.role_name.0 == *role_name_str)
                    {
                        if let Err(e) = self
                            .role_repo
                            .mark_keycloak_synced(
                                &membership.user_id,
                                &membership.role_name.0,
                                membership.valid_from,
                            )
                            .await
                        {
                            tracing::error!(
                                user_id = %user_id,
                                role = %role_name_str,
                                "Failed to mark expired role as kc-synced: {e:?}"
                            );
                        }
                    }

                    removed += 1;
                    self.audit_log
                        .log(
                            actor_user_id,
                            "role_member.kc_sync_remove",
                            "role_member",
                            &format!("{}:{}", user_id, role_name_str),
                            Some(serde_json::json!({
                                "user_id": user_id,
                                "role_name": role_name_str,
                            })),
                        )
                        .await;
                }
            }
        }

        tracing::info!(
            users_processed,
            added,
            failed,
            removed,
            remove_failed,
            "Keycloak role sync complete"
        );

        if added > 0 || removed > 0 {
            self.invalidate_sync_cache();
        }

        Ok(SyncMissingRolesSummary {
            added,
            failed,
            removed,
            remove_failed,
            users_processed,
        })
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

    pub async fn get_keycloak_sync_status(
        &self,
    ) -> ServiceResult<HashMap<Uuid, MemberKeycloakSyncStatus>> {
        const CACHE_KEY: &str = "sync";

        if let Some(cached) = self.keycloak_sync_cache.get(CACHE_KEY).await {
            return Ok(cached);
        }

        let status = self.compute_keycloak_sync_status().await?;
        self.keycloak_sync_cache
            .insert(CACHE_KEY.to_string(), status.clone())
            .await;
        Ok(status)
    }

    async fn compute_keycloak_sync_status(
        &self,
    ) -> ServiceResult<HashMap<Uuid, MemberKeycloakSyncStatus>> {
        // 1. Get all app-managed roles
        let roles = self
            .role_repo
            .fetch_all()
            .await
            .map_err(ServiceError::from)?;

        // 2. For each role, get KC subjects that have it
        let mut kc_roles_by_subject: HashMap<String, HashSet<String>> = HashMap::new();
        for role in &roles {
            match self.role_sync.list_role_members(&role.name).await {
                Ok(subjects) => {
                    for subject in subjects {
                        kc_roles_by_subject
                            .entry(subject.0)
                            .or_default()
                            .insert(role.name.0.clone());
                    }
                }
                Err(e) => {
                    tracing::error!(role = %role.name.0, "Failed to list KC role members: {e:?}");
                }
            }
        }

        // 3. Get all auth provider mappings (subject → user_id)
        let providers = self
            .auth_provider_repo
            .find_all_by_provider_name("keycloak")
            .await
            .map_err(|e| ServiceError::DatabaseError(format!("{e:?}")))?;

        let subject_to_user: HashMap<String, Uuid> = providers
            .iter()
            .map(|p| (p.provider_user_id.clone(), p.user_id))
            .collect();

        // 4. Build KC roles by user_id
        let mut kc_roles_by_user: HashMap<Uuid, HashSet<String>> = HashMap::new();
        for (subject, roles) in &kc_roles_by_subject {
            if let Some(&user_id) = subject_to_user.get(subject) {
                kc_roles_by_user.insert(user_id, roles.clone());
            }
        }

        let today = chrono::Utc::now().date_naive();
        let role_names: Vec<String> = roles.iter().map(|r| r.name.0.clone()).collect();

        // 5a. Active registry role memberships (valid today).
        let active_members_with_roles = self
            .member_service
            .get_members_with_roles(
                None,
                None,
                Some(role_names.clone()),
                None,
                None,
                None,
                Some(today),
                Some(today),
            )
            .await?;

        // 5b. All registry role memberships regardless of validity (to detect expired records).
        let all_members_with_roles = self
            .member_service
            .get_members_with_roles(None, None, Some(role_names), None, None, None, None, None)
            .await?;

        // 5c. All members (no role filter) — to enumerate every known user_id.
        let all_members = self
            .member_service
            .get_members_with_roles(None, None, None, None, None, None, None, None)
            .await?;

        let mut active_registry_roles_by_user: HashMap<Uuid, HashSet<String>> = HashMap::new();
        for mwr in &active_members_with_roles {
            active_registry_roles_by_user
                .insert(mwr.person.id.0, mwr.role_names.iter().cloned().collect());
        }

        let mut all_registry_roles_by_user: HashMap<Uuid, HashSet<String>> = HashMap::new();
        for mwr in &all_members_with_roles {
            all_registry_roles_by_user
                .insert(mwr.person.id.0, mwr.role_names.iter().cloned().collect());
        }

        // 6. Collect all user_ids we know about, and build identity lookup
        let person_by_user_id: HashMap<Uuid, &crate::domain::Person> = all_members
            .iter()
            .map(|m| (m.person.id.0, &m.person))
            .collect();
        let all_user_ids: HashSet<Uuid> = person_by_user_id.keys().copied().collect();

        // 7. Compare
        let mut result = HashMap::new();
        for user_id in all_user_ids {
            let kc_roles = kc_roles_by_user.get(&user_id).cloned().unwrap_or_default();
            let active_roles = active_registry_roles_by_user
                .get(&user_id)
                .cloned()
                .unwrap_or_default();
            let all_roles = all_registry_roles_by_user
                .get(&user_id)
                .cloned()
                .unwrap_or_default();

            let missing_in_keycloak: Vec<String> =
                active_roles.difference(&kc_roles).cloned().collect();

            // Roles KC has that the registry knows about but are expired.
            let expired_roles: HashSet<String> =
                all_roles.difference(&active_roles).cloned().collect();
            let expired_in_keycloak: Vec<String> =
                expired_roles.intersection(&kc_roles).cloned().collect();

            // Roles KC has with no registry record at all.
            let unmanaged_in_keycloak: Vec<String> =
                kc_roles.difference(&all_roles).cloned().collect();

            let in_sync = missing_in_keycloak.is_empty()
                && expired_in_keycloak.is_empty()
                && unmanaged_in_keycloak.is_empty();

            if !in_sync {
                let person = person_by_user_id[&user_id];
                result.insert(
                    user_id,
                    MemberKeycloakSyncStatus {
                        in_sync,
                        first_name: person.first_name.clone(),
                        last_name: person.last_name.clone(),
                        email: person.email.clone().into_inner(),
                        expired_in_keycloak,
                        unmanaged_in_keycloak,
                        missing_in_keycloak,
                    },
                );
            }
        }

        Ok(result)
    }
}
