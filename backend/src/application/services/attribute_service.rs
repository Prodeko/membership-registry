use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::application::ports::{
    attribute_repository_port::{
        AttributeRepositoryPort, CreateAttributeDefinition, UpdateAttributeDefinition,
    },
    attribute_sync_port::{AttributeSyncError, AttributeSyncPort},
    auth_provider_repo_port::AuthProviderRepositoryPort,
};
use crate::domain::{
    AttributeDefinition, AttributeName, AttributeValidationError, AttributeValue, DriftEntry,
    EditableBy, IdpSubject, MemberAttribute, Patch, PersonId, SyncStatus,
};

use super::{
    audit_log_service::AuditLogService,
    errors::{ServiceError, ServiceResult},
};

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SyncMissingAttributesSummary {
    pub applied: u32,
    pub failed: u32,
}

/// Service-level patch for `update_definition`: each field carries explicit
/// "leave" / "set" / "clear" intent, distinct from the resolved values the
/// repository writes.
#[derive(Debug, Clone, Default)]
pub struct UpdateAttributeDefinitionPatch {
    pub description: Patch<String>,
    pub allowed_values: Patch<Vec<AttributeValue>>,
    pub sync_to_keycloak: Option<bool>,
    pub editable_by: Option<EditableBy>,
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AttributeService {
    pub repo: Arc<dyn AttributeRepositoryPort>,
    pub sync: Arc<dyn AttributeSyncPort>,
    pub auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
    pub audit_log: AuditLogService,
    sync_status_cache: Cache<String, SyncStatus>,
}

impl AttributeService {
    pub fn new(
        repo: Arc<dyn AttributeRepositoryPort>,
        sync: Arc<dyn AttributeSyncPort>,
        auth_provider_repo: Arc<dyn AuthProviderRepositoryPort>,
        audit_log: AuditLogService,
    ) -> Self {
        Self {
            repo,
            sync,
            auth_provider_repo,
            audit_log,
            sync_status_cache: Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(60))
                .build(),
        }
    }

    fn invalidate_sync_cache(&self) {
        self.sync_status_cache.invalidate_all();
    }

    // -----------------------------------------------------------------------
    // Catalog CRUD (admin only — auth enforced at HTTP layer)
    // -----------------------------------------------------------------------

    pub async fn create_definition(
        &self,
        input: CreateAttributeDefinition,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<AttributeDefinition> {
        if input.sync_to_keycloak {
            self.sync
                .add_mapper_to_scope(&input.name)
                .await
                .map_err(map_sync_err)?;
        }
        let name_str = input.name.as_str().to_string();
        let editable_by = input.editable_by;
        let sync_flag = input.sync_to_keycloak;
        let created = self.repo.create_definition(input).await?;

        self.audit_log
            .log(
                actor_user_id,
                "attribute_definition.create",
                "attribute_definition",
                &name_str,
                Some(serde_json::json!({
                    "sync_to_keycloak": sync_flag,
                    "editable_by": editable_by.as_str(),
                })),
            )
            .await;

        Ok(created)
    }

    pub async fn update_definition(
        &self,
        name: &AttributeName,
        patch: UpdateAttributeDefinitionPatch,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<AttributeDefinition> {
        let existing = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;

        // Resolve patches against the existing row.
        let description = patch
            .description
            .apply(existing.description().map(str::to_string));
        let allowed_values = patch
            .allowed_values
            .apply(existing.allowed_values().map(<[AttributeValue]>::to_vec));
        let sync_to_keycloak = patch.sync_to_keycloak.unwrap_or(existing.sync_to_keycloak());
        let editable_by = patch.editable_by.unwrap_or(existing.editable_by());

        // Reject Some(empty) at the boundary so the domain invariant holds.
        if matches!(&allowed_values, Some(v) if v.is_empty()) {
            return Err(ServiceError::Constraint(
                "allowed_values must be non-empty (or null to clear)".to_string(),
            ));
        }

        // If allowed_values is being tightened, verify no existing user value is now invalid.
        if let Some(allowed) = &allowed_values {
            let allowed_set: HashSet<&str> =
                allowed.iter().map(AttributeValue::as_str).collect();
            let rows = self.repo.fetch_all_values_for(name).await?;
            for (uid, val) in rows {
                if !allowed_set.contains(val.as_str()) {
                    return Err(ServiceError::Constraint(format!(
                        "user {} has value {:?} which is not in allowed_values",
                        uid,
                        val.as_str()
                    )));
                }
            }
        }

        // Sync-to-keycloak transitions: add or remove the mapper.
        if sync_to_keycloak && !existing.sync_to_keycloak() {
            self.sync
                .add_mapper_to_scope(name)
                .await
                .map_err(map_sync_err)?;
        }
        if !sync_to_keycloak && existing.sync_to_keycloak() {
            self.sync
                .remove_mapper_from_scope(name)
                .await
                .map_err(map_sync_err)?;
        }

        let resolved = UpdateAttributeDefinition {
            description,
            allowed_values,
            sync_to_keycloak,
            editable_by,
        };
        let updated = self.repo.update_definition(name, resolved).await?;

        self.audit_log
            .log(
                actor_user_id,
                "attribute_definition.update",
                "attribute_definition",
                name.as_str(),
                Some(serde_json::json!({
                    "sync_to_keycloak": sync_to_keycloak,
                    "editable_by": editable_by.as_str(),
                })),
            )
            .await;

        self.invalidate_sync_cache();
        Ok(updated)
    }

    pub async fn delete_definition(
        &self,
        name: &AttributeName,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        let existing = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;

        if existing.sync_to_keycloak() {
            self.sync
                .remove_mapper_from_scope(name)
                .await
                .map_err(map_sync_err)?;
        }
        self.repo.delete_definition(name).await?;
        self.audit_log
            .log(
                actor_user_id,
                "attribute_definition.delete",
                "attribute_definition",
                name.as_str(),
                None,
            )
            .await;
        self.invalidate_sync_cache();
        Ok(())
    }

    pub async fn list_definitions(&self) -> ServiceResult<Vec<AttributeDefinition>> {
        Ok(self.repo.fetch_all_definitions().await?)
    }

    pub async fn get_definition(
        &self,
        name: &AttributeName,
    ) -> ServiceResult<Option<AttributeDefinition>> {
        Ok(self.repo.fetch_definition(name).await?)
    }

    // -----------------------------------------------------------------------
    // Per-member values
    // -----------------------------------------------------------------------

    /// Set a member's value when an admin is acting. Rejects if the
    /// definition is `editable_by = User`.
    pub async fn set_as_admin(
        &self,
        user_id: Uuid,
        name: &AttributeName,
        value: AttributeValue,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        let def = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;
        if def.editable_by() == EditableBy::User {
            return Err(ServiceError::Forbidden);
        }
        self.set_inner(&def, user_id, &value, actor_user_id, "admin")
            .await
    }

    /// Set a member's own value. Rejects if `editable_by = Admin`.
    pub async fn set_as_self(
        &self,
        user_id: Uuid,
        name: &AttributeName,
        value: AttributeValue,
    ) -> ServiceResult<()> {
        let def = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;
        if def.editable_by() == EditableBy::Admin {
            return Err(ServiceError::Forbidden);
        }
        self.set_inner(&def, user_id, &value, Some(user_id), "self")
            .await
    }

    pub async fn clear_as_admin(
        &self,
        user_id: Uuid,
        name: &AttributeName,
        actor_user_id: Option<Uuid>,
    ) -> ServiceResult<()> {
        let def = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;
        if def.editable_by() == EditableBy::User {
            return Err(ServiceError::Forbidden);
        }
        self.clear_inner(&def, user_id, actor_user_id, "admin")
            .await
    }

    pub async fn clear_as_self(&self, user_id: Uuid, name: &AttributeName) -> ServiceResult<()> {
        let def = self
            .repo
            .fetch_definition(name)
            .await?
            .ok_or(ServiceError::NotFound)?;
        if def.editable_by() == EditableBy::Admin {
            return Err(ServiceError::Forbidden);
        }
        self.clear_inner(&def, user_id, Some(user_id), "self").await
    }

    pub async fn fetch_for_member(&self, user_id: Uuid) -> ServiceResult<Vec<MemberAttribute>> {
        Ok(self.repo.fetch_member_values(&user_id).await?)
    }

    async fn set_inner(
        &self,
        def: &AttributeDefinition,
        user_id: Uuid,
        value: &AttributeValue,
        actor_user_id: Option<Uuid>,
        actor_kind: &'static str,
    ) -> ServiceResult<()> {
        if let Err(AttributeValidationError::NotInAllowedValues) = def.validate(value) {
            return Err(ServiceError::Constraint(format!(
                "value {:?} is not in allowed_values",
                value.as_str()
            )));
        }

        if def.sync_to_keycloak() {
            let providers = self
                .auth_provider_repo
                .find_by_user_id(&user_id)
                .await
                .map_err(|e| ServiceError::DatabaseError(format!("{e:?}")))?;
            for p in &providers {
                self.sync
                    .set_user_attribute(&IdpSubject(p.provider_user_id.clone()), def.name(), value)
                    .await
                    .map_err(map_sync_err)?;
            }
        }

        self.repo
            .upsert_member_value(&user_id, def.name(), value)
            .await?;

        self.audit_log
            .log(
                actor_user_id,
                "member_attribute.set",
                "member_attribute",
                &format!("{}:{}", user_id, def.name().as_str()),
                Some(serde_json::json!({
                    "actor_kind": actor_kind,
                    "value": value.as_str(),
                })),
            )
            .await;

        self.invalidate_sync_cache();
        Ok(())
    }

    async fn clear_inner(
        &self,
        def: &AttributeDefinition,
        user_id: Uuid,
        actor_user_id: Option<Uuid>,
        actor_kind: &'static str,
    ) -> ServiceResult<()> {
        if def.sync_to_keycloak() {
            let providers = self
                .auth_provider_repo
                .find_by_user_id(&user_id)
                .await
                .map_err(|e| ServiceError::DatabaseError(format!("{e:?}")))?;
            for p in &providers {
                self.sync
                    .clear_user_attribute(&IdpSubject(p.provider_user_id.clone()), def.name())
                    .await
                    .map_err(map_sync_err)?;
            }
        }

        self.repo.delete_member_value(&user_id, def.name()).await?;

        self.audit_log
            .log(
                actor_user_id,
                "member_attribute.clear",
                "member_attribute",
                &format!("{}:{}", user_id, def.name().as_str()),
                Some(serde_json::json!({ "actor_kind": actor_kind })),
            )
            .await;

        self.invalidate_sync_cache();
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Drift detection & reconciliation
    // -----------------------------------------------------------------------

    pub async fn get_keycloak_sync_status(&self) -> ServiceResult<SyncStatus> {
        if let Some(cached) = self.sync_status_cache.get(&String::new()).await {
            return Ok(cached);
        }
        let status = self.compute_sync_status().await?;
        self.sync_status_cache
            .insert(String::new(), status.clone())
            .await;
        Ok(status)
    }

    async fn compute_sync_status(&self) -> ServiceResult<SyncStatus> {
        let defs = self.repo.fetch_all_definitions().await?;
        let synced_defs: Vec<_> = defs.iter().filter(|d| d.sync_to_keycloak()).collect();
        let mut entries: Vec<DriftEntry> = Vec::new();

        if synced_defs.is_empty() {
            return Ok(SyncStatus { entries });
        }

        // One paginated KC user listing covers all synced definitions.
        let synced_names: Vec<AttributeName> =
            synced_defs.iter().map(|d| d.name().clone()).collect();
        let kc_users = self
            .sync
            .list_users_with_attributes(&synced_names)
            .await
            .map_err(map_sync_err)?;
        // subject -> (attribute_name -> value)
        let kc_map: HashMap<String, HashMap<String, AttributeValue>> = kc_users
            .into_iter()
            .map(|(subj, attrs)| (subj.0, attrs))
            .collect();

        for def in synced_defs {
            let registry_rows = self.repo.fetch_all_values_for(def.name()).await?;

            // Map registry user_ids → IdP subjects for diffing.
            let mut registry_by_kc: HashMap<String, (Uuid, AttributeValue)> = HashMap::new();
            for (uid, val) in &registry_rows {
                let providers = self
                    .auth_provider_repo
                    .find_by_user_id(uid)
                    .await
                    .map_err(|e| ServiceError::DatabaseError(format!("{e:?}")))?;
                for p in providers {
                    registry_by_kc
                        .insert(p.provider_user_id.clone(), (*uid, val.clone()));
                }
            }

            for (kc_id, (uid, reg_val)) in &registry_by_kc {
                let kc_val = kc_map
                    .get(kc_id)
                    .and_then(|attrs| attrs.get(def.name().as_str()));
                match kc_val {
                    None => entries.push(DriftEntry::RegistryOnly {
                        user_id: PersonId(*uid),
                        idp_subject: IdpSubject(kc_id.clone()),
                        attribute: def.name().clone(),
                        value: reg_val.clone(),
                    }),
                    Some(v) if v != reg_val => entries.push(DriftEntry::ValueMismatch {
                        user_id: PersonId(*uid),
                        idp_subject: IdpSubject(kc_id.clone()),
                        attribute: def.name().clone(),
                        registry_value: reg_val.clone(),
                        keycloak_value: v.clone(),
                    }),
                    _ => {}
                }
            }
            for (kc_id, attrs) in &kc_map {
                if let Some(v) = attrs.get(def.name().as_str()) {
                    if !registry_by_kc.contains_key(kc_id) {
                        entries.push(DriftEntry::KeycloakOnly {
                            idp_subject: IdpSubject(kc_id.clone()),
                            attribute: def.name().clone(),
                            value: v.clone(),
                        });
                    }
                }
            }
        }

        Ok(SyncStatus { entries })
    }

    pub async fn sync_missing_to_keycloak(&self) -> ServiceResult<SyncMissingAttributesSummary> {
        let status = self.compute_sync_status().await?;
        let mut applied = 0u32;
        let mut failed = 0u32;

        for entry in &status.entries {
            match entry {
                DriftEntry::RegistryOnly {
                    user_id,
                    attribute,
                    value,
                    ..
                }
                | DriftEntry::ValueMismatch {
                    user_id,
                    attribute,
                    registry_value: value,
                    ..
                } => {
                    if self.push_one(user_id.0, attribute, value).await {
                        applied += 1;
                    } else {
                        failed += 1;
                    }
                }
                DriftEntry::KeycloakOnly { .. } => {
                    // KC-only entries are not "missing from Keycloak"; this
                    // function only pushes registry → KC. KC-only cleanup is
                    // a separate operation.
                }
            }
        }

        self.invalidate_sync_cache();
        Ok(SyncMissingAttributesSummary { applied, failed })
    }

    async fn push_one(&self, user_id: Uuid, name: &AttributeName, value: &AttributeValue) -> bool {
        let providers = match self.auth_provider_repo.find_by_user_id(&user_id).await {
            Ok(ps) => ps,
            Err(_) => return false,
        };
        let mut all_ok = !providers.is_empty();
        for p in providers {
            if self
                .sync
                .set_user_attribute(&IdpSubject(p.provider_user_id), name, value)
                .await
                .is_err()
            {
                all_ok = false;
            }
        }
        all_ok
    }
}

fn map_sync_err(e: AttributeSyncError) -> ServiceError {
    match e {
        AttributeSyncError::Unavailable => ServiceError::IdpError,
        AttributeSyncError::ScopeMissing => ServiceError::DatabaseError(
            "Keycloak client scope `registry-attributes` is missing — add it to the realm config"
                .to_string(),
        ),
        AttributeSyncError::Unexpected(s) => ServiceError::DatabaseError(s),
    }
}
